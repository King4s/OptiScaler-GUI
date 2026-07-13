//! Window chrome for the frameless window: a slim top strip with a drag
//! area and minimize/maximize/close buttons, plus border resize zones.

use crate::theme::Palette;
use eframe::egui::{self, Align, CursorIcon, Layout, RichText, Sense, ViewportCommand};

pub const TOP_STRIP_HEIGHT: f32 = 30.0;
const RESIZE_MARGIN: f32 = 6.0;

/// Slim custom title strip: drag-to-move, double-click to maximize,
/// window buttons on the right.
pub fn top_strip(ctx: &egui::Context, pal: Palette, title: &str) {
    egui::TopBottomPanel::top("window_chrome")
        .exact_height(TOP_STRIP_HEIGHT)
        .frame(egui::Frame::new().fill(pal.bg))
        .show(ctx, |ui| {
            let strip_rect = ui.max_rect();

            // Register the drag area FIRST so the window buttons added after it
            // sit on top in egui's hit test; registered last, it swallows their clicks.
            let drag_response = ui.interact(
                strip_rect,
                egui::Id::new("window_drag_area"),
                Sense::click_and_drag(),
            );
            if drag_response.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            if drag_response.double_clicked() {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }

            ui.horizontal_centered(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new(title).size(12.0).strong().color(pal.text_dim));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let button = |text: &str| {
                        egui::Button::new(RichText::new(text).size(13.0))
                            .min_size(egui::vec2(34.0, TOP_STRIP_HEIGHT - 4.0))
                            .frame(false)
                    };
                    if ui.add(button("✕")).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    if ui.add(button(if maximized { "🗗" } else { "🗖" })).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if ui.add(button("🗕")).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                });
            });
        });
}

/// Border resize zones for the frameless window (E/W/S edges + corners;
/// the top edge belongs to the drag strip).
pub fn handle_resize(ctx: &egui::Context) {
    use egui::viewport::ResizeDirection;
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let screen = ctx.content_rect();
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else {
        return;
    };
    // The title strip owns the top of the window (drag + window buttons);
    // an east-edge resize zone there would steal clicks from the ✕ button.
    if pos.y <= screen.min.y + TOP_STRIP_HEIGHT {
        return;
    }
    let east = pos.x >= screen.max.x - RESIZE_MARGIN;
    let west = pos.x <= screen.min.x + RESIZE_MARGIN;
    let south = pos.y >= screen.max.y - RESIZE_MARGIN;
    let direction = match (east, west, south) {
        (true, _, true) => Some(ResizeDirection::SouthEast),
        (_, true, true) => Some(ResizeDirection::SouthWest),
        (true, _, false) => Some(ResizeDirection::East),
        (_, true, false) => Some(ResizeDirection::West),
        (false, false, true) => Some(ResizeDirection::South),
        _ => None,
    };
    let Some(direction) = direction else { return };

    let cursor = match direction {
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
        ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
        _ => CursorIcon::Default,
    };
    ctx.set_cursor_icon(cursor);
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}
