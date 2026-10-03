use ratatui::style::Color;
use crate::events::ActivityCategory;

pub const BG_BASE: Color = Color::Rgb(0x0A, 0x0E, 0x14);
pub const BG_PANEL: Color = Color::Rgb(0x10, 0x15, 0x1F);
pub const BORDER_DEFAULT: Color = Color::Rgb(0x2A, 0x33, 0x44);
pub const BORDER_ACTIVE: Color = Color::Rgb(0x00, 0xF0, 0xFF);
pub const TEXT_PRIMARY: Color = Color::Rgb(0xD8, 0xE4, 0xF0);
pub const TEXT_DIM: Color = Color::Rgb(0x5C, 0x6B, 0x80);

pub const ACCENT_INCOMING_DATA: Color = Color::Rgb(0x00, 0xF0, 0xFF);
pub const ACCENT_CHECKPOINT: Color = Color::Rgb(0xB2, 0x4B, 0xFF);
pub const ACCENT_SOURCE_CODE: Color = Color::Rgb(0x39, 0xFF, 0x88);
pub const ACCENT_WORKSPACE: Color = Color::Rgb(0xFF, 0xC2, 0x47);
pub const ACCENT_FILE_MUTATION: Color = Color::Rgb(0x7A, 0x8B, 0xA6);
pub const ACCENT_IDLE: Color = Color::Rgb(0x3A, 0x42, 0x54);
pub const ACCENT_WARNING: Color = Color::Rgb(0xFF, 0x5A, 0x3C);

#[derive(Debug, Clone)]
pub struct CategoryVisual {
    pub glyph_a: char,
    pub glyph_b: char,
    pub caption: &'static str,
    pub label: &'static str,
    pub color: Color,
}

