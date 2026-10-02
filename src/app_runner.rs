use std::io::{self, Stdout};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::Duration;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use rustix::event::epoll;
use rustix::time::{timerfd_create, timerfd_settime, Itimerspec, Timespec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags};

use crate::classifier::{SemanticClassifier, DEBOUNCE_WINDOW_MS};
use crate::correlator::ProcessCorrelator;
use crate::signals::{drain_signals, ReceivedSignal, SignalPipes};
use crate::tui::{render_hud, TuiState};
use crate::watcher::inotify_tier::InotifyWatcher;
use crate::watcher::EventSource;

pub const TAG_INOTIFY: u64 = 1;
pub const TAG_SIGNAL: u64 = 2;
pub const TAG_TIMER: u64 = 3;

pub struct AppRunner {
    epoll_fd: OwnedFd,
    timerfd: OwnedFd,
    signal_pipes: SignalPipes,
    pub watcher: InotifyWatcher,
    pub correlator: ProcessCorrelator,
    pub classifier: SemanticClassifier,
    pub tui_state: TuiState,
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    pub watch_path: PathBuf,
    is_timer_armed: bool,
    debug_mode: bool,
}

impl AppRunner {
    pub fn new(watch_path: &Path, debug_mode: bool) -> io::Result<Self> {
        let watcher = InotifyWatcher::new(watch_path)?;
        let signal_pipes = SignalPipes::new()?;
        let correlator = ProcessCorrelator::new();
        let classifier = SemanticClassifier::new();
        let is_budget_exceeded = watcher.is_budget_exceeded();
        let tui_state = TuiState::new(&watch_path.to_string_lossy(), is_budget_exceeded);

        let timerfd = timerfd_create(
            TimerfdClockId::Monotonic,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        ).map_err(io::Error::other)?;

        let epoll_fd = epoll::create(epoll::CreateFlags::CLOEXEC)
            .map_err(io::Error::other)?;

        // Register inotify fd
        epoll::add(
            &epoll_fd,
            unsafe { rustix::fd::BorrowedFd::borrow_raw(watcher.poll_fd()) },
            epoll::EventData::new_u64(TAG_INOTIFY),
            epoll::EventFlags::IN,
        ).map_err(io::Error::other)?;

        // Register signal pipe
        epoll::add(
            &epoll_fd,
            &signal_pipes.signal_read_fd,
            epoll::EventData::new_u64(TAG_SIGNAL),
            epoll::EventFlags::IN,
        ).map_err(io::Error::other)?;

        // Register timerfd
        epoll::add(
            &epoll_fd,
            &timerfd,
            epoll::EventData::new_u64(TAG_TIMER),
            epoll::EventFlags::IN,
        ).map_err(io::Error::other)?;

        let terminal = if !debug_mode {
            let mut stdout = io::stdout();
            crossterm::terminal::enable_raw_mode()?;
            if let Err(e) = crossterm::execute!(
                stdout,
                crossterm::terminal::EnterAlternateScreen,
                crossterm::cursor::Hide
            ) {
                let _ = crossterm::terminal::disable_raw_mode();
                return Err(e);
            }
            let backend = CrosstermBackend::new(stdout);
            match Terminal::new(backend) {
                Ok(t) => Some(t),
                Err(e) => {
                    crate::restore_terminal();
                    return Err(e);
                }
            }
        } else {
            None
        };

        Ok(Self {
            epoll_fd,
            timerfd,
            signal_pipes,
            watcher,
            correlator,
            classifier,
            tui_state,
            terminal,
            watch_path: watch_path.to_path_buf(),
            is_timer_armed: false,
            debug_mode,
        })
    }

    pub fn arm_timer(&mut self, interval_ms: u64) -> io::Result<()> {
        let sec = (interval_ms / 1000) as i64;
        let nsec = ((interval_ms % 1000) * 1_000_000) as i64;
        let spec = Itimerspec {
            it_interval: Timespec { tv_sec: sec, tv_nsec: nsec },
            it_value: Timespec { tv_sec: sec, tv_nsec: nsec },
        };
        timerfd_settime(&self.timerfd, TimerfdTimerFlags::empty(), &spec)
            .map_err(io::Error::other)?;
        self.is_timer_armed = true;
        Ok(())
    }

