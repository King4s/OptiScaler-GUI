//! GOG metadata helpers: goggame-*.info gameTitle lookup.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Known GOG install roots.
pub fn default_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![PathBuf::from(r"C:\Program Files (x86)\GOG Galaxy\Games")];
    if let Some(home) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(home).join("GOG Games"));
    }
    roots.into_iter().filter(|p| p.is_dir()).collect()
}

/// Read `gameTitle` from the first goggame-*.info file in the folder.
pub fn read_game_title(game_folder: &Path) -> Option<String> {
    let entries = fs::read_dir(game_folder).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("goggame-") && name.ends_with(".info") {
            let content = fs::read_to_string(entry.path()).ok()?;
            let data: Value = serde_json::from_str(&content).ok()?;
            if let Some(title) = data.get("gameTitle").and_then(Value::as_str) {
                return Some(title.to_string());
            }
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_game_title() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("cp2077");
        fs::create_dir_all(&game).unwrap();
        fs::write(
            game.join("goggame-1091500.info"),
            r#"{"gameTitle": "Cyberpunk 2077"}"#,
        )
        .unwrap();
        assert_eq!(read_game_title(&game).as_deref(), Some("Cyberpunk 2077"));
    }

    #[test]
    fn missing_info_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("bare");
        fs::create_dir_all(&game).unwrap();
        assert!(read_game_title(&game).is_none());
    }
}
