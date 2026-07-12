//! Games screen: toolbar (search / platform filter / rescan / add folder),
//! virtualized responsive card grid, and a detail side panel.

use crate::ops::Ops;
use crate::state::{AppState, ArtState, ScanState};
use crate::theme;
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, RichText, Sense, Stroke, Vec2};
use opticore::model::{Engine, Game, Platform};
use opticore::progress::TaskEvent;

const CARD_W: f32 = 210.0;
const CARD_H: f32 = 158.0;
const ART_H: f32 = 92.0;
const GRID_GAP: f32 = 10.0;

const PLATFORM_FILTERS: &[(Option<Platform>, &str)] = &[
    (None, "All platforms"),
    (Some(Platform::Steam), "Steam"),
    (Some(Platform::Epic), "Epic"),
    (Some(Platform::Gog), "GOG"),
    (Some(Platform::Xbox), "Xbox"),
    (Some(Platform::Heroic), "Heroic"),
    (Some(Platform::Manual), "Manual"),
];

pub fn show(ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    // Detail side panel for the selected game
    if let Some(selected_key) = state.selected.clone() {
        let game = state
            .games
            .iter()
            .find(|g| g.key.path_norm == selected_key)
            .cloned();
        if let Some(game) = game {
            egui::SidePanel::right("game_detail")
                .default_width(300.0)
                .show(ctx, |ui| detail_panel(ui, ctx, state, ops, &game));
        } else {
            state.selected = None;
        }
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        toolbar(ui, ctx, state, ops);
        ui.add_space(4.0);

        match state.scan_state {
            ScanState::NotStarted => {
                empty_state(
                    ui,
                    ctx,
                    ops,
                    "Welcome! Scan your system to find installed games.",
                );
            }
            ScanState::Running => {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.spinner();
                    ui.label(RichText::new("Scanning for games…").color(theme::TEXT_DIM));
                });
            }
            ScanState::Done => {
                let indices = state.filtered_indices();
                if indices.is_empty() {
                    empty_state(ui, ctx, ops, "No games match the current search/filter.");
                } else {
                    grid(ui, ctx, state, ops, &indices);
                }
            }
        }
    });
}

fn toolbar(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("Search games…")
                .desired_width(220.0),
        );

        let current_label = PLATFORM_FILTERS
            .iter()
            .find(|(p, _)| *p == state.platform_filter)
            .map(|(_, l)| *l)
            .unwrap_or("All platforms");
        egui::ComboBox::from_id_salt("platform_filter")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for (platform, label) in PLATFORM_FILTERS {
                    ui.selectable_value(&mut state.platform_filter, *platform, *label);
                }
            });

        if ui
            .add_enabled(!ops.scan_running(), egui::Button::new("⟳ Rescan"))
            .clicked()
        {
            state.scan_state = ScanState::Running;
            ops.spawn_scan(ctx);
        }

        if ui.button("📁 Add folder…").clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                match opticore::scan::scan_manual_folder(&folder) {
                    Some(game) => {
                        let _ = ops
                            .tx
                            .send(TaskEvent::Log(format!("Added manual game: {}", game.name)));
                        // replace existing entry for the same path
                        state
                            .games
                            .retain(|g| g.key.path_norm != game.key.path_norm);
                        state.games.push(game);
                        state
                            .games
                            .sort_by(|a, b| a.key.name_lower.cmp(&b.key.name_lower));
                    }
                    None => {
                        let _ = ops.tx.send(TaskEvent::Log(format!(
                            "Folder does not look like a game: {}",
                            folder.display()
                        )));
                    }
                }
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if state.scan_state == ScanState::Done {
                ui.label(
                    RichText::new(format!("{} games", state.filtered_indices().len()))
                        .color(theme::TEXT_DIM),
                );
            }
        });
    });
}

fn empty_state(ui: &mut egui::Ui, ctx: &egui::Context, ops: &mut Ops, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.label(RichText::new(message).size(16.0).color(theme::TEXT_DIM));
        ui.add_space(12.0);
        if !ops.scan_running()
            && ui
                .add(egui::Button::new(
                    RichText::new("🔍 Scan for games").size(16.0),
                ))
                .clicked()
        {
            ops.spawn_scan(ctx);
        }
    });
}

fn grid(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    indices: &[usize],
) {
    let avail = ui.available_width();
    let cols = ((avail + GRID_GAP) / (CARD_W + GRID_GAP)).floor().max(1.0) as usize;
    let rows = indices.len().div_ceil(cols);
    let row_height = CARD_H + GRID_GAP;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, rows, |ui, row_range| {
            for row in row_range {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let Some(&game_idx) = indices.get(row * cols + col) else {
                            break;
                        };
                        let game = state.games[game_idx].clone();
                        card(ui, ctx, state, ops, &game);
                    }
                });
            }
        });
}

