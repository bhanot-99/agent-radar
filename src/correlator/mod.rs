pub mod procfs;

use std::num::NonZeroUsize;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use lru::LruCache;
use regex::Regex;

use crate::watcher::inotify_tier::RawFsEvent;
use self::procfs::{
    check_pid_accessing_path, has_active_socket, is_process_alive, read_process_info,
};

pub const PID_CACHE_CAPACITY: usize = 512;
pub const PID_CACHE_TTL_SECS: u64 = 30;
pub const MIN_PROC_SCAN_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone)]
pub struct CachedPidInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub start_time: u64,
    pub is_ai_agent: bool,
    pub agent_ancestor: Option<String>,
    pub has_socket: bool,
    pub cached_at: Instant,
}

#[derive(Debug, Clone)]
pub struct EnrichedEvent {
    pub raw: RawFsEvent,
    pub timestamp: SystemTime,
    pub pid: Option<u32>,
    pub ppid: Option<u32>,
    pub process_name: String,
    pub is_ai_agent: bool,
    pub agent_ancestor: Option<String>,
    pub active_network_stream: bool,
    pub local_disk_mutation: bool,
    pub is_git_process: bool,
}

pub struct ProcessCorrelator {
    pid_cache: LruCache<u32, CachedPidInfo>,
    agent_patterns: Vec<Regex>,
    last_known_agent_pid: Option<u32>,
    last_proc_scan: Instant,
    own_uid: u32,
}

impl ProcessCorrelator {
    pub fn new() -> Self {
        // True AI CLI agents only. Runtimes like python, node, bash are intermediate hops
        // checked during ancestry walk, not terminal agent matches in their own right.
        let pattern_strings = [
            r"(?i)\bclaude\b",
            r"(?i)\bclaude-code\b",
            r"(?i)\baider\b",
            r"(?i)\bantigravity\b",
            r"(?i)\bagy\b",
            r"(?i)\bcursor\b",
            r"(?i)\bcopilot\b",
            r"(?i)\bcodex\b",
            r"(?i)\bgemini\b",
        ];

        let agent_patterns = pattern_strings
            .iter()
            .map(|pat| Regex::new(pat).expect("valid regex"))
            .collect();

        Self {
            pid_cache: LruCache::new(NonZeroUsize::new(PID_CACHE_CAPACITY).unwrap()),
            agent_patterns,
            last_known_agent_pid: None,
            last_proc_scan: Instant::now().checked_sub(Duration::from_secs(10)).unwrap_or_else(Instant::now),
            own_uid: rustix::process::getuid().as_raw(),
        }
    }

    pub fn matches_agent_pattern(&self, text: &str) -> bool {
        self.agent_patterns.iter().any(|re| re.is_match(text))
    }

    pub fn tracked_pid_count(&self) -> usize {
        self.pid_cache.len()
    }

    pub fn get_or_resolve(&mut self, pid: u32) -> Option<CachedPidInfo> {
        let now = Instant::now();

        // 1. Check cache hit
        if let Some(cached) = self.pid_cache.get(&pid) {
            let age = now.duration_since(cached.cached_at);
            if age <= Duration::from_secs(PID_CACHE_TTL_SECS) {
                // Liveness re-check against /proc/<pid> to verify PID was not recycled
                if is_process_alive(pid, cached.start_time) {
                    let mut info = cached.clone();
                    // Refresh socket check
                    info.has_socket = has_active_socket(pid);
                    return Some(info);
                }
            }
            // Expired or dead process
            self.pid_cache.pop(&pid);
        }

        // 2. Read procfs for target process
        let proc_info = read_process_info(pid).ok()?;

        // 3. Ancestry walk:
        // If the process itself is an AI agent (claude, aider, antigravity, etc.), match immediately.
        // Otherwise, climb up parent chain looking for an AI agent ancestor.
        let mut is_ai_agent = false;
        let mut agent_ancestor = None;

        if self.matches_agent_pattern(&proc_info.name) || self.matches_agent_pattern(&proc_info.cmdline) {
            is_ai_agent = true;
            agent_ancestor = Some(proc_info.name.clone());
        } else {
            let mut current_parent = proc_info.ppid;
            let mut depth = 0;

            while depth < 10 && current_parent > 1 {
                if let Ok(parent_info) = read_process_info(current_parent) {
                    if self.matches_agent_pattern(&parent_info.name) || self.matches_agent_pattern(&parent_info.cmdline) {
                        is_ai_agent = true;
                        agent_ancestor = Some(parent_info.name.clone());
                        break;
                    }
                    if parent_info.ppid <= 1 || parent_info.ppid == current_parent {
                        break;
                    }
                    current_parent = parent_info.ppid;
                    depth += 1;
                } else {
                    break;
                }
            }
        }

        let has_socket = has_active_socket(pid);

        let cached_info = CachedPidInfo {
            pid,
            ppid: proc_info.ppid,
            name: proc_info.name,
            cmdline: proc_info.cmdline,
            start_time: proc_info.start_time,
            is_ai_agent,
            agent_ancestor,
            has_socket,
            cached_at: now,
        };

        if is_ai_agent {
            self.last_known_agent_pid = Some(pid);
        }

        self.pid_cache.put(pid, cached_info.clone());
        Some(cached_info)
    }

