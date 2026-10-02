use eframe::egui;
use opticore::{
    hardware::HardwareProfile, i18n::Translator, model::Game, profiles::LocalProfiles,
    report::UserTestResult,
};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub struct HardwareState {
    pub local: LocalProfiles,
    path: PathBuf,
    pending: Option<Receiver<HardwareProfile>>,
    error: Option<String>,
    writable: bool,
}

impl Default for HardwareState {
    fn default() -> Self {
        let path = crate::ops::base_dir().join("cache/local-profiles.json");
        let (local, error, writable) = match LocalProfiles::load(&path) {
            Ok(local) => (local, None, true),
            Err(e) => (LocalProfiles::default(), Some(e.to_string()), false),
        };
        Self {
            local,
            path,
            pending: None,
            error,
            writable,
        }
    }
}

impl HardwareState {
    pub fn set_game_result(&mut self, game_key: &str, result: UserTestResult) {
        if result == UserTestResult::NotRun {
            self.local.game_results.remove(game_key);
        } else {
            self.local.game_results.insert(game_key.to_string(), result);
        }
        self.save();
    }

    fn save(&mut self) {
        if self.writable {
            self.error = self.local.save(&self.path).err().map(|e| e.to_string());
        }
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.pending else { return };
        match rx.try_recv() {
            Ok(profile) => {
                self.pending = None;
                self.local.hardware = Some(profile);
                // DXGI enumeration order can change after driver/device changes.
                self.local.game_gpus.clear();
                self.save();
            }
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.error = Some("Hardware collection did not complete".into());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    pub fn settings(&mut self, ui: &mut egui::Ui, tr: &Translator) {
        ui.collapsing(tr.tr("hardware.title"), |ui| {
            ui.label(tr.tr("hardware.privacy"));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.pending.is_none(),
                        egui::Button::new(tr.tr("hardware.refresh")),
                    )
                    .clicked()
                {
                    let (tx, rx) = mpsc::channel();
                    self.pending = Some(rx);
                    let ctx = ui.ctx().clone();
                    std::thread::spawn(move || {
                        let _ = tx.send(opticore::hardware::collect());
                        ctx.request_repaint();
                    });
                }
                if ui.button(tr.tr("hardware.delete")).clicked() {
                    // Dropping the receiver prevents a pending refresh from restoring deleted data.
                    self.pending = None;
                    self.local.hardware = None;
                    self.local.game_gpus.clear();
                    if self.writable {
                        self.save();
                    } else {
                        // Only explicit deletion may discard an unreadable profile file.
                        match std::fs::remove_file(&self.path) {
                            Ok(()) => {
                                self.writable = true;
                                self.error = None;
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                self.writable = true;
                                self.error = None;
                            }
                            Err(e) => {
                                self.error = Some(e.to_string());
                            }
                        }
                    }
                }
            });
            if self.pending.is_some() {
                ui.label(tr.tr("hardware.collecting"));
            }
            if let Some(error) = &self.error {
                ui.label(format!("{}: {error}", tr.tr("hardware.failed")));
            }
            let unknown = tr.tr("hardware.unknown");
            if let Some(profile) = &self.local.hardware {
                ui.label(format!(
                    "{}: {}",
                    tr.tr("hardware.windows"),
                    profile.windows_build.as_deref().unwrap_or(&unknown)
                ));
                ui.label(format!(
                    "{}: {}",
                    tr.tr("hardware.ram"),
                    memory(profile.ram_bytes, &unknown)
                ));
                ui.label(format!(
                    "{}: {}",
                    tr.tr("hardware.collected"),
                    profile.collected_at.as_deref().unwrap_or(&unknown)
                ));
                egui::ScrollArea::vertical()
                    .id_salt("hardware_list")
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for gpu in &profile.gpus {
                            ui.separator();
                            ui.strong(gpu.name.as_deref().unwrap_or(&unknown));
                            ui.label(gpu.vendor.as_deref().unwrap_or(&unknown));
                            for (key, value) in [
                                ("hardware.dedicated", memory(gpu.dedicated_bytes, &unknown)),
                                ("hardware.shared", memory(gpu.shared_bytes, &unknown)),
                                (
                                    "hardware.driver",
                                    gpu.driver.clone().unwrap_or_else(|| unknown.clone()),
                                ),
                            ] {
                                ui.label(format!("{}: {value}", tr.tr(key)));
                            }
                        }
                        for warning in &profile.warnings {
                            ui.label(warning);
                        }
                    });
            } else {
                ui.label(tr.tr("hardware.no_profile"));
            }
        });
    }

    pub fn game_selector(&mut self, ui: &mut egui::Ui, game: &Game, tr: &Translator) {
        ui.label(tr.tr("hardware.game_gpu"));
        let gpus = self
            .local
            .hardware
            .as_ref()
            .map(|p| p.gpus.as_slice())
            .unwrap_or(&[]);
        let mut choice = self.local.gpu_for(game).cloned();
        // Never infer that the GUI adapter, first adapter, or a stale selection is the game GPU.
        if choice
            .as_ref()
            .is_some_and(|id| !gpus.iter().any(|g| &g.id == id))
        {
            choice = None;
        }
        let before = choice.clone();
        let label = choice
            .as_ref()
            .and_then(|id| gpus.iter().find(|g| &g.id == id))
            .and_then(|g| g.name.clone())
            .unwrap_or_else(|| tr.tr("hardware.choose_gpu"));
        egui::ComboBox::from_id_salt(("game_gpu", game.key.path_norm.as_str()))
            .selected_text(label)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut choice, None, tr.tr("hardware.choose_gpu"));
                for gpu in gpus {
                    ui.selectable_value(
                        &mut choice,
                        Some(gpu.id.clone()),
                        gpu.name.as_deref().unwrap_or("Unknown"),
                    );
                }
            });
        if choice != before {
            self.local.set_gpu_for(game, choice);
            self.save();
        }
        ui.small(tr.tr("hardware.selection_hint"));
    }
}

fn memory(bytes: Option<u64>, unknown: &str) -> String {
    bytes
        .map(|v| format!("{:.1} GiB", v as f64 / 1_073_741_824.0))
        .unwrap_or_else(|| unknown.into())
}
