//! Store accounts and owned-games libraries (GOG + Epic). Login is a
//! browser round-trip: open the store's login page, paste the code back.
//! GOG installs run entirely in-app (offline installer, silent); Epic
//! installs hand off to the Epic launcher protocol for now.

use crate::ops::Ops;
use crate::state::{AppState, ScanState, Screen};
use crate::theme;
use eframe::egui::{self, RichText};
use opticore::stores::{epic, gog};

pub fn show(ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    let pal = theme::palette(state.dark());

    // First view with a connected account → fetch libraries
    if !state.store_libraries_requested
        && (state.store_auth.gog.is_some() || state.store_auth.epic.is_some())
    {
        state.store_libraries_requested = true;
        ops.spawn_store_libraries(ctx);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading(state.i18n.tr("ui.stores_tab"));
                ui.label(
                    RichText::new(state.i18n.tr("ui.stores_intro"))
                        .small()
                        .color(pal.text_dim),
                );
                if let Some(status) = &state.store_status {
                    ui.add_space(4.0);
                    ui.label(RichText::new(status).color(pal.accent));
                }
                ui.add_space(10.0);

                gog_section(ui, ctx, state, ops, pal);
                ui.add_space(12.0);
                epic_section(ui, ctx, state, ops, pal);
            });
    });
}

fn gog_section(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    pal: theme::Palette,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("GOG").strong().size(16.0));
            if let Some(tokens) = &state.store_auth.gog {
                ui.label(
                    RichText::new(format!(
                        "✓ {} (user {})",
                        state.i18n.tr("ui.store_connected"),
                        tokens.user_id
                    ))
                    .color(pal.badge_ok),
                );
            }
        });

        match state.store_auth.gog.clone() {
            None => {
                connect_row(ui, state, pal, &gog::login_url(), "gog", |state| {
                    &mut state.gog_code_input
                });
                let code = state.gog_code_input.trim().to_string();
                if !code.is_empty()
                    && ui
                        .button(RichText::new(state.i18n.tr("ui.store_connect")).strong())
                        .clicked()
                {
                    state.store_status = Some("GOG: connecting…".into());
                    state.gog_code_input.clear();
                    ops.spawn_gog_connect(ctx, code);
                }
            }
            Some(_) => {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("⟳ {}", state.i18n.tr("ui.store_refresh")))
                        .clicked()
                    {
                        ops.spawn_store_libraries(ctx);
                    }
                    if ui.button(state.i18n.tr("ui.store_logout")).clicked() {
                        state.store_auth.gog = None;
                        let _ = state.store_auth.save(&state.store_auth_path);
                        state.gog_library.clear();
                    }
                });
                owned_list_gog(ui, ctx, state, ops, pal);
            }
        }
    });
}

fn epic_section(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    pal: theme::Palette,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Epic Games").strong().size(16.0));
            if let Some(tokens) = &state.store_auth.epic {
                ui.label(
                    RichText::new(format!(
                        "✓ {} ({})",
                        state.i18n.tr("ui.store_connected"),
                        tokens.display_name
                    ))
                    .color(pal.badge_ok),
                );
            }
        });

        match state.store_auth.epic.clone() {
            None => {
                connect_row(ui, state, pal, &epic::login_url(), "epic", |state| {
                    &mut state.epic_code_input
                });
                let code = state.epic_code_input.trim().to_string();
                if !code.is_empty()
                    && ui
                        .button(RichText::new(state.i18n.tr("ui.store_connect")).strong())
                        .clicked()
                {
                    state.store_status = Some("Epic: connecting…".into());
                    state.epic_code_input.clear();
                    ops.spawn_epic_connect(ctx, code);
                }
            }
            Some(_) => {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("⟳ {}", state.i18n.tr("ui.store_refresh")))
                        .clicked()
                    {
                        ops.spawn_store_libraries(ctx);
                    }
                    if ui.button(state.i18n.tr("ui.store_logout")).clicked() {
                        state.store_auth.epic = None;
                        let _ = state.store_auth.save(&state.store_auth_path);
                        state.epic_library.clear();
                    }
                });
                owned_list_epic(ui, state, pal);
            }
        }
    });
}

