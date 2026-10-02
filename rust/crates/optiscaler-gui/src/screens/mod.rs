pub mod games_grid;
pub mod ini_editor;

use crate::state::AppState;
use crate::theme;
use eframe::egui::{self, RichText};

fn cache_dir_size_mb(dir: &std::path::Path) -> f64 {
    fn walk(dir: &std::path::Path, total: &mut u64) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, total);
                } else if let Ok(meta) = entry.metadata() {
                    *total += meta.len();
                }
            }
        }
    }
    let mut total = 0u64;
    walk(dir, &mut total);
    total as f64 / (1024.0 * 1024.0)
}

pub fn show_settings(ctx: &egui::Context, state: &mut AppState, ops: &mut crate::ops::Ops) {
    let pal = theme::palette(state.dark());
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(state.i18n.tr("ui.global_settings_title"));
            state.hardware.settings(ui, &state.i18n);
            ui.add_space(10.0);
            let mut config_changed = false;

            // Language (live switch — no restart needed, unlike the Python app)
            ui.horizontal(|ui| {
                ui.label(state.i18n.tr("ui.interface_language"));
                let current = opticore::i18n::Lang::from_code(&state.config.language);
                egui::ComboBox::from_id_salt("language")
                    .selected_text(current.label())
                    .show_ui(ui, |ui| {
                        for lang in opticore::i18n::Lang::ALL {
                            if ui.selectable_label(current == lang, lang.label()).clicked() {
                                state.config.language = lang.code().to_string();
                                state.i18n.lang = lang;
                                config_changed = true;
                            }
                        }
                    });
            });

            // Theme
            ui.horizontal(|ui| {
                ui.label(state.i18n.tr("ui.appearance_theme"));
                for (value, label) in [("dark", "Dark"), ("light", "Light")] {
                    if ui
                        .selectable_label(state.config.theme == value, label)
                        .clicked()
                        && state.config.theme != value
                    {
                        state.config.theme = value.to_string();
                        theme::apply(ctx, state.dark());
                        config_changed = true;
                    }
                }
            });

            // Effects toggle (consumed by the M6 shader background)
            if ui
                .checkbox(
                    &mut state.config.effects_enabled,
                    state.i18n.tr("ui.effects_toggle"),
                )
                .changed()
            {
                config_changed = true;
            }

            // Background style picker
            if state.config.effects_enabled {
                ui.horizontal(|ui| {
                    ui.label(state.i18n.tr("ui.effects_style"));
                    let current_label = crate::fx::STYLES
                        .iter()
                        .find(|(value, _)| *value == state.config.effects_style)
                        .map(|(_, label)| *label)
                        .unwrap_or("Orbits");
                    egui::ComboBox::from_id_salt("effects_style")
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            for (value, label) in crate::fx::STYLES {
                                if ui
                                    .selectable_label(state.config.effects_style == value, label)
                                    .clicked()
                                    && state.config.effects_style != value
                                {
                                    state.config.effects_style = value.to_string();
                                    config_changed = true;
                                }
                            }
                        });
                });
            }

            // Update check opt-in
            if ui
                .checkbox(
                    &mut state.config.check_updates,
                    state.i18n.tr("ui.check_for_updates"),
                )
                .changed()
            {
                config_changed = true;
            }

            // Opt-in: auto-update OptiScaler in installed games at startup
            if state.config.check_updates {
                if ui
                    .checkbox(
                        &mut state.config.auto_update_optiscaler,
                        state.i18n.tr("ui.auto_update_optiscaler"),
                    )
                    .changed()
                {
                    config_changed = true;
                }
                if state.config.auto_update_optiscaler {
                    ui.label(
                        RichText::new(state.i18n.tr("ui.auto_update_hint"))
                            .small()
                            .color(pal.text_dim),
                    );
                }
            }

            // Excluded drives
            ui.horizontal(|ui| {
                ui.label(state.i18n.tr("ui.excluded_drives"));
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut state.config.excluded_drives)
                            .desired_width(120.0),
                    )
                    .changed()
                {
                    config_changed = true;
                }
            });
            ui.label(
                RichText::new(state.i18n.tr("ui.language_restart_note"))
                    .small()
                    .color(pal.text_dim),
            );

            ui.add_space(14.0);
            ui.separator();

            // Cache management
            ui.heading(state.i18n.tr("library.roots"));
            ui.label(state.i18n.tr("library.roots_hint"));
            let mut remove = None;
            for (index, root) in state.config.scan_roots.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    config_changed |= ui.checkbox(&mut root.enabled, "").changed();
                    ui.label(root.path.display().to_string());
                    if ui.small_button(state.i18n.tr("library.remove")).clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                state.config.scan_roots.remove(index);
                config_changed = true;
            }
            ui.horizontal(|ui| {
                if ui.button(state.i18n.tr("library.add_root")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        match opticore::scan::custom::validate_root(&path) {
                            Ok(path) => {
                                let key = opticore::model::GameKey::path_key(&path);
                                if !state.config.scan_roots.iter().any(|root| {
                                    opticore::model::GameKey::path_key(&root.path) == key
                                }) {
                                    state.config.scan_roots.push(
                                        opticore::scan::custom::ScanRoot {
                                            path,
                                            enabled: true,
                                        },
                                    );
                                    config_changed = true;
                                }
                            }
                            Err(error) => {
                                rfd::MessageDialog::new().set_description(&error).show();
                                state.push_log(error);
                            }
                        }
                    }
                }
                if ui
                    .add_enabled(
                        !ops.scan_running(),
                        egui::Button::new(state.i18n.tr("library.rescan")),
                    )
                    .clicked()
                {
                    state.scan_state = crate::state::ScanState::Running;
                    ops.spawn_scan(ctx, state.config.scan_config());
                }
            });
            ui.separator();
            ui.heading(state.i18n.tr("ui.cache_settings"));
            ui.add_space(4.0);
            let cache_dir = state
                .config_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}: {:.1} MB",
                    state.i18n.tr("ui.cache_size"),
                    cache_dir_size_mb(&cache_dir)
                ));
                if ui.button(state.i18n.tr("ui.clear_cache")).clicked() {
                    ops.clear_artwork_cache(ctx);
                }
                if ui.button(state.i18n.tr("ui.open_cache_folder")).clicked() {
                    let _ = std::process::Command::new("explorer")
                        .arg(&cache_dir)
                        .spawn();
                }
            });

            if config_changed {
                if let Err(e) = state.config.save(&state.config_path) {
                    state.push_log(format!("Failed to save config: {e}"));
                }
            }
        });
    });
}

