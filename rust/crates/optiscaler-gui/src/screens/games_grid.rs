//! Games screen: toolbar (search / filters / sort / view mode), four
//! Explorer-style presentations (large cards, small cards, list, details),
//! and a detail side panel. Everything is virtualized via show_rows.

use crate::ops::Ops;
use crate::state::{AppState, ArtState, ScanState, SortKey, ViewMode};
use crate::theme;
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, RichText, Sense, Stroke, Vec2};
use opticore::model::{Engine, Game, Platform};
use opticore::progress::TaskEvent;

/// Card geometry per view mode.
struct CardDims {
    w: f32,
    h: f32,
    art_h: f32,
    name_font: f32,
    name_chars: usize,
}

const CARD_LARGE: CardDims = CardDims {
    w: 210.0,
    h: 158.0,
    art_h: 92.0,
    name_font: 13.0,
    name_chars: 26,
};
const CARD_SMALL: CardDims = CardDims {
    w: 150.0,
    h: 118.0,
    art_h: 62.0,
    name_font: 11.5,
    name_chars: 19,
};
const GRID_GAP: f32 = 10.0;
const LIST_ROW_H: f32 = 30.0;

// First entry's empty label is replaced with the translated "all platforms"
const PLATFORM_FILTERS: &[(Option<Platform>, &str)] = &[
    (None, ""),
    (Some(Platform::Steam), "Steam"),
    (Some(Platform::Epic), "Epic"),
    (Some(Platform::Gog), "GOG"),
    (Some(Platform::Xbox), "Xbox"),
    (Some(Platform::Heroic), "Heroic"),
    (Some(Platform::Manual), "Manual"),
];

pub fn show(ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    let pal = theme::palette(state.dark());
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
        // Animated GPU background behind the grid (single fullscreen-triangle
        // pass; skipped entirely when effects are off or reduced motion is on)
        if state.effects_active() {
            let rect = ui.max_rect().expand(8.0);
            ui.painter()
                .add(eframe::egui_wgpu::Callback::new_paint_callback(
                    rect,
                    crate::fx::EffectsCallback {
                        time: ui.input(|i| i.time) as f32,
                        aspect: rect.aspect_ratio(),
                        intensity: 1.0,
                        dark: if state.dark() { 1.0 } else { 0.0 },
                        style: crate::fx::style_index(&state.config.effects_style),
                    },
                ));
        }
        toolbar(ui, ctx, state, ops);
        ui.add_space(4.0);

        match state.scan_state {
            ScanState::NotStarted => {
                let message = state.i18n.tr("ui.welcome_scan");
                empty_state(ui, ctx, state, ops, &message, pal);
            }
            ScanState::Running => {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.spinner();
                    ui.label(RichText::new(state.i18n.tr("ui.scanning_games")).color(pal.text_dim));
                });
            }
            ScanState::Done => {
                let indices = state.filtered_indices();
                if indices.is_empty() {
                    let message = state.i18n.tr("ui.no_games_match");
                    empty_state(ui, ctx, state, ops, &message, pal);
                } else {
                    match state.view_mode {
                        ViewMode::CardsLarge => grid(ui, ctx, state, ops, &indices, &CARD_LARGE),
                        ViewMode::CardsSmall => grid(ui, ctx, state, ops, &indices, &CARD_SMALL),
                        ViewMode::List => list_view(ui, ctx, state, ops, &indices),
                        ViewMode::Details => details_view(ui, ctx, state, ops, &indices),
                    }
                }
            }
        }
    });
}

/// Icon + translation key per view mode (Explorer's View menu).
const VIEW_MODES: [(ViewMode, &str, &str); 4] = [
    (ViewMode::CardsLarge, "🔳", "ui.view_large"),
    (ViewMode::CardsSmall, "▦", "ui.view_small"),
    (ViewMode::List, "📄", "ui.view_list"),
    (ViewMode::Details, "☰", "ui.view_details"),
];

