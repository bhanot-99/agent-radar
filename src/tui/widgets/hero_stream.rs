use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::events::ActivityCategory;
use crate::tui::HeroCardType;
use crate::tui::theme::{
    category_visual, AnimationPrimitive, ACCENT_IDLE, BORDER_DEFAULT, TEXT_DIM, TEXT_PRIMARY,
};

pub struct HeroStreamWidget<'a> {
    pub active_hero: Option<&'a HeroCardType>,
}

impl<'a> Widget for HeroStreamWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.active_hero {
            Some(HeroCardType::Active {
                category,
                started_at,
                frame,
                ..
            }) => {
                let visual = category_visual(category);
                let border_color = visual.color;

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(border_color))
                    .title(format!(" ╔═ {} ═╗ ", visual.caption))
                    .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD));

                let inner = block.inner(area);
                block.render(area, buf);

                if inner.height < 4 || inner.width < 10 {
                    return;
                }

                let elapsed = started_at.elapsed().as_secs();

                // 1. Caption Line with pulsing glyph
                let pulse_glyph = if frame % 2 == 0 {
                    visual.glyph_a
                } else {
                    visual.glyph_b
                };
                buf.set_string(
                    inner.x + 1,
                    inner.y + 1,
                    format!("{} {}", pulse_glyph, visual.caption),
                    Style::default().fg(border_color).add_modifier(Modifier::BOLD),
                );

                // 2. Target / Path Line (Unicode-safe truncated)
                let path_line = match category {
                    ActivityCategory::MassDeletion { count, sample_paths } => {
                        format!("Deleted: {} files ({})", count, sample_paths.join(", "))
                    }
                    ActivityCategory::SystemIdle => "System quiet".to_string(),
                    cat => format!("Path: {}", get_category_path(cat)),
                };
                let max_w = inner.width.saturating_sub(2) as usize;
                let safe_path = crate::tui::unicode_util::truncate_to_width(&path_line, max_w);
                buf.set_string(
                    inner.x + 1,
                    inner.y + 2,
                    safe_path,
                    Style::default().fg(TEXT_PRIMARY),
                );

                // 3. Stats Line
                let stats_line = match category {
                    ActivityCategory::ModelTrainingCheckpoint { size_bytes, .. } => {
                        let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);
                        format!("Size: {:.2} MB  |  Active: {}s", size_mb, elapsed)
                    }
                    ActivityCategory::IncomingDataStream {
                        bytes_per_sec,
                        progress_pct,
                        ..
                    } => {
                        let speed_kb = *bytes_per_sec as f64 / 1024.0;
                        if let Some(pct) = progress_pct {
                            format!(
                                "Speed: {:.1} KB/s  |  Progress: {}%  |  Active: {}s",
                                speed_kb, pct, elapsed
                            )
                        } else {
                            format!("Speed: {:.1} KB/s  |  Active: {}s", speed_kb, elapsed)
                        }
                    }
                    ActivityCategory::RustEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::PythonEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::WebEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::StyleEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::MarkupEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::ConfigEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::DocsEdit { lines_added, lines_removed, .. }
                    | ActivityCategory::ShellScriptEdit { lines_added, lines_removed, .. } => {
                        format!(
                            "Diff: +{}/-{} lines  |  Active: {}s",
                            lines_added, lines_removed, elapsed
                        )
                    }
                    ActivityCategory::MassDeletion { count, .. } => {
                        format!("Burst: {} deletions  |  Active: {}s", count, elapsed)
                    }
                    ActivityCategory::ImageAsset { .. }
                    | ActivityCategory::AudioAsset { .. }
                    | ActivityCategory::VideoAsset { .. }
                    | ActivityCategory::FontAsset { .. }
                    | ActivityCategory::NotebookActivity { .. }
                    | ActivityCategory::ModelConfigEdit { .. }
                    | ActivityCategory::ArchiveWrite { .. }
                    | ActivityCategory::GitOperation { .. }
                    | ActivityCategory::DependencyLockUpdate { .. }
                    | ActivityCategory::TestFileActivity { .. }
                    | ActivityCategory::EnvSecretChange { .. }
                    | ActivityCategory::CiPipelineEdit { .. }
                    | ActivityCategory::ContainerConfigEdit { .. }
                    | ActivityCategory::WorkspaceExpansion { .. }
                    | ActivityCategory::FileMutation { .. }
                    | ActivityCategory::SystemIdle => format!("Active: {}s", elapsed),
                };
                buf.set_string(
                    inner.x + 1,
                    inner.y + 3,
                    &stats_line,
                    Style::default().fg(TEXT_DIM),
                );

                // 4. Cyberpunk Animated Primitive Line
                if inner.height >= 5 {
                    let bar = render_primitive(
                        visual.primitive,
                        *frame as usize,
                        inner.width.saturating_sub(4) as usize,
                        visual.glyph_a,
                        visual.glyph_b,
                    );
                    buf.set_string(
                        inner.x + 1,
                        inner.y + 4,
                        &bar,
                        Style::default().fg(border_color),
                    );
                }
            }
            Some(HeroCardType::IdleCard) | None => {
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

fn get_category_path(category: &ActivityCategory) -> &str {
    match category {
        ActivityCategory::RustEdit { path, .. }
        | ActivityCategory::PythonEdit { path, .. }
        | ActivityCategory::WebEdit { path, .. }
        | ActivityCategory::StyleEdit { path, .. }
        | ActivityCategory::MarkupEdit { path, .. }
        | ActivityCategory::ConfigEdit { path, .. }
        | ActivityCategory::DocsEdit { path, .. }
        | ActivityCategory::ShellScriptEdit { path, .. }
        | ActivityCategory::ImageAsset { path }
        | ActivityCategory::AudioAsset { path }
        | ActivityCategory::VideoAsset { path }
        | ActivityCategory::FontAsset { path }
        | ActivityCategory::NotebookActivity { path }
        | ActivityCategory::ModelTrainingCheckpoint { path, .. }
        | ActivityCategory::ModelConfigEdit { path }
        | ActivityCategory::IncomingDataStream { path, .. }
        | ActivityCategory::ArchiveWrite { path }
        | ActivityCategory::GitOperation { path }
        | ActivityCategory::DependencyLockUpdate { path }
        | ActivityCategory::TestFileActivity { path }
        | ActivityCategory::EnvSecretChange { path }
        | ActivityCategory::CiPipelineEdit { path }
        | ActivityCategory::ContainerConfigEdit { path }
        | ActivityCategory::WorkspaceExpansion { path }
        | ActivityCategory::FileMutation { path, .. } => path,
        ActivityCategory::MassDeletion { .. } => "workspace",
        ActivityCategory::SystemIdle => "idle",
    }
}

fn render_primitive(
    primitive: AnimationPrimitive,
    frame: usize,
    width: usize,
    glyph_a: char,
    glyph_b: char,
) -> String {
    match primitive {
        AnimationPrimitive::PulseDot => build_pulse_dot(frame, width, glyph_a, glyph_b),
        AnimationPrimitive::FlowArrow => build_flow_arrow(frame, width, glyph_a, glyph_b),
        AnimationPrimitive::BounceBar => build_bounce_bar(frame, width, glyph_a, glyph_b),
        AnimationPrimitive::Spinner => build_spinner(frame, width, glyph_a, glyph_b),
        AnimationPrimitive::Wave => build_wave(frame, width, glyph_a, glyph_b),
    }
}

fn build_pulse_dot(frame: usize, width: usize, glyph_a: char, glyph_b: char) -> String {
    if width < 4 {
        return format!("[{}]", glyph_a);
    }
    let bar_len = width.min(30);
    let mut chars = vec![glyph_b; bar_len];
    let pos = frame % bar_len;
    chars[pos] = glyph_a;
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn build_flow_arrow(frame: usize, width: usize, glyph_a: char, _glyph_b: char) -> String {
    if width < 4 {
        return ">>>>".to_string();
    }
    let bar_len = width.min(30);
    let mut s = String::with_capacity(bar_len + 2);
    s.push('[');
    for i in 0..bar_len {
        if (i + frame).is_multiple_of(4) {
            s.push(glyph_a);
        } else {
            s.push('─');
        }
    }
    s.push(']');
    s
}

fn build_bounce_bar(frame: usize, width: usize, glyph_a: char, glyph_b: char) -> String {
    if width < 6 {
        return format!("[{}{}{}]", glyph_b, glyph_a, glyph_b);
    }
    let bar_len = width.min(30);
    let mut chars = vec![glyph_b; bar_len];
    let pos = frame % bar_len;
    chars[pos] = glyph_a;
    if pos > 0 {
        chars[pos - 1] = '▒';
    }
    if pos + 1 < bar_len {
        chars[pos + 1] = '▒';
    }
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn build_spinner(frame: usize, width: usize, _glyph_a: char, _glyph_b: char) -> String {
    let spinners = ['◐', '◓', '◑', '◒'];
    let spin = spinners[frame % 4];
    if width < 6 {
        return format!("[{}]", spin);
    }
    let bar_len = width.min(30);
    let mut s = String::with_capacity(bar_len + 2);
    s.push('[');
    for i in 0..bar_len {
        if (i + frame).is_multiple_of(6) {
            s.push(spin);
        } else {
            s.push('·');
        }
    }
    s.push(']');
    s
}

fn build_wave(frame: usize, width: usize, _glyph_a: char, _glyph_b: char) -> String {
    const WAVE_CHARS: [char; 16] = [
        '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' ',
    ];
    if width < 4 {
        return "~~~~".to_string();
    }
    let bar_len = width.min(30);
    let mut s = String::with_capacity(bar_len + 2);
    s.push('[');
    for i in 0..bar_len {
        let ch = WAVE_CHARS[(i + frame) % 16];
        s.push(ch);
    }
    s.push(']');
    s
}
