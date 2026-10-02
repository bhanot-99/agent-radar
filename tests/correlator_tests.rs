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
    assert!(correlator.matches_agent_pattern("antigravity"));
    assert!(correlator.matches_agent_pattern("cursor"));
    assert!(correlator.matches_agent_pattern("gemini"));

    // Plain runtimes without AI agent ancestor must NOT match agent pattern
    assert!(!correlator.matches_agent_pattern("python3 -m pip install"));
    assert!(!correlator.matches_agent_pattern("node dist/index.js"));
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

#[test]
fn test_plain_runtime_not_classified_as_ai_agent() {
    let mut correlator = ProcessCorrelator::new();

    // PID 1 (systemd/init) is a standard system daemon and must NEVER be classified as an AI agent (Finding 1)
    let info = correlator.get_or_resolve(1).expect("PID 1 should resolve");
    assert!(!info.is_ai_agent, "PID 1 must not be classified as AI agent");
    assert!(info.agent_ancestor.is_none());

    // Agent patterns must match real agents
    assert!(correlator.matches_agent_pattern("claude"));
    assert!(correlator.matches_agent_pattern("aider"));
    assert!(correlator.matches_agent_pattern("antigravity"));
    assert!(correlator.matches_agent_pattern("cursor"));

    // Runtimes must NOT match agent pattern directly
    assert!(!correlator.matches_agent_pattern("python3 script.py"));
    assert!(!correlator.matches_agent_pattern("node server.js"));
    assert!(!correlator.matches_agent_pattern("bash"));
}

#[test]
fn test_correlator_tracked_pid_count() {
    let mut correlator = ProcessCorrelator::new();
    assert_eq!(correlator.tracked_pid_count(), 0);

    let my_pid = process::id();
    correlator.get_or_resolve(my_pid);
    assert_eq!(correlator.tracked_pid_count(), 1, "Tracked PID count must increment on resolution");
}