fn toolbar(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState, ops: &mut Ops) {
    let pal = theme::palette(state.dark());

    // Single row, Explorer-style: search + actions left, count right.
    // Filters, sort, and view each live behind one menu button.
    ui.horizontal(|ui| {
        let search_hint = state.i18n.tr("ui.search_games");
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text(search_hint)
                .desired_width(200.0),
        );

        filter_menu(ui, state);
        sort_menu(ui, state);
        view_menu(ui, state);

        if state.filters_active()
            && ui
                .button("✕")
                .on_hover_text(state.i18n.tr("ui.clear_filters"))
                .clicked()
        {
            state.clear_filters();
        }

        ui.separator();

        if ui
            .add_enabled(
                !ops.scan_running(),
                egui::Button::new(format!("⟳ {}", state.i18n.tr("ui.rescan"))),
            )
            .clicked()
        {
            state.scan_state = ScanState::Running;
            ops.spawn_scan(ctx, state.config.excluded_drive_letters());
        }

        if ui
            .button(format!("📁 {}", state.i18n.tr("ui.add_folder")))
            .clicked()
        {
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
                    RichText::new(format!(
                        "{} {}",
                        state.filtered_indices().len(),
                        state.i18n.tr("ui.games_found")
                    ))
                    .color(pal.text_dim),
                );
            }
        });
    });
}

/// One "Filter" button expanding to per-category submenus. The button label
/// shows how many filters are active (search box not counted — it's visible).
fn filter_menu(ui: &mut egui::Ui, state: &mut AppState) {
    let active = [
        state.platform_filter.is_some(),
        state.engine_filter.is_some(),
        state.optiscaler_filter.is_some(),
        state.anticheat_filter.is_some(),
    ]
    .iter()
    .filter(|on| **on)
    .count();
    let label = if active > 0 {
        format!("🔽 {} ({active})", state.i18n.tr("ui.filter"))
    } else {
        format!("🔽 {}", state.i18n.tr("ui.filter"))
    };

    ui.menu_button(label, |ui| {
        // Platform
        let all_platforms = state.i18n.tr("ui.all_platforms");
        let platform_now = state
            .platform_filter
            .map(|p| p.label().to_string())
            .unwrap_or_else(|| all_platforms.clone());
        ui.menu_button(
            format!("{}: {platform_now}", state.i18n.tr("ui.sort_platform")),
            |ui| {
                ui.selectable_value(&mut state.platform_filter, None, &all_platforms);
                for (platform, label) in PLATFORM_FILTERS.iter().skip(1) {
                    ui.selectable_value(&mut state.platform_filter, *platform, *label);
                }
            },
        );

        // Engine
        let all_engines = state.i18n.tr("ui.all_engines");
        let engine_now = state
            .engine_filter
            .map(|e| e.label().to_string())
            .unwrap_or_else(|| all_engines.clone());
        ui.menu_button(
            format!("{}: {engine_now}", state.i18n.tr("ui.sort_engine")),
            |ui| {
                ui.selectable_value(&mut state.engine_filter, None, &all_engines);
                for engine in Engine::ALL {
                    ui.selectable_value(&mut state.engine_filter, Some(engine), engine.label());
                }
            },
        );

        // OptiScaler status
        let opti_now = match state.optiscaler_filter {
            Some(true) => state.i18n.tr("ui.filter_installed"),
            Some(false) => state.i18n.tr("ui.filter_not_installed"),
            None => state.i18n.tr("ui.all_label"),
        };
        ui.menu_button(format!("OptiScaler: {opti_now}"), |ui| {
            ui.selectable_value(
                &mut state.optiscaler_filter,
                None,
                state.i18n.tr("ui.all_label"),
            );
            ui.selectable_value(
                &mut state.optiscaler_filter,
                Some(true),
                state.i18n.tr("ui.filter_installed"),
            );
            ui.selectable_value(
                &mut state.optiscaler_filter,
                Some(false),
                state.i18n.tr("ui.filter_not_installed"),
            );
        });

        // Anti-cheat
        let ac_now = match state.anticheat_filter {
            Some(true) => state.i18n.tr("ui.ac_with"),
            Some(false) => state.i18n.tr("ui.ac_without"),
            None => state.i18n.tr("ui.all_label"),
        };
        ui.menu_button(format!("Anti-cheat: {ac_now}"), |ui| {
            ui.selectable_value(
                &mut state.anticheat_filter,
                None,
                state.i18n.tr("ui.all_label"),
            );
            ui.selectable_value(
                &mut state.anticheat_filter,
                Some(true),
                state.i18n.tr("ui.ac_with"),
            );
            ui.selectable_value(
                &mut state.anticheat_filter,
                Some(false),
                state.i18n.tr("ui.ac_without"),
            );
        });

        ui.separator();
        if ui
            .button(format!("✕ {}", state.i18n.tr("ui.clear_filters")))
            .clicked()
        {
            state.clear_filters();
        }
    });
}

