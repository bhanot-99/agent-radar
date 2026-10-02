pub mod app_runner;
pub mod classifier;
pub mod correlator;
pub mod events;
pub mod signals;
pub mod tui;
pub mod watcher;

use std::fs::OpenOptions;
use std::path::PathBuf;
use rustix::fs::{flock, FlockOperation};

pub struct SingleInstanceLock {
    _file: std::fs::File,
    pub path: PathBuf,
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn acquire_single_instance_lock(watch_path: &std::path::Path) -> Result<SingleInstanceLock, String> {
    use std::hash::{Hash, Hasher};
    let uid = rustix::process::getuid().as_raw();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let canonical = watch_path.canonicalize().unwrap_or_else(|_| watch_path.to_path_buf());
    canonical.hash(&mut hasher);
    let path_hash = hasher.finish();
    let lock_path = PathBuf::from(format!("/tmp/agent-radar-{}-{:016x}.lock", uid, path_hash));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("failed to open lockfile {}: {}", lock_path.display(), e))?;

    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(SingleInstanceLock {
            _file: file,
            path: lock_path,
        }),
        Err(rustix::io::Errno::WOULDBLOCK) => {
            Err("already running".to_string())
        }
        Err(e) => Err(format!("failed to acquire flock: {}", e)),
    }
}

pub fn restore_terminal() {
    let _ = crossterm::terminal::disable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}

pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}
