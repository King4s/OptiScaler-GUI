use opticore::advice::{self, AdviceStatus, Recommendation};
use opticore::hardware::{GpuProfile, HardwareProfile};
use opticore::ini;
use opticore::observations::GameObservation;
use opticore::profiles::LocalProfiles;
use serde_json::{json, Value};
use std::fs;

const RULES: &str = include_str!("../data/advice-rules.json");
const EN: &str = include_str!("../../../../src/translations/en.json");
const DA: &str = include_str!("../../../../src/translations/da.json");
const PL: &str = include_str!("../../../../src/translations/pl.json");

fn game(name: &str, steam_id: Option<&str>) -> GameObservation {
    GameObservation {
        name: name.into(),
        platform: "Steam".into(),
        store_id: steam_id.map(str::to_owned),
        ..GameObservation::default()
    }
}

fn gpu(vendor: &str, name: &str) -> GpuProfile {
    GpuProfile {
        id: name.into(),
        vendor: Some(vendor.into()),
        name: Some(name.into()),
        ..GpuProfile::default()
    }
}

fn recommendation<'a>(items: &'a [Recommendation], id: &str) -> &'a Recommendation {
    items.iter().find(|item| item.id == id).unwrap()
}

#[test]
fn explicit_gpu_facts_do_not_become_blanket_compatibility() {
    let observation = game("An Unlisted Game", None);
    let cases = [
        (
            Some(gpu("AMD", "Radeon RX 7800 XT")),
            AdviceStatus::Unsupported,
        ),
        (
            Some(gpu("NVIDIA", "GeForce RTX 4070")),
            AdviceStatus::Conditional,
        ),
        (
            Some(gpu("NVIDIA", "GeForce GTX 1080")),
            AdviceStatus::Unsupported,
        ),
        (
            Some(gpu("Intel", "Intel UHD Graphics")),
            AdviceStatus::Unsupported,
        ),
        (
            Some(gpu("NVIDIA", "Unidentified adapter")),
            AdviceStatus::Unknown,
        ),
        (None, AdviceStatus::Unknown),
    ];
    for (selected, expected) in cases {
        let first = advice::evaluate(selected.as_ref(), &observation, "en");
        assert_eq!(
            first,
            advice::evaluate(selected.as_ref(), &observation, "en")
        );
        assert_eq!(recommendation(&first, "dlss-sr-output").status, expected);
        assert_eq!(
            recommendation(&first, "game-unknown").status,
            AdviceStatus::Unknown
        );
        assert!(first
            .iter()
            .all(|item| item.status != AdviceStatus::Documented));
    }
}

#[test]
fn multiple_gpus_require_the_caller_to_select_one() {
    let profile = HardwareProfile {
        gpus: vec![
            gpu("Intel", "Intel UHD Graphics"),
            gpu("NVIDIA", "GeForce RTX 4070"),
        ],
        ..HardwareProfile::default()
    };
    let observation = game("An Unlisted Game", None);
    assert_eq!(
        recommendation(
            &advice::evaluate(None, &observation, "en"),
            "dlss-sr-output"
        )
        .status,
        AdviceStatus::Unknown
    );
    assert_eq!(
        recommendation(
            &advice::evaluate(Some(&profile.gpus[0]), &observation, "en"),
            "dlss-sr-output"
        )
        .status,
        AdviceStatus::Unsupported
    );
    assert_eq!(
        recommendation(
            &advice::evaluate(Some(&profile.gpus[1]), &observation, "en"),
            "dlss-sr-output"
        )
        .status,
        AdviceStatus::Conditional
    );
}

#[test]
fn seven_game_rules_keep_local_and_wiki_evidence_distinct() {
    let cases = [
        (
            "Fatekeeper",
            None,
            "fatekeeper-local",
            AdviceStatus::Unknown,
        ),
        (
            "Satisfactory",
            Some("526870"),
            "satisfactory",
            AdviceStatus::Conditional,
        ),
        (
            "Cyberpunk 2077",
            Some("1091500"),
            "cyberpunk-2077",
            AdviceStatus::Conditional,
        ),
        (
            "HITMAN World of Assassination",
            Some("1659040"),
            "hitman-woa",
            AdviceStatus::Conditional,
        ),
        (
            "The Witcher 3: Wild Hunt",
            Some("292030"),
            "witcher-3",
            AdviceStatus::Conditional,
        ),
        (
            "Forspoken",
            Some("1680880"),
            "forspoken",
            AdviceStatus::Conditional,
        ),
        (
            "A Quiet Place: The Road Ahead",
            Some("2233120"),
            "a-quiet-place-road-ahead",
            AdviceStatus::Conditional,
        ),
    ];
    for (name, id, rule, status) in cases {
        let items = advice::evaluate(None, &game(name, id), "en");
        let matched = recommendation(&items, rule);
        assert_eq!(matched.status, status, "{name}");
        assert!(
            !items.iter().any(|item| item.id == "game-unknown"),
            "{name}"
        );
        assert_ne!(matched.status, AdviceStatus::Documented, "{name}");
        if name == "Fatekeeper" {
            assert_eq!(matched.source_url, "local:investigation");
            assert!(!matched.source_url.contains("wiki"));
        } else {
            assert!(matched
                .source_url
                .contains("github.com/optiscaler/OptiScaler/wiki/"));
        }
    }
}

