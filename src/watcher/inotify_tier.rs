use std::collections::HashMap;
use std::fs;
use std::os::fd::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use crate::watcher::EventSource;

#[derive(Debug, Clone)]
pub struct RawFsEvent {
    pub wd: WatchDescriptor,
    pub mask: EventMask,
    pub cookie: u32,
    pub name: Option<String>,
    pub dir_path: PathBuf,
    pub full_path: Option<PathBuf>,
}

pub fn is_ignored_component(name: &str) -> bool {
    matches!(name, ".git" | "node_modules" | "__pycache__" | "target")
}

pub fn is_ignored_path(path: &Path) -> bool {
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp {
            if let Some(s) = c.to_str() {
                if is_ignored_component(s) {
                    return true;
                }
            }
        }
    }
    false
}

pub struct InotifyWatcher {
    inotify: Inotify,
    watch_descriptors: HashMap<PathBuf, WatchDescriptor>,
    path_by_wd: HashMap<WatchDescriptor, PathBuf>,
    max_user_watches: usize,
    buffer: [u8; 8192],
    watch_budget_exceeded: bool,
}

impl InotifyWatcher {
    pub fn new(root: &Path) -> std::io::Result<Self> {
        let inotify = Inotify::init()?;
        let max_user_watches = Self::read_max_user_watches();

        let mut watcher = Self {
            inotify,
            watch_descriptors: HashMap::new(),
            path_by_wd: HashMap::new(),
            max_user_watches,
            buffer: [0u8; 8192],
            watch_budget_exceeded: false,
        };

        watcher.add_watch_recursive(root);
        Ok(watcher)
    }

    fn read_max_user_watches() -> usize {
        fs::read_to_string("/proc/sys/fs/inotify/max_user_watches")
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(8192)
    }

    pub fn max_watches(&self) -> usize {
        self.max_user_watches
    }

    pub fn active_watch_count(&self) -> usize {
        self.watch_descriptors.len()
    }

    pub fn is_budget_exceeded(&self) -> bool {
        self.watch_budget_exceeded
    }

    pub fn add_watch_dir(&mut self, dir: &Path) -> Option<WatchDescriptor> {
        if is_ignored_path(dir) {
            return None;
        }

        if let Some(wd) = self.watch_descriptors.get(dir) {
            return Some(wd.clone());
        }

        if self.watch_descriptors.len() >= self.max_user_watches {
            if !self.watch_budget_exceeded {
                self.watch_budget_exceeded = true;
                eprintln!(
                    "[WARN] inotify watch budget reached (max_user_watches = {}). Stopping recursion at {:?}",
                    self.max_user_watches, dir
                );
            }
            return None;
        }

        let mask = WatchMask::CREATE
            | WatchMask::CLOSE_WRITE
            | WatchMask::MOVED_FROM
            | WatchMask::MOVED_TO
            | WatchMask::DELETE
            | WatchMask::MODIFY
            | WatchMask::ATTRIB;

        match self.inotify.watches().add(dir, mask) {
            Ok(wd) => {
                let dir_buf = dir.to_path_buf();
                self.watch_descriptors.insert(dir_buf.clone(), wd.clone());
                self.path_by_wd.insert(wd.clone(), dir_buf);
                Some(wd)
            }
            Err(e) => {
                eprintln!("[WARN] Failed to watch directory {:?}: {}", dir, e);
                None
            }
        }
    }

    pub fn add_watch_recursive(&mut self, root: &Path) {
        if is_ignored_path(root) {
            return;
        }

        if root.is_dir() {
            self.add_watch_dir(root);
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && !is_ignored_path(&path) {
                        self.add_watch_recursive(&path);
                    }
                }
            }
        }
    }

    pub fn remove_watch_dir(&mut self, dir: &Path) {
        if let Some(wd) = self.watch_descriptors.remove(dir) {
            self.path_by_wd.remove(&wd);
            let _ = self.inotify.watches().remove(wd);
        }
    }

    // Handles the create-then-watch race condition
    fn handle_new_directory(&mut self, dir: &Path, events: &mut Vec<RawFsEvent>) {
        if let Some(wd) = self.add_watch_dir(dir) {
            // Immediately rescan newly created directory to close race window
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    if is_ignored_path(&child_path) {
                        continue;
                    }
                    let child_name = child_path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned());
                    let is_dir = child_path.is_dir();

                    let mask = if is_dir {
                        EventMask::CREATE | EventMask::ISDIR
                    } else {
                        EventMask::CREATE
                    };

                    events.push(RawFsEvent {
                        wd: wd.clone(),
                        mask,
                        cookie: 0,
                        name: child_name,
                        dir_path: dir.to_path_buf(),
                        full_path: Some(child_path.clone()),
                    });

                    if is_dir {
                        self.handle_new_directory(&child_path, events);
                    }
                }
            }
        }
    }
}

struct ParsedEvent {
    wd: WatchDescriptor,
    mask: EventMask,
    cookie: u32,
    name: Option<String>,
}

impl EventSource for InotifyWatcher {
    fn poll_fd(&self) -> RawFd {
        self.inotify.as_raw_fd()
    }

    fn drain(&mut self) -> Vec<RawFsEvent> {
        let mut raw_events = Vec::new();
        let parsed_events: Vec<ParsedEvent> = match self.inotify.read_events(&mut self.buffer) {
            Ok(events) => events
                .map(|ev| ParsedEvent {
                    wd: ev.wd.clone(),
                    mask: ev.mask,
                    cookie: ev.cookie,
                    name: ev.name.map(|n| n.to_string_lossy().into_owned()),
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        let mut new_dirs_to_process = Vec::new();

        for ev in parsed_events {
            let dir_path = match self.path_by_wd.get(&ev.wd) {
                Some(p) => p.clone(),
                None => continue,
            };

            let full_path = ev.name.as_ref().map(|n| dir_path.join(n));

            if let Some(ref fp) = full_path {
                if is_ignored_path(fp) {
                    continue;
                }
            }

            let is_dir = ev.mask.contains(EventMask::ISDIR);

            if ev.mask.contains(EventMask::CREATE) && is_dir {
                if let Some(ref fp) = full_path {
                    new_dirs_to_process.push(fp.clone());
                }
            } else if (ev.mask.contains(EventMask::DELETE) || ev.mask.contains(EventMask::MOVED_FROM)) && is_dir {
                if let Some(ref fp) = full_path {
                    self.remove_watch_dir(fp);
                }
            }

            raw_events.push(RawFsEvent {
                wd: ev.wd,
                mask: ev.mask,
                cookie: ev.cookie,
                name: ev.name,
                dir_path,
                full_path,
            });
        }

        for new_dir in new_dirs_to_process {
            self.handle_new_directory(&new_dir, &mut raw_events);
        }

        raw_events
    }
}
