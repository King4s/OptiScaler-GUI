//! Theme: dark-first gaming palette with a light variant, applied to egui
//! visuals and exposed as a palette for custom-painted widgets (cards, badges).

use eframe::egui::{self, Color32, CornerRadius, Stroke};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub accent: Color32,
    pub text_dim: Color32,
    pub badge_platform: Color32,
    pub badge_ok: Color32,
    pub badge_warn: Color32,
    pub badge_danger: Color32,
}

pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x0e, 0x11, 0x16),
    panel: Color32::from_rgb(0x15, 0x1a, 0x21),
    card: Color32::from_rgb(0x1b, 0x22, 0x2b),
    card_hover: Color32::from_rgb(0x23, 0x2c, 0x38),
    accent: Color32::from_rgb(0x4f, 0x9d, 0xff),
    text_dim: Color32::from_rgb(0x8a, 0x94, 0xa3),
    badge_platform: Color32::from_rgb(0x2c, 0x36, 0x44),
    badge_ok: Color32::from_rgb(0x1f, 0x6f, 0x43),
    badge_warn: Color32::from_rgb(0x8a, 0x6d, 0x1a),
    badge_danger: Color32::from_rgb(0x8a, 0x2a, 0x2a),
};

pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xf2, 0xf4, 0xf7),
    panel: Color32::from_rgb(0xe8, 0xec, 0xf1),
    card: Color32::from_rgb(0xff, 0xff, 0xff),
    card_hover: Color32::from_rgb(0xdd, 0xe6, 0xf2),
    accent: Color32::from_rgb(0x1f, 0x6f, 0xd4),
    text_dim: Color32::from_rgb(0x5a, 0x64, 0x73),
    badge_platform: Color32::from_rgb(0x51, 0x5e, 0x70),
    badge_ok: Color32::from_rgb(0x1f, 0x6f, 0x43),
    badge_warn: Color32::from_rgb(0x9a, 0x74, 0x0f),
    badge_danger: Color32::from_rgb(0xa3, 0x2a, 0x2a),
};

pub fn palette(dark: bool) -> Palette {
    if dark {
        DARK
    } else {
        LIGHT
    }
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let pal = palette(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = pal.panel;
    visuals.window_fill = pal.bg;
    visuals.extreme_bg_color = pal.bg;
    visuals.faint_bg_color = pal.card;
    visuals.selection.bg_fill = pal.accent.linear_multiply(0.35);
    visuals.hyperlink_color = pal.accent;
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, pal.accent);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}
