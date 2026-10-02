use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::tui::theme::{BORDER_DEFAULT, TEXT_DIM, TEXT_PRIMARY, ACCENT_SOURCE_CODE, ACCENT_WARNING};

pub struct HeaderBarWidget<'a> {
    pub watch_path: &'a str,
    pub active_agent: Option<&'a str>,
    pub is_budget_exceeded: bool,
}

impl<'a> Widget for HeaderBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_DEFAULT))
            .title(" AGENT-RADAR // HUD ")
            .title_style(Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let agent_label = self.active_agent.unwrap_or("SCANNING");
        let agent_color = if self.active_agent.is_some() {
            ACCENT_SOURCE_CODE
        } else {
            TEXT_DIM
        };

        let status_text = if self.is_budget_exceeded {
            " [WATCH_BUDGET_OVERRUN] "
        } else {
            " [ONLINE] "
        };
        let status_color = if self.is_budget_exceeded {
            ACCENT_WARNING
        } else {
            ACCENT_SOURCE_CODE
        };

        // Render left: Watching path
        let left_str = format!(" WATCH: {} ", self.watch_path);
        let left_len = left_str.len().min(inner.width as usize);
        buf.set_string(
            inner.x,
            inner.y,
            &left_str[..left_len],
            Style::default().fg(TEXT_PRIMARY),
        );

        // Render center/agent
        let agent_str = format!(" AGENT: {} ", agent_label);
        let agent_x = inner.x + (inner.width / 2).saturating_sub((agent_str.len() / 2) as u16);
        if agent_x > inner.x + left_len as u16 && agent_x + (agent_str.len() as u16) < inner.x + inner.width {
            buf.set_string(
                agent_x,
                inner.y,
                &agent_str,
                Style::default().fg(agent_color).add_modifier(Modifier::BOLD),
            );
        }

        // Render right: Status
        let status_len = status_text.len() as u16;
        if inner.width > status_len {
            let right_x = inner.x + inner.width - status_len;
            buf.set_string(
                right_x,
                inner.y,
                status_text,
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            );
        }
    }
}
