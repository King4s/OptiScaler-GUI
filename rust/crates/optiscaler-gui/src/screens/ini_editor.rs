//! OptiScaler.ini editor: sections as collapsing headers, widgets chosen by
//! inferred value kind, key search, dirty-state guard, GPU Auto Settings.

use crate::state::{AppState, Screen};
use crate::theme;
use eframe::egui::{self, Align, Layout, RichText};
use opticore::ini::{auto_settings, ValueKind};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    let pal = theme::palette(state.dark());
    let gpu_vendor = state.gpu_vendor;
    // Take ownership for the frame so the closure doesn't borrow `state`
    let Some(mut editor) = state.editor.take() else {
        state.screen = Screen::Games;
        return;
    };
    let mut close = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        let editor = &mut editor;
        // Header row
        ui.horizontal(|ui| {
            let back_label = if editor.discard_armed {
                "← Discard changes?"
            } else {
                "← Back"
            };
            if ui.button(back_label).clicked() {
                if editor.dirty && !editor.discard_armed {
                    editor.discard_armed = true;
                } else {
                    close = true;
                }
            }
            if editor.discard_armed {
                ui.label(
                    RichText::new("Unsaved changes — click Back again to discard")
                        .color(pal.badge_warn),
                );
            }
            ui.heading(format!("Settings — {}", editor.game_name));
        });
        if close {
            return;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut editor.search)
                    .hint_text("Search settings…")
                    .desired_width(220.0),
            );
            if ui
                .button(format!("✨ Auto Settings ({})", gpu_vendor.label()))
                .clicked()
            {
                let mut changes = Vec::new();
                for (section, key, value) in auto_settings(gpu_vendor) {
                    let old = editor.doc.get(section, key).map(|e| e.value.clone());
                    match old {
                        Some(old) if old != value => {
                            editor.doc.set_value(section, key, value);
                            changes.push(format!("{section}.{key}: {old} → {value}"));
                        }
                        _ => {}
                    }
                }
                editor.dirty = editor.dirty || !changes.is_empty();
                editor.status = Some(format!(
                    "Auto Settings ({}): {} changes — review below and Save",
                    gpu_vendor.label(),
                    changes.len()
                ));
                editor.applied_changes = changes;
            }

            // Restore upstream defaults from the cached release payload
            let restore_button = egui::Button::new("↩ Restore defaults");
            let restore = ui.add_enabled(editor.defaults.is_some(), restore_button);
            let restore = restore.on_disabled_hover_text(
                "Defaults come from the downloaded OptiScaler release — install or update once to enable",
            );
            if restore.clicked() {
                if let Some(defaults) = editor.defaults.clone() {
                    let mut changes = Vec::new();
                    for section in &defaults.sections {
                        for entry in &section.entries {
                            let old = editor
                                .doc
                                .get(&section.name, &entry.key)
                                .map(|e| e.value.clone());
                            if let Some(old) = old {
                                if old != entry.value {
                                    editor.doc.set_value(&section.name, &entry.key, &entry.value);
                                    changes.push(format!(
                                        "{}.{}: {old} → {}",
                                        section.name, entry.key, entry.value
                                    ));
                                }
                            }
                        }
                    }
                    editor.dirty = editor.dirty || !changes.is_empty();
                    editor.status =
                        Some(format!("Restored defaults: {} changes — Save to keep", changes.len()));
                    editor.applied_changes = changes;
                }
            }
            let save = egui::Button::new(RichText::new("💾 Save").strong());
            if ui.add_enabled(editor.dirty, save).clicked() {
                match opticore::ini::write_file(&editor.ini_path, &editor.doc) {
                    Ok(()) => {
                        editor.dirty = false;
                        editor.discard_armed = false;
                        editor.applied_changes.clear();
                        editor.status = Some("Saved (previous file kept as .ini.backup)".into());
                    }
                    Err(e) => editor.status = Some(format!("Save failed: {e}")),
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(status) = &editor.status {
                    ui.label(RichText::new(status).color(pal.text_dim));
                }
            });
        });
        ui.add_space(6.0);

        // What Auto Settings / Restore defaults actually changed
        if !editor.applied_changes.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new(format!("Changes applied ({})", editor.applied_changes.len()))
                    .color(pal.accent),
            )
            .default_open(true)
            .show(ui, |ui| {
                for change in &editor.applied_changes {
                    ui.label(RichText::new(change).monospace().size(12.0));
                }
            });
            ui.add_space(6.0);
        }

        let needle = editor.search.to_lowercase();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for section_idx in 0..editor.doc.sections.len() {
                    let (section_name, entry_count) = {
                        let s = &editor.doc.sections[section_idx];
                        (s.name.clone(), s.entries.len())
                    };
                    // Section filter: match section name or any key
                    let section_matches = needle.is_empty()
                        || section_name.to_lowercase().contains(&needle)
                        || editor.doc.sections[section_idx]
                            .entries
                            .iter()
                            .any(|e| e.key.to_lowercase().contains(&needle));
                    if !section_matches || entry_count == 0 {
                        continue;
                    }
                    egui::CollapsingHeader::new(
                        RichText::new(format!("{section_name}  ({entry_count})")).strong(),
                    )
                    .id_salt(&section_name)
                    .default_open(!needle.is_empty())
                    .show(ui, |ui| {
                        for entry_idx in 0..editor.doc.sections[section_idx].entries.len() {
                            let (key, comment, kind) = {
                                let e = &editor.doc.sections[section_idx].entries[entry_idx];
                                (e.key.clone(), e.comment.clone(), e.kind.clone())
                            };
                            if !needle.is_empty()
                                && !key.to_lowercase().contains(&needle)
                                && !section_name.to_lowercase().contains(&needle)
                            {
                                continue;
                            }
                            let default_value = editor
                                .defaults
                                .as_ref()
                                .and_then(|d| d.get(&section_name, &key))
                                .map(|e| e.value.clone());
                            let value =
                                &mut editor.doc.sections[section_idx].entries[entry_idx].value;
                            let changed = entry_row(
                                ui,
                                &section_name,
                                &key,
                                &comment,
                                &kind,
                                value,
                                default_value.as_deref(),
                            );
                            if changed {
                                editor.dirty = true;
                                editor.discard_armed = false;
                            }
                        }
                    });
                }
            });
    });

    if close {
        state.screen = Screen::Games;
    } else {
        state.editor = Some(editor);
    }
}

