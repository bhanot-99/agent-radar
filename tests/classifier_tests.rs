use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::thread;

use agent_radar::classifier::rules::{evaluate_rules, is_checkpoint_path_context, RuleVerdict};
use agent_radar::classifier::SemanticClassifier;
use agent_radar::correlator::EnrichedEvent;
use agent_radar::events::{ActivityCategory, FileOp};
use agent_radar::watcher::inotify_tier::RawFsEvent;
use inotify::EventMask;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!("agent_radar_class_{}_{}", prefix, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn make_dummy_event(path: PathBuf, mask: EventMask, cookie: u32, network: bool) -> EnrichedEvent {
    make_dummy_event_with_git(path, mask, cookie, network, false)
}

fn make_dummy_event_with_git(path: PathBuf, mask: EventMask, cookie: u32, network: bool, is_git: bool) -> EnrichedEvent {
    let dir = path.parent().unwrap_or(Path::new("/tmp")).to_path_buf();
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let inotify = inotify::Inotify::init().unwrap();
    let wd = inotify.watches().add(&dir, inotify::WatchMask::CREATE).unwrap();

    EnrichedEvent {
        raw: RawFsEvent {
            wd,
            mask,
            cookie,
            name,
            dir_path: dir,
            full_path: Some(path),
        },
        timestamp: SystemTime::now(),
        pid: Some(1234),
        ppid: Some(100),
        process_name: if is_git { "git".to_string() } else { "claude-code".to_string() },
        is_ai_agent: !is_git,
        agent_ancestor: if is_git { None } else { Some("claude-code".to_string()) },
        active_network_stream: network,
        local_disk_mutation: !network,
        is_git_process: is_git,
    }
}

#[test]
fn test_checkpoint_detection() {
    let temp = TempDir::new("ckpt");
    let model_path = temp.path.join("model.safetensors");
    {
        let mut f = File::create(&model_path).unwrap();
        f.write_all(&vec![0u8; 1024 * 1024]).unwrap(); // 1MB
    }

    let verdict = evaluate_rules(&model_path, false, FileOp::Modified, false, false, Path::new(""));
    match verdict {
        RuleVerdict::ModelTrainingCheckpoint { size_bytes } => {
            assert_eq!(size_bytes, 1024 * 1024);
        }
        other => panic!("Expected ModelTrainingCheckpoint, got {:?}", other),
    }
}

#[test]
fn test_bin_disambiguation_with_context() {
    let temp = TempDir::new("bin_disambig");

    // Case 1: In a directory named "models"
    let models_dir = temp.path.join("models");
    fs::create_dir(&models_dir).unwrap();
    let shard_bin = models_dir.join("pytorch_model-00001.bin");
    fs::write(&shard_bin, b"weights data").unwrap();

    assert!(is_checkpoint_path_context(&shard_bin, Path::new("")));
    let verdict1 = evaluate_rules(&shard_bin, false, FileOp::Modified, false, false, Path::new(""));
    assert!(matches!(verdict1, RuleVerdict::ModelTrainingCheckpoint { .. }));

    // Case 2: In a directory with sibling *.index.json
    let sibling_dir = temp.path.join("custom_weights");
    fs::create_dir(&sibling_dir).unwrap();
    fs::write(sibling_dir.join("model.safetensors.index.json"), b"{}").unwrap();
    let sibling_bin = sibling_dir.join("data.bin");
    fs::write(&sibling_bin, b"tensor data").unwrap();

    assert!(is_checkpoint_path_context(&sibling_bin, Path::new("")));
    let verdict2 = evaluate_rules(&sibling_bin, false, FileOp::Modified, false, false, Path::new(""));
    assert!(matches!(verdict2, RuleVerdict::ModelTrainingCheckpoint { .. }));

    // Case 3: Plain .bin without checkpoint context, with active_network_stream
    let plain_dir = temp.path.join("downloads");
    fs::create_dir(&plain_dir).unwrap();
    let plain_bin = plain_dir.join("firmware.bin");
    fs::write(&plain_bin, b"firmware").unwrap();

    assert!(!is_checkpoint_path_context(&plain_bin, Path::new("")));
    let verdict3 = evaluate_rules(&plain_bin, false, FileOp::Modified, true, false, Path::new(""));
    assert_eq!(verdict3, RuleVerdict::IncomingDataStream);

    // Case 4: Plain .bin without network stream -> FileMutation
    let verdict4 = evaluate_rules(&plain_bin, false, FileOp::Modified, false, false, Path::new(""));
    assert_eq!(verdict4, RuleVerdict::FileMutation(FileOp::Modified));
}

#[test]
fn test_dataset_stream_vs_local_mutation() {
    let temp = TempDir::new("dataset");
    let parquet_path = temp.path.join("dataset.parquet");
    fs::write(&parquet_path, b"parquet data").unwrap();

    let stream_verdict = evaluate_rules(&parquet_path, false, FileOp::Modified, true, false, Path::new(""));
    assert_eq!(stream_verdict, RuleVerdict::IncomingDataStream);

    // Without active network stream, archive/dataset formats classify as ArchiveWrite
    let local_verdict = evaluate_rules(&parquet_path, false, FileOp::Modified, false, false, Path::new(""));
    assert_eq!(local_verdict, RuleVerdict::ArchiveWrite);
}

#[test]
fn test_workspace_expansion() {
    let temp = TempDir::new("workspace");
    let new_dir = temp.path.join("new_feature");
    let verdict = evaluate_rules(&new_dir, true, FileOp::Created, false, false, Path::new(""));
    assert_eq!(verdict, RuleVerdict::WorkspaceExpansion);
}

#[test]
fn test_rust_edit_and_line_diff() {
    let temp = TempDir::new("source");
    let code_file = temp.path.join("main.rs");
    let mut classifier = SemanticClassifier::new(PathBuf::new());

    // 1. Initial 5 lines
    fs::write(&code_file, "1\n2\n3\n4\n5\n").unwrap();
    let ev1 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te1 = classifier.classify_direct(&ev1, &code_file, false, FileOp::Created);

    match te1.category {
        ActivityCategory::RustEdit { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 5);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected RustEdit, got {:?}", other),
    }

    // 2. Add 3 more lines (8 total)
    fs::write(&code_file, "1\n2\n3\n4\n5\n6\n7\n8\n").unwrap();
    let ev2 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te2 = classifier.classify_direct(&ev2, &code_file, false, FileOp::Modified);

    match te2.category {
        ActivityCategory::RustEdit { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 3);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected RustEdit, got {:?}", other),
    }

    // 3. Remove lines (shrink to 2 lines)
    fs::write(&code_file, "1\n2\n").unwrap();
    let ev3 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te3 = classifier.classify_direct(&ev3, &code_file, false, FileOp::Modified);

    match te3.category {
        ActivityCategory::RustEdit { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 0);
            assert_eq!(lines_removed, 6);
        }
        other => panic!("Expected RustEdit, got {:?}", other),
    }
}