/// One "Sort" button: column list + direction, Explorer's Sort menu.
fn sort_menu(ui: &mut egui::Ui, state: &mut AppState) {
    let arrow = if state.sort_ascending { "⬆" } else { "⬇" };
    let label = format!(
        "{} {} {arrow}",
        state.i18n.tr("ui.sort_by"),
        state.i18n.tr(state.sort_key.tr_key())
    );
    ui.menu_button(label, |ui| {
        for key in SortKey::ALL {
            if ui
                .selectable_label(state.sort_key == key, state.i18n.tr(key.tr_key()))
                .clicked()
                && state.sort_key != key
            {
                state.set_sort(key);
            }
        }
        ui.separator();
        if ui
            .selectable_label(
                state.sort_ascending,
                format!("⬆ {}", state.i18n.tr("ui.ascending")),
            )
            .clicked()
            && !state.sort_ascending
        {
            state.set_sort(state.sort_key); // same key → flips direction
        }
        if ui
            .selectable_label(
                !state.sort_ascending,
                format!("⬇ {}", state.i18n.tr("ui.descending")),
            )
            .clicked()
            && state.sort_ascending
        {
            state.set_sort(state.sort_key);
        }
    });
}

/// One "View" button listing the four presentations, Explorer's View menu.
fn view_menu(ui: &mut egui::Ui, state: &mut AppState) {
    let (_, icon, tr_key) = VIEW_MODES
        .iter()
        .find(|(mode, _, _)| *mode == state.view_mode)
        .unwrap_or(&VIEW_MODES[0]);
    ui.menu_button(format!("{icon} {}", state.i18n.tr(tr_key)), |ui| {
        for (mode, icon, tr_key) in VIEW_MODES {
            if ui
                .selectable_label(
                    state.view_mode == mode,
                    format!("{icon} {}", state.i18n.tr(tr_key)),
                )
                .clicked()
            {
                state.set_view_mode(mode);
            }
        }
    });
}

fn empty_state(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    message: &str,
    pal: theme::Palette,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.label(RichText::new(message).size(16.0).color(pal.text_dim));
        ui.add_space(12.0);
        if !ops.scan_running()
            && ui
                .add(egui::Button::new(
                    RichText::new(format!("🔍 {}", state.i18n.tr("ui.scan_for_games"))).size(16.0),
                ))
                .clicked()
        {
            state.scan_state = ScanState::Running;
            ops.spawn_scan(ctx, state.config.excluded_drive_letters());
        }
    });
}

fn grid(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    indices: &[usize],
    dims: &CardDims,
) {
    let avail = ui.available_width();
    let cols = ((avail + GRID_GAP) / (dims.w + GRID_GAP)).floor().max(1.0) as usize;
    let rows = indices.len().div_ceil(cols);
    let row_height = dims.h + GRID_GAP;

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
                        card(ui, ctx, state, ops, &game, dims);
                    }
                });
            }
        });
}

