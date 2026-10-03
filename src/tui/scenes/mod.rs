mod group_a_code;
mod group_b_media;
mod group_c_aiml;
mod group_d_network;
mod group_e_process;
mod group_f_structural;
pub mod dots;

use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::events::ActivityCategory;
use crate::tui::theme::category_visual;
use crate::tui::unicode_util::truncate_to_width;
use self::dots::{DotField, Palette};

/// Every scene (full or compact) attempts the full multi-line layout once
/// the panel is at least this big; below it, every category falls back to
/// the shared compact view.
pub const MIN_HERO_SIZE: (u16, u16) = (46, 12);

/// A category's visual identity: the flavor data (caption/color/glyphs)
/// carried over verbatim from `theme.rs`. Each category now renders its own
/// bespoke scene (see `group_*` modules) rather than sharing one of a
/// handful of generic templates.
pub struct CategorySkin {
    pub caption: &'static str,
    pub color: ratatui::style::Color,
    pub glyph_a: char,
    pub glyph_b: char,
    pub label: &'static str,
}

pub fn category_skin(cat: &ActivityCategory) -> CategorySkin {
    let visual = category_visual(cat);
    CategorySkin {
        caption: visual.caption,
        color: visual.color,
        glyph_a: visual.glyph_a,
        glyph_b: visual.glyph_b,
        label: visual.label,
    }
}

/// Whether a scene has a real, measured completion fraction (only
/// `IncomingDataStream` with `progress_pct: Some(_)` does) or must show
/// honest indeterminate motion instead of a fabricated percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressKind {
    Determinate { pct: u8 },
    Indeterminate,
}

/// Real stat data for a scene: elapsed time (always available), whether
/// progress is measured, and an ordered list of extra `(label, value)` rows
/// for whatever numeric fields the category actually carries. Never
/// contains a fabricated number.
pub struct SceneStats {
    pub elapsed_secs: u64,
    pub progress: ProgressKind,
    pub rows: Vec<(&'static str, String)>,
}

pub fn build_scene_stats(category: &ActivityCategory, started_at: Instant) -> SceneStats {
    use ActivityCategory::*;

    let elapsed_secs = started_at.elapsed().as_secs();
    let mut rows = Vec::new();

    let progress = match category {
        IncomingDataStream { bytes_per_sec, progress_pct, .. } => {
            let speed_kb = *bytes_per_sec as f64 / 1024.0;
            rows.push(("Speed", format!("{:.1} KB/s", speed_kb)));
            match progress_pct {
                Some(pct) => ProgressKind::Determinate { pct: *pct },
                None => ProgressKind::Indeterminate,
            }
        }
        ModelTrainingCheckpoint { size_bytes, .. } => {
            let size_mb = *size_bytes as f64 / (1024.0 * 1024.0);
            rows.push(("Size", format!("{:.2} MB", size_mb)));
            ProgressKind::Indeterminate
        }
        RustEdit { lines_added, lines_removed, .. }
        | PythonEdit { lines_added, lines_removed, .. }
        | WebEdit { lines_added, lines_removed, .. }
        | StyleEdit { lines_added, lines_removed, .. }
        | MarkupEdit { lines_added, lines_removed, .. }
        | ConfigEdit { lines_added, lines_removed, .. }
        | DocsEdit { lines_added, lines_removed, .. }
        | ShellScriptEdit { lines_added, lines_removed, .. } => {
            rows.push(("Diff", format!("+{} / -{}", lines_added, lines_removed)));
            ProgressKind::Indeterminate
        }
        MassDeletion { count, .. } => {
            rows.push(("Count", format!("{} files", count)));
            ProgressKind::Indeterminate
        }
        FileMutation { op, .. } => {
            let op_label = match op {
                crate::events::FileOp::Created => "CREATED",
                crate::events::FileOp::Modified => "MODIFIED",
                crate::events::FileOp::Deleted => "DELETED",
                crate::events::FileOp::Renamed => "RENAMED",
            };
            rows.push(("Op", op_label.to_string()));
            ProgressKind::Indeterminate
        }
        _ => ProgressKind::Indeterminate,
    };

    SceneStats { elapsed_secs, progress, rows }
}

/// Gets the raw source path for a category, or a full-sentence synthetic
/// description for the 2 categories with no single path (`MassDeletion`,
/// `SystemIdle`). Callers that want a labeled line (e.g. "File: {path}",
/// "Source: {path}") add their own label -- this never prefixes one itself,
/// so labels never double up.
pub fn category_path_line(category: &ActivityCategory) -> String {
    use ActivityCategory::*;
    match category {
        MassDeletion { count, sample_paths } => {
            format!("Deleted: {} files ({})", count, sample_paths.join(", "))
        }
        SystemIdle => "System quiet".to_string(),
        RustEdit { path, .. } | PythonEdit { path, .. } | WebEdit { path, .. }
        | StyleEdit { path, .. } | MarkupEdit { path, .. } | ConfigEdit { path, .. }
        | DocsEdit { path, .. } | ShellScriptEdit { path, .. }
        | ImageAsset { path } | AudioAsset { path } | VideoAsset { path } | FontAsset { path }
        | NotebookActivity { path } | ModelTrainingCheckpoint { path, .. } | ModelConfigEdit { path }
        | IncomingDataStream { path, .. } | ArchiveWrite { path } | GitOperation { path }
        | DependencyLockUpdate { path } | TestFileActivity { path } | EnvSecretChange { path }
        | CiPipelineEdit { path } | ContainerConfigEdit { path } | WorkspaceExpansion { path }
        | FileMutation { path, .. } => path.clone(),
    }
}

/// A drawing surface scoped to exactly one panel's interior rect. Every
/// write is clamped to `inner`'s own width/height before it reaches the
/// buffer, so a scene can never bleed into a neighboring panel or across
/// its own border -- the guarantee is structural, not per-call-site
/// discipline. Scene code never touches `Buffer::set_string` directly.
pub struct SceneCanvas<'a> {
    buf: &'a mut Buffer,
    inner: Rect,
}