#[test]
fn test_rename_cookie_pairing() {
    let temp = TempDir::new("ren_pair");
    let from_file = temp.path.join("old.dat");
    let to_file = temp.path.join("new.dat");
    fs::write(&to_file, "atomic save content").unwrap();

    let mut classifier = SemanticClassifier::new(PathBuf::new());
    let cookie = 999;

    let ev_from = make_dummy_event(from_file, EventMask::MOVED_FROM, cookie, false);
    let opt1 = classifier.push_event(ev_from);
    assert!(opt1.is_none(), "MOVED_FROM should wait for paired MOVED_TO");

    let ev_to = make_dummy_event(to_file, EventMask::MOVED_TO, cookie, false);
    let opt2 = classifier.push_event(ev_to);
    assert!(opt2.is_some(), "MOVED_TO should complete atomic rename pair");

    let te = opt2.unwrap();
    assert!(matches!(te.category, ActivityCategory::FileMutation { op: FileOp::Renamed, .. }));
}

#[test]
fn test_debounce_coalescing() {
    let temp = TempDir::new("debounce");
    let file = temp.path.join("rapid.rs");
    fs::write(&file, "code\n").unwrap();

    let mut classifier = SemanticClassifier::new(PathBuf::new());
    let ev1 = make_dummy_event(file.clone(), EventMask::MODIFY, 0, false);
    let opt = classifier.push_event(ev1);
    assert!(opt.is_none(), "Event should be in debounce window");

    // Flushed immediately with 0ms window -> should be ready
    thread::sleep(Duration::from_millis(50));
    let flushed = classifier.flush_ready(Duration::from_millis(40));
    assert_eq!(flushed.len(), 1, "Expected 1 flushed event");
    assert!(matches!(flushed[0].category, ActivityCategory::RustEdit { .. }));
}

