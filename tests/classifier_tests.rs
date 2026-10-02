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