impl<'a> SceneCanvas<'a> {
    pub fn new(buf: &'a mut Buffer, inner: Rect) -> Self {
        Self { buf, inner }
    }

    pub fn width(&self) -> u16 {
        self.inner.width
    }

    pub fn height(&self) -> u16 {
        self.inner.height
    }

    /// Writes `text` at local (col, row), relative to the panel's own
    /// interior top-left. No-ops (never panics) if the row is outside the
    /// panel, and truncates to whatever width remains on that row.
    pub fn put_line(&mut self, col: u16, row: u16, text: &str, style: Style) {
        if row >= self.inner.height || col >= self.inner.width {
            return;
        }
        let x = self.inner.x + col;
        let max_w = (self.inner.width - col) as usize;
        let safe = truncate_to_width(text, max_w);
        self.buf.set_string(x, self.inner.y + row, safe, style);
    }

    /// Writes one character cell at local (col, row); no-op outside the panel.
    pub fn put_char(&mut self, col: u16, row: u16, ch: char, style: Style) {
        if row >= self.inner.height || col >= self.inner.width {
            return;
        }
        self.buf[(self.inner.x + col, self.inner.y + row)].set_char(ch).set_style(style);
    }

    pub fn put_line_centered(&mut self, row: u16, text: &str, style: Style) {
        use unicode_width::UnicodeWidthStr;
        let text_w = text.width() as u16;
        let col = (self.inner.width.saturating_sub(text_w)) / 2;
        self.put_line(col, row, text, style);
    }

