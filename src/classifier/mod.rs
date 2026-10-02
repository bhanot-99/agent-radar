pub mod rules;

use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use lru::LruCache;
use inotify::EventMask;

use crate::correlator::EnrichedEvent;
use crate::events::{ActivityCategory, FileOp, TelemetryEvent};
use self::rules::{evaluate_rules, RuleVerdict};

pub const RATE_TRACKERS_CAPACITY: usize = 2048;
pub const RATE_TRACKERS_TTL_SECS: u64 = 30;
pub const DEBOUNCE_WINDOW_MS: u64 = 300;

#[derive(Debug, Clone)]
pub struct RateTrackerEntry {
    pub last_size: u64,
    pub last_time: Instant,
    pub bytes_per_sec: u64,
}

#[derive(Debug, Clone)]
pub struct PendingDebounce {
    pub enriched: EnrichedEvent,
    pub op: FileOp,
    pub first_seen: Instant,
    pub last_seen: Instant,
}

pub struct SemanticClassifier {
    debounce_map: HashMap<PathBuf, PendingDebounce>,
    pending_renames: HashMap<u32, (PathBuf, EnrichedEvent, Instant)>,
    rate_trackers: LruCache<PathBuf, RateTrackerEntry>,
    line_counts: LruCache<PathBuf, usize>,
}

