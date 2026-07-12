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
        let mut state = AppState::default();
        // Load persisted config (cache/config.json — shared with the Python
        // app; its language/excluded_drives are imported on first run)
        state.config_path = opticore::config::AppConfig::config_path(&crate::ops::base_dir());
        state.config = opticore::config::AppConfig::load(&state.config_path);
        state.i18n = opticore::i18n::Translator::new(opticore::i18n::Lang::from_code(
            &state.config.language,
        ));
        theme::apply(&cc.egui_ctx, state.dark());
        // GPU vendor from the running wgpu adapter (for Auto Settings) —
        // no WMI/PowerShell needed
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let info = render_state.adapter.get_info();
            state.gpu_vendor = opticore::ini::GpuVendor::from_pci_vendor_id(info.vendor);
        }
        Self {
            state,
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
                TaskEvent::OpProgress { path_norm, label } => {
                    self.state.busy_ops.insert(path_norm, label);
                }
                TaskEvent::OpFinished {
                    path_norm,
                    ok,
                    message,
                } => {
                    self.state.busy_ops.remove(&path_norm);
                    self.state.push_log(format!(
                        "{}: {}",
                        if ok { "Done" } else { "Failed" },
                        message
                    ));
                    self.state
                        .op_results
                        .insert(path_norm.clone(), (ok, message));
                    if ok {
                        // Refresh install state for the affected game
                        if let Some(game) = self
                            .state
                            .games
                            .iter_mut()
                            .find(|g| g.key.path_norm == path_norm)
                        {
                            if let Some(facts) = opticore::scan::folder_facts::collect(&game.path) {
                                game.optiscaler_installed =
                                    opticore::scan::folder_facts::detect_optiscaler(
                                        &game.path, &facts,
                                    );
                            }
                        }
                    }
                }
                TaskEvent::LatestRelease { version } => {
                    self.state
                        .push_log(format!("Latest OptiScaler release: {version}"));
                    self.state.latest_release = Some(version);
                }
                TaskEvent::Log(line) => self.state.push_log(line),
            }
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        let pal = theme::palette(self.state.dark());
        egui::SidePanel::left("sidebar")
            .exact_width(150.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("OPTISCALER")
                        .strong()
                        .size(15.0)
                        .color(pal.accent),
                );
                ui.label(RichText::new("GUI").size(12.0).color(pal.text_dim));
                ui.add_space(16.0);

                for (screen, label) in [
                    (
                        Screen::Games,
                        format!("🎮  {}", self.state.i18n.tr("ui.games_tab")),
                    ),
                    (
                        Screen::Settings,
                        format!("⚙  {}", self.state.i18n.tr("ui.settings_tab")),
                    ),
                    (
                        Screen::Log,
                        format!("📜  {}", self.state.i18n.tr("ui.log_tab")),
                    ),
                    (
                        Screen::About,
                        format!("ℹ  {}", self.state.i18n.tr("ui.about_tab")),
                    ),
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
                            .color(pal.text_dim),
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
            self.ops.spawn_release_check(ctx);
            self.state.scan_state = ScanState::Running;
            self.ops
                .spawn_scan(ctx, self.state.config.excluded_drive_letters());
        }

        self.sidebar(ctx);
        match self.state.screen {
            Screen::Games => screens::games_grid::show(ctx, &mut self.state, &mut self.ops),
            Screen::IniEditor => screens::ini_editor::show(ctx, &mut self.state),
            Screen::Settings => screens::show_settings(ctx, &mut self.state),
            Screen::Log => screens::show_log(ctx, &mut self.state),
            Screen::About => screens::show_about(ctx, &mut self.state),
        }
    }
}
