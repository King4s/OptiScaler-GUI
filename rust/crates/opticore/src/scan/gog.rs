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

const GOG_PREFIX: &str = "goggame-";
const GOG_SUFFIX: &str = ".info";

/// The GOG product id carried by a `goggame-<id>.info` file name.
///
/// GOG product ids are digits only, so anything else is not an id: a folder
/// file called `goggame-backup.info` must not be reported as the product id
/// `backup`, or the scanner would hand a store id to a metadata lookup that
/// cannot possibly resolve it.
pub fn product_id_from_file_name(file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    let digits = lower.strip_prefix(GOG_PREFIX)?.strip_suffix(GOG_SUFFIX)?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Lowercasing cannot have changed a digits-only string, so this is the id
    // exactly as the file name spells it.
    Some(digits.to_string())
}

/// This folder's `goggame-<id>.info` store files: regular files whose name
/// carries a real product id, sorted so that no choice below ever depends on
/// `read_dir` order.
fn gog_info_candidates(game_folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(game_folder) else {
        return Vec::new();
    };
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|entry| product_id_from_file_name(&entry.file_name().to_string_lossy()).is_some())
        .map(|entry| entry.path())
        .collect();
    candidates.sort();
    candidates
}

/// Read `gameTitle` from this folder's GOG store file.
///
/// The candidates are the same validated, sorted ones the id lookup uses, so a
/// stray file such as `goggame-backup.info` can neither supply the title nor
/// change which file it is read from.
pub fn read_game_title(game_folder: &Path) -> Option<String> {
    let content = fs::read_to_string(gog_info_candidates(game_folder).first()?).ok()?;
    let data: Value = serde_json::from_str(&content).ok()?;
    data.get("gameTitle")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Read this folder's GOG product id.
///
/// GOG keys its games by this id, so it is stable identity. A folder that
/// offers more than one distinct id yields `None` rather than a guess: a wrong
/// store id would send a later cover lookup after the wrong game, and no id is
/// a better answer than a coin flip.
pub fn read_game_id(game_folder: &Path) -> Option<String> {
    let mut ids: Vec<String> = gog_info_candidates(game_folder)
        .iter()
        .filter_map(|path| path.file_name())
        .filter_map(|name| product_id_from_file_name(&name.to_string_lossy()))
        .collect();
    ids.sort();
    ids.dedup();
    match ids.len() {
        1 => ids.pop(),
        _ => None,
    }
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

    #[test]
    fn non_numeric_file_name_is_neither_an_id_nor_a_title_source() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("backup");
        fs::create_dir_all(&game).unwrap();
        fs::write(
            game.join("goggame-backup.info"),
            r#"{"gameTitle": "Not This One"}"#,
        )
        .unwrap();

        assert_eq!(read_game_id(&game), None);
        assert_eq!(read_game_title(&game), None);
    }

    #[test]
    fn two_distinct_ids_yield_no_id_and_a_deterministic_title() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("two");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("goggame-2.info"), r#"{"gameTitle": "Second"}"#).unwrap();
        fs::write(game.join("goggame-1.info"), r#"{"gameTitle": "First"}"#).unwrap();

        // Choosing one of two ids would be a guess.
        assert_eq!(read_game_id(&game), None);
        // The title still comes from a defined candidate, not read_dir order.
        assert_eq!(read_game_title(&game).as_deref(), Some("First"));
    }

    #[test]
    fn a_directory_named_like_a_store_file_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("dir");
        fs::create_dir_all(game.join("goggame-1091500.info")).unwrap();

        assert_eq!(read_game_id(&game), None);
        assert_eq!(read_game_title(&game), None);
    }
}
