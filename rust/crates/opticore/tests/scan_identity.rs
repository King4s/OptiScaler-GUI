//! Fixture tests for the store-identity / discovery-provenance contract and for
//! the legacy-JSON compatibility it must not break.
//!
//! Everything here is built in a tempdir from synthetic manifests; no test
//! reads this machine's real game libraries.

use opticore::model::{DiscoverySource, Game, Platform, StoreIdentity};
use opticore::scan::{dedup_games, epic, gog, heroic, heroic_identity, scan_steam_root_for_tests};
use std::fs;
use std::path::{Path, PathBuf};

/// A folder that satisfies the scanner's game-folder heuristic (an exe plus
/// more than five files).
fn make_game_dir(base: &Path, name: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("game.exe"), b"exe").unwrap();
    for i in 0..6 {
        fs::write(dir.join(format!("asset{i}.pak")), b"pak").unwrap();
    }
    dir
}

fn write_file_at(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Build one Steam library root holding a single game folder, plus its
/// `appmanifest_*.acf` when an appid is given.
fn steam_library(root: &Path, appid: Option<u32>, installdir: &str, title: &str) -> PathBuf {
    let game = make_game_dir(&root.join("steamapps").join("common"), installdir);
    if let Some(appid) = appid {
        write_file_at(
            &root.join("steamapps").join(format!("appmanifest_{appid}.acf")),
            &format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"name\"\t\t\"{title}\"\n\t\"installdir\"\t\t\"{installdir}\"\n}}\n"
            ),
        );
    }
    game
}

#[test]
fn store_identity_round_trips_through_json() {
    let identity = StoreIdentity::new(Platform::Gog, "1091500");
    let json = serde_json::to_string(&identity).unwrap();
    let back: StoreIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(back, identity);
    assert_eq!(back.platform, Platform::Gog);
    assert_eq!(back.store_id, "1091500");
}

#[test]
fn legacy_game_json_without_identity_fields_still_deserializes() {
    // The shape the scanner produced before `store_identity` and
    // `discovery_source` existed: neither field is present, and loading must
    // still succeed without losing anything.
    let legacy = r#"{
        "key": { "name_lower": "cyberpunk 2077", "path_norm": "c:\\games\\cyberpunk 2077" },
        "name": "Cyberpunk 2077",
        "path": "C:\\Games\\Cyberpunk 2077",
        "platform": "Steam",
        "steam_appid": 1091500,
        "engine": "Unreal",
        "engine_supported": true,
        "anti_cheat": ["EasyAntiCheat"],
        "community_verified": true,
        "optiscaler_installed": false,
        "art_url": null
    }"#;

    let game: Game = serde_json::from_str(legacy).unwrap();
    assert_eq!(game.name, "Cyberpunk 2077");
    assert_eq!(game.path, PathBuf::from(r"C:\Games\Cyberpunk 2077"));
    assert_eq!(game.steam_appid, Some(1091500));
    assert!(game.engine_supported);
    assert_eq!(game.anti_cheat.len(), 1);
    assert!(game.community_verified);
    // The two fields that did not exist yet fall back to the least-claiming
    // values instead of failing the load or inventing an identity.
    assert_eq!(game.store_identity, None);
    assert_eq!(game.discovery_source, DiscoverySource::FolderScan);
}

#[test]
fn steam_manifest_entry_carries_appid_identity_and_manifest_source() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("lib");
    steam_library(&lib, Some(1091500), "CyberpunkGame", "Cyberpunk 2077");

    let games = scan_steam_root_for_tests(&lib);

    assert_eq!(games.len(), 1);
    assert_eq!(games[0].name, "Cyberpunk 2077");
    assert_eq!(games[0].steam_appid, Some(1091500));
    assert_eq!(games[0].store_identity, Some(StoreIdentity::steam(1091500)));
    assert_eq!(games[0].discovery_source, DiscoverySource::StoreManifest);
}