#[test]
fn test_unpaired_moved_from_emits_deletion() {
    let temp = TempDir::new("unpaired_move");
    let from_file = temp.path.join("vanished.rs");

    let mut classifier = SemanticClassifier::new(PathBuf::new());
    let cookie = 12345;

    let ev_from = make_dummy_event(from_file.clone(), EventMask::MOVED_FROM, cookie, false);
    let opt = classifier.push_event(ev_from);
    assert!(opt.is_none(), "MOVED_FROM initially waits in pending_renames");

    // Sleep > 1s for rename expiration
    thread::sleep(Duration::from_millis(1050));

    let flushed = classifier.flush_ready(Duration::from_millis(100));
    assert_eq!(flushed.len(), 1, "Unpaired MOVED_FROM must emit a deletion event");
    match &flushed[0].category {
        ActivityCategory::RustEdit { lines_added, lines_removed: _, path } => {
            assert_eq!(*lines_added, 0);
            assert_eq!(path, &from_file.to_string_lossy().into_owned());
        }
        ActivityCategory::FileMutation { op, path } => {
            assert_eq!(*op, FileOp::Deleted);
            assert_eq!(path, &from_file.to_string_lossy().into_owned());
        }
        other => panic!("Expected Deleted event, got {:?}", other),
    }
}

#[test]
fn test_rename_preserves_line_count_no_false_added_lines() {
    let temp = TempDir::new("rename_diff");
    let from_file = temp.path.join("original.rs");
    let to_file = temp.path.join("renamed.rs");

    // Write 50 lines to original file
    let content = "println!(\"hello\");\n".repeat(50);
    fs::write(&from_file, &content).unwrap();

    let mut classifier = SemanticClassifier::new(PathBuf::new());

    // 1. Initial write establishes line count of 50
    let ev_init = make_dummy_event(from_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te_init = classifier.classify_direct(&ev_init, &from_file, false, FileOp::Created);
    match te_init.category {
        ActivityCategory::RustEdit { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 50);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected RustEdit, got {:?}", other),
    }

    // 2. Rename original.rs to renamed.rs (same 50 lines)
    fs::rename(&from_file, &to_file).unwrap();
    let cookie = 777;
    let ev_from = make_dummy_event(from_file, EventMask::MOVED_FROM, cookie, false);
    assert!(classifier.push_event(ev_from).is_none());

    let ev_to = make_dummy_event(to_file, EventMask::MOVED_TO, cookie, false);
    let te_renamed = classifier.push_event(ev_to).expect("Paired rename should emit event");

    // 3. Renamed file has IDENTICAL lines -> must report 0 added, 0 removed! (Finding 9)
    match te_renamed.category {
        ActivityCategory::RustEdit { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 0, "Pure rename should not report added lines");
            assert_eq!(lines_removed, 0, "Pure rename should not report removed lines");
        }
        other => panic!("Expected RustEdit, got {:?}", other),
    }
}

#[test]
fn test_incoming_data_stream_progress_percentage() {
    let temp = TempDir::new("stream_prog");
    let file_path = temp.path.join("large_dataset.parquet");
    let meta_path = temp.path.join("large_dataset.parquet.size");

    // Total expected size is 1,000,000 bytes
    fs::write(&meta_path, "1000000").unwrap();
    // Current written size is 250,000 bytes (25%)
    let data = vec![0u8; 250_000];
    fs::write(&file_path, &data).unwrap();

    let mut classifier = SemanticClassifier::new(PathBuf::new());
    let ev = make_dummy_event(file_path.clone(), EventMask::MODIFY, 0, true);
    let te = classifier.classify_direct(&ev, &file_path, false, FileOp::Modified);

    match te.category {
        ActivityCategory::IncomingDataStream { progress_pct, .. } => {
            assert_eq!(progress_pct, Some(25), "Expected progress_pct to be 25%");
        }
        other => panic!("Expected IncomingDataStream, got {:?}", other),
    }

    // Now update file to 750,000 bytes total (75%)
    let data75 = vec![0u8; 750_000];
    fs::write(&file_path, &data75).unwrap();

    let ev2 = make_dummy_event(file_path.clone(), EventMask::MODIFY, 0, true);
    let te2 = classifier.classify_direct(&ev2, &file_path, false, FileOp::Modified);

    match te2.category {
        ActivityCategory::IncomingDataStream { progress_pct, .. } => {
            assert_eq!(progress_pct, Some(75), "Expected progress_pct to be 75%");
        }
        other => panic!("Expected IncomingDataStream, got {:?}", other),
    }
}