/// Shared "open login page + paste code" row.
fn connect_row(
    ui: &mut egui::Ui,
    state: &mut AppState,
    pal: theme::Palette,
    login_url: &str,
    id: &str,
    code_field: impl FnOnce(&mut AppState) -> &mut String,
) {
    ui.label(
        RichText::new(state.i18n.tr("ui.store_login_help"))
            .small()
            .color(pal.text_dim),
    );
    ui.horizontal(|ui| {
        if ui
            .button(format!("🌐 {}", state.i18n.tr("ui.store_open_login")))
            .clicked()
        {
            let _ = std::process::Command::new("explorer.exe")
                .arg(login_url)
                .spawn();
        }
        let hint = state.i18n.tr("ui.store_paste_code");
        ui.add(
            egui::TextEdit::singleline(code_field(state))
                .hint_text(hint)
                .id_salt(("store_code", id))
                .desired_width(280.0),
        );
    });
}

fn owned_list_gog(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    pal: theme::Palette,
) {
    if state.gog_library.is_empty() {
        ui.label(RichText::new("…").color(pal.text_dim));
        return;
    }
    search_row(ui, state);
    let needle = state.store_search.to_lowercase();
    let installed = installed_names(state);
    ui.add_space(4.0);
    let games: Vec<opticore::stores::gog::GogOwned> = state
        .gog_library
        .iter()
        .filter(|g| needle.is_empty() || g.title.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    ui.label(
        RichText::new(format!(
            "{} {}",
            games.len(),
            state.i18n.tr("ui.games_found")
        ))
        .small()
        .color(pal.text_dim),
    );
    for game in games {
        let key = format!("gog:{}", game.id);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&game.title).size(13.0));
            if !game.works_on_windows {
                ui.label(
                    RichText::new("(not for Windows)")
                        .small()
                        .color(pal.text_dim),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(label) = state.busy_ops.get(&key) {
                    ui.label(RichText::new(label.clone()).small().color(pal.accent));
                    ui.spinner();
                } else if installed.contains(&game.title.to_lowercase()) {
                    ui.label(RichText::new("✓").color(pal.badge_ok));
                } else if game.works_on_windows
                    && ui.button(state.i18n.tr("ui.store_install")).clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new().set_title(&game.title).pick_folder() {
                        let target = dir.join(sanitize(&game.title));
                        ops.spawn_gog_install(ctx, game.clone(), target);
                    }
                }
                if let Some((_, message)) = state.op_results.get(&key) {
                    ui.label(RichText::new(message.clone()).small().color(pal.text_dim));
                }
            });
        });
    }
}

fn owned_list_epic(ui: &mut egui::Ui, state: &mut AppState, pal: theme::Palette) {
    if state.epic_library.is_empty() {
        ui.label(RichText::new("…").color(pal.text_dim));
        return;
    }
    search_row(ui, state);
    let needle = state.store_search.to_lowercase();
    let installed = installed_names(state);
    ui.add_space(4.0);
    let games: Vec<opticore::stores::epic::EpicOwned> = state
        .epic_library
        .iter()
        .filter(|g| needle.is_empty() || g.title.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    ui.label(
        RichText::new(format!(
            "{} {}",
            games.len(),
            state.i18n.tr("ui.games_found")
        ))
        .small()
        .color(pal.text_dim),
    );
    for game in games {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&game.title).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if installed.contains(&game.title.to_lowercase()) {
                    ui.label(RichText::new("✓").color(pal.badge_ok));
                } else if ui
                    .button(state.i18n.tr("ui.store_install_epic"))
                    .on_hover_text(state.i18n.tr("ui.store_install_epic_hint"))
                    .clicked()
                {
                    let _ = std::process::Command::new("explorer.exe")
                        .arg(opticore::stores::epic::install_url(&game.app_name))
                        .spawn();
                }
            });
        });
    }
}

fn search_row(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        let hint = state.i18n.tr("ui.search_games");
        ui.add(
            egui::TextEdit::singleline(&mut state.store_search)
                .hint_text(hint)
                .desired_width(200.0),
        );
        if ui
            .button(format!("⟳ {}", state.i18n.tr("ui.rescan")))
            .clicked()
        {
            state.screen = Screen::Games;
            state.scan_state = ScanState::NotStarted;
        }
    });
}

/// Lowercased names of already-installed (scanned) games, to mark owned
/// games that are on disk.
fn installed_names(state: &AppState) -> std::collections::HashSet<String> {
    state.games.iter().map(|g| g.name.to_lowercase()).collect()
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect()
}
