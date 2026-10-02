pub mod procfs;

use std::num::NonZeroUsize;
use std::time::{Duration, Instant, SystemTime};
use lru::LruCache;
use regex::Regex;

use crate::watcher::inotify_tier::RawFsEvent;
use self::procfs::{has_active_socket, is_process_alive, read_process_info, find_pid_accessing_path};

pub const PID_CACHE_CAPACITY: usize = 512;
pub const PID_CACHE_TTL_SECS: u64 = 30;

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
}

pub struct ProcessCorrelator {
    pid_cache: LruCache<u32, CachedPidInfo>,
    agent_patterns: Vec<Regex>,
    last_known_agent_pid: Option<u32>,
}

impl ProcessCorrelator {
    pub fn new() -> Self {
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
            r"(?i)\bpython3?\b",
            r"(?i)\bnode\b",
            r"(?i)\bnpx\b",
        ];

        let agent_patterns = pattern_strings
            .iter()
            .map(|pat| Regex::new(pat).expect("valid regex"))
            .collect();

        Self {
            pid_cache: LruCache::new(NonZeroUsize::new(PID_CACHE_CAPACITY).unwrap()),
            agent_patterns,
            last_known_agent_pid: None,
        }
    }

    pub fn matches_agent_pattern(&self, text: &str) -> bool {
        self.agent_patterns.iter().any(|re| re.is_match(text))
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

        // 2. Read procfs
        let proc_info = read_process_info(pid).ok()?;

        // 3. Ancestry walk to map child processes (git, curl, pip, etc.) to agent
        let mut is_ai_agent = false;
        let mut agent_ancestor = None;
        let mut current_pid = pid;
        let mut depth = 0;

        while depth < 10 {
            if let Ok(info) = read_process_info(current_pid) {
                if self.matches_agent_pattern(&info.name) || self.matches_agent_pattern(&info.cmdline) {
                    is_ai_agent = true;
                    agent_ancestor = Some(info.name.clone());
                    break;
                }
                if info.ppid <= 1 || info.ppid == current_pid {
                    break;
                }
                current_pid = info.ppid;
                depth += 1;
            } else {
                break;
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

    pub fn correlate(&mut self, raw: RawFsEvent) -> EnrichedEvent {
        let timestamp = SystemTime::now();

        // 1. Try to find the exact PID accessing this path
        let resolved_pid = if let Some(ref path) = raw.full_path {
            find_pid_accessing_path(path)
        } else {
            find_pid_accessing_path(&raw.dir_path)
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
        }
    }
}