fn card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    game: &Game,
    dims: &CardDims,
) {
    let pal = theme::palette(state.dark());
    let (rect, response) = ui.allocate_exact_size(Vec2::new(dims.w, dims.h), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let selected = state.selected.as_deref() == Some(game.key.path_norm.as_str());
    // Pointer-based hover: the play overlay sits on top of the card, so
    // response.hovered() would flicker off while the pointer is on it.
    let hovered = ui.rect_contains_pointer(rect);
    let fill = if selected || hovered {
        pal.card_hover
    } else {
        pal.card
    };
    let stroke = if selected {
        Stroke::new(1.5_f32, pal.accent)
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
        Vec2::new(dims.w - 8.0, dims.art_h),
    );
    paint_art(ui, ctx, state, ops, game, art_rect, pal);

    // Name
    let name_pos = egui::pos2(rect.min.x + 8.0, art_rect.max.y + 8.0);
    let name = truncate(&game.name, dims.name_chars);
    ui.painter().text(
        name_pos,
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(dims.name_font),
        ui.visuals().text_color(),
    );

    // Badge row
    let mut badge_pos = egui::pos2(rect.min.x + 8.0, rect.max.y - 24.0);
    badge_pos = badge(ui, badge_pos, game.platform.label(), pal.badge_platform);
    if game.optiscaler_installed {
        let label = if dims.w < 180.0 {
            "✓"
        } else {
            "OptiScaler ✓"
        };
        badge_pos = badge(ui, badge_pos, label, pal.badge_ok);
    }
    if !game.anti_cheat.is_empty() {
        badge_pos = badge(ui, badge_pos, "⚠ AC", pal.badge_warn);
    }
    if !game.engine_supported && game.engine != Engine::Unknown {
        badge(ui, badge_pos, "engine", pal.badge_danger);
    }

    // Play badge: same shape/height/row as the platform badges, always
    // visible, right-aligned, accent blue. Its interact is registered after
    // the card's click sense, so it wins the hit test on top of it.
    {
        let label = if dims.w < 180.0 {
            "▶".to_string()
        } else {
            format!("▶ {}", state.i18n.tr("ui.play"))
        };
        let font = egui::FontId::proportional(10.0);
        let galley = ui.painter().layout_no_wrap(label, font, Color32::WHITE);
        let padding = Vec2::new(6.0, 3.0);
        let size = galley.size() + padding * 2.0;
        let play_rect = egui::Rect::from_min_size(
            egui::pos2(rect.max.x - size.x - 8.0, rect.max.y - 24.0),
            size,
        );
        let play = ui.interact(
            play_rect,
            ui.id().with(("play", game.key.path_norm.as_str())),
            Sense::click(),
        );
        let stroke = if play.hovered() {
            Stroke::new(1.0_f32, Color32::WHITE)
        } else {
            Stroke::NONE
        };
        ui.painter().rect(
            play_rect,
            CornerRadius::same(4),
            pal.accent,
            stroke,
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(play_rect.min + padding, galley, Color32::WHITE);
        if play.clicked() {
            match opticore::launch::launch(game, true) {
                Ok(message) => state.push_log(message),
                Err(error) => state.push_log(format!("Launch failed: {error}")),
            }
        }
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

/// Draw a game's artwork into `art_rect` (requesting a fetch when unknown).
fn paint_art(
    ui: &egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    game: &Game,
    art_rect: egui::Rect,
    pal: theme::Palette,
) {
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
            placeholder_art(ui, art_rect, "…", pal);
        }
        ArtState::Fetching => placeholder_art(ui, art_rect, "…", pal),
        ArtState::Missing => placeholder_art(ui, art_rect, &game.name, pal),
    }
}

fn truncate(name: &str, max_chars: usize) -> String {
    if name.chars().count() > max_chars {
        let truncated: String = name.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    } else {
        name.to_string()
    }
}

/// Compact rows: thumbnail, name, badges. Explorer's "List".
fn list_view(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    ops: &mut Ops,
    indices: &[usize],
) {
    let pal = theme::palette(state.dark());
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, LIST_ROW_H, indices.len(), |ui, row_range| {
            for row in row_range {
                let game = state.games[indices[row]].clone();
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), LIST_ROW_H - 2.0),
                    Sense::click(),
                );
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let selected = state.selected.as_deref() == Some(game.key.path_norm.as_str());
                if selected || response.hovered() {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(4), pal.card_hover);
                }

                let art_rect = egui::Rect::from_min_size(
                    rect.min + Vec2::new(4.0, 2.0),
                    Vec2::new(42.0, LIST_ROW_H - 6.0),
                );
                paint_art(ui, ctx, state, ops, &game, art_rect, pal);

                ui.painter().text(
                    egui::pos2(art_rect.max.x + 10.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &game.name,
                    egui::FontId::proportional(13.0),
                    ui.visuals().text_color(),
                );

                let mut badge_pos = egui::pos2(rect.max.x - 170.0, rect.min.y + 6.0);
                badge_pos = badge(ui, badge_pos, game.platform.label(), pal.badge_platform);
                if game.optiscaler_installed {
                    badge_pos = badge(ui, badge_pos, "✓", pal.badge_ok);
                }
                if !game.anti_cheat.is_empty() {
                    badge(ui, badge_pos, "⚠", pal.badge_warn);
                }

                if response.clicked() {
                    state.selected = if selected {
                        None
                    } else {
                        Some(game.key.path_norm.clone())
                    };
                }
            }
        });
}

