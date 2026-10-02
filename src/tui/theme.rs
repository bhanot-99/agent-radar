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

pub fn category_color(cat: &ActivityCategory) -> Color {
    match cat {
        ActivityCategory::IncomingDataStream { .. } => ACCENT_INCOMING_DATA,
        ActivityCategory::ModelTrainingCheckpoint { .. } => ACCENT_CHECKPOINT,
        ActivityCategory::SourceCodeMutation { .. } => ACCENT_SOURCE_CODE,
        ActivityCategory::WorkspaceExpansion { .. } => ACCENT_WORKSPACE,
        ActivityCategory::FileMutation { .. } => ACCENT_FILE_MUTATION,
        ActivityCategory::SystemIdle => ACCENT_IDLE,
    }
}

pub fn category_label(cat: &ActivityCategory) -> &'static str {
    match cat {
        ActivityCategory::IncomingDataStream { .. } => "DATA_STREAM",
        ActivityCategory::ModelTrainingCheckpoint { .. } => "CHECKPOINT",
        ActivityCategory::SourceCodeMutation { .. } => "CODE_MUTATION",
        ActivityCategory::WorkspaceExpansion { .. } => "EXPANSION",
        ActivityCategory::FileMutation { .. } => "FILE_MUTATION",
        ActivityCategory::SystemIdle => "IDLE",
    }
}