#[test]
fn missing_local_version_or_game_build_does_not_prove_tested_game() {
    let observation = game("Cyberpunk 2077", Some("1091500"));
    assert!(observation.build_version.is_none());
    let item = recommendation(
        &advice::evaluate(None, &observation, "en"),
        "cyberpunk-2077",
    )
    .clone();
    assert_eq!(item.status, AdviceStatus::Conditional);
    assert!(item.explanation.contains("unknown"));
    assert!(item.version_scope.contains("game build unspecified"));

    let mut matched_version = observation;
    matched_version.optiscaler_version = Some("v0.9.3".into());
    let items = advice::evaluate(None, &matched_version, "en");
    let item = recommendation(&items, "cyberpunk-2077");
    assert_eq!(item.status, AdviceStatus::Conditional);
    assert!(!item
        .explanation
        .contains("local OptiScaler version differs"));
}

#[test]
fn unknown_game_has_unknown_wiki_entry() {
    let items = advice::evaluate(None, &game("Never Listed", None), "en");
    let item = recommendation(&items, "game-unknown");
    assert_eq!(item.status, AdviceStatus::Unknown);
    assert_eq!(
        item.source_url,
        "https://github.com/optiscaler/OptiScaler/wiki/Compatibility-List"
    );
}

#[test]
fn invalid_and_conflicting_rule_fixtures_fail_closed() {
    let observation = game("Satisfactory", Some("526870"));
    for malformed in ["{", "[]", r#"{"rules_version":1}"#] {
        let items = advice::evaluate_rules(malformed, None, &observation, "en");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "rules-invalid");
        assert_eq!(items[0].status, AdviceStatus::Unknown);
    }

    let mut fixture: Value = serde_json::from_str(RULES).unwrap();
    let duplicate = fixture["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["id"] == "satisfactory")
        .unwrap()
        .clone();
    let mut conflicting = duplicate;
    conflicting["id"] = json!("satisfactory-conflict");
    conflicting["status"] = json!("unsupported");
    fixture["games"].as_array_mut().unwrap().push(conflicting);
    let items = advice::evaluate_rules(&fixture.to_string(), None, &observation, "en");
    assert_eq!(
        recommendation(&items, "game-rules-conflict").status,
        AdviceStatus::Unknown
    );
    assert!(!items
        .iter()
        .any(|item| item.id == "satisfactory" || item.id == "satisfactory-conflict"));

    fixture["rules_version"] = json!(2);
    let items = advice::evaluate_rules(&fixture.to_string(), None, &observation, "en");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "rules-version-unknown");
}

#[test]
fn local_profiles_preserve_unknown_fields_and_missing_values() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/profiles.json");
    let fixture = json!({
        "hardware": {"gpus": [{"id": "gpu-0", "name": "Intel UHD", "vendor": "Intel", "dedicated_bytes": null, "shared_bytes": null, "driver": null}], "windows_build": null, "ram_bytes": null, "collected_at": null, "warnings": []},
        "game_gpus": {"game-key": "gpu-0"},
        "future_field": {"enabled": true, "count": 7}
    });
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let loaded = LocalProfiles::load(&path).unwrap();
    assert_eq!(
        loaded.hardware.as_ref().unwrap().gpus[0].dedicated_bytes,
        None
    );
    assert_eq!(loaded.hardware.as_ref().unwrap().gpus[0].driver, None);
    assert_eq!(loaded.hardware.as_ref().unwrap().ram_bytes, None);
    loaded.save(&path).unwrap();
    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved, fixture);
    assert_eq!(
        LocalProfiles::load(&path).unwrap().game_gpus["game-key"],
        "gpu-0"
    );
}

#[test]
fn ini_preview_is_read_only_and_repeated_writes_keep_first_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("OptiScaler.ini");
    let original = "[Upscalers]\nMode=old\n";
    fs::write(&path, original).unwrap();
    let before = ini::read_file(&path).unwrap();
    let mut after = before.clone();
    assert!(after.set_value("Upscalers", "Mode", "new"));
    assert_eq!(
        ini::preview_changes(&before, &after),
        ["Upscalers.Mode: old -> new"]
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(before.get("Upscalers", "Mode").unwrap().value, "old");
    assert!(!tmp.path().join("OptiScaler.ini.backup").exists());

    ini::write_file(&path, &after).unwrap();
    let first = tmp.path().join("OptiScaler.ini.backup");
    assert_eq!(fs::read_to_string(&first).unwrap(), original);
    assert!(after.set_value("Upscalers", "Mode", "newer"));
    ini::write_file(&path, &after).unwrap();
    assert_eq!(fs::read_to_string(&first).unwrap(), original);
    assert!(fs::read_to_string(&path).unwrap().contains("Mode=newer"));
    let backups: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("OptiScaler.ini.backup"))
        .collect();
    assert_eq!(backups.len(), 2);
    let second = backups
        .iter()
        .find(|name| name.as_str() != "OptiScaler.ini.backup")
        .unwrap();
    assert!(fs::read_to_string(tmp.path().join(second))
        .unwrap()
        .contains("Mode=new"));
}

#[test]
fn hardware_and_advice_translations_have_matching_nonempty_keys() {
    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }
    let en: Value = serde_json::from_str(EN).unwrap();
    for (language, text) in [("da", DA), ("pl", PL)] {
        let translated: Value = serde_json::from_str(text).unwrap();
        for section in ["hardware", "advice", "report"] {
            let canonical = &en[section];
            let target = &translated[section];
            assert_eq!(keys(target), keys(canonical), "{language}.{section}");
            for key in keys(canonical) {
                assert!(
                    !canonical[&key].as_str().unwrap().trim().is_empty(),
                    "en.{section}.{key}"
                );
                assert!(
                    !target[&key].as_str().unwrap().trim().is_empty(),
                    "{language}.{section}.{key}"
                );
            }
        }
    }
}