/// Explorer's "Details": columns with clickable, sort-aware headers.
fn details_view(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    state: &mut AppState,
    _ops: &mut Ops,
    indices: &[usize],
) {
    let pal = theme::palette(state.dark());
    const COL_PLATFORM: f32 = 90.0;
    const COL_ENGINE: f32 = 90.0;
    const COL_OPTI: f32 = 110.0;
    let name_w = (ui.available_width() * 0.30).max(180.0);

    // Header row: click sorts, click again reverses
    ui.horizontal(|ui| {
        let columns: [(Option<SortKey>, &str, f32); 5] = [
            (Some(SortKey::Name), "ui.sort_name", name_w),
            (Some(SortKey::Platform), "ui.sort_platform", COL_PLATFORM),
            (Some(SortKey::Engine), "ui.sort_engine", COL_ENGINE),
            (Some(SortKey::Optiscaler), "ui.sort_optiscaler", COL_OPTI),
            (None, "ui.column_path", 0.0),
        ];
        for (key, tr_key, width) in columns {
            let mut label = state.i18n.tr(tr_key);
            if key == Some(state.sort_key) {
                label = format!("{label} {}", if state.sort_ascending { "▲" } else { "▼" });
            }
            let text = RichText::new(label).strong();
            let clicked = if width > 0.0 {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::click());
                ui.painter().text(
                    egui::pos2(rect.min.x + 4.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    text.text(),
                    egui::FontId::proportional(12.5),
                    if response.hovered() {
                        pal.accent
                    } else {
                        ui.visuals().text_color()
                    },
                );
                response.clicked()
            } else {
                ui.label(text.size(12.5)).clicked()
            };
            if clicked {
                if let Some(key) = key {
                    state.set_sort(key);
                }
            }
        }
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, 24.0, indices.len(), |ui, row_range| {
            for row in row_range {
                let game = state.games[indices[row]].clone();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let selected = state.selected.as_deref() == Some(game.key.path_norm.as_str());
                if selected || response.hovered() {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(3), pal.card_hover);
                }

                let font = egui::FontId::proportional(12.5);
                let y = rect.center().y;
                let mut x = rect.min.x + 4.0;
                let cells: [(String, f32, Color32); 4] = [
                    (game.name.clone(), name_w, ui.visuals().text_color()),
                    (
                        game.platform.label().to_string(),
                        COL_PLATFORM,
                        pal.text_dim,
                    ),
                    (game.engine.label().to_string(), COL_ENGINE, pal.text_dim),
                    (
                        if game.optiscaler_installed {
                            "✓ OptiScaler".to_string()
                        } else {
                            "—".to_string()
                        },
                        COL_OPTI,
                        if game.optiscaler_installed {
                            pal.badge_ok
                        } else {
                            pal.text_dim
                        },
                    ),
                ];
                for (text, width, color) in cells {
                    ui.painter().text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        color,
                    );
                    x += width + ui.spacing().item_spacing.x;
                }
                ui.painter().text(
                    egui::pos2(x, y),
                    egui::Align2::LEFT_CENTER,
                    game.path.display().to_string(),
                    egui::FontId::proportional(11.5),
                    pal.text_dim,
                );

                if response.clicked() {
                    state.selected = if selected {
                        None
                    } else {
                        Some(game.key.path_norm.clone())
                    };
                }
            }
        });
}

