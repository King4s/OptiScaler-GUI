//! Core domain types shared by the scanner, installer, and GUI.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

impl GameKey {
    /// The one place an install path is folded into a library key.
    ///
    /// Folds case, both separator kinds and trailing separators, and folds to
    /// backslashes because that is what `os.path.normcase` produces on Windows:
    /// the Rust and Python scanners then agree on the same install instead of
    /// keeping one entry each. A path spelled the native way keeps the exact key
    /// older builds wrote, so nothing persisted for it moves.
    ///
    /// It deliberately does **not** collapse `.` or `..` segments. A junction or
    /// symlink can make a purely textual shortening point at a different install,
    /// and merging two genuinely different installs is worse than showing one
    /// twice. Resolving through the filesystem would be correct but costs a stat
    /// per path on every scan, so it is a separate decision (see `tasks/todo.md`,
    /// T2b).
    ///
    /// Every path-keyed comparison and map goes through this: `Game::new`,
    /// `build_game` and the scanner's own duplicate guards, plus the persisted
    /// `game_gpus` and `game_results` maps. The result is a key, never a path:
    /// use `Game::path` for anything that touches the filesystem.
    pub fn path_key(path: &Path) -> String {
        path.to_string_lossy()
            .to_lowercase()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_string()
    }
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

/// Where an entry's title came from.
///
/// This is deliberately not the discovery source. A GOG folder can be found
/// through its own store file while that file carries no `gameTitle`, and then
/// the title is only prettified from the folder name even though the entry was
/// discovered through a store manifest. Ranking titles by the discovery source
/// let such a folder name outrank a launcher's real title for the same install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TitleSource {
    /// Prettified from the folder name. The weakest source.
    #[default]
    Folder,
    /// A launcher's own library metadata.
    Launcher,
    /// The store's own metadata for this game.
    Store,
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
    /// Where this entry's title came from. Absent from older JSON, which loads
    /// as `Folder` — the least-claiming default.
    #[serde(default)]
    pub title_source: TitleSource,
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
            path_norm: GameKey::path_key(&path),
        };
        Self {
            key,
            name,
            path,
            platform,
            steam_appid: None,
            store_identity: None,
            discovery_source: DiscoverySource::FolderScan,
            title_source: TitleSource::Folder,
            engine: Engine::Unknown,
            engine_supported: true,
            anti_cheat: Vec::new(),
            community_verified: false,
            optiscaler_installed: false,
            art_url: None,
        }
    }

    /// The key this game's path produced before [`GameKey::path_key`] folded
    /// separators and trailing separators.
    ///
    /// Only for reading data an older build persisted under that spelling: the
    /// per-game GPU choice and the user's own test result. Never write a new
    /// entry under it.
    pub fn legacy_path_norm(&self) -> String {
        self.path.to_string_lossy().to_lowercase()
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

    #[test]
    fn path_key_folds_separators_case_and_a_trailing_separator() {
        let native = GameKey::path_key(Path::new(r"C:\Games\Cyberpunk 2077"));
        assert_eq!(native, r"c:\games\cyberpunk 2077");

        // The same install as a launcher's JSON spells it.
        assert_eq!(
            GameKey::path_key(Path::new("C:/Games/Cyberpunk 2077")),
            native
        );
        assert_eq!(
            GameKey::path_key(Path::new("C:/Games/Cyberpunk 2077/")),
            native
        );
        assert_eq!(
            GameKey::path_key(Path::new("c:\\games\\cyberpunk 2077\\")),
            native
        );
    }

    #[test]
    fn path_key_leaves_parent_segments_alone() {
        // Textual `..` folding is wrong across a junction or a symlink, so the
        // segment has to stay and the key has to stay a different one.
        let with_parent = GameKey::path_key(Path::new(r"C:\Games\..\Other"));
        assert_eq!(with_parent, r"c:\games\..\other");
        assert_ne!(with_parent, GameKey::path_key(Path::new(r"C:\Other")));
    }

    #[test]
    fn legacy_path_norm_keeps_the_spelling_older_builds_wrote() {
        let forward = Game::new("Test Game", PathBuf::from("C:/Games/Test"), Platform::Steam);
        assert_eq!(forward.key.path_norm, r"c:\games\test");
        assert_eq!(forward.legacy_path_norm(), "c:/games/test");

        let trailing = Game::new(
            "Test Game",
            PathBuf::from("C:\\Games\\Test\\"),
            Platform::Steam,
        );
        assert_eq!(trailing.key.path_norm, r"c:\games\test");
        assert_eq!(trailing.legacy_path_norm(), "c:\\games\\test\\");
    }
}