    /// Places a multi-line connected shape (e.g. a box, a plunger shaft) as
    /// one unit: the column is computed once from the widest line and
    /// shared by every row, so characters meant to line up vertically
    /// (a box's left edge, an assembly's spine) actually do -- unlike
    /// calling `put_line_centered` per row, which re-centers each line
    /// independently and drifts whenever line lengths differ.
    pub fn put_block_centered(&mut self, start_row: u16, lines: &[(&str, Style)]) {
        use unicode_width::UnicodeWidthStr;
        let max_w = lines.iter().map(|(s, _)| s.width() as u16).max().unwrap_or(0);
        let col = self.inner.width.saturating_sub(max_w) / 2;
        for (i, (text, style)) in lines.iter().enumerate() {
            self.put_line(col, start_row + i as u16, text, *style);
        }
    }
}

fn would_render_full(min_size: (u16, u16), width: u16, height: u16) -> bool {
    width >= min_size.0 && height >= min_size.1
}

/// Milliseconds per animation frame (~25 fps). The scenes are continuous
/// functions of time, so this only sets smoothness, not speed.
pub const ANIM_FRAME_MS: u64 = 40;

pub fn frame_time(frame: u64) -> f32 {
    (frame as f64 * ANIM_FRAME_MS as f64 / 1000.0) as f32
}

/// Everything a full scene shader may read: time, a per-file seed (so two
/// different files of the same category never look identical), and the
/// real stats (only ever used to *shape* visuals, never to invent numbers).
pub struct Ctx<'a> {
    pub t: f32,
    pub seed: u32,
    pub stats: &'a SceneStats,
    pub path: &'a str,
}

type SceneFn = fn(&mut DotField, &Ctx);

fn scene_for(category: &ActivityCategory) -> Option<SceneFn> {
    use ActivityCategory::*;
    Some(match category {
        RustEdit { .. } => group_a_code::rust_edit,
        PythonEdit { .. } => group_a_code::python_edit,
        WebEdit { .. } => group_a_code::web_edit,
        StyleEdit { .. } => group_a_code::style_edit,
        MarkupEdit { .. } => group_a_code::markup_edit,
        ConfigEdit { .. } => group_a_code::config_edit,
        DocsEdit { .. } => group_a_code::docs_edit,
        ShellScriptEdit { .. } => group_a_code::shell_script_edit,

        ImageAsset { .. } => group_b_media::image_asset,
        AudioAsset { .. } => group_b_media::audio_asset,
        VideoAsset { .. } => group_b_media::video_asset,
        FontAsset { .. } => group_b_media::font_asset,
        NotebookActivity { .. } => group_b_media::notebook_activity,

        ModelTrainingCheckpoint { .. } => group_c_aiml::model_training_checkpoint,
        ModelConfigEdit { .. } => group_c_aiml::model_config_edit,

        IncomingDataStream { .. } => group_d_network::incoming_data_stream,
        ArchiveWrite { .. } => group_d_network::archive_write,

        GitOperation { .. } => group_e_process::git_operation,
        DependencyLockUpdate { .. } => group_e_process::dependency_lock_update,
        TestFileActivity { .. } => group_e_process::test_file_activity,
        EnvSecretChange { .. } => group_e_process::env_secret_change,
        CiPipelineEdit { .. } => group_e_process::ci_pipeline_edit,
        ContainerConfigEdit { .. } => group_e_process::container_config_edit,

        WorkspaceExpansion { .. } => group_f_structural::workspace_expansion,
        MassDeletion { .. } => group_f_structural::mass_deletion,
        FileMutation { .. } => group_f_structural::file_mutation,

        SystemIdle => return None,
    })
}