fn placeholder_art(ui: &egui::Ui, rect: egui::Rect, label: &str, pal: theme::Palette) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), pal.bg);
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
        pal.text_dim,
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
    let pal = theme::palette(state.dark());
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
            ui.label(RichText::new("Platform").color(pal.text_dim));
            ui.label(game.platform.label());
            ui.end_row();
            ui.label(RichText::new("Engine").color(pal.text_dim));
            ui.label(format!("{:?}", game.engine));
            ui.end_row();
            if let Some(appid) = game.steam_appid {
                ui.label(RichText::new("Steam AppID").color(pal.text_dim));
                ui.label(appid.to_string());
                ui.end_row();
            }
            ui.label(RichText::new("OptiScaler").color(pal.text_dim));
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
            .color(pal.text_dim),
    );
    ui.add_space(8.0);
    ui.separator();

    play_section(ui, state, game, pal);
    ui.separator();

    install_section(ui, ctx, state, ops, game);

    ui.add_space(8.0);
    if ui
        .button(format!("📂 {}", state.i18n.tr("ui.open_folder")))
        .clicked()
    {
        let _ = std::process::Command::new("explorer")
            .arg(&game.path)
            .spawn();
    }
}

/// Play buttons: with OptiScaler (restores the proxy first) or without
/// (renames our proxy away for a clean boot). Single Play when nothing is
/// installed. Steam games start via the Steam client, Xbox via the bundled
/// gamelaunchhelper, everything else via the game's main exe.
fn play_section(ui: &mut egui::Ui, state: &mut AppState, game: &Game, pal: theme::Palette) {
    ui.add_space(4.0);
    let mut result: Option<Result<String, String>> = None;
    if game.optiscaler_installed {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(format!("▶ {}", state.i18n.tr("ui.play_with"))).strong())
                .clicked()
            {
                result = Some(opticore::launch::launch(game, true));
            }
            if ui
                .button(format!("▶ {}", state.i18n.tr("ui.play_without")))
                .clicked()
            {
                result = Some(opticore::launch::launch(game, false));
            }
        });
        if opticore::launch::optiscaler_bypassed(&game.path) {
            ui.label(
                RichText::new(state.i18n.tr("ui.optiscaler_bypassed"))
                    .small()
                    .color(pal.badge_warn),
            );
        }
    } else if ui
        .button(RichText::new(format!("▶ {}", state.i18n.tr("ui.play"))).strong())
        .clicked()
    {
        result = Some(opticore::launch::launch(game, true));
    }
    match result {
        Some(Ok(message)) => state.push_log(message),
        Some(Err(error)) => state.push_log(format!("Launch failed: {error}")),
        None => {}
    }
    ui.add_space(4.0);
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
    let pal = theme::palette(state.dark());
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
        let color = if *ok { pal.badge_ok } else { pal.badge_danger };
        ui.colored_label(color, message.clone());
        ui.add_space(4.0);
    }

    // Anti-cheat warning requires explicit confirmation before install/update
    let needs_confirm = !game.anti_cheat.is_empty();
    if needs_confirm {
        ui.colored_label(
            pal.badge_warn,
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
            state.i18n.tr("ui.anticheat_confirm"),
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
                    pal.accent,
                    format!("Update available: {installed} → {latest}"),
                );
                if ui
                    .add_enabled(
                        allowed,
                        egui::Button::new(
                            RichText::new(format!("⬆ {}", state.i18n.tr("ui.update_optiscaler")))
                                .strong(),
                        ),
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
                        .color(pal.text_dim),
                );
            }
        } else if let Some(installed) = installed {
            ui.label(RichText::new(format!("Installed: {installed}")).color(pal.text_dim));
        }

        ui.horizontal(|ui| {
            if ui
                .button(format!("⚙ {}", state.i18n.tr("ui.edit_settings")))
                .clicked()
            {
                state.open_editor(game);
            }
            if ui
                .add_enabled(
                    allowed,
                    egui::Button::new(format!("🗑 {}", state.i18n.tr("ui.uninstall"))),
                )
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
            ui.label(RichText::new(state.i18n.tr("ui.install_as")).color(pal.text_dim));
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
                egui::Button::new(
                    RichText::new(format!("⬇ {}", state.i18n.tr("ui.install_optiscaler"))).strong(),
                ),
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
