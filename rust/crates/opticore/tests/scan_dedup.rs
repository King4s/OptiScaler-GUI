//! Fixture tests for the path-aware dedup rule that `scan_all`
//! applies to every scanner's output via `dedup_games`.
//!
//! The rule replaced a `(title, platform)` key that could silently drop a
//! second install of a title. These tests pin the new behaviour, the legacy
//! path normalisation it keeps, and the ordering guarantee that keeps a scan
//! deterministic. Games are constructed directly: `dedup_games` is a pure
//! function over `Vec<Game>`, which is exactly what `scan_all` hands it.
//!
//! They deliberately never touch this machine's real game libraries, and they
//! do not re-derive the rule — they call the shipping function.

use opticore::model::{DiscoverySource, Game, Platform, StoreIdentity, TitleSource};
use opticore::scan::dedup_games;
use std::path::PathBuf;

fn game_at(path: &str, name: &str, platform: Platform, store_id: Option<&str>) -> Game {
    let mut game = Game::new(name, PathBuf::from(path), platform);
    game.store_identity = store_id.map(|id| StoreIdentity::new(platform, id));
    game
}

fn paths(games: &[Game]) -> Vec<String> {
    games
        .iter()
        .map(|g| g.path.to_string_lossy().to_string())
        .collect()
}

#[test]
fn two_installs_of_one_title_in_different_paths_both_survive() {
    // The regression this rule exists for: one title, one store, one app id,
    // two install locations. The old (title, platform) key kept only one.
    let first = game_at(
        r"C:\Games\First\Same",
        "Same Title",
        Platform::Steam,
        Some("1234"),
    );
    let second = game_at(
        r"D:\Games\Second\Same",
        "Same Title",
        Platform::Steam,
        Some("1234"),
    );

    let kept = dedup_games(vec![first, second]);

    assert_eq!(kept.len(), 2);
    assert_eq!(
        paths(&kept),
        vec![
            r"C:\Games\First\Same".to_string(),
            r"D:\Games\Second\Same".to_string()
        ]
    );
    assert!(kept.iter().all(|g| g.store_identity.is_some()));
}

#[test]
fn one_install_hit_twice_collapses_to_a_single_entry() {
    // Two scanner passes over the very same install are still one entry, and
    // the first hit is the one kept.
    let first = game_at(r"C:\Games\Once", "Once Game", Platform::Steam, Some("42"));
    let again = game_at(r"C:\Games\Once", "Once Game", Platform::Steam, Some("42"));

    let kept = dedup_games(vec![first, again]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].steam_appid, None); // built by hand, so no legacy field
    assert_eq!(kept[0].store_identity, Some(StoreIdentity::steam(42)));
}

#[test]
fn an_install_without_metadata_still_collapses_on_its_path() {
    // Legacy path-key behaviour survives: no store metadata at all, so the
    // normalized path is the only thing that identifies the entry.
    let first = game_at(r"C:\Games\NoMeta", "No Meta Game", Platform::Manual, None);
    let again = game_at(r"C:\Games\NoMeta", "No Meta Game", Platform::Manual, None);

    let kept = dedup_games(vec![first, again]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].store_identity, None);
}

#[test]
fn path_matching_stays_case_insensitive() {
    // The normalized path is lowercased, as it always was; that must not
    // regress into a case-sensitive comparison.
    let upper = game_at(r"C:\Games\Case", "Case Game", Platform::Steam, Some("7"));
    let lower = game_at(r"c:\games\case", "Case Game", Platform::Steam, Some("7"));

    let kept = dedup_games(vec![upper, lower]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].path, PathBuf::from(r"C:\Games\Case"));
}