#[test]
fn steam_folder_without_a_manifest_has_no_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("lib");
    steam_library(&lib, None, "LooseGame", "");

    let games = scan_steam_root_for_tests(&lib);

    assert_eq!(games.len(), 1);
    assert_eq!(games[0].steam_appid, None);
    assert_eq!(games[0].store_identity, None);
    assert_eq!(games[0].discovery_source, DiscoverySource::FolderScan);
}

#[test]
fn two_installs_of_one_title_are_separate_entries_with_their_own_paths() {
    // The scanner's side of the dedup contract: the same app id in two library
    // roots yields two entries, each keeping its own install path, so an
    // identity- and path-aware rule can keep both.
    let tmp = tempfile::tempdir().unwrap();
    let lib_a = tmp.path().join("libA");
    let lib_b = tmp.path().join("libB");
    steam_library(&lib_a, Some(1234), "SameTitle", "Same Title");
    steam_library(&lib_b, Some(1234), "SameTitle", "Same Title");

    let mut games = scan_steam_root_for_tests(&lib_a);
    games.extend(scan_steam_root_for_tests(&lib_b));

    assert_eq!(games.len(), 2);
    assert_ne!(games[0].path, games[1].path);
    assert_eq!(games[0].store_identity, games[1].store_identity);
    assert_eq!(games[0].store_identity, Some(StoreIdentity::steam(1234)));

    // And the rule that runs on this output keeps both, so an install cannot
    // be lost between the scanner and the UI.
    let kept = dedup_games(games);
    assert_eq!(kept.len(), 2);
}

#[test]
fn gog_id_comes_from_the_info_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("cp2077");
    fs::create_dir_all(&game).unwrap();
    assert_eq!(gog::read_game_id(&game), None);

    write_file_at(
        &game.join("goggame-1091500.info"),
        r#"{"gameTitle": "Cyberpunk 2077"}"#,
    );

    assert_eq!(gog::read_game_id(&game).as_deref(), Some("1091500"));
    // The title still resolves from the very same file.
    assert_eq!(
        gog::read_game_title(&game).as_deref(),
        Some("Cyberpunk 2077")
    );
}

#[test]
fn gog_info_file_without_an_id_yields_no_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("no_id");
    fs::create_dir_all(&game).unwrap();
    write_file_at(&game.join("goggame-.info"), r#"{"gameTitle": "Nameless"}"#);

    assert_eq!(gog::read_game_id(&game), None);
}

#[test]
fn epic_metadata_exposes_display_name_and_app_name() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("SomeGame");
    write_file_at(
        &game.join(".egstore").join("abc.mancfg"),
        r#"{"DisplayName": "Some Game: Deluxe", "AppName": "Sugar"}"#,
    );

    let meta = epic::read_metadata(&game).expect("metadata present");
    assert_eq!(meta.title, "Some Game: Deluxe");
    assert_eq!(meta.app_name.as_deref(), Some("Sugar"));
    assert_eq!(
        epic::read_game_name(&game).as_deref(),
        Some("Some Game: Deluxe")
    );
}

#[test]
fn epic_manifest_with_only_app_name_uses_it_for_both() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("OnlyApp");
    write_file_at(
        &game.join(".egstore").join("y.mancfg"),
        r#"{"AppName": "Sugar"}"#,
    );

    let meta = epic::read_metadata(&game).expect("metadata present");
    assert_eq!(meta.title, "Sugar");
    assert_eq!(meta.app_name.as_deref(), Some("Sugar"));
}

#[test]
fn epic_manifest_without_app_name_reports_no_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("NoId");
    write_file_at(
        &game.join(".egstore").join("z.mancfg"),
        r#"{"DisplayName": "No Id Game"}"#,
    );

    let meta = epic::read_metadata(&game).expect("metadata present");
    assert_eq!(meta.title, "No Id Game");
    assert_eq!(meta.app_name, None);
}

#[test]
fn epic_metadata_absent_or_unreadable_yields_none() {
    let tmp = tempfile::tempdir().unwrap();

    let bare = tmp.path().join("Bare");
    fs::create_dir_all(&bare).unwrap();
    assert!(epic::read_metadata(&bare).is_none());
    assert!(epic::read_game_name(&bare).is_none());

    // A corrupt manifest is skipped, never guessed at.
    let broken = tmp.path().join("Broken");
    write_file_at(&broken.join(".egstore").join("x.mancfg"), "{not json");
    assert!(epic::read_metadata(&broken).is_none());
}

