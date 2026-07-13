//! Epic Games metadata helpers. Port of `_read_epic_game_name` and the
//! .egstore/.mancfg install markers.

use serde_json::Value;
use std::fs;
use std::path::Path;

/// Known Epic install roots.
pub fn default_roots() -> Vec<std::path::PathBuf> {
    [
        r"C:\Program Files\Epic Games",
        r"C:\Program Files (x86)\Epic Games",
    ]
    .iter()
    .map(std::path::PathBuf::from)
    .filter(|p| p.is_dir())
    .collect()
}

/// True if the folder carries Epic install metadata.
pub fn has_epic_markers(game_folder: &Path) -> bool {
    if game_folder.join(".egstore").exists() {
        return true;
    }
    mancfg_files(game_folder).next().is_some()
}

fn mancfg_files(dir: &Path) -> impl Iterator<Item = std::path::PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "mancfg").unwrap_or(false))
}

/// Read the real game title from Epic metadata (DisplayName / AppName in
/// .egstore/*.mancfg or top-level *.mancfg). Empty result → caller falls
/// back to a prettified folder name.
pub fn read_game_name(game_folder: &Path) -> Option<String> {
    let egstore = game_folder.join(".egstore");
    let candidates = mancfg_files(&egstore).chain(mancfg_files(game_folder));
    for mf in candidates {
        let Ok(content) = fs::read_to_string(&mf) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        let title = data
            .get("DisplayName")
            .or_else(|| data.get("AppName"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if title.len() > 1 {
            return Some(title.trim().to_string());
        }
    }
    None
}

/// The technical AppName (launcher-protocol id), distinct from DisplayName.
pub fn read_app_name(game_folder: &Path) -> Option<String> {
    let egstore = game_folder.join(".egstore");
    let candidates = mancfg_files(&egstore).chain(mancfg_files(game_folder));
    for mf in candidates {
        let Ok(content) = fs::read_to_string(&mf) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        let app = data.get("AppName").and_then(Value::as_str).unwrap_or("");
        if !app.is_empty() {
            return Some(app.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_app_name_distinct_from_display_name() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("G");
        let egstore = game.join(".egstore");
        fs::create_dir_all(&egstore).unwrap();
        fs::write(
            egstore.join("abc.mancfg"),
            r#"{"DisplayName": "Nice Name", "AppName": "Sugar"}"#,
        )
        .unwrap();
        assert_eq!(read_app_name(&game).as_deref(), Some("Sugar"));
        assert_eq!(read_game_name(&game).as_deref(), Some("Nice Name"));
    }

    #[test]
    fn reads_display_name_from_egstore() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("SomeGame");
        let egstore = game.join(".egstore");
        fs::create_dir_all(&egstore).unwrap();
        fs::write(
            egstore.join("abc.mancfg"),
            r#"{"DisplayName": "Some Game: Deluxe"}"#,
        )
        .unwrap();
        assert!(has_epic_markers(&game));
        assert_eq!(read_game_name(&game).as_deref(), Some("Some Game: Deluxe"));
    }

    #[test]
    fn missing_metadata_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Bare");
        fs::create_dir_all(&game).unwrap();
        assert!(!has_epic_markers(&game));
        assert!(read_game_name(&game).is_none());
    }
}
