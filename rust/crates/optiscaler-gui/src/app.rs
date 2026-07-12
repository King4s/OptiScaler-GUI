//! Top-level app: event draining, sidebar navigation, screen routing.

use crate::ops::Ops;
use crate::screens;
use crate::state::{AppState, ArtState, ScanState, Screen};
use crate::theme;
use eframe::egui::{self, RichText};
use opticore::progress::TaskEvent;

pub struct App {
    state: AppState,
    ops: Ops,
    started: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self {
            state: AppState::default(),
            ops: Ops::new(),
            started: false,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.ops.rx.try_recv() {
            match event {
                TaskEvent::ScanFinished { mut games } => {
                    games.sort_by(|a, b| a.key.name_lower.cmp(&b.key.name_lower));
                    self.state.games = games;
                    self.state.scan_state = ScanState::Done;
                    self.ops.scan_finished();
                }
                TaskEvent::ImageReady {
                    path_norm,
                    image_path,
                } => {
                    self.state
                        .art
                        .insert(path_norm, ArtState::Ready(image_path));
                }
                TaskEvent::ImageMissing { path_norm } => {
                    self.state.art.insert(path_norm, ArtState::Missing);
                }
                TaskEvent::AppListReady => {
                    // Retry artwork that had no appid before the catalogue loaded
                    self.ops.clear_inflight();
                    self.state
                        .art
                        .retain(|_, art_state| *art_state != ArtState::Missing);
                    self.state
                        .push_log("App list ready — retrying missing artwork".into());
                }
                TaskEvent::Log(line) => self.state.push_log(line),
            }
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .exact_width(150.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("OPTISCALER")
                        .strong()
                        .size(15.0)
                        .color(theme::ACCENT),
                );
                ui.label(RichText::new("GUI").size(12.0).color(theme::TEXT_DIM));
                ui.add_space(16.0);

                for (screen, label) in [
                    (Screen::Games, "🎮  Games"),
                    (Screen::Settings, "⚙  Settings"),
                    (Screen::Log, "📜  Log"),
                    (Screen::About, "ℹ  About"),
                ] {
                    let selected = self.state.screen == screen;
                    if ui
                        .selectable_label(selected, RichText::new(label).size(14.0))
                        .clicked()
                    {
                        self.state.screen = screen;
                    }
                    ui.add_space(2.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("v{}", opticore::VERSION))
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        // First-frame startup: catalogue load + initial scan
        if !self.started {
            self.started = true;
            self.ops.spawn_catalogue_load(ctx);
            self.state.scan_state = ScanState::Running;
            self.ops.spawn_scan(ctx);
        }

        self.sidebar(ctx);
        match self.state.screen {
            Screen::Games => screens::games_grid::show(ctx, &mut self.state, &mut self.ops),
            Screen::Settings => screens::show_settings(ctx, &mut self.state),
            Screen::Log => screens::show_log(ctx, &mut self.state),
            Screen::About => screens::show_about(ctx, &mut self.state),
        }
    }
}
