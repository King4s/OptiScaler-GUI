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
    /// Startup OptiScaler auto-update has been dispatched this session.
    auto_update_done: bool,
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
        state.sort_key = crate::state::SortKey::from_code(&state.config.sort_key);
        state.sort_ascending = state.config.sort_ascending;
        state.view_mode = crate::state::ViewMode::from_code(&state.config.view_mode);
        theme::apply(&cc.egui_ctx, state.dark());
        // GPU vendor from the running wgpu adapter (for Auto Settings) —
        // no WMI/PowerShell needed
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let info = render_state.adapter.get_info();
            state.gpu_vendor = opticore::ini::GpuVendor::from_pci_vendor_id(info.vendor);
            crate::fx::EffectsRenderer::register(render_state);
        }
        // Respect the system accessibility setting: animations disabled →
        // background effects stay off regardless of the config toggle
        state.reduced_motion = crate::fx::reduced_motion();
        // On-disk log for tester bug reports (logs/ next to the exe)
        state.file_log = opticore::logging::FileLog::new(&crate::ops::base_dir().join("logs"));
        state.push_log(format!(
            "OptiScaler GUI {} started (GPU vendor: {})",
            opticore::VERSION,
            state.gpu_vendor.label()
        ));
        // Remove leftovers from a completed or interrupted self-update
        if let Ok(exe) = std::env::current_exe() {
            opticore::selfupdate::cleanup_old(&exe);
        }
        Self {
            state,
            ops: Ops::new(),
            started: false,
            auto_update_done: false,
        }
    }

    /// Opt-in startup auto-update: once the scan AND the latest-release
    /// check have both landed, update every outdated install sequentially.
    fn maybe_auto_update_optiscaler(&mut self, ctx: &egui::Context) {
        if self.auto_update_done
            || !self.state.config.auto_update_optiscaler
            || self.state.scan_state != ScanState::Done
        {
            return;
        }
        let Some(latest) = self.state.latest_release.clone() else {
            return;
        };
        self.auto_update_done = true;
        let outdated: Vec<opticore::model::Game> = self
            .state
            .games
            .iter()
            .filter(|g| {
                g.optiscaler_installed
                    && opticore::install::installed_version(&g.path)
                        .is_some_and(|v| opticore::install::is_update_available(&v, &latest))
                    && !self.state.busy_ops.contains_key(&g.key.path_norm)
            })
            .cloned()
            .collect();
        for game in &outdated {
            self.state
                .busy_ops
                .insert(game.key.path_norm.clone(), "Queued for update…".into());
        }
        self.ops.spawn_auto_updates(ctx, outdated);
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
                TaskEvent::GuiUpdateAvailable { version, url } => {
                    self.state
                        .push_log(format!("GUI update available: {version}"));
                    self.state.gui_update = Some((version, url));
                }
                TaskEvent::GuiUpdateStatus { phase, label } => {
                    if phase == opticore::progress::GuiUpdatePhase::Failed {
                        self.state.push_log(format!("GUI update failed: {label}"));
                    }
                    self.state.gui_update_phase = Some((phase, label));
                }
                TaskEvent::DefaultsFetched { ini_path, message } => {
                    self.state.push_log(message.clone());
                    if let Some(editor) = self.state.editor.as_mut() {
                        editor.fetching_defaults = false;
                        editor.defaults = ini_path.as_deref().and_then(opticore::ini::read_file);
                        editor.status = Some(message);
                    }
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
                    if let Some((version, url)) = self.state.gui_update.clone() {
                        use opticore::progress::GuiUpdatePhase;
                        match self.state.gui_update_phase.clone() {
                            Some((GuiUpdatePhase::Downloading, label)) => {
                                ui.label(
                                    RichText::new(format!(
                                        "⬇ {} {label}",
                                        self.state.i18n.tr("ui.update_downloading")
                                    ))
                                    .small()
                                    .color(pal.accent),
                                );
                            }
                            Some((GuiUpdatePhase::Staged, _)) => {
                                if ui
                                    .button(
                                        RichText::new(format!(
                                            "🔄 {}",
                                            self.state.i18n.tr("ui.update_restart_now")
                                        ))
                                        .small()
                                        .strong(),
                                    )
                                    .clicked()
                                {
                                    if let Ok(exe) = std::env::current_exe() {
                                        match opticore::selfupdate::apply_and_restart(&exe) {
                                            Ok(()) => {
                                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                            }
                                            Err(e) => {
                                                self.state.gui_update_phase =
                                                    Some((GuiUpdatePhase::Failed, e.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                            Some((GuiUpdatePhase::Failed, label)) => {
                                ui.label(
                                    RichText::new(format!(
                                        "⚠ {}",
                                        self.state.i18n.tr("ui.update_failed")
                                    ))
                                    .small()
                                    .color(pal.badge_danger),
                                )
                                .on_hover_text(label);
                                ui.hyperlink_to(
                                    RichText::new(format!(
                                        "⬆ {} {version}",
                                        self.state.i18n.tr("ui.update_available")
                                    ))
                                    .small()
                                    .color(pal.accent),
                                    url,
                                );
                            }
                            None => {
                                if ui
                                    .button(
                                        RichText::new(format!(
                                            "⬆ {} {version}",
                                            self.state.i18n.tr("ui.update_download_restart")
                                        ))
                                        .small()
                                        .strong(),
                                    )
                                    .on_hover_text(url)
                                    .clicked()
                                {
                                    self.ops.spawn_gui_update_download(ctx);
                                }
                            }
                        }
                    }
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        self.maybe_auto_update_optiscaler(ctx);

        // First-frame startup: catalogue load + initial scan
        if !self.started {
            self.started = true;
            self.ops.spawn_catalogue_load(ctx);
            if self.state.config.check_updates {
                self.ops.spawn_release_check(ctx);
                self.ops.spawn_gui_update_check(ctx);
            }
            self.state.scan_state = ScanState::Running;
            self.ops
                .spawn_scan(ctx, self.state.config.excluded_drive_letters());
        }

        // Repaint pacing for the animated background: ~30 fps while effects
        // are on and the window is focused; otherwise purely event-driven
        // (zero idle GPU/CPU cost — the toggle's real value).
        if self.state.effects_active() && ctx.input(|i| i.focused) {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }

        // Frameless-window chrome: top drag strip + border resize zones
        crate::chrome::top_strip(ctx, theme::palette(self.state.dark()), "OPTISCALER GUI");
        crate::chrome::handle_resize(ctx);

        self.sidebar(ctx);
        match self.state.screen {
            Screen::Games => screens::games_grid::show(ctx, &mut self.state, &mut self.ops),
            Screen::IniEditor => screens::ini_editor::show(ctx, &mut self.state, &mut self.ops),
            Screen::Settings => screens::show_settings(ctx, &mut self.state),
            Screen::Log => screens::show_log(ctx, &mut self.state),
            Screen::About => screens::show_about(ctx, &mut self.state),
        }
    }
}
