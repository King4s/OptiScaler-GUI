use eframe::egui;
use opticore::{
    advice::{self, AdviceStatus},
    i18n::Translator,
    model::Game,
    observations::GameObservation,
    report::{self, UserTestResult},
};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};

#[derive(Default)]
pub struct AdviceState {
    pub observations: HashMap<String, GameObservation>,
    pending: HashMap<String, Receiver<GameObservation>>,
    report_open: HashSet<String>,
    report_message: HashMap<String, String>,
}

impl AdviceState {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        game: &Game,
        hardware: &mut crate::hardware_view::HardwareState,
        tr: &Translator,
    ) {
        let key = &game.key.path_norm;
        if let Some(rx) = self.pending.get(key) {
            match rx.try_recv() {
                Ok(observed) => {
                    self.observations.insert(key.clone(), observed);
                    self.pending.remove(key);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending.remove(key);
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        ui.collapsing(tr.tr("advice.title"), |ui| {
            let refresh = ui
                .add_enabled(
                    !self.pending.contains_key(key),
                    egui::Button::new(tr.tr("advice.refresh")),
                )
                .clicked();
            if (refresh || !self.observations.contains_key(key)) && !self.pending.contains_key(key)
            {
                let game = game.clone();
                let (tx, rx) = mpsc::channel();
                self.pending.insert(key.clone(), rx);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let _ = tx.send(opticore::observations::observe(&game));
                    ctx.request_repaint();
                });
            }
            if self.pending.contains_key(key) {
                ui.label(tr.tr("advice.pending"));
            }
            let Some(observation) = self.observations.get(key) else {
                return;
            };
            let unknown = tr.tr("advice.unknown");
            ui.label(tr.tr(if observation.installed {
                "advice.installed"
            } else {
                "advice.not_detected"
            }));
            ui.label(tr.tr(if observation.loaded_from_log == Some(true) {
                "advice.loaded"
            } else {
                "advice.log_unknown"
            }));
            let test_result = hardware
                .local
                .game_results
                .get(key)
                .copied()
                .unwrap_or_default();
            let test_key = match test_result {
                UserTestResult::NotRun => "report.result_not_run",
                UserTestResult::Passed => "report.result_passed",
                UserTestResult::Failed => "report.result_failed",
            };
            ui.label(format!(
                "{}: {}",
                tr.tr("report.result_label"),
                tr.tr(test_key)
            ));
            if let Some(at) = &observation.log_modified {
                ui.small(at);
            }
            ui.label(format!(
                "{}: {}",
                tr.tr("advice.build"),
                observation.build_version.as_deref().unwrap_or(&unknown)
            ));
            ui.label(format!(
                "{}: {}",
                tr.tr("advice.version"),
                observation
                    .optiscaler_version
                    .as_deref()
                    .unwrap_or(&unknown)
            ));
            ui.small(tr.tr("advice.evidence"));
            ui.label(observation.dll_hints.join(", "));
            let gpu = hardware.local.game_gpus.get(key).and_then(|id| {
                hardware
                    .local
                    .hardware
                    .as_ref()
                    .and_then(|p| p.gpus.iter().find(|g| &g.id == id))
            });
            for recommendation in advice::evaluate(gpu, observation, tr.lang.code()) {
                ui.separator();
                let status = match recommendation.status {
                    AdviceStatus::Documented => "advice.documented",
                    AdviceStatus::Conditional => "advice.conditional",
                    AdviceStatus::Unsupported => "advice.unsupported",
                    AdviceStatus::Unknown => "advice.unknown",
                };
                ui.strong(format!("{}: {}", recommendation.title, tr.tr(status)));
                ui.label(recommendation.explanation);
                if recommendation.source_url.starts_with("https://") {
                    ui.hyperlink_to(tr.tr("advice.source"), recommendation.source_url);
                } else {
                    ui.small(recommendation.source_url);
                }
                ui.small(format!(
                    "{}: {} | {}: {}",
                    tr.tr("advice.checked"),
                    recommendation.checked_at,
                    tr.tr("advice.scope"),
                    recommendation.version_scope
                ));
            }
            ui.collapsing(tr.tr("advice.overlay"), |ui| {
                if let Some(target) = &observation.target_directory {
                    ui.monospace(target.display().to_string());
                }
                ui.label(tr.tr("advice.guide"));
                ui.label(tr.tr("advice.warning"));
            });
            ui.collapsing(tr.tr("report.title"), |ui| {
                ui.label(tr.tr("report.privacy"));
                if hardware.local.hardware.is_none() {
                    ui.label(tr.tr("report.no_hardware"));
                }
                let mut result = hardware
                    .local
                    .game_results
                    .get(key)
                    .copied()
                    .unwrap_or_default();
                let previous = result;
                egui::ComboBox::from_id_salt(("report_result", key))
                    .selected_text(tr.tr(match result {
                        UserTestResult::NotRun => "report.result_not_run",
                        UserTestResult::Passed => "report.result_passed",
                        UserTestResult::Failed => "report.result_failed",
                    }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut result,
                            UserTestResult::NotRun,
                            tr.tr("report.result_not_run"),
                        );
                        ui.selectable_value(
                            &mut result,
                            UserTestResult::Passed,
                            tr.tr("report.result_passed"),
                        );
                        ui.selectable_value(
                            &mut result,
                            UserTestResult::Failed,
                            tr.tr("report.result_failed"),
                        );
                    });
                if result != previous {
                    hardware.set_game_result(key, result);
                    self.report_open.remove(key);
                    self.report_message.remove(key);
                }
                if ui.button(tr.tr("report.preview")).clicked() {
                    self.report_open.insert(key.clone());
                }
                if !self.report_open.contains(key) {
                    return;
                }
                let empty = opticore::hardware::HardwareProfile::default();
                let profile = hardware.local.hardware.as_ref().unwrap_or(&empty);
                let gpu = hardware
                    .local
                    .game_gpus
                    .get(key)
                    .and_then(|id| profile.gpus.iter().find(|gpu| &gpu.id == id));
                let report = report::build_report(profile, gpu, observation, result);
                let Ok(mut preview) = report::preview_json(&report) else {
                    ui.label(tr.tr("report.save_failed"));
                    return;
                };
                ui.add(
                    egui::TextEdit::multiline(&mut preview)
                        .interactive(false)
                        .desired_rows(14),
                );
                if ui.button(tr.tr("report.save")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("optiscaler-report.json")
                        .save_file()
                    {
                        let message = match report::write_report(&path, &report) {
                            Ok(()) => tr.tr("report.save_ok"),
                            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                                tr.tr("report.overwrite_refused")
                            }
                            Err(_) => tr.tr("report.save_failed"),
                        };
                        self.report_message.insert(key.clone(), message);
                    }
                }
                if let Some(message) = self.report_message.get(key) {
                    ui.label(message);
                }
            });
        });
    }
}