fn card(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState, ops: &mut Ops, game: &Game) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(CARD_W, CARD_H), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let selected = state.selected.as_deref() == Some(game.key.path_norm.as_str());
    let fill = if selected || response.hovered() {
        theme::CARD_HOVER
    } else {
        theme::CARD
    };
    let stroke = if selected {
        Stroke::new(1.5_f32, theme::ACCENT)
    } else {
        Stroke::NONE
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    // Artwork area
    let art_rect = egui::Rect::from_min_size(
        rect.min + Vec2::new(4.0, 4.0),
        Vec2::new(CARD_W - 8.0, ART_H),
    );
    match state.art_state(&game.key.path_norm) {
        ArtState::Ready(image_path) => {
            if let Some(texture) = state.texture_for(ctx, &image_path) {
                ui.painter().image(
                    texture.id(),
                    art_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        ArtState::Unknown => {
            ops.request_image(ctx, game);
            state
                .art
                .insert(game.key.path_norm.clone(), ArtState::Fetching);
            placeholder_art(ui, art_rect, "…");
        }
        ArtState::Fetching => placeholder_art(ui, art_rect, "…"),
        ArtState::Missing => placeholder_art(ui, art_rect, &game.name),
    }

    // Name
    let name_pos = egui::pos2(rect.min.x + 8.0, art_rect.max.y + 8.0);
    let name = if game.name.chars().count() > 26 {
        let truncated: String = game.name.chars().take(25).collect();
        format!("{truncated}…")
    } else {
        game.name.clone()
    };
    ui.painter().text(
        name_pos,
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(13.0),
        ui.visuals().text_color(),
    );

    // Badge row
    let mut badge_pos = egui::pos2(rect.min.x + 8.0, rect.max.y - 24.0);
    badge_pos = badge(ui, badge_pos, game.platform.label(), theme::BADGE_PLATFORM);
    if game.optiscaler_installed {
        badge_pos = badge(ui, badge_pos, "OptiScaler ✓", theme::BADGE_OK);
    }
    if !game.anti_cheat.is_empty() {
        badge_pos = badge(ui, badge_pos, "⚠ AC", theme::BADGE_WARN);
    }
    if !game.engine_supported && game.engine != Engine::Unknown {
        badge(ui, badge_pos, "engine", theme::BADGE_DANGER);
    }

    if response.clicked() {
        state.selected = if selected {
            None
        } else {
            Some(game.key.path_norm.clone())
        };
    }
    ui.add_space(GRID_GAP - ui.spacing().item_spacing.x);
}

fn placeholder_art(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), theme::BG);
    let text = if label.chars().count() > 20 {
        label.chars().take(19).collect::<String>() + "…"
    } else {
        label.to_string()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        theme::TEXT_DIM,
    );
}

fn badge(ui: &egui::Ui, pos: egui::Pos2, text: &str, color: Color32) -> egui::Pos2 {
    let font = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let padding = Vec2::new(6.0, 3.0);
    let rect = egui::Rect::from_min_size(pos, galley.size() + padding * 2.0);
    ui.painter().rect_filled(rect, CornerRadius::same(4), color);
    ui.painter()
        .galley(rect.min + padding, galley, Color32::WHITE);
    egui::pos2(rect.max.x + 6.0, pos.y)
}

fn detail_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    game: &Game,
) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.heading(&game.name);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("✕").clicked() {
                state.selected = None;
            }
        });
    });
    ui.add_space(4.0);

    if let ArtState::Ready(image_path) = state.art_state(&game.key.path_norm) {
        if let Some(texture) = state.texture_for(ctx, &image_path) {
            let w = ui.available_width().min(280.0);
            let aspect = texture.aspect_ratio();
            ui.image((texture.id(), Vec2::new(w, w / aspect)));
        }
    }

    ui.add_space(8.0);
    egui::Grid::new("detail_grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(RichText::new("Platform").color(theme::TEXT_DIM));
            ui.label(game.platform.label());
            ui.end_row();
            ui.label(RichText::new("Engine").color(theme::TEXT_DIM));
            ui.label(format!("{:?}", game.engine));
            ui.end_row();
            if let Some(appid) = game.steam_appid {
                ui.label(RichText::new("Steam AppID").color(theme::TEXT_DIM));
                ui.label(appid.to_string());
                ui.end_row();
            }
            ui.label(RichText::new("OptiScaler").color(theme::TEXT_DIM));
            ui.label(if game.optiscaler_installed {
                "Installed"
            } else {
                "Not installed"
            });
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(
        RichText::new(game.path.display().to_string())
            .small()
            .color(theme::TEXT_DIM),
    );
    ui.add_space(8.0);
    ui.separator();

    install_section(ui, ctx, state, ops, game);

    ui.add_space(8.0);
    if ui.button("📂 Open folder").clicked() {
        let _ = std::process::Command::new("explorer")
            .arg(&game.path)
            .spawn();
    }
}