#[test]
fn test_incoming_data_stream_sparse_preallocation_progress() {
    use std::io::{Seek, SeekFrom};
    let temp = TempDir::new("stream_sparse");
    let file_path = temp.path.join("sparse_download.parquet");

    // Pre-allocate a 10MB sparse download file and write 1MB into it
    let mut f = fs::File::create(&file_path).unwrap();
    f.seek(SeekFrom::Start(10_000_000 - 1)).unwrap();
    f.write_all(b"\0").unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    f.write_all(&vec![b'x'; 1_000_000]).unwrap();
    f.flush().unwrap();

    let mut classifier = SemanticClassifier::new(PathBuf::new());
    let ev = make_dummy_event(file_path.clone(), EventMask::MODIFY, 0, true);
    let te = classifier.classify_direct(&ev, &file_path, false, FileOp::Modified);

    match te.category {
        ActivityCategory::IncomingDataStream { progress_pct, .. } => {
            assert!(progress_pct.is_some(), "Progress percentage must be detected for sparse pre-allocation");
            let pct = progress_pct.unwrap();
            // 1MB out of 10MB is approximately 10%
            assert!((9..=11).contains(&pct), "Expected progress around 10%, got {}%", pct);
        }
        other => panic!("Expected IncomingDataStream, got {:?}", other),
    }
}

