use opticore::model::{Game, Platform, StoreIdentity, TitleSource};
use opticore::scan::custom::{scan_roots, validate_root, ScanRoot};
use opticore::scan::dedup_games;
use std::fs;
use std::path::{Path, PathBuf};

fn game(parent: &Path, name: &str) -> PathBuf {
    let path = parent.join(name);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("game.exe"), b"").unwrap();
    for n in 0..6 {
        fs::write(path.join(format!("asset{n}.pak")), b"").unwrap();
    }
    path
}

fn enabled(path: PathBuf) -> ScanRoot {
    ScanRoot {
        path,
        enabled: true,
    }
}

#[test]
fn malformed_and_missing_roots_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("file.txt");
    fs::write(&file, b"not a directory").unwrap();
    assert!(validate_root(Path::new("")).is_err());
    assert!(validate_root(Path::new("relative/library")).is_err());
    assert!(validate_root(&tmp.path().join("child").join("..")).is_err());
    assert!(validate_root(&file).is_err());
    assert!(validate_root(&tmp.path().join("missing")).is_err());
    let result = scan_roots(&[enabled(file), enabled(tmp.path().join("missing"))]);
    assert!(result.entries.is_empty());
    assert_eq!(result.warnings.len(), 2);
}

#[test]
fn system_and_profile_roots_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let volume_root = tmp
        .path()
        .ancestors()
        .find(|path| path.is_absolute() && path.parent().is_none())
        .expect("absolute volume root");
    assert!(validate_root(volume_root).is_err());
    let profile = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .expect("profile directory");
    assert!(validate_root(&profile).is_err());
    #[cfg(windows)]
    assert!(validate_root(&PathBuf::from(std::env::var_os("WINDIR").unwrap())).is_err());
    #[cfg(not(windows))]
    assert!(validate_root(Path::new("/etc")).is_err());
}

#[test]
fn discovery_stops_after_four_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let mut parent = tmp.path().to_path_buf();
    for level in 0..4 {
        parent = parent.join(format!("level{level}"));
    }
    let too_deep = game(&parent, "too_deep").canonicalize().unwrap();
    let at_limit = game(parent.parent().unwrap(), "at_limit")
        .canonicalize()
        .unwrap();
    let result = scan_roots(&[enabled(tmp.path().to_path_buf())]);
    assert!(!result.entries.iter().any(|entry| entry.path == too_deep));
    assert!(result.entries.iter().any(|entry| entry.path == at_limit));
}

#[test]
fn directory_budget_stops_large_trees_with_warning() {
    let tmp = tempfile::tempdir().unwrap();
    for n in 0..300 {
        fs::create_dir(tmp.path().join(format!("empty{n}"))).unwrap();
    }
    let result = scan_roots(&[enabled(tmp.path().to_path_buf())]);
    assert!(result.entries.is_empty());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("budget")));
}

#[test]
fn ordinary_library_does_not_spend_budget_rewalking_all_games() {
    let tmp = tempfile::tempdir().unwrap();
    for n in 0..60 {
        let root = game(tmp.path(), &format!("Game{n}"));
        fs::create_dir(root.join("Data")).unwrap();
        fs::create_dir(root.join("Config")).unwrap();
    }
    let result = scan_roots(&[enabled(tmp.path().to_path_buf())]);
    assert_eq!(result.entries.len(), 60);
    assert!(result.warnings.is_empty());
}

#[test]
fn selected_game_and_nested_library_are_found_without_guessing_executable() {
    let tmp = tempfile::tempdir().unwrap();
    let direct = game(tmp.path(), "direct_game");
    let nested = game(&tmp.path().join("Library").join("common"), "nested-game");
    let direct_result = scan_roots(&[enabled(direct.clone())]);
    assert_eq!(direct_result.entries.len(), 1);
    assert_eq!(
        direct_result.entries[0].path,
        direct.canonicalize().unwrap()
    );
    assert_eq!(direct_result.entries[0].name, "Direct Game");
    assert!(direct_result.entries[0].facts.found_exe);
    let library_result = scan_roots(&[enabled(tmp.path().to_path_buf())]);
    assert!(library_result
        .entries
        .iter()
        .any(|entry| entry.path == nested.canonicalize().unwrap()));
    assert!(library_result
        .entries
        .iter()
        .any(|entry| entry.path == direct.canonicalize().unwrap()));
}

