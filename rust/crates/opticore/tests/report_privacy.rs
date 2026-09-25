use opticore::hardware::{GpuProfile, HardwareProfile};
use opticore::observations::GameObservation;
use opticore::profiles::LocalProfiles;
use opticore::report::{build_report, UserTestResult};
use serde_json::Value;
use std::fs;

fn report_json(
    hardware: &HardwareProfile,
    gpu: Option<&GpuProfile>,
    game: &GameObservation,
    result: UserTestResult,
) -> Value {
    serde_json::to_value(build_report(hardware, gpu, game, result))
        .expect("report should serialize as JSON")
}

fn assert_contains(value: &Value, expected: &str) {
    assert!(
        value.to_string().contains(expected),
        "missing {expected:?} in {value}"
    );
}

fn assert_not_contains(value: &Value, forbidden: &str) {
    assert!(
        !value.to_string().contains(forbidden),
        "unexpected sensitive value {forbidden:?} in {value}"
    );
}

#[test]
fn report_contains_allowlisted_facts_and_independent_statuses() {
    let gpu = GpuProfile {
        id: "enumeration-only-id".into(),
        name: Some("RTX 5090".into()),
        vendor: Some("NVIDIA".into()),
        dedicated_bytes: Some(16 * 1024 * 1024 * 1024),
        shared_bytes: Some(8 * 1024 * 1024 * 1024),
        driver: Some("576.02".into()),
    };
    let hardware = HardwareProfile {
        gpus: vec![gpu.clone()],
        windows_build: Some("26100.4484".into()),
        ram_bytes: Some(32 * 1024 * 1024 * 1024),
        collected_at: Some("private-hardware-timestamp".into()),
        warnings: vec!["private-hardware-warning".into()],
    };
    let game = GameObservation {
        name: "Example Game".into(),
        platform: "Steam".into(),
        store_id: Some("123456".into()),
        build_version: Some("987654321".into()),
        executable: Some(r"C:\Users\alice\Games\Example\game.exe".into()),
        target_directory: Some(r"C:\Users\alice\Games\Example".into()),
        optiscaler_version: Some("v0.9.4".into()),
        installed: true,
        dll_hints: vec!["private-proxy.dll".into()],
        loaded_from_log: Some(false),
        log_modified: Some("private-log-timestamp".into()),
    };

    let report = report_json(&hardware, Some(&gpu), &game, UserTestResult::Passed);
    for expected in [
        "RTX 5090",
        "NVIDIA",
        "17179869184",
        "8589934592",
        "576.02",
        "26100.4484",
        "34359738368",
        "Steam",
        "123456",
        "987654321",
        "v0.9.4",
    ] {
        assert_contains(&report, expected);
    }
    for key in ["schema_version", "hardware", "game", "test_result"] {
        assert!(report.get(key).is_some(), "missing top-level {key}");
    }
    assert_eq!(report["game"]["installed"], true);
    assert_eq!(report["game"]["loaded"], false);
    assert_eq!(report["game"]["user_tested"], true);

    for forbidden in [
        "enumeration-only-id",
        "C:\\Users\\alice",
        "game.exe",
        "private-proxy.dll",
        "private-hardware-timestamp",
        "private-hardware-warning",
        "private-log-timestamp",
        "Example Game",
    ] {
        assert_not_contains(&report, forbidden);
    }
}

#[test]
fn filesystem_derived_game_name_is_never_exported_even_if_it_looks_safe() {
    let game = GameObservation {
        name: "Alice Private Game".into(),
        platform: "Manual".into(),
        ..GameObservation::default()
    };
    let report = report_json(
        &HardwareProfile::default(),
        None,
        &game,
        UserTestResult::NotRun,
    );
    assert!(report["game"].get("name").is_none());
    assert_not_contains(&report, "Alice");
}