/// One setting row. Returns true if the value changed this frame.
#[allow(clippy::too_many_arguments)]
fn entry_row(
    ui: &mut egui::Ui,
    section: &str,
    key: &str,
    comment: &str,
    kind: &ValueKind,
    value: &mut String,
    default_value: Option<&str>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let label = ui.label(RichText::new(key).size(13.0));
        if !comment.is_empty() {
            label.on_hover_text(comment);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Per-key reset when the value differs from the upstream default
            if let Some(default) = default_value {
                if default != value.as_str()
                    && ui
                        .small_button("↺")
                        .on_hover_text(format!("Reset to default: {default}"))
                        .clicked()
                {
                    *value = default.to_string();
                    changed = true;
                }
            }
            let widget_id = format!("{section}.{key}");
            match kind {
                ValueKind::BoolOptions => {
                    egui::ComboBox::from_id_salt(&widget_id)
                        .selected_text(value.clone())
                        .width(110.0)
                        .show_ui(ui, |ui| {
                            for option in ["true", "false", "auto"] {
                                if ui
                                    .selectable_value(value, option.to_string(), option)
                                    .clicked()
                                {
                                    changed = true;
                                }
                            }
                        });
                }
                ValueKind::Options(options) => {
                    let display = options
                        .get(value.as_str())
                        .cloned()
                        .unwrap_or_else(|| value.clone());
                    egui::ComboBox::from_id_salt(&widget_id)
                        .selected_text(display)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for (raw, label) in options {
                                if ui.selectable_value(value, raw.clone(), label).clicked() {
                                    changed = true;
                                }
                            }
                        });
                }
                ValueKind::Int => {
                    let mut parsed = value.parse::<i64>().unwrap_or(0);
                    let response = ui.add(egui::DragValue::new(&mut parsed).speed(1));
                    if response.changed() {
                        *value = parsed.to_string();
                        changed = true;
                    }
                }
                ValueKind::Float => {
                    let mut parsed = value.parse::<f64>().unwrap_or(0.0);
                    let response = ui.add(egui::DragValue::new(&mut parsed).speed(0.05));
                    if response.changed() {
                        *value = format!("{parsed}");
                        changed = true;
                    }
                }
                ValueKind::Text => {
                    let response = ui.add(egui::TextEdit::singleline(value).desired_width(180.0));
                    if response.changed() {
                        changed = true;
                    }
                }
            }
        });
    });
    changed
}
