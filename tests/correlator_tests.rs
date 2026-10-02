use std::path::PathBuf;
use std::process;
use agent_radar::correlator::procfs::{is_process_alive, read_process_info, has_active_socket};
use agent_radar::correlator::ProcessCorrelator;
use agent_radar::watcher::inotify_tier::InotifyWatcher;
use inotify::EventMask;

#[test]
fn test_procfs_self_read() {
    let my_pid = process::id();
    let info = read_process_info(my_pid).expect("Should be able to read own process info");

    assert_eq!(info.pid, my_pid);
    assert!(!info.name.is_empty());
    assert!(info.start_time > 0, "start_time should be non-zero");

    assert!(is_process_alive(my_pid, info.start_time), "Self process should be alive");
    assert!(!is_process_alive(999_999_999, 12345), "Non-existent PID should not be alive");
}

#[test]
fn test_agent_pattern_matching() {
    let correlator = ProcessCorrelator::new();

    assert!(correlator.matches_agent_pattern("claude"));
    assert!(correlator.matches_agent_pattern("claude-code"));
    assert!(correlator.matches_agent_pattern("/usr/bin/aider --model gpt-4"));
    assert!(correlator.matches_agent_pattern("python3 -m pip install"));
    assert!(correlator.matches_agent_pattern("node dist/index.js"));
    assert!(correlator.matches_agent_pattern("antigravity"));
    assert!(correlator.matches_agent_pattern("cursor"));

    assert!(!correlator.matches_agent_pattern("ls -la"));
    assert!(!correlator.matches_agent_pattern("cat /etc/passwd"));
    assert!(!correlator.matches_agent_pattern("grep something"));
}

#[test]
fn test_pid_cache_resolution() {
    let mut correlator = ProcessCorrelator::new();
    let my_pid = process::id();

    let info1 = correlator.get_or_resolve(my_pid).expect("Should resolve self");
    assert_eq!(info1.pid, my_pid);

    // Second call should hit LRU cache with liveness check
    let info2 = correlator.get_or_resolve(my_pid).expect("Should resolve from cache");
    assert_eq!(info2.pid, my_pid);
    assert_eq!(info2.start_time, info1.start_time);
}

#[test]
fn test_has_active_socket_does_not_panic() {
    let my_pid = process::id();
    // Own process socket check should run without panic
    let _ = has_active_socket(my_pid);
}

#[test]
fn test_correlate_enrichment() {
    let mut watcher = InotifyWatcher::new(std::path::Path::new("/tmp")).unwrap();
    let mut correlator = ProcessCorrelator::new();
    let raw = agent_radar::watcher::inotify_tier::RawFsEvent {
        wd: watcher.add_watch_dir(std::path::Path::new("/tmp")).unwrap(),
        mask: EventMask::CREATE,
        cookie: 0,
        name: Some("test.rs".to_string()),
        dir_path: PathBuf::from("/tmp"),
        full_path: Some(PathBuf::from("/tmp/test.rs")),
    };

    let enriched = correlator.correlate(raw);
    assert_eq!(enriched.raw.name.as_deref(), Some("test.rs"));
    assert!(enriched.local_disk_mutation || enriched.active_network_stream);
}
