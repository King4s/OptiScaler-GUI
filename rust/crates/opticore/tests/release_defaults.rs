use opticore::release_defaults::{cached, remember, valid_tag};
use std::fs;

#[test]
fn restore_uses_only_the_requested_tag_even_when_another_is_newer() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tempfile::tempdir().unwrap();
    for (tag, value) in [("v0.9.4", "false"), ("v0.10.0", "3")] {
        fs::write(
            payload.path().join("OptiScaler.ini"),
            format!("[FSR]\nMode={value}\n"),
        )
        .unwrap();
        remember(tmp.path(), tag, payload.path()).unwrap();
    }
    assert_eq!(
        cached(tmp.path(), "v0.9.4")
            .unwrap()
            .get("FSR", "Mode")
            .unwrap()
            .value,
        "false"
    );
    assert_eq!(
        cached(tmp.path(), "v0.10.0")
            .unwrap()
            .get("FSR", "Mode")
            .unwrap()
            .value,
        "3"
    );
    assert!(cached(tmp.path(), "v0.9.5").is_none());
    for tag in ["v0.9.5", "0.10.0", "v1.0.0+build.1"] {
        assert!(valid_tag(tag));
    }
    for bad in ["", "Unknown", "latest", "../v0.9.5", "v0/9", "v0.9:5"] {
        assert!(!valid_tag(bad));
    }
}

#[test]
fn altered_or_misfiled_defaults_are_not_used() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tempfile::tempdir().unwrap();
    fs::write(payload.path().join("OptiScaler.ini"), "[FSR]\nMode=3\n").unwrap();
    remember(tmp.path(), "v0.9.5", payload.path()).unwrap();
    let dir = tmp.path().join("ini-defaults");
    let original = dir.join("v0.9.5.json");
    fs::copy(&original, dir.join("v0.9.4.json")).unwrap();
    assert!(cached(tmp.path(), "v0.9.4").is_none());
    let contents = fs::read_to_string(&original)
        .unwrap()
        .replace("Mode=3", "Mode=1");
    fs::write(original, contents).unwrap();
    assert!(cached(tmp.path(), "v0.9.5").is_none());
}