#[test]
fn heroic_entries_carry_the_launcher_store_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("heroic");

    write_file_at(
        &root
            .join("legendaryConfig")
            .join("legendary")
            .join("installed.json"),
        r#"{"app1": {"title": "Epic Game", "install_path": "C:\\Games\\Epic"}}"#,
    );
    write_file_at(
        &root.join("gog_store").join("installed.json"),
        r#"{"installed": [{"appName": "42", "install_path": "C:\\Games\\Gog"}]}"#,
    );
    write_file_at(
        &root.join("store_cache").join("gog_library.json"),
        r#"{"games": [{"app_name": "42", "title": "GOG Game"}]}"#,
    );
    write_file_at(
        &root.join("nile_config").join("nile").join("installed.json"),
        r#"[{"id": "amzn1", "path": "C:\\Games\\Amazon"}]"#,
    );
    write_file_at(
        &root.join("nile_config").join("nile").join("library.json"),
        r#"[{"id": "amzn1", "product": {"title": "Amazon Game"}}]"#,
    );
    write_file_at(
        &root.join("sideload_apps").join("library.json"),
        r#"{"games": [{"title": "Side Game", "is_installed": true, "folder_name": "C:\\Games\\Side"}]}"#,
    );

    let entries = heroic::installed_entries(&root);
    assert_eq!(entries.len(), 4);

    let id_for = |title: &str| -> Option<String> {
        entries
            .iter()
            .find(|e| e.title.as_deref() == Some(title))
            .and_then(|e| e.store_id.clone())
    };

    assert_eq!(id_for("Epic Game").as_deref(), Some("app1"));
    assert_eq!(id_for("GOG Game").as_deref(), Some("42"));
    assert_eq!(id_for("Amazon Game").as_deref(), Some("amzn1"));
    // Sideloaded apps carry no store id of their own.
    assert_eq!(id_for("Side Game"), None);
}

#[test]
fn heroic_gog_entry_without_a_cached_title_keeps_its_store_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("heroic");
    write_file_at(
        &root.join("gog_store").join("installed.json"),
        r#"{"installed": [{"appName": "7", "install_path": "C:\\G\\NoTitle"}]}"#,
    );

    let entries = heroic::installed_entries(&root);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, None);
    assert_eq!(entries[0].store_id.as_deref(), Some("7"));
}

#[test]
fn game_with_identity_round_trips_through_json() {
    let mut game = Game::new(
        "Steam Game",
        PathBuf::from(r"C:\Games\Steam Game"),
        Platform::Steam,
    );
    game.store_identity = Some(StoreIdentity::steam(1091500));
    game.discovery_source = DiscoverySource::StoreManifest;

    let json = serde_json::to_string(&game).unwrap();
    let back: Game = serde_json::from_str(&json).unwrap();

    assert_eq!(back.name, game.name);
    assert_eq!(back.path, game.path);
    assert_eq!(back.store_identity, Some(StoreIdentity::steam(1091500)));
    assert_eq!(back.discovery_source, DiscoverySource::StoreManifest);
}

#[test]
fn game_without_identity_still_round_trips() {
    let game = Game::new("Plain", PathBuf::from(r"C:\Games\Plain"), Platform::Manual);

    let json = serde_json::to_string(&game).unwrap();
    let back: Game = serde_json::from_str(&json).unwrap();

    assert_eq!(back.store_identity, None);
    assert_eq!(back.discovery_source, DiscoverySource::FolderScan);
}

#[test]
fn each_discovery_source_round_trips_distinctly() {
    let sources = [
        DiscoverySource::StoreManifest,
        DiscoverySource::LauncherLibrary,
        DiscoverySource::FolderScan,
        DiscoverySource::UserSelected,
    ];
    let mut encoded: Vec<String> = Vec::new();

    for source in sources {
        let json = serde_json::to_string(&source).unwrap();
        let back: DiscoverySource = serde_json::from_str(&json).unwrap();
        assert_eq!(back, source, "round trip changed {json}");
        encoded.push(json);
    }

    // The four sources must stay distinguishable on the wire, otherwise a
    // stored entry could not be told apart from another after a reload.
    encoded.sort();
    encoded.dedup();
    assert_eq!(encoded.len(), sources.len());
}

