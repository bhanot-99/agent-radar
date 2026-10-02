pub mod inotify_tier;

use std::os::fd::RawFd;
use inotify_tier::RawFsEvent;

pub trait EventSource {
    fn poll_fd(&self) -> RawFd;
    fn drain(&mut self) -> Vec<RawFsEvent>;
}
