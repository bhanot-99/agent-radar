use std::collections::VecDeque;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::events::{ActivityCategory, FileOp, TelemetryEvent};
use crate::tui::theme::{category_color, category_label, BG_PANEL, BORDER_DEFAULT, TEXT_DIM, TEXT_PRIMARY};

pub struct LogFeedTableWidget<'a> {
    pub events: &'a VecDeque<TelemetryEvent>,
}

/// Column geometry for the event log table, computed from the panel's
/// current interior width so the header and data rows can never drift out
/// of alignment or overflow past the panel's own border.
struct Columns {
    time_x: u16,
    agent_x: u16,
    agent_w: u16,
    category_x: u16,
    category_w: u16,
    details_x: u16,
    details_w: u16,
}

const TIME_W: u16 = 8; // "HH:MM:SS"
const GAP: u16 = 1;
const AGENT_MIN: u16 = 8;
const AGENT_PREF: u16 = 20;
const CATEGORY_MIN: u16 = 8;
const CATEGORY_PREF: u16 = 16;
const DETAILS_MIN: u16 = 10;

fn compute_columns(inner: Rect) -> Option<Columns> {
    let after_time = inner.width.checked_sub(1 + TIME_W + GAP)?;
    if after_time < AGENT_MIN + GAP + CATEGORY_MIN {
        return None;
    }

    let agent_w = after_time
        .saturating_sub(GAP + CATEGORY_MIN)
        .min(AGENT_PREF)
        .max(AGENT_MIN);
    let after_agent = after_time.saturating_sub(agent_w + GAP);
    let category_w = after_agent.min(CATEGORY_PREF).max(CATEGORY_MIN.min(after_agent));
    let after_category = after_agent.saturating_sub(category_w + GAP);
    let details_w = if after_category >= DETAILS_MIN { after_category } else { 0 };

    let time_x = inner.x + 1;
    let agent_x = time_x + TIME_W + GAP;
    let category_x = agent_x + agent_w + GAP;
    let details_x = category_x + category_w + GAP;

    Some(Columns {
        time_x,
        agent_x,
        agent_w,
        category_x,
        category_w,
        details_x,
        details_w,
    })
}

