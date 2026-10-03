use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::tui::scenes::{render_active_scene, render_idle_scene, SceneCanvas};
use crate::tui::HeroCardType;

/// The hero panel is deliberately borderless: every scene is a full-bleed
/// micro-dot field that runs edge to edge, with its HUD floating on top.
/// Containment is still structural -- `SceneCanvas` clamps every write to
/// `area`, so nothing can spill into the header or the log panel.
pub struct HeroStreamWidget<'a> {
    pub active_hero: Option<&'a HeroCardType>,
    pub idle_frame: u64,
}

impl<'a> Widget for HeroStreamWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 10 {
            return;
        }
        let mut canvas = SceneCanvas::new(buf, area);
        match self.active_hero {
            Some(HeroCardType::Active { category, started_at, frame, .. }) => {
                render_active_scene(&mut canvas, category, started_at, *frame);
            }
            Some(HeroCardType::IdleCard) | None => {
                render_idle_scene(&mut canvas, self.idle_frame);
            }
        }
    }
}
