//! Full game page (Heroic-style): hero artwork, store metadata, playtime,
//! and every action in one place. Opened from the detail panel's Details
//! button; metadata and installed size load in the background on first view.

use crate::ops::Ops;
use crate::state::{AppState, ArtState, Screen};
use crate::theme;
use eframe::egui::{self, RichText, Vec2};
use opticore::model::Game;

pub fn show(ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    let Some(game) = state
        .page_game
        .clone()
        .and_then(|key| state.games.iter().find(|g| g.key.path_norm == key))
        .cloned()
    else {
        state.screen = Screen::Games;
        return;
    };
    let pal = theme::palette(state.dark());

    // Kick off background loads once per game
    ops.request_metadata(ctx, &game);
    ops.request_size(ctx, &game);

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                if ui
                    .button(format!("← {}", state.i18n.tr("ui.games_tab")))
                    .clicked()
                {
                    state.screen = Screen::Games;
                    return;
                }
                ui.add_space(8.0);

                ui.horizontal_top(|ui| {
                    // Hero artwork
                    if let ArtState::Ready(image_path) = state.art_state(&game.key.path_norm) {
                        if let Some(texture) = state.texture_for(ctx, &image_path) {
                            let w = (ui.available_width() * 0.42).clamp(220.0, 420.0);
                            let aspect = texture.aspect_ratio();
                            ui.image((texture.id(), Vec2::new(w, w / aspect)));
                        }
                    }
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        header_block(ui, ctx, state, ops, &game, pal);
                    });
                });

                ui.add_space(10.0);

                // Description
                if let Some(Some(meta)) = state.metadata.get(&game.key.path_norm) {
                    if !meta.description.is_empty() {
                        ui.label(RichText::new(meta.description.clone()).size(13.0));
                        ui.add_space(6.0);
                    }
                    if !meta.store_url.is_empty() {
                        ui.hyperlink_to(
                            format!("🌐 {}", state.i18n.tr("ui.store_page")),
                            meta.store_url.clone(),
                        );
                    }
                } else if !state.metadata.contains_key(&game.key.path_norm) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("…").color(pal.text_dim));
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                super::games_grid::play_section(ui, ctx, state, ops, &game, pal);
                launch_options_section(ui, state, &game, pal);
                ui.separator();
                super::games_grid::install_section(ui, ctx, state, ops, &game);

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if game.optiscaler_installed
                        && ui
                            .button(format!("⚙ {}", state.i18n.tr("ui.edit_settings")))
                            .clicked()
                    {
                        state.open_editor(&game);
                    }
                    if ui
                        .button(format!("📂 {}", state.i18n.tr("ui.open_folder")))
                        .clicked()
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(&game.path)
                            .spawn();
                    }
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(game.path.display().to_string())
                        .small()
                        .color(pal.text_dim),
                );
            });
    });
}

/// Collapsible per-game launch options: extra arguments, exe override, and
/// environment variables (one KEY=VALUE per line). Saved on every edit —
/// entries that go back to all-default are pruned from library.json.
fn launch_options_section(
    ui: &mut egui::Ui,
    state: &mut AppState,
    game: &Game,
    pal: theme::Palette,
) {
    let key = game.key.path_norm.clone();
    let entry = state.library.entry(&key);
    let configured =
        !entry.launch_args.is_empty() || !entry.exe_override.is_empty() || !entry.env.is_empty();
    let title = if configured {
        format!("⚙ {} •", state.i18n.tr("ui.launch_options"))
    } else {
        format!("⚙ {}", state.i18n.tr("ui.launch_options"))
    };
    egui::CollapsingHeader::new(RichText::new(title).size(13.0))
        .id_salt("launch_options")
        .show(ui, |ui| {
            let mut changed = false;
            let entry = state.library.entry_mut(&key);

            ui.label(RichText::new(state.i18n.tr("ui.launch_args")).color(pal.text_dim));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut entry.launch_args)
                        .hint_text("-dx12 -windowed")
                        .desired_width(f32::INFINITY),
                )
                .changed();

            ui.add_space(4.0);
            ui.label(RichText::new(state.i18n.tr("ui.exe_override")).color(pal.text_dim));
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut entry.exe_override)
                            .hint_text("(auto)")
                            .desired_width(ui.available_width() - 40.0),
                    )
                    .changed();
                if ui.button("…").clicked() {
                    if let Some(exe) = rfd::FileDialog::new()
                        .add_filter("exe", &["exe"])
                        .set_directory(&game.path)
                        .pick_file()
                    {
                        entry.exe_override = exe.display().to_string();
                        changed = true;
                    }
                }
            });

            ui.add_space(4.0);
            ui.label(RichText::new(state.i18n.tr("ui.env_vars")).color(pal.text_dim));
            // Edited as KEY=VALUE lines; parsed back into the map on change
            let mut env_text = entry
                .env
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n");
            if ui
                .add(
                    egui::TextEdit::multiline(&mut env_text)
                        .hint_text("MANGOHUD=1")
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                entry.env = env_text
                    .lines()
                    .filter_map(|line| {
                        let (k, v) = line.split_once('=')?;
                        let k = k.trim();
                        (!k.is_empty()).then(|| (k.to_string(), v.trim().to_string()))
                    })
                    .collect();
                changed = true;
            }

            if changed {
                state.save_library();
            }
        });
}

/// Title, genre badges, and the facts grid to the right of the hero art.
fn header_block(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    state: &mut AppState,
    _ops: &mut Ops,
    game: &Game,
    pal: theme::Palette,
) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new(&game.name).size(22.0));
        let entry = state.library.entry(&game.key.path_norm);
        let star = if entry.favorite { "⭐" } else { "☆" };
        if ui.button(star).clicked() {
            let e = state.library.entry_mut(&game.key.path_norm);
            e.favorite = !e.favorite;
            state.save_library();
        }
    });

    let meta = state
        .metadata
        .get(&game.key.path_norm)
        .cloned()
        .flatten()
        .unwrap_or_default();

    if !meta.genres.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for genre in &meta.genres {
                ui.label(
                    RichText::new(format!(" {genre} "))
                        .size(11.0)
                        .background_color(pal.badge_platform)
                        .color(egui::Color32::WHITE),
                );
            }
        });
    }
    ui.add_space(6.0);

    let entry = state.library.entry(&game.key.path_norm);
    egui::Grid::new("game_page_grid")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            let mut row = |label: String, value: String| {
                if !value.is_empty() {
                    ui.label(RichText::new(label).color(pal.text_dim));
                    ui.label(value);
                    ui.end_row();
                }
            };
            row("Platform".into(), game.platform.label().to_string());
            row("Engine".into(), game.engine.label().to_string());
            row(state.i18n.tr("ui.developer"), meta.developers.join(", "));
            row(state.i18n.tr("ui.publisher"), meta.publishers.join(", "));
            row(state.i18n.tr("ui.release_date"), meta.release_date.clone());
            row(
                state.i18n.tr("ui.installed_size"),
                state
                    .sizes
                    .get(&game.key.path_norm)
                    .map(|b| opticore::metadata::format_size(*b))
                    .unwrap_or_default(),
            );
            row(
                state.i18n.tr("ui.playtime"),
                opticore::library::format_playtime(entry.playtime_minutes),
            );
            row(
                state.i18n.tr("ui.last_played"),
                entry.last_played.chars().take(10).collect(),
            );
            row(
                "OptiScaler".into(),
                if game.optiscaler_installed {
                    format!("✓ {}", state.i18n.tr("ui.filter_installed"))
                } else {
                    state.i18n.tr("ui.filter_not_installed")
                },
            );
        });
}
