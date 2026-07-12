//! Heroic Games Launcher store parsing. Direct port of the Python
//! `_heroic_installed_entries` (v0.5.1) — one store file per backend:
//! - Epic: legendaryConfig/legendary/installed.json (dict by app_name)
//! - GOG: gog_store/installed.json + title cache (store_cache/gog_library.json
//!   or the older gog_store/library.json)
//! - Amazon: nile_config/nile/installed.json + library.json
//! - Sideload: sideload_apps/library.json

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// (title, install_path) — title is None when only the folder name is known.
pub type HeroicEntry = (Option<String>, PathBuf);

fn read_json(path: &Path) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Existing Heroic config roots (%APPDATA%/heroic, %LOCALAPPDATA%/heroic).
pub fn config_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for var in ["APPDATA", "LOCALAPPDATA"] {
        if let Some(base) = std::env::var_os(var) {
            let candidate = PathBuf::from(base).join("heroic");
            let key = candidate.to_string_lossy().to_lowercase();
            if candidate.is_dir() && seen.insert(key) {
                roots.push(candidate);
            }
        }
    }
    roots
}

/// Collect (title, install_path) pairs from one Heroic config root.
pub fn installed_entries(root: &Path) -> Vec<HeroicEntry> {
    let mut entries = Vec::new();

    // Epic games via Heroic's bundled legendary
    if let Some(Value::Object(map)) = read_json(
        &root
            .join("legendaryConfig")
            .join("legendary")
            .join("installed.json"),
    ) {
        for meta in map.values() {
            if let Some(install_path) = meta.get("install_path").and_then(Value::as_str) {
                let title = meta.get("title").and_then(Value::as_str).map(String::from);
                entries.push((title, PathBuf::from(install_path)));
            }
        }
    }

    // GOG titles from the library cache (newer then older location)
    let mut gog_titles: HashMap<String, String> = HashMap::new();
    for lib_rel in [
        root.join("store_cache").join("gog_library.json"),
        root.join("gog_store").join("library.json"),
    ] {
        if let Some(lib) = read_json(&lib_rel) {
            for game in lib
                .get("games")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let app = game
                    .get("app_name")
                    .or_else(|| game.get("appName"))
                    .and_then(Value::as_str);
                let title = game.get("title").and_then(Value::as_str);
                if let (Some(app), Some(title)) = (app, title) {
                    gog_titles
                        .entry(app.to_string())
                        .or_insert_with(|| title.to_string());
                }
            }
        }
    }
    if let Some(data) = read_json(&root.join("gog_store").join("installed.json")) {
        for meta in data
            .get("installed")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            if let Some(install_path) = meta.get("install_path").and_then(Value::as_str) {
                let app = meta
                    .get("appName")
                    .or_else(|| meta.get("app_name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                entries.push((gog_titles.get(app).cloned(), PathBuf::from(install_path)));
            }
        }
    }

    // Amazon games via nile
    let nile_dir = root.join("nile_config").join("nile");
    let mut nile_titles: HashMap<String, String> = HashMap::new();
    if let Some(Value::Array(lib)) = read_json(&nile_dir.join("library.json")) {
        for game in &lib {
            let product = game.get("product");
            let title = game
                .get("title")
                .and_then(Value::as_str)
                .or_else(|| product.and_then(|p| p.get("title")).and_then(Value::as_str));
            let id = game
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| product.and_then(|p| p.get("id")).and_then(Value::as_str));
            if let (Some(id), Some(title)) = (id, title) {
                nile_titles.insert(id.to_string(), title.to_string());
            }
        }
    }
    if let Some(Value::Array(installed)) = read_json(&nile_dir.join("installed.json")) {
        for meta in &installed {
            if let Some(path) = meta.get("path").and_then(Value::as_str) {
                let title = meta
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| nile_titles.get(id).cloned());
                entries.push((title, PathBuf::from(path)));
            }
        }
    }

    // Sideloaded apps
    if let Some(data) = read_json(&root.join("sideload_apps").join("library.json")) {
        for game in data
            .get("games")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            if game.get("is_installed").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let folder = game
                .get("folder_name")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .or_else(|| {
                    game.get("install")
                        .and_then(|i| i.get("executable"))
                        .and_then(Value::as_str)
                        .and_then(|exe| PathBuf::from(exe).parent().map(Path::to_path_buf))
                });
            if let Some(folder) = folder {
                let title = game.get("title").and_then(Value::as_str).map(String::from);
                entries.push((title, folder));
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_json(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn parses_all_four_backends() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("heroic");

        write_json(
            &root
                .join("legendaryConfig")
                .join("legendary")
                .join("installed.json"),
            r#"{"app1": {"title": "Epic Game", "install_path": "C:\\Games\\Epic"}}"#,
        );
        write_json(
            &root.join("gog_store").join("installed.json"),
            r#"{"installed": [{"appName": "42", "install_path": "C:\\Games\\Gog"}]}"#,
        );
        write_json(
            &root.join("store_cache").join("gog_library.json"),
            r#"{"games": [{"app_name": "42", "title": "GOG Game"}]}"#,
        );
        write_json(
            &root.join("nile_config").join("nile").join("installed.json"),
            r#"[{"id": "amzn1", "path": "C:\\Games\\Amazon"}]"#,
        );
        write_json(
            &root.join("nile_config").join("nile").join("library.json"),
            r#"[{"id": "amzn1", "product": {"title": "Amazon Game"}}]"#,
        );
        write_json(
            &root.join("sideload_apps").join("library.json"),
            r#"{"games": [{"title": "Side Game", "is_installed": true, "folder_name": "C:\\Games\\Side"}]}"#,
        );

        let mut titles: Vec<Option<String>> = installed_entries(&root)
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        titles.sort();
        assert_eq!(titles.len(), 4);
        assert!(titles.contains(&Some("Epic Game".into())));
        assert!(titles.contains(&Some("GOG Game".into())));
        assert!(titles.contains(&Some("Amazon Game".into())));
        assert!(titles.contains(&Some("Side Game".into())));
    }

    #[test]
    fn gog_title_missing_yields_none() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("heroic");
        write_json(
            &root.join("gog_store").join("installed.json"),
            r#"{"installed": [{"appName": "7", "install_path": "C:\\G\\NoTitle"}]}"#,
        );
        let entries = installed_entries(&root);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].0.is_none());
    }

    #[test]
    fn uninstalled_sideload_skipped_and_malformed_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("heroic");
        write_json(
            &root.join("sideload_apps").join("library.json"),
            r#"{"games": [{"title": "Gone", "is_installed": false, "folder_name": "C:\\G\\Gone"}]}"#,
        );
        write_json(
            &root
                .join("legendaryConfig")
                .join("legendary")
                .join("installed.json"),
            "{not valid json",
        );
        assert!(installed_entries(&root).is_empty());
    }
}