    fn find_pid_for_path(&mut self, path: &Path) -> Option<u32> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Fast path 1: check candidate PIDs (recently tracked active PIDs + last known agent)
        let mut candidates = Vec::with_capacity(16);
        if let Some(agent_pid) = self.last_known_agent_pid {
            candidates.push(agent_pid);
        }
        for (&pid, _) in self.pid_cache.iter() {
            if !candidates.contains(&pid) {
                candidates.push(pid);
            }
        }

        for &pid in &candidates {
            if check_pid_accessing_path(pid, path, &canonical) {
                return Some(pid);
            }
        }

        // Fast path 2: rate-limit full /proc scan to at most once per 500ms
        let now = Instant::now();
        if now.duration_since(self.last_proc_scan) < Duration::from_millis(MIN_PROC_SCAN_INTERVAL_MS) {
            return None;
        }

        self.last_proc_scan = now;

        // Slow path: scan own processes in /proc
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
                    if candidates.contains(&pid) {
                        continue;
                    }
                    if let Ok(meta) = entry.metadata() {
                        use std::os::unix::fs::MetadataExt;
                        if meta.uid() != self.own_uid {
                            continue;
                        }
                    }
                    if check_pid_accessing_path(pid, path, &canonical) {
                        return Some(pid);
                    }
                }
            }
        }

        None
    }

    pub fn correlate(&mut self, raw: RawFsEvent) -> EnrichedEvent {
        let timestamp = SystemTime::now();

        // 1. Try to find the exact PID accessing this path
        let resolved_pid = if let Some(ref path) = raw.full_path {
            self.find_pid_for_path(path)
        } else {
            self.find_pid_for_path(&raw.dir_path)
        };

        // If no direct PID has the file open (e.g. write already closed), fallback to last known agent PID
        // if still alive
        let effective_pid = resolved_pid.or_else(|| {
            if let Some(agent_pid) = self.last_known_agent_pid {
                if let Some(info) = self.get_or_resolve(agent_pid) {
                    if info.is_ai_agent {
                        return Some(agent_pid);
                    }
                }
            }
            None
        });

        if let Some(pid) = effective_pid {
            if let Some(info) = self.get_or_resolve(pid) {
                let is_git_process = info.name == "git";
                return EnrichedEvent {
                    raw,
                    timestamp,
                    pid: Some(pid),
                    ppid: Some(info.ppid),
                    process_name: info.name,
                    is_ai_agent: info.is_ai_agent,
                    agent_ancestor: info.agent_ancestor,
                    active_network_stream: info.has_socket,
                    local_disk_mutation: !info.has_socket,
                    is_git_process,
                };
            }
        }

        // Default: un-attributed / background I/O
        EnrichedEvent {
            raw,
            timestamp,
            pid: None,
            ppid: None,
            process_name: "background-io".to_string(),
            is_ai_agent: false,
            agent_ancestor: None,
            active_network_stream: false,
            local_disk_mutation: true,
            is_git_process: false,
        }
    }
}

impl Default for ProcessCorrelator {
    fn default() -> Self {
        Self::new()
    }
}
