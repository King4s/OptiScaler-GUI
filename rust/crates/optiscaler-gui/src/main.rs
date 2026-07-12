//! OptiScaler GUI (Rust rewrite) — M0 scaffold: dark GPU-rendered window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OptiScaler GUI")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OptiScaler GUI",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(App))
        }),
    )
}

struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OptiScaler GUI");
            ui.label(format!(
                "M0 scaffold — GPU-rendered via wgpu — core {}",
                opticore::VERSION
            ));
        });
    }
}
