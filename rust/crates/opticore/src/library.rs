//! Per-game launcher data: favorites, hidden games, playtime, and launch
//! options. Persisted to `cache/library.json` — ours alone (the Python app
//! never touches it). Entries that are all-default are pruned on save so the
//! file only holds games the user actually customized.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameEntry {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub playtime_minutes: u64,
    /// ISO-8601 local timestamp of the last GUI launch ("" = never).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_played: String,
    /// Extra command-line arguments for launches.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub launch_args: String,
    /// Absolute path overriding the auto-detected game exe.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub exe_override: String,
    /// Extra environment variables applied to direct-exe launches.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl GameEntry {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    /// Keyed by the game's normalized path (`Game.key.path_norm`).
    #[serde(default)]
    pub games: BTreeMap<String, GameEntry>,
}

impl Library {
    pub fn library_path(base_dir: &Path) -> PathBuf {
        base_dir.join("cache").join("library.json")
    }

    /// Load from disk; a missing or corrupt file is an empty library.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Save, dropping entries that carry no information.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pruned = Library {
            games: self
                .games
                .iter()
                .filter(|(_, e)| !e.is_default())
                .map(|(k, e)| (k.clone(), e.clone()))
                .collect(),
        };
        let json = serde_json::to_string_pretty(&pruned)?;
        std::fs::write(path, json)
    }

    /// Read-only view of a game's entry (default when absent).
    pub fn entry(&self, key: &str) -> GameEntry {
        self.games.get(key).cloned().unwrap_or_default()
    }

    pub fn entry_mut(&mut self, key: &str) -> &mut GameEntry {
        self.games.entry(key.to_string()).or_default()
    }

    /// Record a finished play session.
    pub fn add_playtime(&mut self, key: &str, minutes: u64) {
        self.entry_mut(key).playtime_minutes += minutes;
    }

    /// Stamp "last played" with the current local time.
    pub fn touch_last_played(&mut self, key: &str) {
        self.entry_mut(key).last_played = iso_now();
    }
}

/// ISO-8601 local timestamp (same shape as the install manifest's).
fn iso_now() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    now.format(&format)
        .unwrap_or_else(|_| "1970-01-01T00:00:00".to_string())
}

/// "2 h 15 min"-style playtime label ("" for zero).
pub fn format_playtime(minutes: u64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, 0) => String::new(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.json");

        let mut lib = Library::default();
        lib.entry_mut("c:/games/a").favorite = true;
        lib.add_playtime("c:/games/a", 95);
        lib.entry_mut("c:/games/b").hidden = true;
        // Touched but all-default → pruned on save
        lib.entry_mut("c:/games/c");
        lib.save(&path).unwrap();

        let loaded = Library::load(&path);
        assert_eq!(loaded.games.len(), 2);
        assert!(loaded.entry("c:/games/a").favorite);
        assert_eq!(loaded.entry("c:/games/a").playtime_minutes, 95);
        assert!(loaded.entry("c:/games/b").hidden);
        // Absent keys read as defaults
        assert!(loaded.entry("c:/games/zzz").is_default());
    }

    #[test]
    fn corrupt_or_missing_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.json");
        assert!(Library::load(&path).games.is_empty());
        std::fs::write(&path, "{ not json").unwrap();
        assert!(Library::load(&path).games.is_empty());
    }

    #[test]
    fn playtime_formatting() {
        assert_eq!(format_playtime(0), "");
        assert_eq!(format_playtime(45), "45 min");
        assert_eq!(format_playtime(60), "1 h");
        assert_eq!(format_playtime(135), "2 h 15 min");
    }

    #[test]
    fn last_played_is_stamped() {
        let mut lib = Library::default();
        lib.touch_last_played("k");
        let stamp = lib.entry("k").last_played;
        assert_eq!(stamp.len(), "2026-07-13T12:00:00".len());
        assert!(stamp.starts_with("20"));
    }
}
