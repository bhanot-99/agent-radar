pub mod theme;
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
    DataStreamCard {
        category: crate::events::ActivityCategory,
        started_at: Instant,
        last_updated: Instant,
        frame: u64,
    },
    ModelTrainingCard {
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
            HeroCardType::ModelTrainingCard { last_updated, .. } => {
                last_updated.elapsed() < Duration::from_secs(10)
            }
            HeroCardType::DataStreamCard { last_updated, .. } => {
                last_updated.elapsed() < Duration::from_secs(10)
            }
            HeroCardType::IdleCard => false,
        }
    }

    pub fn advance_frame(&mut self) {
        match self {
            HeroCardType::ModelTrainingCard { frame, .. } => *frame = frame.wrapping_add(1),
            HeroCardType::DataStreamCard { frame, .. } => *frame = frame.wrapping_add(1),
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
    pub active_agent: Option<String>,
    pub watch_path: String,
    pub is_budget_exceeded: bool,
}

impl TuiState {
    pub fn new(watch_path: &str, is_budget_exceeded: bool) -> Self {
        Self {
            events: VecDeque::with_capacity(500),
            max_events: 500,
            active_hero: Some(HeroCardType::IdleCard),
            fps_counter: FpsTracker::default(),
            system_metrics: SysStats::default(),
            active_agent: None,
            watch_path: watch_path.to_string(),
            is_budget_exceeded,
        }
    }

    pub fn push_event(&mut self, event: TelemetryEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_back();
        }

        if event.process_name != "background-io" {
            self.active_agent = Some(event.process_name.clone());
        }

        let now = Instant::now();

        // Check if event triggers a Hero Card
        match &event.category {
            crate::events::ActivityCategory::ModelTrainingCheckpoint { .. } => {
                self.active_hero = Some(HeroCardType::ModelTrainingCard {
                    category: event.category.clone(),
                    started_at: now,
                    last_updated: now,
                    frame: 0,
                });
            }
            crate::events::ActivityCategory::IncomingDataStream { .. } => {
                self.active_hero = Some(HeroCardType::DataStreamCard {
                    category: event.category.clone(),
                    started_at: now,
                    last_updated: now,
                    frame: 0,
                });
            }
            _ => {}
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
                // Animation completed; revert to quiet IdleCard
                *hero = HeroCardType::IdleCard;
                state_changed = true;
            }
        }
        state_changed
    }

    pub fn is_animating(&self) -> bool {
        self.active_hero
            .as_ref()
            .map(|h| h.is_animating())
            .unwrap_or(false)
    }

    pub fn update_sys_stats(&mut self, tracked_pids: usize) {
        self.system_metrics = read_self_sys_stats(tracked_pids);
    }
}

pub fn read_self_sys_stats(tracked_pids: usize) -> SysStats {
    let rss_bytes = read_self_rss().unwrap_or(0);
    SysStats {
        cpu_pct: 0.0,
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

pub fn render_hud(f: &mut Frame, state: &TuiState) {
    let size = f.area();

    // Background fill
    let bg_block = ratatui::widgets::Block::default().style(Style::default().bg(BG_BASE));
    f.render_widget(bg_block, size);

    // Main layout: Header (3), Main (min 10), Footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(size);

    // 1. Header Bar
    let header_widget = HeaderBarWidget {
        watch_path: &state.watch_path,
        active_agent: state.active_agent.as_deref(),
        is_budget_exceeded: state.is_budget_exceeded,
    };
    f.render_widget(header_widget, chunks[0]);

    // 2. Middle area: Split Hero Card (Left, ~38%) vs Log Feed (Right, ~62%)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(62),
        ])
        .split(chunks[1]);

    // Left: Hero Stream
    let hero_widget = HeroStreamWidget {
        active_hero: state.active_hero.as_ref(),
    };
    f.render_widget(hero_widget, middle_chunks[0]);

    // Right: Log Feed Table
    let log_widget = LogFeedTableWidget {
        events: &state.events,
    };
    f.render_widget(log_widget, middle_chunks[1]);

    // 3. Footer: Self resource telemetry
    render_footer(f, chunks[2], state);
}

fn render_footer(f: &mut Frame, area: Rect, state: &TuiState) {
    let rss_mb = state.system_metrics.rss_bytes as f64 / (1024.0 * 1024.0);
    let footer_text = format!(
        " AGENT-RADAR v0.1.0  |  RSS: {:.2} MB  |  CPU: ~{:.1}%  |  PIDS: {}  |  REDRAWS: {}  |  [Ctrl-C / SIGTERM to Exit]",
        rss_mb, state.system_metrics.cpu_pct, state.system_metrics.tracked_pids, state.fps_counter.frame_count
    );

    let footer_widget = ratatui::widgets::Paragraph::new(footer_text)
        .style(Style::default().fg(TEXT_DIM));
    f.render_widget(footer_widget, area);
}
