//! Core domain types shared by the scanner, installer, and GUI.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Steam,
    Epic,
    Gog,
    Amazon,
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
            Platform::Amazon => "Amazon",
            Platform::Xbox => "Xbox",
            Platform::Heroic => "Heroic",
            Platform::Registry => "Installed",
            Platform::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Engine {
    Unreal,
    Unity,
    Godot,
    Prism3D,
    Unknown,
}

impl Engine {
    pub const ALL: [Engine; 5] = [
        Engine::Unreal,
        Engine::Unity,
        Engine::Godot,
        Engine::Prism3D,
        Engine::Unknown,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Unreal => "Unreal",
            Engine::Unity => "Unity",
            Engine::Godot => "Godot",
            Engine::Prism3D => "Prism3D",
            Engine::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiCheat {
    EasyAntiCheat,
    BattlEye,
    Vanguard,
}

/// Name-plus-path key for one library entry: the lowercased title plus the
/// normalized install path, mirroring the Python scanner's
/// (name, normcase(path)) pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameKey {
    pub name_lower: String,
    pub path_norm: String,
}

/// A stable identifier for one game inside one store.
///
/// `store_id` is only ever a value the scanner actually read from store
/// metadata on disk — a Steam appid, a GOG product id, a launcher's own app
/// name. It is never derived from a title or a folder name: a game the scanner
/// cannot identify keeps `Game::store_identity == None` rather than receiving a
/// guessed id. Identity is what lets two installs of the same title stay
/// distinct; it says nothing about whether the game is compatible with
/// OptiScaler, and it never authorizes an install.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoreIdentity {
    pub platform: Platform,
    pub store_id: String,
}

impl StoreIdentity {
    pub fn new(platform: Platform, store_id: impl Into<String>) -> Self {
        Self {
            platform,
            store_id: store_id.into(),
        }
    }

    /// Steam identifies a game by its appid, read from `appmanifest_*.acf`.
    pub fn steam(appid: u32) -> Self {
        Self::new(Platform::Steam, appid.to_string())
    }
}

/// Where a library entry came from — the evidence the scanner observed, not a
/// claim about the game's compatibility or its install target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// A store manifest next to the game folder (Steam `appmanifest_*.acf`,
    /// Epic `.mancfg`, GOG `goggame-*.info`).
    StoreManifest,
    /// A launcher's own installed-games database (Heroic store files, Xbox
    /// package folders).
    LauncherLibrary,
    /// Found by walking a library root with no store metadata for this entry,
    /// so only the folder name identifies it.
    #[default]
    FolderScan,
    /// The user selected this folder explicitly.
    UserSelected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub key: GameKey,
    pub name: String,
    pub path: PathBuf,
    pub platform: Platform,
    pub steam_appid: Option<u32>,
    /// Stable store identifier when the scanner read one; `None` otherwise.
    /// Absent from JSON written before this field existed.
    #[serde(default)]
    pub store_identity: Option<StoreIdentity>,
    /// How this entry was discovered. Absent from older JSON, which loads as
    /// `FolderScan` — the least-claiming default.
    #[serde(default)]
    pub discovery_source: DiscoverySource,
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
            store_identity: None,
            discovery_source: DiscoverySource::FolderScan,
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