/// Install / Update / Uninstall actions with anti-cheat confirmation and
/// per-game progress.
fn install_section(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    game: &Game,
) {
    // Operation in flight → progress only
    if let Some(label) = state.busy_ops.get(&game.key.path_norm) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(label.clone());
        });
        return;
    }

    // Last operation result
    if let Some((ok, message)) = state.op_results.get(&game.key.path_norm) {
        let color = if *ok {
            theme::BADGE_OK
        } else {
            theme::BADGE_DANGER
        };
        ui.colored_label(color, message.clone());
        ui.add_space(4.0);
    }

    // Anti-cheat warning requires explicit confirmation before install/update
    let needs_confirm = !game.anti_cheat.is_empty();
    if needs_confirm {
        ui.colored_label(
            theme::BADGE_WARN,
            format!(
                "⚠ Anti-cheat detected ({}). Installing mods into online games can trigger bans.",
                game.anti_cheat
                    .iter()
                    .map(|ac| format!("{ac:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        ui.checkbox(
            &mut state.anticheat_confirmed,
            "I understand the risk — continue anyway",
        );
        ui.add_space(4.0);
    }
    let allowed = !needs_confirm || state.anticheat_confirmed;

    if game.optiscaler_installed {
        // Update path: compare installed manifest version vs latest release
        let installed = opticore::install::installed_version(&game.path);
        if let (Some(installed), Some(latest)) =
            (installed.as_deref(), state.latest_release.as_deref())
        {
            if opticore::install::is_update_available(installed, latest) {
                ui.colored_label(
                    theme::ACCENT,
                    format!("Update available: {installed} → {latest}"),
                );
                if ui
                    .add_enabled(
                        allowed,
                        egui::Button::new(RichText::new("⬆ Update OptiScaler").strong()),
                    )
                    .clicked()
                {
                    let target = opticore::install::installed_target_filename(&game.path)
                        .unwrap_or_else(|| "dxgi.dll".to_string());
                    let options = opticore::install::InstallOptions {
                        target_filename: target,
                        overwrite: true,
                        ..Default::default()
                    };
                    state
                        .busy_ops
                        .insert(game.key.path_norm.clone(), "Starting update…".into());
                    ops.spawn_install(ctx, game, options);
                }
                ui.add_space(4.0);
            } else {
                ui.label(
                    RichText::new(format!("Installed: {installed} (up to date)"))
                        .color(theme::TEXT_DIM),
                );
            }
        } else if let Some(installed) = installed {
            ui.label(RichText::new(format!("Installed: {installed}")).color(theme::TEXT_DIM));
        }

        ui.horizontal(|ui| {
            if ui.button("⚙ Edit settings").clicked() {
                state.open_editor(game);
            }
            if ui
                .add_enabled(allowed, egui::Button::new("🗑 Uninstall OptiScaler"))
                .clicked()
            {
                state
                    .busy_ops
                    .insert(game.key.path_norm.clone(), "Uninstalling…".into());
                ops.spawn_uninstall(ctx, game);
            }
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Install as").color(theme::TEXT_DIM));
            egui::ComboBox::from_id_salt("proxy_choice")
                .selected_text(state.proxy_choice.clone())
                .show_ui(ui, |ui| {
                    for name in opticore::install::payload::PROXY_FILENAMES {
                        ui.selectable_value(&mut state.proxy_choice, name.to_string(), *name);
                    }
                });
        });
        if ui
            .add_enabled(
                allowed,
                egui::Button::new(RichText::new("⬇ Install OptiScaler").strong()),
            )
            .clicked()
        {
            let options = opticore::install::InstallOptions {
                target_filename: state.proxy_choice.clone(),
                overwrite: false,
                ..Default::default()
            };
            state
                .busy_ops
                .insert(game.key.path_norm.clone(), "Starting install…".into());
            ops.spawn_install(ctx, game, options);
        }
    }
}
