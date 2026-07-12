//! Scan orchestration: runs all platform scanners, dedups results, and
//! filters launcher/redistributable entries — the Rust port of the Python
//! `GameScanner.scan_games` flow (post-v0.5.2 semantics: each Steam library
//! scanned once, one folder walk per game, no image fetching here).

pub mod discovery;
pub mod epic;
pub mod folder_facts;
pub mod gog;
pub mod heroic;
pub mod names;
pub mod steam;
pub mod xbox;

use crate::model::{Game, GameKey, Platform};
use discovery::{LibraryRoot, RootKind};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct ScanConfig {
    /// Uppercase drive letters to skip during discovery (no colon).
    pub excluded_drives: Vec<char>,
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub games: Vec<Game>,
}

/// Community-verified game list bundled from the Python app's data file
/// (kept as the single source of truth at src/data/).
struct VerifiedList {
    entries: Vec<(String, String)>, // (name_lower, appid)
}

impl VerifiedList {
    fn load() -> Self {
        let raw = include_str!("../../../../../src/data/community_verified_games.json");
        let mut entries = Vec::new();
        if let Ok(data) = serde_json::from_str::<Value>(raw) {
            for g in data
                .get("games")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let name = g.get("name").and_then(Value::as_str).unwrap_or("");
                let appid = g.get("appid").and_then(Value::as_str).unwrap_or("");
                if !name.is_empty() {
                    entries.push((name.to_lowercase(), appid.to_string()));
                }
            }
        }
        Self { entries }
    }

    /// Port of `_is_community_verified`: match by name or by appid.
    /// (The Python original also checks the (name, appid) pair first, but
    /// that branch is subsumed by the name-only check.)
    fn is_verified(&self, name: &str, appid: Option<u32>) -> bool {
        let name_key = name.to_lowercase();
        let appid_key = appid.map(|a| a.to_string()).unwrap_or_default();
        self.entries
            .iter()
            .any(|(n, a)| n == &name_key || (!appid_key.is_empty() && a == &appid_key))
    }
}

/// Build a Game from a validated game folder + its facts.
fn build_game(
    name: String,
    appid: Option<u32>,
    path: &Path,
    platform: Platform,
    facts: &folder_facts::FolderFacts,
    verified: &VerifiedList,
) -> Game {
    let engine = folder_facts::detect_engine(path, facts);
    Game {
        key: GameKey {
            name_lower: name.to_lowercase(),
            path_norm: path.to_string_lossy().to_lowercase(),
        },
        name,
        path: path.to_path_buf(),
        platform,
        steam_appid: appid,
        engine,
        engine_supported: folder_facts::is_engine_supported(engine),
        anti_cheat: folder_facts::detect_anti_cheat(facts),
        community_verified: false, // filled by caller (needs final name)
        optiscaler_installed: folder_facts::detect_optiscaler(path, facts),
    }
    .tap_verify(verified)
}

trait TapVerify {
    fn tap_verify(self, verified: &VerifiedList) -> Self;
}
impl TapVerify for Game {
    fn tap_verify(mut self, verified: &VerifiedList) -> Self {
        self.community_verified = verified.is_verified(&self.name, self.steam_appid);
        self
    }
}

/// Scan one Steam library root exactly once per scan pass.
fn scan_steam_library(
    library_root: &Path,
    scanned_roots: &mut HashSet<String>,
    verified: &VerifiedList,
    games: &mut Vec<Game>,
) {
    let steamapps = library_root.join("steamapps");
    let common = steamapps.join("common");
    if !common.is_dir() {
        return;
    }
    let key = common.to_string_lossy().to_lowercase();
    if !scanned_roots.insert(key) {
        return;
    }
    let manifest_map = steam::build_manifest_map(&steamapps);
    let Ok(entries) = std::fs::read_dir(&common) else {
        return;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let Some(facts) = folder_facts::collect(&folder) else {
            continue;
        };
        if !folder_facts::is_game_folder(&folder, &facts) {
            continue;
        }
        let folder_name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let (name, appid) = match manifest_map.get(&folder_name.to_lowercase()) {
            Some(m) => (m.name.clone(), m.appid),
            None => (folder_name, None),
        };
        games.push(build_game(
            name,
            appid,
            &folder,
            Platform::Steam,
            &facts,
            verified,
        ));
    }
}

