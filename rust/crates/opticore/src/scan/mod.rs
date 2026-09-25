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

use crate::model::{DiscoverySource, Game, GameKey, Platform, StoreIdentity};
use discovery::{LibraryRoot, RootKind};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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

/// What one scanner call observed about an entry. Callers state the evidence
/// explicitly, so `build_game` never has to infer identity from a title.
#[derive(Debug, Clone)]
struct Discovery {
    source: DiscoverySource,
    identity: Option<StoreIdentity>,
}

/// A folder's own name, used only where no store metadata exists for it.
fn folder_stem(folder: &Path) -> String {
    folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Build a Game from a validated game folder + its facts.
fn build_game(
    name: String,
    appid: Option<u32>,
    path: &Path,
    platform: Platform,
    facts: &folder_facts::FolderFacts,
    discovery: Discovery,
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
        store_identity: discovery.identity,
        discovery_source: discovery.source,
        engine,
        engine_supported: folder_facts::is_engine_supported(engine),
        anti_cheat: folder_facts::detect_anti_cheat(facts),
        community_verified: false, // filled by caller (needs final name)
        optiscaler_installed: folder_facts::detect_optiscaler(path, facts),
        art_url: None,
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
        let (name, appid, source) = match manifest_map.get(&folder_name.to_lowercase()) {
            Some(m) => (m.name.clone(), m.appid, DiscoverySource::StoreManifest),
            None => (folder_name, None, DiscoverySource::FolderScan),
        };
        games.push(build_game(
            name,
            appid,
            &folder,
            Platform::Steam,
            &facts,
            Discovery {
                source,
                identity: appid.map(StoreIdentity::steam),
            },
            verified,
        ));
    }
}

/// Scan a root whose child folders are individual games (Epic/GOG style).
/// `describe` yields the entry's title, how it was identified and the store
/// identifier its metadata carried (if any) — never an invented one.
fn scan_children<F>(
    root: &Path,
    platform: Platform,
    verified: &VerifiedList,
    games: &mut Vec<Game>,
    describe: F,
) where
    F: Fn(&Path) -> (String, DiscoverySource, Option<String>),
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
        let (name, source, store_id) = describe(&folder);
        games.push(build_game(
            name,
            None,
            &folder,
            platform,
            &facts,
            Discovery {
                source,
                identity: store_id.map(|id| StoreIdentity::new(platform, id)),
            },
            verified,
        ));
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
            Discovery {
                source: DiscoverySource::LauncherLibrary,
                identity: None,
            },
            verified,
        ));
    }
}

/// The identity a launcher-discovered entry carries.
///
/// The store that published the id wins over the launcher that read the file,
/// so a GOG title found through Heroic is `(Gog, <id>)`, never `(Heroic, <id>)`.
pub fn heroic_identity(entry: &heroic::HeroicEntry) -> Option<StoreIdentity> {
    let id = entry.store_id.as_ref()?;
    Some(StoreIdentity::new(entry.store.platform(), id.clone()))
}

fn scan_heroic(verified: &VerifiedList, games: &mut Vec<Game>) {
    let mut seen_paths = HashSet::new();
    for root in heroic::config_roots() {
        for entry in heroic::installed_entries(&root) {
            // Read the identity before the fields below are moved out of the entry.
            let identity = heroic_identity(&entry);
            let install_path = entry.install_path;
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
            let name = entry.title.unwrap_or_else(|| {
                install_path
                    .file_name()
                    .map(|n| n.to_string_lossy().replace(['_', '-'], " "))
                    .unwrap_or_default()
            });
            let mut game = build_game(
                name,
                None,
                &install_path,
                Platform::Heroic,
                &facts,
                Discovery {
                    source: DiscoverySource::LauncherLibrary,
                    identity,
                },
                verified,
            );
            game.art_url = entry.art_url;
            games.push(game);
        }
    }
}

/// Epic/GOG metadata resolution used by both known roots and discovered
/// roots. The store id comes from the same metadata the title does, so an
/// entry that only has a folder name is `FolderScan` with no identity.
fn epic_describe(folder: &Path) -> (String, DiscoverySource, Option<String>) {
    match epic::read_metadata(folder) {
        Some(meta) => (meta.title, DiscoverySource::StoreManifest, meta.app_name),
        None => {
            let raw = folder_stem(folder).replace(['_', '-'], " ");
            (
                names::title_case(&names::split_camel_case(&raw)),
                DiscoverySource::FolderScan,
                None,
            )
        }
    }
}