#[test]
fn adversarial_strings_and_arbitrary_fields_are_not_serialized() {
    let marker = "LEAK_SENTINEL_7f93";
    let gpu = GpuProfile {
        name: Some(format!("GPU {marker} C:\\Users\\alice\\secret.exe")),
        vendor: Some("AMD".into()),
        driver: Some(format!("driver-{marker}")),
        ..GpuProfile::default()
    };
    let hardware = HardwareProfile {
        gpus: vec![gpu.clone()],
        windows_build: Some("22631.1".into()),
        ram_bytes: None,
        ..HardwareProfile::default()
    };
    let game = GameObservation {
        name: format!("Game {marker} /home/alice/private"),
        platform: "Epic".into(),
        store_id: Some(format!("store-{marker}")),
        build_version: Some(format!("build-{marker}")),
        optiscaler_version: Some(format!("version-{marker}")),
        installed: true,
        loaded_from_log: None,
        ..GameObservation::default()
    };

    let report = report_json(&hardware, Some(&gpu), &game, UserTestResult::NotRun);
    assert_not_contains(&report, marker);
    assert_eq!(report["game"]["installed"], true);
    assert!(report["game"]["loaded"].is_null() || report["game"].get("loaded").is_none());
    assert_eq!(report["game"]["user_tested"], false);
    assert!(
        report["hardware"]["ram_bytes"].is_null() || report["hardware"].get("ram_bytes").is_none()
    );
    for key in [
        "executable",
        "target_directory",
        "warnings",
        "log",
        "dll_hints",
    ] {
        assert!(report.get(key).is_none(), "unexpected report field {key}");
    }
    let json = report.to_string();
    for forbidden_key in ["serial", "machine_id", "raw_log", "username", "path"] {
        assert!(!json.to_lowercase().contains(forbidden_key));
    }
}

#[test]
fn user_test_statuses_and_missing_gpu_are_preserved_without_fabricating_facts() {
    let hardware = HardwareProfile::default();
    let game = GameObservation {
        name: "Unknowns".into(),
        platform: "Unknown".into(),
        installed: false,
        loaded_from_log: None,
        ..GameObservation::default()
    };

    let passed = report_json(&hardware, None, &game, UserTestResult::Passed);
    assert_eq!(passed["game"]["user_tested"], true);
    let failed = report_json(&hardware, None, &game, UserTestResult::Failed);
    assert_eq!(failed["game"]["user_tested"], true);
    let not_run = report_json(&hardware, None, &game, UserTestResult::NotRun);
    assert_eq!(not_run["game"]["user_tested"], false);

    for report in [&passed, &failed, &not_run] {
        assert!(report["hardware"]["gpu"].is_null() || report["hardware"].get("gpu").is_none());
        assert!(
            report["hardware"]["windows_build"].is_null()
                || report["hardware"].get("windows_build").is_none()
        );
        assert!(
            report["hardware"]["ram_bytes"].is_null()
                || report["hardware"].get("ram_bytes").is_none()
        );
        assert!(report["game"]["loaded"].is_null() || report["game"].get("loaded").is_none());
        assert_eq!(report["game"]["installed"], false);
        assert!(!report["hardware"].to_string().contains('0'));
    }
    assert_ne!(passed["test_result"], failed["test_result"]);
    assert_ne!(passed["test_result"], not_run["test_result"]);
    assert_ne!(failed["test_result"], not_run["test_result"]);
}

#[test]
fn local_test_result_roundtrips_but_game_identity_is_not_exported() {
    let mut local = LocalProfiles::default();
    let private_game_key = r"C:\Users\alice\Games\PrivateTitle";
    local
        .game_results
        .insert(private_game_key.into(), UserTestResult::Passed);
    local
        .extra
        .insert("future_setting".into(), serde_json::json!(true));
    let restored: LocalProfiles = serde_json::from_value(serde_json::to_value(local).unwrap())
        .expect("local results must be backward-compatible");
    assert_eq!(
        restored.game_results.get(private_game_key),
        Some(&UserTestResult::Passed)
    );
    assert_eq!(
        restored.extra.get("future_setting"),
        Some(&serde_json::json!(true))
    );

    let observation = GameObservation {
        name: "Example Game".into(),
        platform: "Steam".into(),
        ..GameObservation::default()
    };
    let report = report_json(
        &HardwareProfile::default(),
        None,
        &observation,
        *restored.game_results.get(private_game_key).unwrap(),
    );
    assert_eq!(report["test_result"], "passed");
    assert_not_contains(&report, private_game_key);
    assert_not_contains(&report, "future_setting");
}

#[test]
fn explicit_report_save_creates_new_file_without_replacing_existing_data() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("reviewed-report.json");
    let observation = GameObservation {
        name: "Example Game".into(),
        platform: "Steam".into(),
        executable: Some(r"C:\Users\alice\game.exe".into()),
        ..GameObservation::default()
    };
    let report = build_report(
        &HardwareProfile::default(),
        None,
        &observation,
        UserTestResult::NotRun,
    );
    let preview = opticore::report::preview_json(&report).unwrap();
    opticore::report::write_report(&destination, &report).unwrap();
    let original = fs::read(&destination).unwrap();
    assert_eq!(original, preview.as_bytes());
    let saved: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(saved["test_result"], "not_run");
    assert!(!String::from_utf8_lossy(&original).contains("C:\\Users"));
    assert!(opticore::report::write_report(&destination, &report).is_err());
    assert_eq!(fs::read(&destination).unwrap(), original);
}