/// Scan a root whose child folders are individual games (Epic/GOG style).
fn scan_children<F>(
    root: &Path,
    platform: Platform,
    verified: &VerifiedList,
    games: &mut Vec<Game>,
    name_for: F,
) where
    F: Fn(&Path) -> String,
{
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let Some(facts) = folder_facts::collect(&folder) else {
            continue;
        };
        if !folder_facts::is_game_folder(&folder, &facts) {
            continue;
        }
        let name = name_for(&folder);
        games.push(build_game(name, None, &folder, platform, &facts, verified));
    }
}

fn scan_xbox_root(root: &Path, verified: &VerifiedList, games: &mut Vec<Game>) {
    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is_xboxgames = root_name == "xboxgames";
    let is_windowsapps = root_name == "windowsapps";
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let folder_name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if is_windowsapps && !names::is_appx_game_candidate(&folder_name) {
            continue; // cheap name filter before walking the package
        }
        let Some(facts) = folder_facts::collect(&folder) else {
            continue;
        };
        let name = if is_xboxgames {
            if !folder_facts::is_game_folder(&folder, &facts) && !xbox::is_xbox_game_folder(&folder)
            {
                continue;
            }
            names::folder_name_to_title(&folder_name)
        } else if is_windowsapps {
            if !folder_facts::is_game_folder(&folder, &facts) {
                continue;
            }
            let parsed = names::title_case(&names::parse_appx_package_name(&folder_name));
            if parsed.len() < 2 {
                continue;
            }
            parsed
        } else {
            if !folder_facts::is_game_folder(&folder, &facts) {
                continue;
            }
            names::folder_name_to_title(&folder_name)
        };
        games.push(build_game(
            name,
            None,
            &folder,
            Platform::Xbox,
            &facts,
            verified,
        ));
    }
}

fn scan_heroic(verified: &VerifiedList, games: &mut Vec<Game>) {
    let mut seen_paths = HashSet::new();
    for root in heroic::config_roots() {
        for (title, install_path) in heroic::installed_entries(&root) {
            let norm = install_path.to_string_lossy().to_lowercase();
            if seen_paths.contains(&norm) || !install_path.is_dir() {
                continue;
            }
            let Some(facts) = folder_facts::collect(&install_path) else {
                continue;
            };
            if !folder_facts::is_game_folder(&install_path, &facts) {
                continue;
            }
            seen_paths.insert(norm);
            let name = title.unwrap_or_else(|| {
                install_path
                    .file_name()
                    .map(|n| n.to_string_lossy().replace(['_', '-'], " "))
                    .unwrap_or_default()
            });
            games.push(build_game(
                name,
                None,
                &install_path,
                Platform::Heroic,
                &facts,
                verified,
            ));
        }
    }
}

/// Epic/GOG name resolution used by both known roots and discovered roots.
fn epic_name(folder: &Path) -> String {
    epic::read_game_name(folder).unwrap_or_else(|| {
        let raw = folder
            .file_name()
            .map(|n| n.to_string_lossy().replace(['_', '-'], " "))
            .unwrap_or_default();
        names::title_case(&names::split_camel_case(&raw))
    })
}