pub fn category_visual(cat: &ActivityCategory) -> CategoryVisual {
    match cat {
        // Group A — Code & Docs
        ActivityCategory::RustEdit { .. } => CategoryVisual {
            glyph_a: '⚡',
            glyph_b: '✦',
            caption: "NEURAL INK FLOWING",
            label: "RUST_EDIT",
            color: Color::Rgb(0x39, 0xFF, 0x88),
        },
        ActivityCategory::PythonEdit { .. } => CategoryVisual {
            glyph_a: '≈',
            glyph_b: '~',
            caption: "PYTHON ON THE PROWL",
            label: "PYTHON_EDIT",
            color: Color::Rgb(0xC9, 0xD9, 0x4A),
        },
        ActivityCategory::WebEdit { .. } => CategoryVisual {
            glyph_a: '✦',
            glyph_b: '✧',
            caption: "WEB SPUN TIGHTER",
            label: "WEB_EDIT",
            color: Color::Rgb(0xFF, 0xD1, 0x66),
        },
        ActivityCategory::StyleEdit { .. } => CategoryVisual {
            glyph_a: '❖',
            glyph_b: '◇',
            caption: "PIXELS GETTING PAINTED",
            label: "STYLE_EDIT",
            color: Color::Rgb(0xFF, 0x6F, 0xB5),
        },
        ActivityCategory::MarkupEdit { .. } => CategoryVisual {
            glyph_a: '▦',
            glyph_b: '▧',
            caption: "SCAFFOLDING RAISED",
            label: "MARKUP_EDIT",
            color: Color::Rgb(0xE8, 0x70, 0x2A),
        },
        ActivityCategory::ConfigEdit { .. } => CategoryVisual {
            glyph_a: '⚙',
            glyph_b: '✲',
            caption: "BOLTS BEING TIGHTENED",
            label: "CONFIG_EDIT",
            color: Color::Rgb(0x9A, 0xA7, 0xB8),
        },
        ActivityCategory::DocsEdit { .. } => CategoryVisual {
            glyph_a: '✎',
            glyph_b: '✏',
            caption: "SCRIBE AT WORK",
            label: "DOCS_EDIT",
            color: Color::Rgb(0xD8, 0xC2, 0x8A),
        },
        ActivityCategory::ShellScriptEdit { .. } => CategoryVisual {
            glyph_a: '$',
            glyph_b: '❯',
            caption: "SHELL INCANTATION CAST",
            label: "SHELL_EDIT",
            color: Color::Rgb(0x5C, 0xF0, 0xC2),
        },

        // Group B — Data & Media
        ActivityCategory::ImageAsset { .. } => CategoryVisual {
            glyph_a: '▣',
            glyph_b: '▢',
            caption: "CANVAS SPLATTERED",
            label: "IMAGE_ASSET",
            color: Color::Rgb(0xFF, 0x8A, 0xD8),
        },
        ActivityCategory::AudioAsset { .. } => CategoryVisual {
            glyph_a: '♪',
            glyph_b: '♫',
            caption: "SOUNDWAVE RIPPLING",
            label: "AUDIO_ASSET",
            color: Color::Rgb(0x8A, 0xD8, 0xFF),
        },
        ActivityCategory::VideoAsset { .. } => CategoryVisual {
            glyph_a: '▶',
            glyph_b: '▷',
            caption: "REEL ROLLING",
            label: "VIDEO_ASSET",
            color: Color::Rgb(0xB5, 0x8A, 0xFF),
        },
        ActivityCategory::FontAsset { .. } => CategoryVisual {
            glyph_a: 'Æ',
            glyph_b: 'æ',
            caption: "GLYPHS FORGED",
            label: "FONT_ASSET",
            color: Color::Rgb(0xFF, 0xD9, 0x8A),
        },
        ActivityCategory::NotebookActivity { .. } => CategoryVisual {
            glyph_a: '≡',
            glyph_b: '≣',
            caption: "LAB NOTEBOOK SCRIBBLED",
            label: "NOTEBOOK",
            color: Color::Rgb(0x8A, 0xFF, 0xC2),
        },

        // Group C — AI/ML
        ActivityCategory::ModelTrainingCheckpoint { .. } => CategoryVisual {
            glyph_a: '●',
            glyph_b: '◈',
            caption: "TENSOR FLUSH IN PROGRESS",
            label: "CHECKPOINT",
            color: Color::Rgb(0xB2, 0x4B, 0xFF),
        },
        ActivityCategory::ModelConfigEdit { .. } => CategoryVisual {
            glyph_a: '≈',
            glyph_b: '∿',
            caption: "SYNAPSES REWIRED",
            label: "MODEL_CONFIG",
            color: Color::Rgb(0xD4, 0x8B, 0xFF),
        },

        // Group D — Network / Transfer
        ActivityCategory::IncomingDataStream { .. } => CategoryVisual {
            glyph_a: '▼',
            glyph_b: '▽',
            caption: "SIGNAL BEING SUCKED DOWN",
            label: "DATA_STREAM",
            color: Color::Rgb(0x00, 0xF0, 0xFF),
        },
        ActivityCategory::ArchiveWrite { .. } => CategoryVisual {
            glyph_a: '▪',
            glyph_b: '▫',
            caption: "BOX TAPED SHUT",
            label: "ARCHIVE_WRITE",
            color: Color::Rgb(0x4F, 0xA8, 0xE8),
        },

        // Group E — Process / Action-based
        ActivityCategory::GitOperation { .. } => CategoryVisual {
            glyph_a: 'Y',
            glyph_b: '⑂',
            caption: "TIMELINE BRANCHING",
            label: "GIT_OP",
            color: Color::Rgb(0xFF, 0x8C, 0x42),
        },
        ActivityCategory::DependencyLockUpdate { .. } => CategoryVisual {
            glyph_a: '≡',
            glyph_b: '◆',
            caption: "ANCHOR CHAIN RATTLING",
            label: "LOCK_UPDATE",
            color: Color::Rgb(0x6B, 0x8C, 0xFF),
        },
        ActivityCategory::TestFileActivity { .. } => CategoryVisual {
            glyph_a: '■',
            glyph_b: '□',
            caption: "BUG HUNT IN PROGRESS",
            label: "TEST_ACTIVITY",
            color: Color::Rgb(0xFF, 0xA1, 0x3C),
        },
        ActivityCategory::EnvSecretChange { .. } => CategoryVisual {
            glyph_a: '▓',
            glyph_b: '▒',
            caption: "VAULT DOOR CREAKING",
            label: "ENV_SECRET",
            color: Color::Rgb(0xFF, 0x3C, 0x6E),
        },
        ActivityCategory::CiPipelineEdit { .. } => CategoryVisual {
            glyph_a: '⚙',
            glyph_b: '⟲',
            caption: "ROBOT ARMS RECALIBRATED",
            label: "CI_PIPELINE",
            color: Color::Rgb(0x8A, 0x8A, 0xFF),
        },
        ActivityCategory::ContainerConfigEdit { .. } => CategoryVisual {
            glyph_a: '▢',
            glyph_b: '◫',
            caption: "WHALE SURFACING",
            label: "CONTAINER_CFG",
            color: Color::Rgb(0x3C, 0x9A, 0xFF),
        },

        // Group F — Structural
        ActivityCategory::WorkspaceExpansion { .. } => CategoryVisual {
            glyph_a: '▲',
            glyph_b: '△',
            caption: "NEW WING UNDER CONSTRUCTION",
            label: "EXPANSION",
            color: Color::Rgb(0xFF, 0xC2, 0x47),
        },
        ActivityCategory::MassDeletion { .. } => CategoryVisual {
            glyph_a: '✕',
            glyph_b: '×',
            caption: "CONTROLLED DEMOLITION",
            label: "MASS_DELETION",
            color: Color::Rgb(0xFF, 0x2E, 0x2E),
        },
        ActivityCategory::FileMutation { .. } => CategoryVisual {
            glyph_a: '•',
            glyph_b: '◦',
            caption: "SOMETHING STIRRED",
            label: "FILE_MUTATION",
            color: Color::Rgb(0x7A, 0x8B, 0xA6),
        },

        // System state
        ActivityCategory::SystemIdle => CategoryVisual {
            glyph_a: '○',
            glyph_b: ' ',
            caption: "SYSTEM QUIET",
            label: "IDLE",
            color: Color::Rgb(0x3A, 0x42, 0x54),
        },
    }
}

pub fn category_color(cat: &ActivityCategory) -> Color {
    category_visual(cat).color
}

pub fn category_label(cat: &ActivityCategory) -> &'static str {
    category_visual(cat).label
}