impl SemanticClassifier {
    pub fn new() -> Self {
        Self {
            debounce_map: HashMap::new(),
            pending_renames: HashMap::new(),
            rate_trackers: LruCache::new(NonZeroUsize::new(RATE_TRACKERS_CAPACITY).unwrap()),
            line_counts: LruCache::new(NonZeroUsize::new(RATE_TRACKERS_CAPACITY).unwrap()),
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.debounce_map.is_empty()
    }

    pub fn time_until_next_flush(&self, window: Duration) -> Option<Duration> {
        if self.debounce_map.is_empty() {
            return None;
        }
        let now = Instant::now();
        let mut min_wait = window;
        for item in self.debounce_map.values() {
            let elapsed = now.duration_since(item.last_seen);
            let remaining = window.saturating_sub(elapsed);
            if remaining < min_wait {
                min_wait = remaining;
            }
        }
        Some(min_wait)
    }

    pub fn push_event(&mut self, enriched: EnrichedEvent) -> Option<TelemetryEvent> {
        let mask = enriched.raw.mask;
        let is_dir = mask.contains(EventMask::ISDIR);
        let path = enriched.raw.full_path.clone().unwrap_or_else(|| enriched.raw.dir_path.clone());

        // 1. Rename-cookie pairing
        if mask.contains(EventMask::MOVED_FROM) && enriched.raw.cookie > 0 {
            self.pending_renames.insert(enriched.raw.cookie, (path, enriched, Instant::now()));
            return None;
        }

        if mask.contains(EventMask::MOVED_TO) && enriched.raw.cookie > 0 {
            if let Some((from_path, _from_enriched, _)) = self.pending_renames.remove(&enriched.raw.cookie) {
                // Paired atomic rename:
                // Transfer line count from old path to new path if target doesn't already have one,
                // so a pure file rename isn't misreported as all lines added!
                if !self.line_counts.contains(&path) {
                    if let Some(prev) = self.line_counts.pop(&from_path) {
                        self.line_counts.put(path.clone(), prev);
                    }
                }
                return Some(self.classify_direct(&enriched, &path, is_dir, FileOp::Renamed));
            }
        }

        // 2. Directory creation -> immediate WorkspaceExpansion
        if is_dir && mask.contains(EventMask::CREATE) {
            return Some(self.classify_direct(&enriched, &path, true, FileOp::Created));
        }

        // 3. Deletion -> immediate FileMutation(Deleted)
        if mask.contains(EventMask::DELETE) {
            self.debounce_map.remove(&path);
            return Some(self.classify_direct(&enriched, &path, is_dir, FileOp::Deleted));
        }

        // 4. Writes / modifies / creates -> enqueue into per-path debounce window
        let now = Instant::now();
        let op = if mask.contains(EventMask::CREATE) {
            FileOp::Created
        } else {
            FileOp::Modified
        };

        if let Some(entry) = self.debounce_map.get_mut(&path) {
            entry.last_seen = now;
            entry.enriched = enriched;
            entry.op = op;
        } else {
            self.debounce_map.insert(
                path,
                PendingDebounce {
                    enriched,
                    op,
                    first_seen: now,
                    last_seen: now,
                },
            );
        }

        None
    }

    pub fn flush_ready(&mut self, window: Duration) -> Vec<TelemetryEvent> {
        let now = Instant::now();
        let ready_paths: Vec<PathBuf> = self
            .debounce_map
            .iter()
            .filter(|(_, v)| now.duration_since(v.last_seen) >= window)
            .map(|(k, _)| k.clone())
            .collect();

        let mut results = Vec::new();
        for p in ready_paths {
            if let Some(debounced) = self.debounce_map.remove(&p) {
                let is_dir = debounced.enriched.raw.mask.contains(EventMask::ISDIR);
                let event = self.classify_direct(&debounced.enriched, &p, is_dir, debounced.op);
                results.push(event);
            }
        }

        // Sweep expired unpaired renames (> 1s).
        // If a MOVED_FROM had no matching MOVED_TO, the file was moved out of the
        // watched workspace — classify it as a deletion!
        let mut expired_renames = Vec::new();
        self.pending_renames.retain(|_, (from_path, enriched, ts)| {
            if now.duration_since(*ts) >= Duration::from_secs(1) {
                expired_renames.push((from_path.clone(), enriched.clone()));
                false
            } else {
                true
            }
        });

        for (from_path, enriched) in expired_renames {
            let is_dir = enriched.raw.mask.contains(EventMask::ISDIR);
            self.line_counts.pop(&from_path);
            let event = self.classify_direct(&enriched, &from_path, is_dir, FileOp::Deleted);
            results.push(event);
        }

        // Enforce RATE_TRACKERS_TTL_SECS using LRU order
        while let Some((_, entry)) = self.rate_trackers.peek_lru() {
            if now.duration_since(entry.last_time) >= Duration::from_secs(RATE_TRACKERS_TTL_SECS) {
                self.rate_trackers.pop_lru();
            } else {
                break;
            }
        }

        results
    }

    pub fn classify_direct(
        &mut self,
        enriched: &EnrichedEvent,
        path: &Path,
        is_dir: bool,
        op: FileOp,
    ) -> TelemetryEvent {
        let verdict = evaluate_rules(path, is_dir, op, enriched.active_network_stream);
        let path_str = path.to_string_lossy().into_owned();

        let category = match verdict {
            RuleVerdict::WorkspaceExpansion => ActivityCategory::WorkspaceExpansion { path: path_str },
            RuleVerdict::ModelTrainingCheckpoint { size_bytes } => {
                ActivityCategory::ModelTrainingCheckpoint {
                    path: path_str,
                    size_bytes,
                }
            }
            RuleVerdict::IncomingDataStream => {
                let current_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let now = Instant::now();
                let bytes_per_sec = if let Some(entry) = self.rate_trackers.get_mut(path) {
                    let age = now.duration_since(entry.last_time);
                    if age >= Duration::from_secs(RATE_TRACKERS_TTL_SECS) {
                        // Reset expired tracker
                        entry.last_size = current_size;
                        entry.last_time = now;
                        entry.bytes_per_sec = 0;
                        0
                    } else {
                        let elapsed = age.as_secs_f64();
                        let delta = current_size.saturating_sub(entry.last_size);
                        let rate = if elapsed > 0.05 {
                            (delta as f64 / elapsed) as u64
                        } else {
                            entry.bytes_per_sec
                        };
                        entry.last_size = current_size;
                        entry.last_time = now;
                        entry.bytes_per_sec = rate;
                        rate
                    }
                } else {
                    self.rate_trackers.put(
                        path.to_path_buf(),
                        RateTrackerEntry {
                            last_size: current_size,
                            last_time: now,
                            bytes_per_sec: 0,
                        },
                    );
                    0
                };

                ActivityCategory::IncomingDataStream {
                    path: path_str,
                    bytes_per_sec,
                    progress_pct: None,
                }
            }
            RuleVerdict::SourceCodeMutation => {
                // Debounced line diff computation
                let (lines_added, lines_removed) = if op == FileOp::Deleted {
                    let prev_lines = self.line_counts.pop(path).unwrap_or(0);
                    (0, prev_lines)
                } else {
                    self.compute_line_diff(path)
                };
                ActivityCategory::SourceCodeMutation {
                    path: path_str,
                    lines_added,
                    lines_removed,
                }
            }
            RuleVerdict::FileMutation(file_op) => ActivityCategory::FileMutation {
                path: path_str,
                op: file_op,
            },
        };

        TelemetryEvent {
            timestamp: enriched.timestamp,
            pid: enriched.pid.unwrap_or(0),
            ppid: enriched.ppid.unwrap_or(0),
            process_name: enriched.process_name.clone(),
            category,
        }
    }

    fn compute_line_diff(&mut self, path: &Path) -> (usize, usize) {
        let current_lines = fs::read_to_string(path)
            .map(|content| content.lines().count())
            .unwrap_or(0);

        if let Some(&prev_lines) = self.line_counts.get(path) {
            let (added, removed) = if current_lines >= prev_lines {
                (current_lines - prev_lines, 0)
            } else {
                (0, prev_lines - current_lines)
            };
            self.line_counts.put(path.to_path_buf(), current_lines);
            (added, removed)
        } else {
            self.line_counts.put(path.to_path_buf(), current_lines);
            (current_lines, 0)
        }
    }
}

impl Default for SemanticClassifier {
    fn default() -> Self {
        Self::new()
    }
}