fn gog_name(folder: &Path) -> String {
    gog::read_game_title(folder).unwrap_or_else(|| {
        names::folder_name_to_title(
            &folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
    })
}

/// Full scan across all platforms. Port of `GameScanner.scan_games`.
pub fn scan_all(config: &ScanConfig) -> ScanResult {
    let verified = VerifiedList::load();
    let mut games: Vec<Game> = Vec::new();
    let mut scanned_steam_roots: HashSet<String> = HashSet::new();

    // Steam: install roots + libraryfolders.vdf libraries, each scanned once
    for library_root in steam::all_library_roots() {
        scan_steam_library(
            &library_root,
            &mut scanned_steam_roots,
            &verified,
            &mut games,
        );
    }

    // Epic / GOG / Xbox known roots
    for root in epic::default_roots() {
        scan_children(&root, Platform::Epic, &verified, &mut games, epic_name);
    }
    for root in gog::default_roots() {
        scan_children(&root, Platform::Gog, &verified, &mut games, gog_name);
    }
    for root in xbox::default_roots() {
        scan_xbox_root(&root, &verified, &mut games);
    }

    // Heroic store files
    scan_heroic(&verified, &mut games);

    // Drive discovery for library roots outside the defaults
    for LibraryRoot { kind, path } in discovery::discover_roots(&config.excluded_drives) {
        match kind {
            RootKind::Steam => {
                // discovered path is the library root itself (contains steamapps)
                scan_steam_library(&path, &mut scanned_steam_roots, &verified, &mut games);
            }
            RootKind::Epic => {
                scan_children(&path, Platform::Epic, &verified, &mut games, epic_name)
            }
            RootKind::Gog => scan_children(&path, Platform::Gog, &verified, &mut games, gog_name),
            RootKind::Xbox => scan_xbox_root(&path, &verified, &mut games),
        }
    }

    // Launcher/redistributable filter, then dedup (same keys as Python)
    games.retain(|g| !names::is_launcher_entry(&g.name));
    let mut unique: Vec<Game> = Vec::with_capacity(games.len());
    let mut seen_path_keys: HashSet<(String, String)> = HashSet::new();
    let mut seen_name_platform: HashSet<(String, Platform)> = HashSet::new();
    for game in games {
        let path_key = (game.key.name_lower.clone(), game.key.path_norm.clone());
        if seen_path_keys.contains(&path_key) {
            continue;
        }
        let plat_key = (game.key.name_lower.clone(), game.platform);
        if seen_name_platform.contains(&plat_key) {
            continue;
        }
        seen_path_keys.insert(path_key);
        seen_name_platform.insert(plat_key);
        unique.push(game);
    }

    ScanResult { games: unique }
}

/// Scan a single, explicitly chosen folder (the "manual path" flow).
pub fn scan_manual_folder(folder: &Path) -> Option<Game> {
    let verified = VerifiedList::load();
    let facts = folder_facts::collect(folder)?;
    if !folder_facts::is_game_folder(folder, &facts) {
        return None;
    }
    let name = names::folder_name_to_title(
        &folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    Some(build_game(
        name,
        None,
        folder,
        Platform::Manual,
        &facts,
        &verified,
    ))
}

/// Extra Steam library roots callers may inject (used by tests).
pub fn scan_steam_root_for_tests(library_root: &Path) -> Vec<Game> {
    let verified = VerifiedList::load();
    let mut games = Vec::new();
    let mut scanned = HashSet::new();
    scan_steam_library(library_root, &mut scanned, &verified, &mut games);
    games
}

pub type PathList = Vec<PathBuf>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn make_game_dir(base: &Path, name: &str) -> PathBuf {
        let d = base.join(name);
        fs::create_dir_all(&d).unwrap();
        File::create(d.join("game.exe")).unwrap();
        for i in 0..6 {
            File::create(d.join(format!("asset{i}.pak"))).unwrap();
        }
        d
    }

    #[test]
    fn steam_library_scanned_once_with_manifest_names() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        let steamapps = lib.join("steamapps");
        fs::create_dir_all(steamapps.join("common")).unwrap();
        make_game_dir(&steamapps.join("common"), "TestGame");
        let mut f = File::create(steamapps.join("appmanifest_111.acf")).unwrap();
        f.write_all(
            b"\"AppState\"\n{\n\t\"appid\"\t\t\"111\"\n\t\"name\"\t\t\"Test Game\"\n\t\"installdir\"\t\t\"TestGame\"\n}\n",
        )
        .unwrap();

        let verified = VerifiedList::load();
        let mut games = Vec::new();
        let mut scanned = HashSet::new();
        scan_steam_library(lib, &mut scanned, &verified, &mut games);
        scan_steam_library(lib, &mut scanned, &verified, &mut games); // second scan: no-op

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Test Game");
        assert_eq!(games[0].steam_appid, Some(111));
        assert_eq!(games[0].platform, Platform::Steam);
    }

    #[test]
    fn community_verified_matches() {
        let verified = VerifiedList::load();
        assert!(verified.is_verified("Cyberpunk 2077", Some(1091500)));
        assert!(verified.is_verified("cyberpunk 2077", None)); // name-only
        assert!(!verified.is_verified("Some Unknown Game", None));
    }

    #[test]
    fn manual_folder_scan() {
        let tmp = tempfile::tempdir().unwrap();
        let d = make_game_dir(tmp.path(), "my_manual-game");
        let game = scan_manual_folder(&d).expect("valid game folder");
        assert_eq!(game.name, "My Manual Game");
        assert_eq!(game.platform, Platform::Manual);
    }
}
