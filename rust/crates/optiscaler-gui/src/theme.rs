//! Dark gaming theme: custom palette on top of egui's dark visuals.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

pub const BG: Color32 = Color32::from_rgb(0x0e, 0x11, 0x16);
pub const PANEL: Color32 = Color32::from_rgb(0x15, 0x1a, 0x21);
pub const CARD: Color32 = Color32::from_rgb(0x1b, 0x22, 0x2b);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x23, 0x2c, 0x38);
pub const ACCENT: Color32 = Color32::from_rgb(0x4f, 0x9d, 0xff);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x94, 0xa3);

pub const BADGE_PLATFORM: Color32 = Color32::from_rgb(0x2c, 0x36, 0x44);
pub const BADGE_OK: Color32 = Color32::from_rgb(0x1f, 0x6f, 0x43);
pub const BADGE_WARN: Color32 = Color32::from_rgb(0x8a, 0x6d, 0x1a);
pub const BADGE_DANGER: Color32 = Color32::from_rgb(0x8a, 0x2a, 0x2a);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = CARD;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}