#[test]
fn one_title_reached_through_two_stores_is_kept_twice() {
    let via_steam = game_at(
        r"C:\Games\SteamCopy",
        "Same Title",
        Platform::Steam,
        Some("1234"),
    );
    let via_gog = game_at(
        r"C:\Games\GogCopy",
        "Same Title",
        Platform::Gog,
        Some("1091500"),
    );

    let kept = dedup_games(vec![via_steam, via_gog]);

    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].store_identity, Some(StoreIdentity::steam(1234)));
    assert_eq!(
        kept[1].store_identity,
        Some(StoreIdentity::new(Platform::Gog, "1091500"))
    );
}

#[test]
fn unidentifiable_installs_at_distinct_paths_are_not_merged() {
    // No metadata for either copy: a title is not proof that two paths are the
    // same install, so nothing may be dropped on the strength of it.
    let first = game_at(r"C:\Games\AnonA", "Anon Game", Platform::Epic, None);
    let second = game_at(r"D:\Games\AnonB", "Anon Game", Platform::Epic, None);

    let kept = dedup_games(vec![first, second]);

    assert_eq!(kept.len(), 2);
    assert!(kept.iter().all(|g| g.store_identity.is_none()));
}

#[test]
fn dedup_keeps_the_input_order_and_does_not_rescan() {
    let a = game_at(r"C:\Games\A", "A", Platform::Steam, Some("1"));
    let b = game_at(r"C:\Games\B", "B", Platform::Gog, Some("2"));
    let c = game_at(r"C:\Games\C", "C", Platform::Epic, None);

    let kept = dedup_games(vec![a, b, c]);

    assert_eq!(
        paths(&kept),
        vec![r"C:\Games\A", r"C:\Games\B", r"C:\Games\C"]
    );
    assert_eq!(kept.len(), 3);
}

#[test]
fn dedup_of_an_empty_scan_is_empty() {
    assert!(dedup_games(Vec::new()).is_empty());
}

/// The same install as a store root reports it: the store's own title, and
/// neither an id nor any artwork.
fn store_root_hit(title: &str) -> Game {
    let mut game = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        title,
        Platform::Epic,
        None,
    );
    game.discovery_source = DiscoverySource::StoreManifest;
    game.title_source = TitleSource::Store;
    game
}

/// The same install as the launcher that manages it reports it: a folder-name
/// title, the store id, and artwork.
fn launcher_hit(title: &str) -> Game {
    let mut game = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        title,
        Platform::Heroic,
        None,
    );
    game.discovery_source = DiscoverySource::LauncherLibrary;
    game.title_source = TitleSource::Launcher;
    // The launcher is only the route; the id belongs to the store behind it.
    game.store_identity = Some(StoreIdentity::new(Platform::Epic, "app1"));
    game.art_url = Some("https://cdn.example/cover.jpg".to_string());
    game
}

#[test]
fn merging_keeps_the_store_title_and_gains_the_launcher_id_and_art() {
    let kept = dedup_games(vec![
        store_root_hit("Some Game: Deluxe"),
        launcher_hit("Somegame"),
    ]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "Some Game: Deluxe");
    assert_eq!(kept[0].platform, Platform::Epic);
    assert_eq!(kept[0].discovery_source, DiscoverySource::StoreManifest);
    assert_eq!(
        kept[0].art_url.as_deref(),
        Some("https://cdn.example/cover.jpg")
    );
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Epic, "app1"))
    );
}

#[test]
fn merging_is_order_independent_for_the_title() {
    let kept = dedup_games(vec![
        launcher_hit("Somegame"),
        store_root_hit("Some Game: Deluxe"),
    ]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "Some Game: Deluxe");
    assert_eq!(kept[0].discovery_source, DiscoverySource::StoreManifest);
    assert_eq!(
        kept[0].art_url.as_deref(),
        Some("https://cdn.example/cover.jpg")
    );
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Epic, "app1"))
    );
}

