use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::thread;

use agent_radar::watcher::inotify_tier::{is_ignored_path, InotifyWatcher};
use agent_radar::watcher::EventSource;
use agent_radar::acquire_single_instance_lock;
use inotify::EventMask;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!("agent_radar_test_{}_{}", prefix, std::process::id()));
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

#[test]
fn test_ignored_paths() {
    assert!(is_ignored_path(Path::new(".git")));
    assert!(is_ignored_path(Path::new("/home/user/project/.git/config")));
    assert!(is_ignored_path(Path::new("node_modules")));
    assert!(is_ignored_path(Path::new("project/node_modules/package/index.js")));
    assert!(is_ignored_path(Path::new("__pycache__")));
    assert!(is_ignored_path(Path::new("target")));
    assert!(is_ignored_path(Path::new("target/debug/agent-radar")));

    assert!(!is_ignored_path(Path::new("src/main.rs")));
    assert!(!is_ignored_path(Path::new("checkpoints/epoch-1.safetensors")));
}

#[test]
fn test_watcher_create_modify_delete() {
    let temp = TempDir::new("lifecycle");
    let mut watcher = InotifyWatcher::new(&temp.path).unwrap();
    assert!(watcher.active_watch_count() >= 1);

    // Give inotify a moment
    thread::sleep(Duration::from_millis(50));

    // 1. Create file
    let file_path = temp.path.join("test.txt");
    {
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"hello world").unwrap();
        f.sync_all().unwrap();
    }

    thread::sleep(Duration::from_millis(50));
    let events = watcher.drain();
    assert!(!events.is_empty(), "Expected events on file create");
    let created = events.iter().any(|e| {
        e.mask.contains(EventMask::CREATE) && e.name.as_deref() == Some("test.txt")
    });
    assert!(created, "Did not find CREATE event for test.txt: {:?}", events);

    // 2. Modify file
    {
        let mut f = fs::OpenOptions::new().append(true).open(&file_path).unwrap();
        f.write_all(b"\nmore data").unwrap();
        f.sync_all().unwrap();
    }

    thread::sleep(Duration::from_millis(50));
    let events = watcher.drain();
    assert!(!events.is_empty(), "Expected events on file modify");
    let modified = events.iter().any(|e| {
        e.mask.contains(EventMask::MODIFY) || e.mask.contains(EventMask::CLOSE_WRITE)
    });
    assert!(modified, "Did not find MODIFY or CLOSE_WRITE event: {:?}", events);

    // 3. Delete file
    fs::remove_file(&file_path).unwrap();
    thread::sleep(Duration::from_millis(50));
    let events = watcher.drain();
    assert!(!events.is_empty(), "Expected events on file delete");
    let deleted = events.iter().any(|e| e.mask.contains(EventMask::DELETE));
    assert!(deleted, "Did not find DELETE event: {:?}", events);
}

#[test]
fn test_create_then_watch_race_closer() {
    let temp = TempDir::new("race");
    let mut watcher = InotifyWatcher::new(&temp.path).unwrap();

    thread::sleep(Duration::from_millis(50));

    // Create sub directory and immediately write a file inside it
    let sub_dir = temp.path.join("nested_sub");
    fs::create_dir(&sub_dir).unwrap();
    let sub_file = sub_dir.join("rapid.rs");
    fs::write(&sub_file, b"fn main() {}").unwrap();

    thread::sleep(Duration::from_millis(80));
    let events = watcher.drain();

    let found_sub_file = events.iter().any(|e| {
        e.name.as_deref() == Some("rapid.rs")
    });
    assert!(found_sub_file, "Race closer failed to detect rapid.rs: {:?}", events);
}

#[test]
fn test_rename_cookie_pairing() {
    let temp = TempDir::new("rename");
    let mut watcher = InotifyWatcher::new(&temp.path).unwrap();

    let src = temp.path.join("old_name.txt");
    let dst = temp.path.join("new_name.txt");
    fs::write(&src, b"test content").unwrap();

    thread::sleep(Duration::from_millis(50));
    let _ = watcher.drain(); // clear create events

    fs::rename(&src, &dst).unwrap();
    thread::sleep(Duration::from_millis(50));
    let events = watcher.drain();

    let from_event = events.iter().find(|e| e.mask.contains(EventMask::MOVED_FROM));
    let to_event = events.iter().find(|e| e.mask.contains(EventMask::MOVED_TO));

    assert!(from_event.is_some(), "Expected MOVED_FROM event: {:?}", events);
    assert!(to_event.is_some(), "Expected MOVED_TO event: {:?}", events);

    let cookie_from = from_event.unwrap().cookie;
    let cookie_to = to_event.unwrap().cookie;
    assert!(cookie_from > 0, "Cookie must be non-zero");
    assert_eq!(cookie_from, cookie_to, "Cookies must match for atomic rename pairing");
}

#[test]
fn test_flock_single_instance() {
    let test_dir1 = std::env::temp_dir().join(format!("agent_radar_test_lock1_{}", std::process::id()));
    let test_dir2 = std::env::temp_dir().join(format!("agent_radar_test_lock2_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&test_dir1);
    let _ = std::fs::create_dir_all(&test_dir2);

    let lock1 = acquire_single_instance_lock(&test_dir1);
    assert!(lock1.is_ok(), "First lock on dir1 should succeed");

    // Second lock on same dir must fail
    let lock2 = acquire_single_instance_lock(&test_dir1);
    assert!(lock2.is_err(), "Second lock on same dir must fail");
    assert_eq!(lock2.err().unwrap(), "already running");

    // Lock on different dir must succeed simultaneously
    let lock_diff = acquire_single_instance_lock(&test_dir2);
    assert!(lock_diff.is_ok(), "Lock on different dir must succeed simultaneously");

    drop(lock1);

    let lock3 = acquire_single_instance_lock(&test_dir1);
    assert!(lock3.is_ok(), "Lock should succeed again after previous holder drops");

    drop(lock_diff);
    drop(lock3);
    let _ = std::fs::remove_dir_all(&test_dir1);
    let _ = std::fs::remove_dir_all(&test_dir2);
}
