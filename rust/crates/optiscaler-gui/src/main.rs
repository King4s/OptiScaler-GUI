//! OptiScaler GUI (Rust rewrite) — GPU-rendered installer/manager for OptiScaler.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod fx;
mod ops;
mod screens;
mod state;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    // Crash log for tester bug reports: any panic lands in logs/crash.log
    // next to the exe before the process dies.
    let logs_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        opticore::logging::write_crash_log(&logs_dir, info);
        previous_hook(info);
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OptiScaler GUI")
            .with_decorations(false)
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
