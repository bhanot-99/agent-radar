pub mod scenes;
pub mod theme;
pub mod unicode_util;
pub mod widgets {
    pub mod header_bar;
    pub mod hero_stream;
    pub mod log_feed_table;
}

use std::collections::VecDeque;
use std::fs;
use std::time::{Duration, Instant};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::Frame;

use crate::events::TelemetryEvent;
use self::theme::{BG_BASE, TEXT_DIM};
use self::widgets::header_bar::HeaderBarWidget;
use self::widgets::hero_stream::HeroStreamWidget;
use self::widgets::log_feed_table::LogFeedTableWidget;

#[derive(Debug, Clone)]
pub enum HeroCardType {
    Active {
        category: crate::events::ActivityCategory,
        started_at: Instant,
        last_updated: Instant,
        frame: u64,
    },
    IdleCard,
}

impl HeroCardType {
    pub fn is_animating(&self) -> bool {
        match self {
            HeroCardType::Active { last_updated, .. } => {
                last_updated.elapsed() < Duration::from_secs(10)
            }
            HeroCardType::IdleCard => false,
        }
    }

    pub fn advance_frame(&mut self) {
        match self {
            HeroCardType::Active { frame, .. } => *frame = frame.wrapping_add(1),
            HeroCardType::IdleCard => {}
        }
    }
}

#[derive(Debug, Default)]
pub struct FpsTracker {
    pub frame_count: u64,
}

#[derive(Debug, Default, Clone)]
pub struct SysStats {
    pub cpu_pct: f32,
    pub rss_bytes: u64,
    pub tracked_pids: usize,
}

pub struct TuiState {
    pub events: VecDeque<TelemetryEvent>,
    pub max_events: usize,
    pub active_hero: Option<HeroCardType>,
    pub fps_counter: FpsTracker,
    pub system_metrics: SysStats,
    pub cpu_tracker: CpuTracker,
    pub active_agent: Option<String>,
    pub watch_path: String,
    pub is_budget_exceeded: bool,
    /// Frame counter for the idle sentinel scene, which animates too.
    pub idle_frame: u64,
}

#[derive(Debug, Clone)]
pub struct CpuTracker {
    last_sample_time: Instant,
    last_cpu_ticks: u64,
    current_cpu_pct: f64,
}

impl Default for CpuTracker {
    fn default() -> Self {
        Self {
            last_sample_time: Instant::now(),
            last_cpu_ticks: read_self_cpu_ticks().unwrap_or(0),
            current_cpu_pct: 0.0,
        }
    }
}

impl CpuTracker {
    pub fn sample(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();
        if elapsed >= 0.2 {
            if let Some(ticks) = read_self_cpu_ticks() {
                let delta_ticks = ticks.saturating_sub(self.last_cpu_ticks);
                // System clock ticks per second on Linux is 100 (CLK_TCK)
                let cpu = (delta_ticks as f64 / 100.0) / elapsed * 100.0;
                self.current_cpu_pct = cpu.clamp(0.0, 100.0);
                self.last_cpu_ticks = ticks;
                self.last_sample_time = now;
            }
        }
        self.current_cpu_pct
    }
}

fn read_self_cpu_ticks() -> Option<u64> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    let close_paren = stat.rfind(')')?;
    let after_paren = stat[close_paren + 1..].trim_start();
    let fields: Vec<&str> = after_paren.split_whitespace().collect();
    // After '(comm)' field 2:
    // field 14 (utime) is index 11
    // field 15 (stime) is index 12
    if fields.len() > 12 {
        let utime = fields[11].parse::<u64>().ok()?;
        let stime = fields[12].parse::<u64>().ok()?;
        Some(utime + stime)
    } else {
        None
    }
}

impl TuiState {
    pub fn new(watch_path: &str, is_budget_exceeded: bool) -> Self {
        let mut state = Self {
            events: VecDeque::with_capacity(500),
            max_events: 500,
            active_hero: Some(HeroCardType::IdleCard),
            fps_counter: FpsTracker::default(),
            system_metrics: SysStats::default(),
            cpu_tracker: CpuTracker::default(),
            active_agent: None,
            watch_path: watch_path.to_string(),
            is_budget_exceeded,
            idle_frame: 0,
        };
        state.push_event(TelemetryEvent {
            timestamp: std::time::SystemTime::now(),
            pid: std::process::id(),
            ppid: 0,
            process_name: "system".to_string(),
            category: crate::events::ActivityCategory::SystemIdle,
        });
        state
    }

    pub fn push_event(&mut self, event: TelemetryEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_back();
        }

        if event.process_name != "background-io" && event.process_name != "system" {
            self.active_agent = Some(event.process_name.clone());
        }

        let now = Instant::now();

