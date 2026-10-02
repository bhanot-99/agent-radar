use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::events::ActivityCategory;
use crate::tui::HeroCardType;
use crate::tui::theme::{
    ACCENT_CHECKPOINT, ACCENT_IDLE, ACCENT_INCOMING_DATA, BORDER_DEFAULT, TEXT_DIM, TEXT_PRIMARY,
};

pub struct HeroStreamWidget<'a> {
    pub active_hero: Option<&'a HeroCardType>,
}

impl<'a> Widget for HeroStreamWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.active_hero {
            Some(HeroCardType::ModelTrainingCard { category, started_at, frame, .. }) => {
                let border_color = ACCENT_CHECKPOINT;
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(border_color))
                    .title(" ╔═ MODEL TRAINING CHECKPOINT ═╗ ")
                    .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD));

                let inner = block.inner(area);
                block.render(area, buf);

                if inner.height < 4 || inner.width < 10 {
                    return;
                }

                if let ActivityCategory::ModelTrainingCheckpoint { path, size_bytes } = category {
                    let elapsed = started_at.elapsed().as_secs();
                    let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);

                    // Pulsing glyph based on frame
                    let pulse_icon = if frame % 2 == 0 { "●" } else { "◈" };
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 1,
                        format!("{} TENSOR FLUSH IN PROGRESS", pulse_icon),
                        Style::default().fg(border_color).add_modifier(Modifier::BOLD),
                    );

                    let path_line = format!("Path: {}", path);
                    let max_w = inner.width.saturating_sub(2) as usize;
                    let safe_path = crate::tui::unicode_util::truncate_to_width(&path_line, max_w);
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 2,
                        safe_path,
                        Style::default().fg(TEXT_PRIMARY),
                    );

                    let size_line = format!("Size: {:.2} MB  |  Active: {}s", size_mb, elapsed);
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 3,
                        &size_line,
                        Style::default().fg(TEXT_DIM),
                    );

                    // Cyberpunk animated pulse gauge
                    if inner.height >= 5 {
                        let bar = build_pulse_bar(*frame as usize, inner.width.saturating_sub(4) as usize);
                        buf.set_string(
                            inner.x + 1,
                            inner.y + 4,
                            &bar,
                            Style::default().fg(border_color),
                        );
                    }
                }
            }
            Some(HeroCardType::DataStreamCard { category, started_at, frame, .. }) => {
                let border_color = ACCENT_INCOMING_DATA;
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(border_color))
                    .title(" ╔═ INCOMING DATA STREAM ═╗ ")
                    .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD));

                let inner = block.inner(area);
                block.render(area, buf);

                if inner.height < 4 || inner.width < 10 {
                    return;
                }

                if let ActivityCategory::IncomingDataStream { path, bytes_per_sec, progress_pct } = category {
                    let elapsed = started_at.elapsed().as_secs();
                    let speed_kb = *bytes_per_sec as f64 / 1024.0;

                    let pulse_icon = if frame % 2 == 0 { "▼" } else { "▽" };
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 1,
                        format!("{} NETWORK DOWNLOAD ACTIVE", pulse_icon),
                        Style::default().fg(border_color).add_modifier(Modifier::BOLD),
                    );

                    let path_line = format!("Target: {}", path);
                    let max_w = inner.width.saturating_sub(2) as usize;
                    let safe_path = crate::tui::unicode_util::truncate_to_width(&path_line, max_w);
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 2,
                        safe_path,
                        Style::default().fg(TEXT_PRIMARY),
                    );

                    let speed_line = if let Some(pct) = progress_pct {
                        format!("Speed: {:.1} KB/s  |  Progress: {}%  |  Active: {}s", speed_kb, pct, elapsed)
                    } else {
                        format!("Speed: {:.1} KB/s  |  Active: {}s", speed_kb, elapsed)
                    };
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 3,
                        &speed_line,
                        Style::default().fg(TEXT_DIM),
                    );

                    // Cyberpunk streaming flow gauge
                    if inner.height >= 5 {
                        let bar = build_stream_bar(*frame as usize, inner.width.saturating_sub(4) as usize);
                        buf.set_string(
                            inner.x + 1,
                            inner.y + 4,
                            &bar,
                            Style::default().fg(border_color),
                        );
                    }
                }
            }
            _ => {
                // Static Idle Card
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(BORDER_DEFAULT))
                    .title(" SYSTEM RADAR: IDLE ")
                    .title_style(Style::default().fg(ACCENT_IDLE).add_modifier(Modifier::BOLD));

                let inner = block.inner(area);
                block.render(area, buf);

                if inner.height < 3 || inner.width < 10 {
                    return;
                }

                buf.set_string(
                    inner.x + 1,
                    inner.y + 1,
                    "○ SYSTEM QUIET",
                    Style::default().fg(ACCENT_IDLE),
                );

                buf.set_string(
                    inner.x + 1,
                    inner.y + 2,
                    "Waiting for agent activity...",
                    Style::default().fg(TEXT_DIM),
                );
            }
        }
    }
}

fn build_pulse_bar(frame: usize, width: usize) -> String {
    if width < 6 {
        return "░░░░".to_string();
    }
    let bar_len = width.min(30);
    let mut chars = vec!['░'; bar_len];
    let pos = frame % bar_len;
    chars[pos] = '█';
    if pos > 0 {
        chars[pos - 1] = '▓';
    }
    if pos + 1 < bar_len {
        chars[pos + 1] = '▓';
    }
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn build_stream_bar(frame: usize, width: usize) -> String {
    if width < 6 {
        return ">>>>".to_string();
    }
    let bar_len = width.min(30);
    let mut s = String::with_capacity(bar_len + 2);
    s.push('[');
    for i in 0..bar_len {
        if (i + frame).is_multiple_of(4) {
            s.push('►');
        } else {
            s.push('─');
        }
    }
    s.push(']');
    s
}