#[test]
fn a_worse_title_never_displaces_the_kept_one() {
    // Two folder scans for one install: neither title outranks the other, so
    // the first stands and the second may only fill gaps.
    let first = game_at(r"C:\Games\Tie", "Tie Game", Platform::Manual, None);
    let mut second = game_at(r"C:\Games\Tie", "TIE GAME", Platform::Manual, None);
    second.art_url = Some("https://cdn.example/tie.jpg".to_string());

    let kept = dedup_games(vec![first, second]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "Tie Game");
    assert_eq!(
        kept[0].art_url.as_deref(),
        Some("https://cdn.example/tie.jpg")
    );
}

#[test]
fn one_install_seen_by_two_scanners_collapses_to_one() {
    // A Heroic/legendary copy that also sits under a store's own root is ONE
    // install. The two scanners disagree on platform and on which identity they
    // attach, but not on the path, so it must not become two cards.
    let via_store_root = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        "Some Game",
        Platform::Epic,
        Some("app1"),
    );
    let via_heroic = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        "Some Game",
        Platform::Heroic,
        Some("app1"),
    );

    let kept = dedup_games(vec![via_store_root, via_heroic]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].platform, Platform::Epic); // the first hit is kept
}

#[test]
fn collapsing_keeps_the_hit_that_carries_an_identity() {
    // Scan order is not provenance order: a store root can report an install
    // with no id at all before a launcher reports the same folder with a real
    // one. Collapsing must keep the id, not the blank.
    let from_store_root = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        "Some Game",
        Platform::Epic,
        None,
    );
    let from_heroic = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        "Some Game",
        Platform::Heroic,
        Some("app1"),
    );

    let kept = dedup_games(vec![from_store_root, from_heroic]);

    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Heroic, "app1"))
    );
}

#[test]
fn a_blank_hit_never_displaces_an_identity_bearing_one() {
    let with_id = game_at(r"C:\Games\Pos", "Pos Game", Platform::Heroic, Some("app1"));
    let without_id = game_at(r"C:\Games\Pos", "Pos Game", Platform::Epic, None);

    let kept = dedup_games(vec![with_id, without_id]);

    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Heroic, "app1"))
    );
}

/// A GOG folder found through its own store file, carrying a real id but no
/// `gameTitle`, so its title is only prettified from the folder name.
fn gog_hit_without_a_store_title() -> Game {
    let mut game = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        "Somegame",
        Platform::Gog,
        Some("1091500"),
    );
    game.discovery_source = DiscoverySource::StoreManifest;
    game.title_source = TitleSource::Folder;
    game
}

/// The same install as a launcher reports it: a real title, no id of its own.
fn launcher_title_only(title: &str) -> Game {
    let mut game = game_at(
        r"C:\Program Files\Epic Games\Some Game",
        title,
        Platform::Heroic,
        None,
    );
    game.discovery_source = DiscoverySource::LauncherLibrary;
    game.title_source = TitleSource::Launcher;
    game
}

#[test]
fn a_store_manifest_folder_title_does_not_outrank_a_launcher_title() {
    // The bug this pins: the GOG entry really is a store-manifest discovery, but
    // its title came from the folder. Ranking titles by the discovery source let
    // that folder name win over the launcher's real title for the same install.
    let kept = dedup_games(vec![
        gog_hit_without_a_store_title(),
        launcher_title_only("Some Game: Deluxe"),
    ]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "Some Game: Deluxe");
    assert_eq!(kept[0].title_source, TitleSource::Launcher);
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Gog, "1091500"))
    );
}

#[test]
fn a_store_manifest_folder_title_does_not_outrank_a_launcher_title_either_order() {
    let kept = dedup_games(vec![
        launcher_title_only("Some Game: Deluxe"),
        gog_hit_without_a_store_title(),
    ]);

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "Some Game: Deluxe");
    assert_eq!(kept[0].title_source, TitleSource::Launcher);
    assert_eq!(
        kept[0].store_identity,
        Some(StoreIdentity::new(Platform::Gog, "1091500"))
    );
}
