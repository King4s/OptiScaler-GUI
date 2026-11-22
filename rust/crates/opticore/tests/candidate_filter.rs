use opticore::config::AppConfig;
use opticore::model::{AntiCheat, Game, Platform};
use opticore::scan::{custom, dedup_games, folder_facts, scan_manual_folder};
use std::path::PathBuf;

#[test]
fn candidates_need_evidence_and_no_detected_exclusion() {
    let mut game = Game::new("Unknown", PathBuf::from("C:/unknown"), Platform::Steam);
    assert!(!game.is_optiscaler_candidate());
    game.has_upscaler_hint = true;
    assert!(game.is_optiscaler_candidate());
    game.anti_cheat.push(AntiCheat::EasyAntiCheat);
    assert!(!game.is_optiscaler_candidate());
    game.anti_cheat.clear();
    game.engine_supported = false;
    assert!(!game.is_optiscaler_candidate());
    game.engine_supported = true;
    game.has_upscaler_hint = false;
    game.optiscaler_installed = true;
    assert!(game.is_optiscaler_candidate());
    game.optiscaler_installed = false;
    game.community_verified = true;
    assert!(game.is_optiscaler_candidate());
}

#[test]
fn nested_hint_reaches_manual_and_custom_scans_but_proxy_does_not() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("Example");
    let bin = game.join("Binaries/Win64");
    std::fs::create_dir_all(&bin).unwrap();
    for file in ["game.exe", "dxgi.dll", "a.bin", "b.bin", "c.bin", "d.bin"] {
        std::fs::write(bin.join(file), b"fixture").unwrap();
    }
    assert!(!folder_facts::collect(&game).unwrap().has_upscaler_hint);
    std::fs::write(bin.join("NVNGX_DLSS.DLL"), b"fixture").unwrap();
    assert!(scan_manual_folder(&game).unwrap().has_upscaler_hint);
    let scanned = custom::scan_roots(&[custom::ScanRoot {
        path: game,
        enabled: true,
    }]);
    assert!(scanned
        .entries
        .iter()
        .any(|entry| entry.facts.has_upscaler_hint));
}

#[test]
fn dedup_cannot_erase_exclusions_or_hints() {
    let mut a = Game::new("Game", PathBuf::from("C:/game"), Platform::Steam);
    a.has_upscaler_hint = true;
    let mut b = a.clone();
    b.has_upscaler_hint = false;
    b.anti_cheat.push(AntiCheat::BattlEye);
    for games in [vec![a.clone(), b.clone()], vec![b, a]] {
        let result = dedup_games(games);
        assert!(result[0].has_upscaler_hint);
        assert!(!result[0].is_optiscaler_candidate());
    }
}

#[test]
fn filter_defaults_on_but_remembers_all_games_and_keeps_foreign_config() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    std::fs::write(&path, br#"{"python_setting":42}"#).unwrap();
    let mut config = AppConfig::load(&path);
    assert!(config.candidates_only);
    config.candidates_only = false;
    config.save(&path).unwrap();
    assert!(!AppConfig::load(&path).candidates_only);
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(json["python_setting"], 42);
}
