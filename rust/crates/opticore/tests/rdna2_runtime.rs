use opticore::{
    hardware::GpuProfile,
    install::{transaction, InstallOptions},
};
#[test]
fn community_requires_explicit_rx6000_and_consent_before_game_mutation() {
    let game = tempfile::tempdir().unwrap();
    let payload = tempfile::tempdir().unwrap();
    std::fs::write(payload.path().join("OptiScaler.dll"), b"proxy").unwrap();
    std::fs::write(game.path().join("dxgi.dll"), b"existing").unwrap();
    let options = InstallOptions {
        community_rdna2: true,
        overwrite: true,
        ..Default::default()
    };
    assert!(transaction::install(
        game.path(),
        payload.path(),
        &options,
        "test",
        None,
        "now".into(),
        |_| {}
    )
    .is_err());
    assert_eq!(
        std::fs::read(game.path().join("dxgi.dll")).unwrap(),
        b"existing"
    );
}
#[test]
fn official_restore_requires_an_official_runtime_before_mutation() {
    let game = tempfile::tempdir().unwrap();
    let payload = tempfile::tempdir().unwrap();
    std::fs::write(payload.path().join("OptiScaler.dll"), b"proxy").unwrap();
    let options = InstallOptions::default();
    let mut m = transaction::install(
        game.path(),
        payload.path(),
        &options,
        "test",
        None,
        "now".into(),
        |_| {},
    )
    .unwrap();
    m.extra.insert(
        "fsr_runtime".into(),
        serde_json::json!({"community_opt_in":true}),
    );
    opticore::install::manifest::write(game.path(), &m).unwrap();
    let before = std::fs::read(opticore::install::manifest::manifest_path(game.path())).unwrap();
    let update = InstallOptions {
        overwrite: true,
        ..Default::default()
    };
    assert!(transaction::install(
        game.path(),
        payload.path(),
        &update,
        "update",
        None,
        "now".into(),
        |_| {}
    )
    .is_err());
    assert_eq!(
        std::fs::read(opticore::install::manifest::manifest_path(game.path())).unwrap(),
        before
    );
}

#[test]
fn eligibility_does_not_infer_a_rendering_adapter() {
    let gpu = GpuProfile {
        name: Some("AMD Radeon RX 6700 XT".into()),
        ..Default::default()
    };
    assert!(!opticore::install::rdna2::eligible(None, true));
    assert!(!opticore::install::rdna2::eligible(Some(&gpu), false));
    assert!(opticore::install::rdna2::eligible(Some(&gpu), true));
}

#[test]
fn correct_size_but_wrong_hash_cannot_replace_existing_game_runtime() {
    let game = tempfile::tempdir().unwrap();
    let payload = tempfile::tempdir().unwrap();
    let runtime = tempfile::NamedTempFile::new().unwrap();
    runtime
        .as_file()
        .set_len(opticore::install::rdna2::DLL_SIZE)
        .unwrap();
    std::fs::write(game.path().join("dxgi.dll"), b"existing proxy").unwrap();
    std::fs::write(
        game.path().join(opticore::install::rdna2::DLL_NAME),
        b"existing runtime",
    )
    .unwrap();
    let options = InstallOptions {
        overwrite: true,
        community_rdna2: true,
        rendering_gpu: Some(GpuProfile {
            name: Some("AMD Radeon RX 6700 XT".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let error = transaction::install_with_runtime(
        game.path(),
        payload.path(),
        &options,
        "test",
        None,
        "now".into(),
        Some(runtime.path()),
        |_| {},
    )
    .unwrap_err();
    assert!(error.to_string().contains("SHA256"));
    assert_eq!(
        std::fs::read(game.path().join("dxgi.dll")).unwrap(),
        b"existing proxy"
    );
    assert_eq!(
        std::fs::read(game.path().join(opticore::install::rdna2::DLL_NAME)).unwrap(),
        b"existing runtime"
    );
}