impl<'a> Widget for LogFeedTableWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_DEFAULT))
            .style(Style::default().bg(BG_PANEL))
            .title(" REAL-TIME EVENT LOG ")
            .title_style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 {
            return;
        }
        let Some(cols) = compute_columns(inner) else {
            return;
        };

        // Header line, built from the same column geometry as the data rows
        // so they can never misalign.
        let mut header = String::new();
        header.push_str(&crate::tui::unicode_util::pad_or_truncate("TIME", TIME_W as usize));
        header.push(' ');
        header.push_str(&crate::tui::unicode_util::pad_or_truncate("AGENT / PID", cols.agent_w as usize));
        header.push(' ');
        header.push_str(&crate::tui::unicode_util::pad_or_truncate("CATEGORY", cols.category_w as usize));
        if cols.details_w > 0 {
            header.push(' ');
            header.push_str(&crate::tui::unicode_util::pad_or_truncate("DETAILS", cols.details_w as usize));
        }
        let safe_header = crate::tui::unicode_util::truncate_to_width(&header, inner.width as usize);
        buf.set_string(
            inner.x,
            inner.y,
            safe_header,
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD),
        );

        let max_visible_rows = (inner.height - 1) as usize;
        let rows_to_show = self.events.len().min(max_visible_rows);

        for (idx, event) in self.events.iter().take(rows_to_show).enumerate() {
            let row_y = inner.y + 1 + idx as u16;

            // 1. Time
            let dur = event.timestamp.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let secs = dur.as_secs();
            let time_str = format!("{:02}:{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60);
            buf.set_string(cols.time_x, row_y, &time_str, Style::default().fg(TEXT_DIM));

            // 2. Agent / PID
            let agent_str = if event.pid > 0 {
                format!("{} ({})", event.process_name, event.pid)
            } else {
                event.process_name.clone()
            };
            let safe_agent = crate::tui::unicode_util::pad_or_truncate(&agent_str, cols.agent_w as usize);
            buf.set_string(cols.agent_x, row_y, &safe_agent, Style::default().fg(TEXT_PRIMARY));

            // 3. Category
            let cat_name = category_label(&event.category);
            let cat_color = category_color(&event.category);
            let safe_cat = crate::tui::unicode_util::pad_or_truncate(cat_name, cols.category_w as usize);
            buf.set_string(
                cols.category_x,
                row_y,
                &safe_cat,
                Style::default().fg(cat_color).add_modifier(Modifier::BOLD),
            );

            // 4. Details (hidden entirely, not crushed, below DETAILS_MIN)
            if cols.details_w > 0 {
                let details_str = format_details(&event.category);
                let safe_details = crate::tui::unicode_util::truncate_to_width(&details_str, cols.details_w as usize);
                buf.set_string(
                    cols.details_x,
                    row_y,
                    safe_details,
                    Style::default().fg(TEXT_PRIMARY),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compute_columns;
    use ratatui::layout::Rect;

    #[test]
    fn test_compute_columns_ranges_are_ordered_and_fit_within_inner() {
        for width in (20..=200u16).step_by(5) {
            let inner = Rect::new(0, 0, width, 10);
            let Some(cols) = compute_columns(inner) else {
                continue;
            };

            assert!(cols.time_x < cols.agent_x, "width {}: time_x {} should precede agent_x {}", width, cols.time_x, cols.agent_x);
            assert!(
                cols.agent_x + cols.agent_w <= cols.category_x,
                "width {}: agent column [{}, {}) overlaps category_x {}",
                width, cols.agent_x, cols.agent_x + cols.agent_w, cols.category_x
            );
            assert!(
                cols.category_x + cols.category_w <= cols.details_x,
                "width {}: category column [{}, {}) overlaps details_x {}",
                width, cols.category_x, cols.category_x + cols.category_w, cols.details_x
            );
            if cols.details_w > 0 {
                assert!(
                    cols.details_x + cols.details_w <= inner.right(),
                    "width {}: details column [{}, {}) exceeds inner.right() {}",
                    width, cols.details_x, cols.details_x + cols.details_w, inner.right()
                );
            }
        }
    }
}

fn format_details(category: &ActivityCategory) -> String {
    match category {
        ActivityCategory::RustEdit { path, lines_added, lines_removed }
        | ActivityCategory::PythonEdit { path, lines_added, lines_removed }
        | ActivityCategory::WebEdit { path, lines_added, lines_removed }
        | ActivityCategory::StyleEdit { path, lines_added, lines_removed }
        | ActivityCategory::MarkupEdit { path, lines_added, lines_removed }
        | ActivityCategory::ConfigEdit { path, lines_added, lines_removed }
        | ActivityCategory::DocsEdit { path, lines_added, lines_removed }
        | ActivityCategory::ShellScriptEdit { path, lines_added, lines_removed } => {
            if *lines_added > 0 || *lines_removed > 0 {
                format!("{} (+{} / -{})", path, lines_added, lines_removed)
            } else {
                path.clone()
            }
        }
        ActivityCategory::ImageAsset { path }
        | ActivityCategory::AudioAsset { path }
        | ActivityCategory::VideoAsset { path }
        | ActivityCategory::FontAsset { path }
        | ActivityCategory::NotebookActivity { path }
        | ActivityCategory::ModelConfigEdit { path }
        | ActivityCategory::ArchiveWrite { path }
        | ActivityCategory::GitOperation { path }
        | ActivityCategory::DependencyLockUpdate { path }
        | ActivityCategory::TestFileActivity { path }
        | ActivityCategory::EnvSecretChange { path }
        | ActivityCategory::CiPipelineEdit { path }
        | ActivityCategory::ContainerConfigEdit { path } => path.clone(),
        ActivityCategory::ModelTrainingCheckpoint { path, size_bytes } => {
            let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);
            format!("{} ({:.2} MB)", path, size_mb)
        }
        ActivityCategory::IncomingDataStream { path, bytes_per_sec, progress_pct } => {
            let kb_sec = *bytes_per_sec as f64 / 1024.0;
            if let Some(pct) = progress_pct {
                format!("{} ({:.1} KB/s, {}%)", path, kb_sec, pct)
            } else {
                format!("{} ({:.1} KB/s)", path, kb_sec)
            }
        }
        ActivityCategory::WorkspaceExpansion { path } => {
            format!("new directory: {}", path)
        }
        ActivityCategory::MassDeletion { count, sample_paths } => {
            format!("deleted {} files ({})", count, sample_paths.join(", "))
        }
        ActivityCategory::FileMutation { path, op } => {
            let op_str = match op {
                FileOp::Created => "created",
                FileOp::Modified => "modified",
                FileOp::Deleted => "deleted",
                FileOp::Renamed => "renamed",
            };
            format!("{} ({})", path, op_str)
        }
        ActivityCategory::SystemIdle => "idle".to_string(),
    }
}
