//! OptiScaler GUI (Rust rewrite) — GPU-rendered installer/manager for OptiScaler.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod fx;
mod ops;
mod screens;
mod state;
mod theme;

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
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
