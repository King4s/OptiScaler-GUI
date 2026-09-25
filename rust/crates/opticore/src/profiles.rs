//! Local-only profile persistence. Never serialize this type for public reports.
use crate::hardware::HardwareProfile;
use crate::model::Game;
use crate::report::UserTestResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalProfiles {
    pub hardware: Option<HardwareProfile>,
    pub game_gpus: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub game_results: BTreeMap<String, UserTestResult>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl LocalProfiles {
    /// The keys a game's path may have been recorded under, current spelling
    /// first.
    ///
    /// The second is the key `GameKey::path_key` produced before it folded
    /// separators and trailing separators ([`Game::legacy_path_norm`]). Reading
    /// both keeps a GPU choice or a test result the user already saved working;
    /// new entries are only ever written under the first.
    fn recorded_keys(game: &Game) -> [String; 2] {
        [game.key.path_norm.clone(), game.legacy_path_norm()]
    }

    /// The GPU the user picked for this game, if any.
    pub fn gpu_for(&self, game: &Game) -> Option<&String> {
        Self::recorded_keys(game)
            .iter()
            .find_map(|key| self.game_gpus.get(key))
    }

    /// The user's own test result for this game, if any.
    pub fn result_for(&self, game: &Game) -> Option<&UserTestResult> {
        Self::recorded_keys(game)
            .iter()
            .find_map(|key| self.game_results.get(key))
    }
}

impl LocalProfiles {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(std::io::Error::other),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| {
            file.write_all(&serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    }
}