pub fn show_log(ctx: &egui::Context, state: &mut AppState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(state.i18n.tr("ui.log_tab"));
            if ui.button(state.i18n.tr("ui.copy_all")).clicked() {
                let all: String = state.log.iter().cloned().collect::<Vec<_>>().join("\n");
                ctx.copy_text(all);
            }
            if let Some(dir) = state.file_log.as_ref().and_then(|l| l.dir()) {
                if ui.button("📂 logs").clicked() {
                    let _ = std::process::Command::new("explorer").arg(dir).spawn();
                }
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

pub fn show_about(ctx: &egui::Context, state: &mut AppState) {
    let pal = theme::palette(state.dark());
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(state.i18n.tr("ui.app_title"));
        ui.label(format!("{} {}", state.i18n.tr("ui.version"), opticore::VERSION));
        ui.add_space(8.0);
        ui.label("An unofficial installer and manager for OptiScaler.");
        ui.label(
            RichText::new(
                "All upscaling technology is the work of the OptiScaler team — this app only installs and manages it.",
            )
            .color(pal.text_dim),
        );
        ui.add_space(8.0);
        ui.hyperlink_to(
            "OptiScaler (official project)",
            "https://github.com/optiscaler/OptiScaler",
        );
        ui.hyperlink_to(
            "OptiScaler-GUI on GitHub",
            "https://github.com/King4s/OptiScaler-GUI",
        );
    });
}
