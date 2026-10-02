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
        process_name: "claude-code".to_string(),
        is_ai_agent: true,
        agent_ancestor: Some("claude-code".to_string()),
        active_network_stream: network,
        local_disk_mutation: !network,
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

    let verdict = evaluate_rules(&model_path, false, FileOp::Modified, false);
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

    assert!(is_checkpoint_path_context(&shard_bin));
    let verdict1 = evaluate_rules(&shard_bin, false, FileOp::Modified, false);
    assert!(matches!(verdict1, RuleVerdict::ModelTrainingCheckpoint { .. }));

    // Case 2: In a directory with sibling *.index.json
    let sibling_dir = temp.path.join("custom_weights");
    fs::create_dir(&sibling_dir).unwrap();
    fs::write(sibling_dir.join("model.safetensors.index.json"), b"{}").unwrap();
    let sibling_bin = sibling_dir.join("data.bin");
    fs::write(&sibling_bin, b"tensor data").unwrap();

    assert!(is_checkpoint_path_context(&sibling_bin));
    let verdict2 = evaluate_rules(&sibling_bin, false, FileOp::Modified, false);
    assert!(matches!(verdict2, RuleVerdict::ModelTrainingCheckpoint { .. }));

    // Case 3: Plain .bin without checkpoint context, with active_network_stream
    let plain_dir = temp.path.join("downloads");
    fs::create_dir(&plain_dir).unwrap();
    let plain_bin = plain_dir.join("firmware.bin");
    fs::write(&plain_bin, b"firmware").unwrap();

    assert!(!is_checkpoint_path_context(&plain_bin));
    let verdict3 = evaluate_rules(&plain_bin, false, FileOp::Modified, true);
    assert_eq!(verdict3, RuleVerdict::IncomingDataStream);

    // Case 4: Plain .bin without network stream -> FileMutation
    let verdict4 = evaluate_rules(&plain_bin, false, FileOp::Modified, false);
    assert_eq!(verdict4, RuleVerdict::FileMutation(FileOp::Modified));
}

#[test]
fn test_dataset_stream_vs_local_mutation() {
    let temp = TempDir::new("dataset");
    let parquet_path = temp.path.join("dataset.parquet");
    fs::write(&parquet_path, b"parquet data").unwrap();

    let stream_verdict = evaluate_rules(&parquet_path, false, FileOp::Modified, true);
    assert_eq!(stream_verdict, RuleVerdict::IncomingDataStream);

    let local_verdict = evaluate_rules(&parquet_path, false, FileOp::Modified, false);
    assert_eq!(local_verdict, RuleVerdict::FileMutation(FileOp::Modified));
}

#[test]
fn test_workspace_expansion() {
    let temp = TempDir::new("workspace");
    let new_dir = temp.path.join("new_feature");
    let verdict = evaluate_rules(&new_dir, true, FileOp::Created, false);
    assert_eq!(verdict, RuleVerdict::WorkspaceExpansion);
}

#[test]
fn test_source_code_mutation_and_line_diff() {
    let temp = TempDir::new("source");
    let code_file = temp.path.join("main.rs");
    let mut classifier = SemanticClassifier::new();

    // 1. Initial 5 lines
    fs::write(&code_file, "1\n2\n3\n4\n5\n").unwrap();
    let ev1 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te1 = classifier.classify_direct(&ev1, &code_file, false, FileOp::Created);

    match te1.category {
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 5);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected SourceCodeMutation, got {:?}", other),
    }

    // 2. Add 3 more lines (8 total)
    fs::write(&code_file, "1\n2\n3\n4\n5\n6\n7\n8\n").unwrap();
    let ev2 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te2 = classifier.classify_direct(&ev2, &code_file, false, FileOp::Modified);

    match te2.category {
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 3);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected SourceCodeMutation, got {:?}", other),
    }

    // 3. Remove lines (shrink to 2 lines)
    fs::write(&code_file, "1\n2\n").unwrap();
    let ev3 = make_dummy_event(code_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te3 = classifier.classify_direct(&ev3, &code_file, false, FileOp::Modified);

    match te3.category {
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 0);
            assert_eq!(lines_removed, 6);
        }
        other => panic!("Expected SourceCodeMutation, got {:?}", other),
    }
}

#[test]
fn test_rename_cookie_pairing() {
    let temp = TempDir::new("ren_pair");
    let from_file = temp.path.join("old.txt");
    let to_file = temp.path.join("new.txt");
    fs::write(&to_file, "atomic save content").unwrap();

    let mut classifier = SemanticClassifier::new();
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

    let mut classifier = SemanticClassifier::new();
    let ev1 = make_dummy_event(file.clone(), EventMask::MODIFY, 0, false);
    let opt = classifier.push_event(ev1);
    assert!(opt.is_none(), "Event should be in debounce window");

    // Flushed immediately with 0ms window -> should be ready
    thread::sleep(Duration::from_millis(50));
    let flushed = classifier.flush_ready(Duration::from_millis(40));
    assert_eq!(flushed.len(), 1, "Expected 1 flushed event");
    assert!(matches!(flushed[0].category, ActivityCategory::SourceCodeMutation { .. }));
}

#[test]
fn test_unpaired_moved_from_emits_deletion() {
    let temp = TempDir::new("unpaired_move");
    let from_file = temp.path.join("vanished.rs");

    let mut classifier = SemanticClassifier::new();
    let cookie = 12345;

    let ev_from = make_dummy_event(from_file.clone(), EventMask::MOVED_FROM, cookie, false);
    let opt = classifier.push_event(ev_from);
    assert!(opt.is_none(), "MOVED_FROM initially waits in pending_renames");

    // Sleep > 1s for rename expiration
    thread::sleep(Duration::from_millis(1050));

    let flushed = classifier.flush_ready(Duration::from_millis(100));
    assert_eq!(flushed.len(), 1, "Unpaired MOVED_FROM must emit a deletion event");
    match &flushed[0].category {
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed: _, path } => {
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

    let mut classifier = SemanticClassifier::new();

    // 1. Initial write establishes line count of 50
    let ev_init = make_dummy_event(from_file.clone(), EventMask::CLOSE_WRITE, 0, false);
    let te_init = classifier.classify_direct(&ev_init, &from_file, false, FileOp::Created);
    match te_init.category {
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 50);
            assert_eq!(lines_removed, 0);
        }
        other => panic!("Expected SourceCodeMutation, got {:?}", other),
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
        ActivityCategory::SourceCodeMutation { lines_added, lines_removed, .. } => {
            assert_eq!(lines_added, 0, "Pure rename should not report added lines");
            assert_eq!(lines_removed, 0, "Pure rename should not report removed lines");
        }
        other => panic!("Expected SourceCodeMutation, got {:?}", other),
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

    let mut classifier = SemanticClassifier::new();
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
    use std::io::{Seek, SeekFrom, Write};
    let temp = TempDir::new("stream_sparse");
    let file_path = temp.path.join("sparse_download.parquet");

    // Pre-allocate a 10MB sparse download file and write 1MB into it
    let mut f = fs::File::create(&file_path).unwrap();
    f.seek(SeekFrom::Start(10_000_000 - 1)).unwrap();
    f.write_all(b"\0").unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    f.write_all(&vec![b'x'; 1_000_000]).unwrap();
    f.flush().unwrap();

    let mut classifier = SemanticClassifier::new();
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