        // Check if event triggers a Hero Card, preserving started_at across updates of the same category
        if event.category != crate::events::ActivityCategory::SystemIdle {
            if let Some(HeroCardType::Active {
                ref mut category,
                ref mut last_updated,
                ..
            }) = self.active_hero {
                if std::mem::discriminant(category) == std::mem::discriminant(&event.category) {
                    *category = event.category.clone();
                    *last_updated = now;
                } else {
                    self.active_hero = Some(HeroCardType::Active {
                        category: event.category.clone(),
                        started_at: now,
                        last_updated: now,
                        frame: 0,
                    });
                }
            } else {
                self.active_hero = Some(HeroCardType::Active {
                    category: event.category.clone(),
                    started_at: now,
                    last_updated: now,
                    frame: 0,
                });
            }
        }

        self.events.push_front(event);
    }

    pub fn tick_animation(&mut self) -> bool {
        let mut state_changed = false;
        if let Some(ref mut hero) = self.active_hero {
            if hero.is_animating() {
                hero.advance_frame();
                state_changed = true;
            } else if !matches!(hero, HeroCardType::IdleCard) {
                // Animation completed; revert to quiet IdleCard and record SystemIdle event
                *hero = HeroCardType::IdleCard;
                self.push_event(TelemetryEvent {
                    timestamp: std::time::SystemTime::now(),
                    pid: std::process::id(),
                    ppid: 0,
                    process_name: "system".to_string(),
                    category: crate::events::ActivityCategory::SystemIdle,
                });
                state_changed = true;
            } else {
                // The idle radar keeps sweeping.
                self.idle_frame = self.idle_frame.wrapping_add(1);
                state_changed = true;
            }
        }
        state_changed
    }

    /// Whether the frame timer should run. Every hero scene -- including the
    /// idle sentinel -- is a continuously moving dot animation.
    pub fn wants_frames(&self) -> bool {
        self.active_hero.is_some()
    }

    pub fn is_animating(&self) -> bool {
        self.active_hero
            .as_ref()
            .map(|h| h.is_animating())
            .unwrap_or(false)
    }

    pub fn update_sys_stats(&mut self, tracked_pids: usize) {
        let cpu_pct = self.cpu_tracker.sample() as f32;
        let rss_bytes = read_self_rss().unwrap_or(0);
        self.system_metrics = SysStats {
            cpu_pct,
            rss_bytes,
            tracked_pids,
        };
    }
}

pub fn read_self_sys_stats(tracked_pids: usize) -> SysStats {
    let mut tracker = CpuTracker::default();
    let cpu_pct = tracker.sample() as f32;
    let rss_bytes = read_self_rss().unwrap_or(0);
    SysStats {
        cpu_pct,
        rss_bytes,
        tracked_pids,
    }
}

fn read_self_rss() -> Option<u64> {
    let content = fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>().ok()?;
                return Some(kb * 1024);
            }
        }
    }
    None
}

/// The four fixed regions of the HUD for a given terminal size: header and
/// footer are fixed-height; hero (top, ~70%) and log (bottom, ~30%) stack
/// vertically and are both full terminal width. Exposed so tests can assert
/// exact widget placement without duplicating the layout math.
pub struct HudRegions {
    pub header: Rect,
    pub hero: Rect,
    pub log: Rect,
    pub footer: Rect,
}

pub fn compute_regions(size: Rect) -> HudRegions {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(size);

    let middle_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(chunks[1]);

    HudRegions {
        header: chunks[0],
        hero: middle_chunks[0],
        log: middle_chunks[1],
        footer: chunks[2],
    }
}

pub fn render_hud(f: &mut Frame, state: &TuiState) {
    let size = f.area();

    // Background fill
    let bg_block = ratatui::widgets::Block::default().style(Style::default().bg(BG_BASE));
    f.render_widget(bg_block, size);

    let regions = compute_regions(size);

    // 1. Header Bar
    let header_widget = HeaderBarWidget {
        watch_path: &state.watch_path,
        active_agent: state.active_agent.as_deref(),
        is_budget_exceeded: state.is_budget_exceeded,
    };
    f.render_widget(header_widget, regions.header);

    // 2. Hero Stream (top, ~70%, full width)
    let hero_widget = HeroStreamWidget {
        active_hero: state.active_hero.as_ref(),
        idle_frame: state.idle_frame,
    };
    f.render_widget(hero_widget, regions.hero);

    // 3. Log Feed Table (bottom, ~30%, full width)
    let log_widget = LogFeedTableWidget {
        events: &state.events,
    };
    f.render_widget(log_widget, regions.log);

    // 4. Footer: Self resource telemetry
    render_footer(f, regions.footer, state);
}

fn render_footer(f: &mut Frame, area: Rect, state: &TuiState) {
    let rss_mb = state.system_metrics.rss_bytes as f64 / (1024.0 * 1024.0);
    let footer_text = format!(
        " AGENT-RADAR v0.1.0  |  RSS: {:.2} MB  |  CPU: ~{:.1}%  |  PIDS: {}  |  REDRAWS: {}  |  [q / Ctrl-C to Exit]",
        rss_mb, state.system_metrics.cpu_pct, state.system_metrics.tracked_pids, state.fps_counter.frame_count
    );

    let footer_widget = ratatui::widgets::Paragraph::new(footer_text)
        .style(Style::default().fg(TEXT_DIM));
    f.render_widget(footer_widget, area);
}
