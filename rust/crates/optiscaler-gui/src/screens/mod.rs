pub mod games_grid;

use crate::state::AppState;
use crate::theme;
use eframe::egui::{self, RichText};

pub fn show_settings(ctx: &egui::Context, _state: &mut AppState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Settings");
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Language, theme, effects toggle, cache management and more arrive in M5.",
            )
            .color(theme::TEXT_DIM),
        );
    });
}

pub fn show_log(ctx: &egui::Context, state: &mut AppState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Log");
            if ui.button("Copy all").clicked() {
                let all: String = state.log.iter().cloned().collect::<Vec<_>>().join("\n");
                ctx.copy_text(all);
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &state.log {
                    ui.label(RichText::new(line).monospace().size(12.0));
                }
            });
    });
}

pub fn show_about(ctx: &egui::Context, _state: &mut AppState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("OptiScaler GUI");
        ui.label(format!("Version {}", opticore::VERSION));
        ui.add_space(8.0);
        ui.label("An unofficial installer and manager for OptiScaler.");
        ui.label(
            RichText::new(
                "All upscaling technology is the work of the OptiScaler team — this app only installs and manages it.",
            )
            .color(theme::TEXT_DIM),
        );
        ui.add_space(8.0);
        ui.hyperlink_to("OptiScaler (official project)", "https://github.com/optiscaler/OptiScaler");
        ui.hyperlink_to("OptiScaler-GUI on GitHub", "https://github.com/King4s/OptiScaler-GUI");
    });
}
