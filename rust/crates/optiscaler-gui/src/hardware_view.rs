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
    // Cached facts are display-only until this session receives a fresh collection.
    current_session_valid: bool,
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
            current_session_valid: false,
            error,
            writable,
        }
    }
}

impl HardwareState {
    pub fn current_gpu(&self, id: &str) -> Option<&opticore::hardware::GpuProfile> {
        if !self.current_session_valid || self.pending.is_some() {
            return None;
        }
        self.local
            .hardware
            .as_ref()?
            .gpus
            .iter()
            .find(|gpu| gpu.id == id)
    }

    fn delete_hardware(&mut self) {
        // Dropping the receiver prevents a pending refresh from restoring deleted data.
        self.pending = None;
        self.current_session_valid = false;
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

    pub fn refresh(&mut self, ctx: &egui::Context) {
        self.current_session_valid = false;
        if self.pending.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        // A previously saved enumeration must not authorize a new runtime choice.
        self.local.game_gpus.clear();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(opticore::hardware::collect());
            ctx.request_repaint();
        });
    }

    pub fn recommendations(&mut self, ui: &mut egui::Ui, tr: &Translator) {
        ui.strong(tr.tr("hardware.recommendations"));
        ui.small(tr.tr("hardware.selection_hint"));
        if ui
            .add_enabled(
                self.pending.is_none(),
                egui::Button::new(tr.tr("hardware.refresh")),
            )
            .clicked()
        {
            self.refresh(ui.ctx());
        }
        if self.pending.is_some() {
            ui.label(tr.tr("hardware.collecting"));
        }
        if let Some(profile) = &self.local.hardware {
            for gpu in &profile.gpus {
                ui.label(format!(
                    "{}: {}",
                    gpu.name.as_deref().unwrap_or("?"),
                    tr.tr(gpu.recommendation_key())
                ));
            }
            if profile.gpus.is_empty() {
                ui.label(tr.tr("hardware.conservative"));
            }
        } else {
            ui.label(tr.tr("hardware.conservative"));
        }
    }

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
                self.current_session_valid = true;
                // DXGI enumeration order can change after driver/device changes.
                self.local.game_gpus.clear();
                self.save();
            }
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.current_session_valid = false;
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
                    self.refresh(ui.ctx());
                }
                if ui.button(tr.tr("hardware.delete")).clicked() {
                    self.delete_hardware();
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
        let gpus = if !self.current_session_valid || self.pending.is_some() {
            &[][..]
        } else {
            self.local
                .hardware
                .as_ref()
                .map(|p| p.gpus.as_slice())
                .unwrap_or(&[])
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cached_rx6000_with_failed_refresh() -> HardwareState {
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let mut state = HardwareState {
            local: LocalProfiles {
                hardware: Some(HardwareProfile {
                    gpus: vec![opticore::hardware::GpuProfile {
                        id: "gpu-0".into(),
                        name: Some("AMD Radeon RX 6700 XT".into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
            path: PathBuf::new(),
            pending: Some(rx),
            current_session_valid: false,
            error: None,
            writable: false,
        };
        state.poll();
        assert!(state.pending.is_none());
        assert!(state.error.is_some());
        assert!(
            state.local.hardware.is_some(),
            "cached facts remain displayable"
        );
        state
    }

    fn selector_text(state: &mut HardwareState, game: &Game) -> String {
        fn text(shape: &egui::Shape) -> String {
            match shape {
                egui::Shape::Text(shape) => shape.galley.job.text.clone(),
                egui::Shape::Vec(shapes) => shapes.iter().map(text).collect::<Vec<_>>().join("\n"),
                _ => String::new(),
            }
        }
        let ctx = egui::Context::default();
        let tr = Translator::default();
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    state.game_selector(ui, game, &tr);
                });
            });
        }
        output
            .shapes
            .iter()
            .map(|shape| text(&shape.shape))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn disconnected_refresh_cannot_authorize_cached_rx6000() {
        let state = cached_rx6000_with_failed_refresh();
        assert!(state.current_gpu("gpu-0").is_none());
    }

    #[test]
    fn disconnected_refresh_selector_rejects_cached_rx6000_choice() {
        let mut state = cached_rx6000_with_failed_refresh();
        let game = Game::new(
            "Test",
            PathBuf::from("test-game"),
            opticore::model::Platform::Steam,
        );
        state.local.set_gpu_for(&game, Some("gpu-0".into()));
        let text = selector_text(&mut state, &game);
        assert!(
            !text.contains("AMD Radeon RX 6700 XT"),
            "stale GPU shown as selected: {text}"
        );
        assert!(text.contains(&Translator::default().tr("hardware.choose_gpu")));
    }

    #[test]
    fn fresh_result_authorizes_until_refresh_or_delete() {
        let mut state = cached_rx6000_with_failed_refresh();
        let profile = state.local.hardware.clone().unwrap();
        let game = Game::new(
            "Test",
            PathBuf::from("test-game"),
            opticore::model::Platform::Steam,
        );
        let (tx, rx) = mpsc::channel();
        state.pending = Some(rx);
        tx.send(profile.clone()).unwrap();
        state.poll();
        assert!(state.current_gpu("gpu-0").is_some());
        assert!(state.local.game_gpus.is_empty());
        state.local.set_gpu_for(&game, Some("gpu-0".into()));
        assert!(selector_text(&mut state, &game).contains("AMD Radeon RX 6700 XT"));

        state.refresh(&egui::Context::default());
        assert!(state.current_gpu("gpu-0").is_none());
        assert!(state.local.game_gpus.is_empty());
        assert!(!selector_text(&mut state, &game).contains("AMD Radeon RX 6700 XT"));
        // Replace the worker's receiver deterministically with a failed refresh.
        let (tx, rx) = mpsc::channel();
        drop(tx);
        state.pending = Some(rx);
        state.poll();
        assert!(state.current_gpu("gpu-0").is_none());

        let (tx, rx) = mpsc::channel();
        state.pending = Some(rx);
        tx.send(profile).unwrap();
        state.poll();
        assert!(state.current_gpu("gpu-0").is_some());
        state.delete_hardware();
        assert!(state.current_gpu("gpu-0").is_none());
        assert!(state.local.hardware.is_none());
    }

    #[test]
    fn pending_snapshot_cannot_authorize_runtime_and_delete_cancels_late_result() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(format!("hardware-test-{}", std::process::id()));
        let (tx, rx) = mpsc::channel();
        let mut state = HardwareState {
            local: LocalProfiles::default(),
            path: dir.join("profiles.json"),
            pending: Some(rx),
            current_session_valid: true,
            error: None,
            writable: true,
        };
        state.local.hardware = Some(HardwareProfile {
            gpus: vec![opticore::hardware::GpuProfile {
                id: "gpu-0".into(),
                name: Some("AMD Radeon RX 6700 XT".into()),
                ..Default::default()
            }],
            ..Default::default()
        });
        assert!(state.current_gpu("gpu-0").is_none());
        state.delete_hardware();
        assert!(tx.send(HardwareProfile::default()).is_err());
        state.poll();
        assert!(state.local.hardware.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
