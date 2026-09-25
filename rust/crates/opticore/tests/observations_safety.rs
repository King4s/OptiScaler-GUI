use opticore::{
    install::manifest::{self, InstallManifest},
    model::{Game, Platform},
    observations::observe,
};
use std::fs;

#[test]
fn native_dlls_are_indications_only() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Game.exe"), b"fixture").unwrap();
    fs::write(tmp.path().join("nvngx_dlss.dll"), b"native").unwrap();
    fs::write(tmp.path().join("dxgi.dll"), b"foreign").unwrap();
    let game = Game::new("Fixture", tmp.path().to_path_buf(), Platform::Gog);
    let observation = observe(&game);
    assert!(!observation.installed);
    assert_eq!(observation.loaded_from_log, None);
    assert!(observation.dll_hints.contains(&"nvngx_dlss.dll".into()));
}

#[test]
fn installation_is_not_loading_or_rendering_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Game.exe"), b"fixture").unwrap();
    fs::write(tmp.path().join("dxgi.dll"), b"proxy").unwrap();
    let m = InstallManifest::new(
        "dxgi.dll",
        &["dxgi.dll".into()],
        &[],
        "v0.9.4",
        None,
        "fixture".into(),
    );
    manifest::write(tmp.path(), &m).unwrap();
    let game = Game::new("Fixture", tmp.path().to_path_buf(), Platform::Gog);
    let observation = observe(&game);
    assert!(observation.installed);
    assert_eq!(observation.optiscaler_version.as_deref(), Some("v0.9.4"));
    assert_eq!(observation.loaded_from_log, None);
    fs::write(
        tmp.path().join("OptiScaler.log"),
        "[12:00:00.000000] [E] Could not complete Init done\n",
    )
    .unwrap();
    assert_eq!(observe(&game).loaded_from_log, None);
}

#[test]
fn steam_build_is_read_from_matching_store_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("steamapps/common/Fixture");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Game.exe"), b"fixture").unwrap();
    let path = tmp.path().join("steamapps/appmanifest_123.acf");
    fs::write(&path, "\"AppState\"\n{\n\"buildid\" \"24475133\"\n}\n").unwrap();
    let mut game = Game::new("Fixture", root, Platform::Steam);
    game.steam_appid = Some(123);
    let observation = observe(&game);
    assert_eq!(observation.store_id.as_deref(), Some("123"));
    assert_eq!(observation.build_version.as_deref(), Some("24475133"));
    fs::write(path, "\"buildid\" \"C:\\Users\\private\"\n").unwrap();
    assert_eq!(observe(&game).build_version, None);
}

#[test]
fn missing_target_remains_unknown() {
    let tmp = tempfile::tempdir().unwrap();
    let game = Game::new("Fixture", tmp.path().to_path_buf(), Platform::Gog);
    let observation = observe(&game);
    assert_eq!(observation.executable, None);
    assert_eq!(observation.target_directory, None);
    assert_eq!(observation.build_version, None);
    assert!(!observation.installed);
}

#[test]
fn verified_v094_info_marker_is_loading_evidence_only() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Game.exe"), b"fixture").unwrap();
    fs::write(tmp.path().join("dxgi.dll"), b"proxy").unwrap();
    let mut m = InstallManifest::new(
        "dxgi.dll",
        &["dxgi.dll".into()],
        &[],
        "v0.9.4",
        None,
        "fixture".into(),
    );
    manifest::write(tmp.path(), &m).unwrap();
    let game = Game::new("Fixture", tmp.path().to_path_buf(), Platform::Gog);
    // v0.9.4 Logger.cpp and dllmain.cpp: direct spdlog::info, no function prefix.
    for line in [
        "[12:34:56.123456] [I] Init done\n",
        "12:34:56.123456\tI\tInit done\n",
    ] {
        fs::write(tmp.path().join("OptiScaler.log"), line).unwrap();
        let observation = observe(&game);
        assert_eq!(observation.loaded_from_log, Some(true));
        assert!(observation.log_modified.is_some());
    }
    for line in [
        "[12:34:56.123456] [E] Init done\n",
        "[12:34:56.123456] [I] Init done failed\n",
        "Init done",
        "[invalid] [I] Init done\n",
    ] {
        fs::write(tmp.path().join("OptiScaler.log"), line).unwrap();
        assert_eq!(observe(&game).loaded_from_log, None);
    }
    m.optiscaler_version = "v0.8.0".into();
    manifest::write(tmp.path(), &m).unwrap();
    fs::write(
        tmp.path().join("OptiScaler.log"),
        "[12:34:56.123456] [I] Init done\n",
    )
    .unwrap();
    assert_eq!(observe(&game).loaded_from_log, None);
}
