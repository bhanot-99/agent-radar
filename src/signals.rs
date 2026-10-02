use std::io;
use std::os::fd::{AsRawFd, OwnedFd};
use rustix::pipe::{pipe_with, PipeFlags};
use signal_hook::consts::{SIGINT, SIGTERM, SIGWINCH};
use signal_hook::low_level::pipe::register;

pub struct SignalPipes {
    pub signal_read_fd: OwnedFd,
    _signal_write_fd: OwnedFd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivedSignal {
    Resize,
    Shutdown,
}

impl SignalPipes {
    pub fn new() -> io::Result<Self> {
        let (signal_read_fd, signal_write_fd) = pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        register(SIGWINCH, signal_write_fd.as_raw_fd())?;
        register(SIGTERM, signal_write_fd.as_raw_fd())?;
        register(SIGINT, signal_write_fd.as_raw_fd())?;

        Ok(Self {
            signal_read_fd,
            _signal_write_fd: signal_write_fd,
        })
    }
}

pub fn drain_signals(fd: &OwnedFd) -> Vec<ReceivedSignal> {
    let mut buf = [0u8; 64];
    let mut sigs = Vec::new();
    loop {
        match rustix::io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for &b in &buf[..n] {
                    match b as i32 {
                        SIGWINCH => sigs.push(ReceivedSignal::Resize),
                        SIGTERM | SIGINT => sigs.push(ReceivedSignal::Shutdown),
                        _ => {}
                    }
                }
            }
            Err(rustix::io::Errno::AGAIN) => break,
            Err(_) => break,
        }
    }
    sigs
}