fn gog_describe(folder: &Path) -> (String, DiscoverySource, Option<String>) {
    let id = gog::read_game_id(folder);
    let title = gog::read_game_title(folder);
    let source = if id.is_some() || title.is_some() {
        DiscoverySource::StoreManifest
    } else {
        DiscoverySource::FolderScan
    };
    let name = title.unwrap_or_else(|| names::folder_name_to_title(&folder_stem(folder)));
    (name, source, id)
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
        scan_children(&root, Platform::Epic, &verified, &mut games, epic_describe);
    }
    for root in gog::default_roots() {
        scan_children(&root, Platform::Gog, &verified, &mut games, gog_describe);
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
                scan_children(&path, Platform::Epic, &verified, &mut games, epic_describe)
            }
            RootKind::Gog => {
                scan_children(&path, Platform::Gog, &verified, &mut games, gog_describe)
            }
            RootKind::Xbox => scan_xbox_root(&path, &verified, &mut games),
        }
    }

    // Launcher/redistributable filter, then the path-aware dedup pass: one
    // entry per normalized install path, so two installs of one title both
    // survive while every repeated hit for one install is merged. No entry is
    // dropped on the strength of its title or its identity - those decide which
    // facts the merged entry reports.
    games.retain(|g| !names::is_launcher_entry(&g.name));

    ScanResult {
        games: dedup_games(games),
    }
}

/// How far a discovery source's *title* can be trusted, highest first. A store
/// manifest names the game itself, a launcher library carries the store's own
/// title, and a folder walk can only prettify the folder name.
fn title_quality(source: DiscoverySource) -> u8 {
    match source {
        DiscoverySource::StoreManifest => 3,
        DiscoverySource::LauncherLibrary => 2,
        DiscoverySource::UserSelected => 1,
        DiscoverySource::FolderScan => 0,
    }
}

/// Copy the facts `source` has and `target` lacks. Nothing is ever overwritten,
/// so merging two hits for one install can only add information.
fn fill_gaps(target: &mut Game, source: Game) {
    if target.store_identity.is_none() {
        target.store_identity = source.store_identity;
    }
    if target.art_url.is_none() {
        target.art_url = source.art_url;
    }
    if target.steam_appid.is_none() {
        target.steam_appid = source.steam_appid;
    }
}

/// Collapse the repeated hits one install can produce during a scan.
///
/// One entry survives per normalized install path: two installs of one title —
/// different paths, or the same title reached through different stores — both
/// stay, while every repeated hit for ONE install is folded into that entry.
/// Identity is provenance, not a key: a Heroic copy that also sits under a
/// store's own root is ONE install, and keying on identity would show it twice.
///
/// Folding is a merge, never a replacement. The hit with the better title keeps
/// that title, and with it its platform and source, because those say where the
/// title came from; the hit it displaced contributes the store identity,
/// artwork and Steam appid that the winner lacks. So an Epic manifest's real
/// `DisplayName` survives a launcher hit that only had a folder name for the
/// same install, and the launcher's id survives next to it.
pub fn dedup_games(games: Vec<Game>) -> Vec<Game> {
    let mut unique: Vec<Game> = Vec::with_capacity(games.len());
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for game in games {
        match index_of.get(&game.key.path_norm).copied() {
            Some(at)
                if title_quality(game.discovery_source)
                    > title_quality(unique[at].discovery_source) =>
            {
                // The incoming hit has the better title, so it becomes the entry
                // and the occupant it displaces only fills the gaps it left.
                let displaced = std::mem::replace(&mut unique[at], game);
                fill_gaps(&mut unique[at], displaced);
            }
            Some(at) => fill_gaps(&mut unique[at], game),
            None => {
                index_of.insert(game.key.path_norm.clone(), unique.len());
                unique.push(game);
            }
        }
    }
    unique
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
        Discovery {
            source: DiscoverySource::UserSelected,
            identity: None,
        },
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