#[test]
fn custom_root_and_store_hits_for_same_game_deduplicate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("library");
    let install = game(&root, "Example Game");
    let canonical_install = install.canonicalize().unwrap();
    let custom = scan_roots(&[enabled(root)]);
    let custom_entry = custom
        .entries
        .into_iter()
        .find(|entry| entry.path == canonical_install)
        .expect("custom root finds the game");
    let custom_game = Game::new(custom_entry.name, custom_entry.path, Platform::Manual);
    #[cfg(windows)]
    let store_path = {
        let path = canonical_install.to_string_lossy();
        assert!(path.starts_with(r"\\?\"), "canonical path: {path}");
        let plain = path.strip_prefix(r"\\?\").unwrap_or(&path);
        PathBuf::from(plain)
    };
    #[cfg(not(windows))]
    let store_path = canonical_install;
    let mut store_game = Game::new("Example Game", store_path, Platform::Steam);
    store_game.store_identity = Some(StoreIdentity::steam(123));
    store_game.title_source = TitleSource::Store;

    let deduplicated = dedup_games(vec![custom_game, store_game]);

    assert_eq!(deduplicated.len(), 1);
    assert_eq!(
        deduplicated[0].store_identity,
        Some(StoreIdentity::steam(123))
    );
}

#[test]
fn game_with_nested_binary_is_reported_at_game_root() {
    let tmp = tempfile::tempdir().unwrap();
    let game_root = tmp.path().join("MyGame");
    let binaries = game_root.join("Binaries").join("Win64");
    game(&game_root.join("Binaries"), "Win64");
    assert!(binaries.is_dir());
    let result = scan_roots(&[enabled(tmp.path().to_path_buf())]);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].path, game_root.canonicalize().unwrap());
    assert!(result.entries[0].facts.found_exe);
}

#[test]
fn disabled_root_is_not_accessed_and_launcher_is_excluded() {
    let tmp = tempfile::tempdir().unwrap();
    game(tmp.path(), "Epic Games Launcher");
    let disabled = ScanRoot {
        path: tmp.path().join("missing"),
        enabled: false,
    };
    let result = scan_roots(&[disabled, enabled(tmp.path().to_path_buf())]);
    assert!(result.entries.is_empty());
    assert!(result.warnings.is_empty());
}

#[cfg(unix)]
#[test]
fn out_of_root_symlink_is_not_followed() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("selected");
    fs::create_dir(&selected).unwrap();
    let outside = game(tmp.path(), "outside_game");
    symlink(&outside, selected.join("linked_game")).unwrap();
    assert!(validate_root(&selected.join("linked_game")).is_err());
    let result = scan_roots(&[enabled(selected)]);
    assert!(result.entries.is_empty());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("linked")));
}

#[cfg(windows)]
#[test]
fn out_of_root_link_is_not_followed_when_symlinks_are_available() {
    use std::os::windows::fs::symlink_dir;
    let tmp = tempfile::tempdir().unwrap();
    let selected = tmp.path().join("selected");
    fs::create_dir(&selected).unwrap();
    let outside = game(tmp.path(), "outside_game");
    if symlink_dir(&outside, selected.join("linked_game")).is_err() {
        return; // Symlink creation can require Developer Mode or privilege.
    }
    assert!(validate_root(&selected.join("linked_game")).is_err());
    let result = scan_roots(&[enabled(selected)]);
    assert!(result.entries.is_empty());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("linked")));
}

#[cfg(windows)]
#[test]
fn junction_cannot_escape_or_supply_game_facts() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = game(tmp.path(), "selected_game");
    let outside = game(tmp.path(), "outside_game");
    let linked = selected.join("escape");
    let output = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&linked)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(output.status.success(), "junction fixture: {:?}", output);
    assert!(validate_root(&linked).is_err());
    assert!(validate_root(&linked.join("child")).is_err());
    let result = scan_roots(&[enabled(selected)]);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].facts.file_count, 7);
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("linked")));
}
