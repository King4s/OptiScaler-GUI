//! App configuration persisted to `cache/config.json` — the same file the
//! Python app uses. Unknown fields are preserved on save so the two apps can
//! share the file without clobbering each other's settings.

use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub language: String,
    /// "dark" (default) or "light"
    pub theme: String,
    /// GPU background effects (M6) — on by default, off when reduced motion
    pub effects_enabled: bool,
    /// Comma-separated uppercase drive letters, e.g. "D,E" (Python format)
    pub excluded_drives: String,
    pub check_updates: bool,
    /// Fields we don't own (Python app settings) — preserved on save.
    passthrough: Map<String, Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "dark".to_string(),
            effects_enabled: true,
            excluded_drives: String::new(),
            check_updates: true,
            passthrough: Map::new(),
        }
    }
}

impl AppConfig {
    pub fn config_path(base_dir: &Path) -> PathBuf {
        base_dir.join("cache").join("config.json")
    }

    /// Load from cache/config.json. Python-only keys are kept in
    /// passthrough; Python's `language` key is imported on first run.
    pub fn load(path: &Path) -> Self {
        let mut config = Self::default();
        let Ok(raw) = std::fs::read_to_string(path) else {
            return config;
        };
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&raw) else {
            return config;
        };
        for (key, value) in map {
            match key.as_str() {
                "language" => {
                    if let Some(s) = value.as_str() {
                        config.language = s.to_string();
                    }
                }
                "theme" => {
                    if let Some(s) = value.as_str() {
                        // Python stores Light/Dark/System — normalize
                        config.theme = if s.eq_ignore_ascii_case("light") {
                            "light".to_string()
                        } else {
                            "dark".to_string()
                        };
                    }
                }
                "effects_enabled" => {
                    if let Some(b) = value.as_bool() {
                        config.effects_enabled = b;
                    }
                }
                "excluded_drives" => {
                    if let Some(s) = value.as_str() {
                        config.excluded_drives = s.to_string();
                    }
                }
                "check_updates" => {
                    if let Some(b) = value.as_bool() {
                        config.check_updates = b;
                    }
                }
                _ => {
                    config.passthrough.insert(key, value);
                }
            }
        }
        config
    }

    /// Save, merging our fields over the preserved passthrough fields.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut map = self.passthrough.clone();
        map.insert("language".into(), Value::String(self.language.clone()));
        map.insert("theme".into(), Value::String(self.theme.clone()));
        map.insert("effects_enabled".into(), Value::Bool(self.effects_enabled));
        map.insert(
            "excluded_drives".into(),
            Value::String(self.excluded_drives.clone()),
        );
        map.insert("check_updates".into(), Value::Bool(self.check_updates));
        let json = serde_json::to_string_pretty(&Value::Object(map))?;
        std::fs::write(path, json)
    }

    /// Excluded drive letters in the scanner's format.
    pub fn excluded_drive_letters(&self) -> Vec<char> {
        self.excluded_drives
            .split(',')
            .filter_map(|part| part.trim().chars().next())
            .map(|c| c.to_ascii_uppercase())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_python_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        // A Python-written config with keys we don't own
        std::fs::write(
            &path,
            r#"{"language": "da", "debug": true, "max_workers": 8, "excluded_drives": "d, e"}"#,
        )
        .unwrap();

        let mut config = AppConfig::load(&path);
        assert_eq!(config.language, "da");
        assert_eq!(config.excluded_drive_letters(), vec!['D', 'E']);

        config.theme = "light".to_string();
        config.save(&path).unwrap();

        let raw: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Python fields survived
        assert_eq!(raw.get("debug"), Some(&Value::Bool(true)));
        assert_eq!(raw.get("max_workers").and_then(Value::as_i64), Some(8));
        // our fields written
        assert_eq!(raw.get("theme").and_then(Value::as_str), Some("light"));
    }

    #[test]
    fn defaults_when_missing() {
        let config = AppConfig::load(Path::new("Z:/does/not/exist.json"));
        assert_eq!(config.language, "en");
        assert_eq!(config.theme, "dark");
        assert!(config.effects_enabled);
        assert!(config.excluded_drive_letters().is_empty());
    }
}