#[test]
fn test_rule_priority_order() {
    // 1. Directory creation beats everything
    let dir = Path::new("tests/my_dir");
    assert_eq!(
        evaluate_rules(dir, true, FileOp::Created, false, false, Path::new("")),
        RuleVerdict::WorkspaceExpansion
    );

    // 2. Exact match: EnvSecretChange
    assert_eq!(
        evaluate_rules(Path::new(".env"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::EnvSecretChange
    );
    assert_eq!(
        evaluate_rules(Path::new(".env.production"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::EnvSecretChange
    );

    // 2. Exact match: ContainerConfigEdit
    assert_eq!(
        evaluate_rules(Path::new("Dockerfile"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::ContainerConfigEdit
    );
    assert_eq!(
        evaluate_rules(Path::new("Dockerfile.prod"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::ContainerConfigEdit
    );
    assert_eq!(
        evaluate_rules(Path::new("docker-compose.yml"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::ContainerConfigEdit
    );

    // 2. Exact match: DependencyLockUpdate
    assert_eq!(
        evaluate_rules(Path::new("Cargo.lock"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::DependencyLockUpdate
    );
    assert_eq!(
        evaluate_rules(Path::new("package-lock.json"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::DependencyLockUpdate
    );

    // 2. Exact match: CiPipelineEdit
    assert_eq!(
        evaluate_rules(Path::new(".gitlab-ci.yml"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::CiPipelineEdit
    );
    assert_eq!(
        evaluate_rules(Path::new(".github/workflows/deploy.yml"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::CiPipelineEdit
    );

    // 3. Test pattern beats per-language code edit
    assert_eq!(
        evaluate_rules(Path::new("tests/test_service.py"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::TestFileActivity,
        "tests/test_service.py must evaluate to TestFileActivity, not PythonEdit"
    );
    assert_eq!(
        evaluate_rules(Path::new("src/auth_test.rs"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::TestFileActivity,
        "src/auth_test.rs must evaluate to TestFileActivity, not RustEdit"
    );
    assert_eq!(
        evaluate_rules(Path::new("client/app.spec.ts"), false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::TestFileActivity,
        "client/app.spec.ts must evaluate to TestFileActivity, not WebEdit"
    );

    // 4. Git process attribution beats code edit
    assert_eq!(
        evaluate_rules(Path::new("src/main.rs"), false, FileOp::Modified, false, true, Path::new("")),
        RuleVerdict::GitOperation,
        "Git-attributed write to main.rs must evaluate to GitOperation, not RustEdit"
    );

    // 7. ModelConfigEdit beats generic ConfigEdit
    let temp = TempDir::new("model_cfg");
    let model_dir = temp.path.join("weights");
    fs::create_dir(&model_dir).unwrap();
    let cfg_json = model_dir.join("config.json");
    fs::write(&cfg_json, b"{}").unwrap();
    assert_eq!(
        evaluate_rules(&cfg_json, false, FileOp::Modified, false, false, Path::new("")),
        RuleVerdict::ModelConfigEdit,
        "config.json in model directory must evaluate to ModelConfigEdit, not ConfigEdit"
    );
}

#[test]
fn test_extension_coverage_group_a_b_c_d() {
    let cases = [
        // Group A — Code & Docs
        ("src/lib.rs", RuleVerdict::RustEdit),
        ("app/main.py", RuleVerdict::PythonEdit),
        ("ui/index.ts", RuleVerdict::WebEdit),
        ("ui/app.tsx", RuleVerdict::WebEdit),
        ("ui/bundle.js", RuleVerdict::WebEdit),
        ("ui/component.jsx", RuleVerdict::WebEdit),
        ("assets/main.css", RuleVerdict::StyleEdit),
        ("assets/style.scss", RuleVerdict::StyleEdit),
        ("assets/theme.less", RuleVerdict::StyleEdit),
        ("public/index.html", RuleVerdict::MarkupEdit),
        ("scripts/run.sh", RuleVerdict::ShellScriptEdit),
        ("scripts/deploy.bash", RuleVerdict::ShellScriptEdit),
        ("scripts/env.zsh", RuleVerdict::ShellScriptEdit),
        ("README.md", RuleVerdict::DocsEdit),
        ("docs/index.rst", RuleVerdict::DocsEdit),
        ("notes.txt", RuleVerdict::DocsEdit),
        ("settings.toml", RuleVerdict::ConfigEdit),
        ("config.yaml", RuleVerdict::ConfigEdit),
        ("config.yml", RuleVerdict::ConfigEdit),
        ("data.json", RuleVerdict::ConfigEdit),

        // Group B — Media
        ("logo.png", RuleVerdict::ImageAsset),
        ("photo.jpg", RuleVerdict::ImageAsset),
        ("avatar.jpeg", RuleVerdict::ImageAsset),
        ("banner.gif", RuleVerdict::ImageAsset),
        ("icon.svg", RuleVerdict::ImageAsset),
        ("bg.webp", RuleVerdict::ImageAsset),
        ("track.mp3", RuleVerdict::AudioAsset),
        ("sound.wav", RuleVerdict::AudioAsset),
        ("audio.flac", RuleVerdict::AudioAsset),
        ("movie.mp4", RuleVerdict::VideoAsset),
        ("clip.mov", RuleVerdict::VideoAsset),
        ("stream.webm", RuleVerdict::VideoAsset),
        ("font.ttf", RuleVerdict::FontAsset),
        ("font.otf", RuleVerdict::FontAsset),
        ("font.woff", RuleVerdict::FontAsset),
        ("font.woff2", RuleVerdict::FontAsset),
        ("analysis.ipynb", RuleVerdict::NotebookActivity),

        // Group C — Checkpoints
        ("checkpoint.pt", RuleVerdict::ModelTrainingCheckpoint { size_bytes: 0 }),
        ("model.safetensors", RuleVerdict::ModelTrainingCheckpoint { size_bytes: 0 }),
        ("weights.ckpt", RuleVerdict::ModelTrainingCheckpoint { size_bytes: 0 }),
        ("net.onnx", RuleVerdict::ModelTrainingCheckpoint { size_bytes: 0 }),

        // Group D — Archive
        ("bundle.zip", RuleVerdict::ArchiveWrite),
        ("archive.tar.gz", RuleVerdict::ArchiveWrite),
        ("backup.tgz", RuleVerdict::ArchiveWrite),
        ("files.tar", RuleVerdict::ArchiveWrite),
        ("records.arrow", RuleVerdict::ArchiveWrite),
        ("table.csv", RuleVerdict::ArchiveWrite),
        ("lines.jsonl", RuleVerdict::ArchiveWrite),
    ];

    for (path_str, expected) in cases {
        let p = Path::new(path_str);
        let verdict = evaluate_rules(p, false, FileOp::Modified, false, false, Path::new(""));
        assert_eq!(
            verdict, expected,
            "Failed extension test for {}: expected {:?}, got {:?}",
            path_str, expected, verdict
        );
    }
}

#[test]
fn test_mass_deletion_burst_behavior() {
    let mut classifier = SemanticClassifier::new(PathBuf::new());

    // 1. Send 4 deletes (< 5) within the 300ms window
    for i in 1..=4 {
        let p = PathBuf::from(format!("/tmp/file_{}.rs", i));
        let ev = make_dummy_event(p, EventMask::DELETE, 0, false);
        let emitted = classifier.push_event(ev);
        assert!(emitted.is_none(), "Deletes should be buffered in delete_burst");
    }

    // Flush after 300ms window expires
    thread::sleep(Duration::from_millis(320));
    let flushed = classifier.flush_ready(Duration::from_millis(300));

    // Under 5 deletions -> each emitted individually
    assert_eq!(flushed.len(), 4, "4 deletes must be emitted individually");
    for event in flushed {
        assert!(matches!(event.category, ActivityCategory::RustEdit { .. }));
    }

    // 2. Send 5 deletes (>= 5) within the 300ms window -> exactly 1 MassDeletion event
    for i in 1..=5 {
        let p = PathBuf::from(format!("/tmp/mass_delete_{}.rs", i));
        let ev = make_dummy_event(p, EventMask::DELETE, 0, false);
        let emitted = classifier.push_event(ev);
        assert!(emitted.is_none(), "Deletes should be buffered in delete_burst");
    }

    thread::sleep(Duration::from_millis(320));
    let flushed_burst = classifier.flush_ready(Duration::from_millis(300));

    assert_eq!(flushed_burst.len(), 1, "5 deletes must coalesce into 1 MassDeletion event");
    match &flushed_burst[0].category {
        ActivityCategory::MassDeletion { count, sample_paths } => {
            assert_eq!(*count, 5);
            assert_eq!(sample_paths.len(), 3);
        }
        other => panic!("Expected MassDeletion, got {:?}", other),
    }
}

// Regression test for the bug reported against a real watch root named
// "/home/bhanotos/test": is_test_path must only scan path components INSIDE
// the watched tree, never the watch root's own name or its ancestors, or
// every file in a project whose folder happens to be named "test" (or any
// ancestor directory containing "test"/"tests") would misclassify as
// TestFileActivity.
#[test]
fn test_watch_root_named_test_does_not_misclassify_everything() {
    let watch_root = Path::new("/home/bhanotos/test");

    // A plain Rust source file directly under the "test"-named watch root
    // must classify by its actual content/extension, NOT as TestFileActivity.
    let rust_file = watch_root.join("src/sample.rs");
    assert_eq!(
        evaluate_rules(&rust_file, false, FileOp::Modified, false, false, watch_root),
        RuleVerdict::RustEdit,
        "A file under a watch root merely named 'test' must not misclassify as TestFileActivity"
    );

    let py_file = watch_root.join("sample.py");
    assert_eq!(
        evaluate_rules(&py_file, false, FileOp::Modified, false, false, watch_root),
        RuleVerdict::PythonEdit
    );

    // A genuine tests/ subdirectory INSIDE the watched tree must still
    // correctly trigger TestFileActivity.
    let real_test_file = watch_root.join("tests/test_sample.py");
    assert_eq!(
        evaluate_rules(&real_test_file, false, FileOp::Modified, false, false, watch_root),
        RuleVerdict::TestFileActivity,
        "A real tests/ subdirectory inside the project must still classify as TestFileActivity"
    );

    // Same class of bug for the model/checkpoint path-context heuristic: a
    // watch root living under a "model"-named ancestor must not make every
    // file in the project look like model context.
    let model_root = Path::new("/home/alice/my-model-experiments/proj");
    let unrelated_json = model_root.join("package.json");
    assert!(
        !is_checkpoint_path_context(&unrelated_json, model_root),
        "A watch root under a 'model'-named ancestor must not make every file look like model context"
    );
    let real_checkpoint_dir = model_root.join("checkpoints/config.json");
    assert!(
        is_checkpoint_path_context(&real_checkpoint_dir, model_root),
        "A real checkpoints/ subdirectory inside the project must still match model context"
    );
}
