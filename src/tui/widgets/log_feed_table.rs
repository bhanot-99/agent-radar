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

        if inner.height < 2 || inner.width < 20 {
            return;
        }

        // Header line
        let header = " TIME     AGENT / PID          CATEGORY         DETAILS";
        let header_len = header.len().min(inner.width as usize);
        buf.set_string(
            inner.x,
            inner.y,
            &header[..header_len],
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
            buf.set_string(inner.x + 1, row_y, &time_str, Style::default().fg(TEXT_DIM));

            // 2. Agent / PID
            let agent_str = if event.pid > 0 {
                format!("{} ({})", event.process_name, event.pid)
            } else {
                event.process_name.clone()
            };
            let agent_pad = format!("{:<20}", &agent_str);
            let agent_len = agent_pad.len().min(20);
            buf.set_string(inner.x + 11, row_y, &agent_pad[..agent_len], Style::default().fg(TEXT_PRIMARY));

            // 3. Category
            let cat_name = category_label(&event.category);
            let cat_color = category_color(&event.category);
            let cat_pad = format!("{:<16}", cat_name);
            let cat_len = cat_pad.len().min(16);
            buf.set_string(
                inner.x + 32,
                row_y,
                &cat_pad[..cat_len],
                Style::default().fg(cat_color).add_modifier(Modifier::BOLD),
            );

            // 4. Details
            let details_x = inner.x + 49;
            let available_width = inner.width.saturating_sub(50) as usize;
            if available_width > 0 {
                let details_str = format_details(&event.category);
                let details_len = details_str.len().min(available_width);
                buf.set_string(
                    details_x,
                    row_y,
                    &details_str[..details_len],
                    Style::default().fg(TEXT_PRIMARY),
                );
            }
        }
    }
}

fn format_details(category: &ActivityCategory) -> String {
    match category {
        ActivityCategory::SourceCodeMutation { path, lines_added, lines_removed } => {
            if *lines_added > 0 || *lines_removed > 0 {
                format!("{} (+{} / -{})", path, lines_added, lines_removed)
            } else {
                path.clone()
            }
        }
        ActivityCategory::ModelTrainingCheckpoint { path, size_bytes } => {
            let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);
            format!("{} ({:.2} MB)", path, size_mb)
        }
        ActivityCategory::IncomingDataStream { path, bytes_per_sec, .. } => {
            let kb_sec = *bytes_per_sec as f64 / 1024.0;
            format!("{} ({:.1} KB/s)", path, kb_sec)
        }
        ActivityCategory::WorkspaceExpansion { path } => {
            format!("new directory: {}", path)
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
