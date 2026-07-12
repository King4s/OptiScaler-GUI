//! Core domain types shared by the scanner, installer, and GUI.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Steam,
    Epic,
    Gog,
    Xbox,
    Heroic,
    Registry,
    Manual,
}

impl Platform {
    /// Label shown on game cards (matches the Python app's platform tags).
    pub fn label(self) -> &'static str {
        match self {
            Platform::Steam => "Steam",
            Platform::Epic => "Epic",
            Platform::Gog => "GOG",
            Platform::Xbox => "Xbox",
            Platform::Heroic => "Heroic",
            Platform::Registry => "Installed",
            Platform::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Unreal,
    Unity,
    Godot,
    Prism3D,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiCheat {
    EasyAntiCheat,
    BattlEye,
    Vanguard,
}

/// Dedup identity: lowercased name + normalized path, mirroring the Python
/// scanner's (name, normcase(path)) primary key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameKey {
    pub name_lower: String,
    pub path_norm: String,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub key: GameKey,
    pub name: String,
    pub path: PathBuf,
    pub platform: Platform,
    pub steam_appid: Option<u32>,
    pub engine: Engine,
    pub engine_supported: bool,
    pub anti_cheat: Vec<AntiCheat>,
    pub community_verified: bool,
    pub optiscaler_installed: bool,
    /// Artwork URL supplied by the store's own metadata (e.g. Heroic
    /// library art), tried before store-search fallbacks.
    pub art_url: Option<String>,
}

impl Game {
    pub fn new(name: impl Into<String>, path: PathBuf, platform: Platform) -> Self {
        let name = name.into();
        let key = GameKey {
            name_lower: name.to_lowercase(),
            path_norm: path.to_string_lossy().to_lowercase(),
        };
        Self {
            key,
            name,
            path,
            platform,
            steam_appid: None,
            engine: Engine::Unknown,
            engine_supported: true,
            anti_cheat: Vec::new(),
            community_verified: false,
            optiscaler_installed: false,
            art_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_key_normalizes_name_and_path() {
        let g = Game::new(
            "Test Game",
            PathBuf::from(r"C:\Games\Test"),
            Platform::Steam,
        );
        assert_eq!(g.key.name_lower, "test game");
        assert_eq!(g.key.path_norm, r"c:\games\test");
        assert_eq!(g.platform.label(), "Steam");
    }
}