    pub fn disarm_timer(&mut self) -> io::Result<()> {
        if !self.is_timer_armed {
            return Ok(());
        }
        let spec = Itimerspec {
            it_interval: Timespec { tv_sec: 0, tv_nsec: 0 },
            it_value: Timespec { tv_sec: 0, tv_nsec: 0 },
        };
        timerfd_settime(&self.timerfd, TimerfdTimerFlags::empty(), &spec)
            .map_err(io::Error::other)?;
        self.is_timer_armed = false;
        Ok(())
    }

    pub fn drain_timer(&self) {
        let mut buf = [0u8; 8];
        let _ = rustix::io::read(&self.timerfd, &mut buf);
    }

    fn check_hero_animation_state(&mut self) -> io::Result<()> {
        if self.tui_state.is_animating() {
            if !self.is_timer_armed {
                // Arm timer for 250ms animation frames
                self.arm_timer(250)?;
            }
        } else if self.is_timer_armed {
            // Disarm timer to allow epoll to sleep at ~0% CPU
            self.disarm_timer()?;
        }
        Ok(())
    }

    fn redraw_if_needed(&mut self) -> io::Result<()> {
        if let Some(ref mut term) = self.terminal {
            let tracked = self.correlator.tracked_pid_count();
            self.tui_state.update_sys_stats(tracked);
            self.tui_state.fps_counter.frame_count += 1;
            term.draw(|f| render_hud(f, &self.tui_state))?;
        }
        self.check_hero_animation_state()?;
        Ok(())
    }

    pub fn run_loop(&mut self) -> io::Result<()> {
        let mut event_vec = epoll::EventVec::with_capacity(32);

        if self.debug_mode {
            eprintln!(
                "[INFO] AppRunner listening on {:?} (watches: {}, max: {})...",
                self.watch_path,
                self.watcher.active_watch_count(),
                self.watcher.max_watches()
            );
        } else {
            // Initial render
            self.redraw_if_needed()?;
        }

        loop {
            // Compute epoll timeout:
            // If debounce entries are pending, wait only until the earliest debounce deadline.
            // If hero card is animating, timerfd will wake epoll.
            // When idle, timeout is -1 (blocking indefinitely, ~0% idle CPU).
            let timeout_ms = if let Some(remaining) = self.classifier.time_until_next_flush(Duration::from_millis(DEBOUNCE_WINDOW_MS)) {
                remaining.as_millis().max(1) as i32
            } else {
                -1
            };

            match epoll::wait(&self.epoll_fd, &mut event_vec, timeout_ms) {
                Ok(()) => {}
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) => return Err(io::Error::other(e)),
            }

            let mut state_changed = false;

            for ev in &event_vec {
                match ev.data.u64() {
                    TAG_SIGNAL => {
                        let sigs = drain_signals(&self.signal_pipes.signal_read_fd);
                        for sig in sigs {
                            match sig {
                                ReceivedSignal::Shutdown => {
                                    if self.debug_mode {
                                        eprintln!("[INFO] Shutdown signal received, terminating cleanly...");
                                    }
                                    return Ok(());
                                }
                                ReceivedSignal::Resize => {
                                    if let Some(ref mut term) = self.terminal {
                                        term.autoresize()?;
                                    }
                                    state_changed = true;
                                }
                            }
                        }
                    }
                    TAG_TIMER => {
                        self.drain_timer();
                        if self.tui_state.tick_animation() {
                            state_changed = true;
                        }
                    }
                    TAG_INOTIFY => {
                        let raw_events = self.watcher.drain();
                        for raw in raw_events {
                            let enriched = self.correlator.correlate(raw);
                            if let Some(telemetry) = self.classifier.push_event(enriched) {
                                if self.debug_mode {
                                    eprintln!("[CLASSIFIED] {:?}", telemetry);
                                }
                                self.tui_state.push_event(telemetry);
                                state_changed = true;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Flush debounced events that have settled
            let flushed = self.classifier.flush_ready(Duration::from_millis(DEBOUNCE_WINDOW_MS));
            if !flushed.is_empty() {
                for te in flushed {
                    if self.debug_mode {
                        eprintln!("[CLASSIFIED DEBOUNCED] {:?}", te);
                    }
                    self.tui_state.push_event(te);
                }
                state_changed = true;
            }

            if state_changed {
                self.redraw_if_needed()?;
            }
        }
    }
}