fn path_seed(path: &str) -> u32 {
    // FNV-1a: stable across runs, so a given file always gets the same art.
    path.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

/// Renders a category's full-bleed dot scene into a fresh field sized to the
/// whole canvas and returns the field (exposed so tests can measure it).
pub fn render_scene_field(category: &ActivityCategory, stats: &SceneStats, path: &str, frame: u64, cols: u16, rows: u16) -> DotField {
    let mut field = DotField::new(cols as usize * 2, rows as usize * 4);
    let t = frame_time(frame);
    let seed = path_seed(path);
    match scene_for(category) {
        Some(scene) => scene(&mut field, &Ctx { t, seed, stats, path }),
        None => group_f_structural::system_idle(&mut field, t),
    }
    dots::dedsec_fx(&mut field, t, seed);
    field
}

/// Renders an active category's scene into `canvas`: a full-bleed animated
/// micro-dot field with the DedSec HUD floating on top; small panels get the
/// shared compact fallback.
pub fn render_active_scene(canvas: &mut SceneCanvas, category: &ActivityCategory, started_at: &Instant, frame: u64) {
    let skin = category_skin(category);
    let stats = build_scene_stats(category, *started_at);
    let path = category_path_line(category);

    if !would_render_full(MIN_HERO_SIZE, canvas.width(), canvas.height()) {
        render_generic_compact(canvas, &skin, &stats, &path, frame);
        return;
    }

    let field = render_scene_field(category, &stats, &path, frame, canvas.width(), canvas.height());
    dots::blit(&field, canvas, &Palette::dedsec(skin.color));
    draw_hud(canvas, &skin, &stats, &path, frame);
}

const GLITCH_CHARS: &[char] = &['#', '%', '&', '@', '$', '/', '\\', '<', '>', '0', '1', '?', '!'];

/// Corrupts a fraction of characters during a glitch burst.
fn glitch_text(s: &str, t: f32, seed: u32) -> String {
    let burst = dots::glitch_burst(t, seed);
    if burst <= 0.0 {
        return s.to_string();
    }
    let fr = (t * 25.0) as u32;
    s.chars()
        .enumerate()
        .map(|(i, ch)| {
            let h = dots::hash(seed ^ fr ^ (i as u32).wrapping_mul(0x9E37_79B9));
            if ch != ' ' && dots::rnd(h) < 0.25 * burst {
                GLITCH_CHARS[(h as usize >> 9) % GLITCH_CHARS.len()]
            } else {
                ch
            }
        })
        .collect()
}

fn draw_hud(c: &mut SceneCanvas, skin: &CategorySkin, stats: &SceneStats, path: &str, frame: u64) {
    use crate::tui::theme::{TEXT_DIM, TEXT_PRIMARY};
    use ratatui::style::Modifier;
    use unicode_width::UnicodeWidthStr;

    let t = frame_time(frame);
    let seed = path_seed(path);
    let bg = crate::tui::theme::BG_BASE;
    let accent = Style::default().fg(skin.color).bg(bg).add_modifier(Modifier::BOLD);
    let hot = Style::default().fg(ratatui::style::Color::Rgb(0xFF, 0x2A, 0x6D)).bg(bg).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(TEXT_DIM).bg(bg);
    let primary = Style::default().fg(TEXT_PRIMARY).bg(bg);
    let burst = dots::glitch_burst(t, seed) > 0.0;

    let title = format!(" {} {} {} ", skin.glyph_a, glitch_text(skin.caption, t, seed), skin.glyph_b);
    c.put_line(0, 0, "▚▞", if burst { hot } else { accent });
    c.put_line(2, 0, &title, if burst { hot } else { accent });

    let rec = if frame % 25 < 13 { "◉" } else { "○" };
    let tag = format!(" DEDSEC//{} {} LIVE ", skin.label, rec);
    let tag_w = tag.width() as u16;
    if c.width() > tag_w + title.width() as u16 + 4 {
        c.put_line(c.width() - tag_w, 0, &tag, dim);
        c.put_line(c.width() - 7, 0, rec, hot);
    }

    let h = c.height();
    c.put_line(0, h.saturating_sub(2), &format!(" ▶ {} ", path), primary);

    let mut parts: Vec<String> = stats
        .rows
        .iter()
        .map(|(label, value)| format!("{}: {}", label.to_uppercase(), value))
        .collect();
    parts.push(format!("T+{}s", stats.elapsed_secs));
    let mut line = format!(" {} ", parts.join("  ░  "));
    if let ProgressKind::Determinate { pct } = stats.progress {
        let remaining = c.width().saturating_sub(line.width() as u16 + 2).min(40);
        if remaining >= 12 {
            line.push_str(&render_progress_bar_text(remaining, pct));
            line.push(' ');
        }
    }
    c.put_line(0, h.saturating_sub(1), &line, accent);
}

/// Shared compact fallback for every category on a small panel.
pub fn render_generic_compact(
    canvas: &mut SceneCanvas,
    skin: &CategorySkin,
    stats: &SceneStats,
    path: &str,
    frame: u64,
) {
    let pulse = if frame % 2 == 0 { skin.glyph_a } else { skin.glyph_b };
    canvas.put_line(0, 0, &format!("{} {}", pulse, skin.caption), Style::default().fg(skin.color));
    canvas.put_line(0, 1, &format!("Path: {}", path), Style::default().fg(crate::tui::theme::TEXT_PRIMARY));

    let line2 = match stats.progress {
        ProgressKind::Determinate { pct } => render_progress_bar_text(canvas.width().saturating_sub(2), pct),
        ProgressKind::Indeterminate => {
            if let Some((label, value)) = stats.rows.first() {
                format!("{}: {}", label, value)
            } else {
                format!("Active: {}s", stats.elapsed_secs)
            }
        }
    };
    canvas.put_line(0, 2, &line2, Style::default().fg(crate::tui::theme::TEXT_DIM));
}

pub fn render_progress_bar_text(width: u16, pct: u8) -> String {
    let width = width.max(4) as usize;
    let bar_w = width.saturating_sub(6); // room for " NNN%"
    let filled = (bar_w as u32 * pct.min(100) as u32 / 100) as usize;
    let filled = filled.min(bar_w);
    format!(
        "[{}{}] {:>3}%",
        "#".repeat(filled),
        "-".repeat(bar_w - filled),
        pct
    )
}

/// `SystemIdle` -- CELESTIAL-RADAR: a dotted planet under a full-panel radar
/// sweep, shown while nothing is happening. Narrow/short panels get the
/// original 2-line quiet notice.
pub fn render_idle_scene(canvas: &mut SceneCanvas, frame: u64) {
    let idle = Style::default().fg(crate::tui::theme::ACCENT_IDLE);
    let dim = Style::default().fg(crate::tui::theme::TEXT_DIM);

    if !would_render_full(MIN_HERO_SIZE, canvas.width(), canvas.height()) {
        canvas.put_line(0, 0, "○ SYSTEM QUIET", idle);
        canvas.put_line(0, 1, "Waiting for agent activity...", dim);
        return;
    }

    let stats = SceneStats { elapsed_secs: 0, progress: ProgressKind::Indeterminate, rows: Vec::new() };
    let field = render_scene_field(&ActivityCategory::SystemIdle, &stats, "", frame, canvas.width(), canvas.height());
    dots::blit(&field, canvas, &Palette::dedsec(ratatui::style::Color::Rgb(0x2F, 0xD6, 0xA0)));

    let bg = crate::tui::theme::BG_BASE;
    let label = Style::default().fg(crate::tui::theme::TEXT_DIM).bg(bg);
    let blink = if frame % 25 < 13 { "◉" } else { "○" };
    canvas.put_line(0, 0, &format!(" ○ SYSTEM QUIET  ░  STANDBY (awaiting agent activity) {} ", blink), label);
    let h = canvas.height();
    canvas.put_line(0, h - 1, " SENTINEL ONLINE  ░  0 ALERTS  ░  SWEEP 360°  ░  DEDSEC//IDLE ", label);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_would_render_full_flips_at_documented_boundary() {
        assert!(!would_render_full((44, 14), 43, 20));
        assert!(!would_render_full((44, 14), 60, 13));
        assert!(would_render_full((44, 14), 44, 14));
        assert!(would_render_full((44, 14), 80, 24));
    }
}