#[test]
fn foreign_steam_manifest_without_an_install_dir_bestows_no_identity() {
    // Steam binds a manifest to a folder through `installdir`, so a manifest
    // that does not name this folder can never be attached to it: it is skipped
    // rather than guessed at, and the folder stays a plain folder scan.
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("lib");
    steam_library(&lib, None, "BrokenGame", "");
    write_file_at(
        &lib.join("steamapps").join("appmanifest_1.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"1\"\n",
    );

    let games = scan_steam_root_for_tests(&lib);

    assert_eq!(games.len(), 1);
    assert_eq!(games[0].store_identity, None);
    assert_eq!(games[0].discovery_source, DiscoverySource::FolderScan);
}

#[test]
fn steam_manifest_without_a_numeric_appid_keeps_no_identity() {
    // The manifest is real, so this is a manifest hit, but it carries no usable
    // id - and an id must never be invented to fill that gap.
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("lib");
    steam_library(&lib, None, "NoIdGame", "");
    write_file_at(
        &lib.join("steamapps").join("appmanifest_2.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"not-a-number\"\n\t\"name\"\t\t\"No Id Game\"\n\t\"installdir\"\t\t\"NoIdGame\"\n}\n",
    );

    let games = scan_steam_root_for_tests(&lib);

    assert_eq!(games.len(), 1);
    assert_eq!(games[0].name, "No Id Game");
    assert_eq!(games[0].steam_appid, None);
    assert_eq!(games[0].store_identity, None);
    assert_eq!(games[0].discovery_source, DiscoverySource::StoreManifest);
}

#[test]
fn epic_candidate_that_is_not_readable_as_a_file_yields_nothing() {
    // A `.mancfg` that cannot be read as a file (here a directory carrying the
    // extension) must produce no metadata, exactly like a corrupt one.
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("Unreadable");
    fs::create_dir_all(game.join(".egstore").join("x.mancfg")).unwrap();

    assert!(epic::read_metadata(&game).is_none());
}

#[test]
fn corrupt_heroic_store_file_yields_no_entries() {
    // A store file that cannot be parsed must be dropped, never turned into an
    // entry without a title or an id.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("heroic");
    write_file_at(
        &root
            .join("legendaryConfig")
            .join("legendary")
            .join("installed.json"),
        "{not json",
    );

    assert!(heroic::installed_entries(&root).is_empty());
}

#[test]
fn heroic_identity_names_the_store_not_the_launcher() {
    // Heroic is the launcher; the id belongs to the store behind it. If every
    // Heroic entry said (Heroic, id), a GOG id and an Amazon id with the same
    // digits would be one identity and a cover lookup would not know where to
    // look.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("heroic");
    write_file_at(
        &root
            .join("legendaryConfig")
            .join("legendary")
            .join("installed.json"),
        r#"{"app1": {"title": "Epic Game", "install_path": "C:\\Games\\Epic"}}"#,
    );
    write_file_at(
        &root.join("gog_store").join("installed.json"),
        r#"{"installed": [{"appName": "42", "install_path": "C:\\Games\\Gog"}]}"#,
    );
    write_file_at(
        &root.join("nile_config").join("nile").join("installed.json"),
        r#"[{"id": "amzn1", "path": "C:\\Games\\Amazon"}]"#,
    );

    let mut reported: Vec<(String, Platform)> = heroic::installed_entries(&root)
        .iter()
        .filter_map(heroic_identity)
        .map(|identity| (identity.store_id, identity.platform))
        .collect();
    reported.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(
        reported,
        vec![
            ("42".to_string(), Platform::Gog),
            ("amzn1".to_string(), Platform::Amazon),
            ("app1".to_string(), Platform::Epic),
        ]
    );
}
