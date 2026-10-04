//! Opt-in disk-only canary: uses real pinned artifacts, never a user's game.
use opticore::{
    hardware::GpuProfile,
    install::{manifest, rdna2, transaction, InstallOptions},
};
#[test]
#[ignore = "requires OPTISCALER_ARCHIVE and RDNA2_ARCHIVE"]
fn real_archives_install_update_uninstall_and_corrupt_runtime_preserves_install() {
    let official = std::env::var("OPTISCALER_ARCHIVE").unwrap();
    let community = std::env::var("RDNA2_ARCHIVE").unwrap();
    let payload = tempfile::tempdir().unwrap();
    let runtime = tempfile::tempdir().unwrap();
    let game = tempfile::tempdir().unwrap();
    opticore::install::github::verify_digest(
        std::path::Path::new(&community),
        Some(&format!("sha256:{}", rdna2::ARCHIVE_SHA256)),
    )
    .unwrap();
    opticore::archive::extract_7z(std::path::Path::new(&official), payload.path()).unwrap();
    opticore::archive::extract_7z(std::path::Path::new(&community), runtime.path()).unwrap();
    let dll = runtime.path().join(rdna2::ARCHIVE_MEMBER);
    rdna2::validate(&dll).unwrap();
    let original = std::fs::read(payload.path().join(rdna2::DLL_NAME)).unwrap();
    let options = InstallOptions {
        overwrite: true,
        community_rdna2: true,
        rendering_gpu: Some(GpuProfile {
            name: Some("AMD Radeon RX 6700 XT".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    std::fs::write(game.path().join(rdna2::DLL_NAME), b"foreign-original").unwrap();
    let first = transaction::install_with_runtime(
        game.path(),
        payload.path(),
        &options,
        "real",
        None,
        "now".into(),
        Some(&dll),
        |_| {},
    )
    .unwrap();
    assert!(first.extra.contains_key("fsr_runtime"));
    rdna2::validate(&game.path().join(rdna2::DLL_NAME)).unwrap();
    assert_eq!(
        std::fs::read(payload.path().join(rdna2::DLL_NAME)).unwrap(),
        original
    );
    let old_manifest = std::fs::read(manifest::manifest_path(game.path())).unwrap();
    std::fs::write(&dll, b"corrupt").unwrap();
    assert!(transaction::install_with_runtime(
        game.path(),
        payload.path(),
        &options,
        "bad",
        None,
        "now".into(),
        Some(&dll),
        |_| {}
    )
    .is_err());
    assert_eq!(
        std::fs::read(manifest::manifest_path(game.path())).unwrap(),
        old_manifest
    );
    rdna2::validate(&game.path().join(rdna2::DLL_NAME)).unwrap();
    opticore::archive::extract_7z(std::path::Path::new(&community), runtime.path()).unwrap();
    transaction::install_with_runtime(
        game.path(),
        payload.path(),
        &options,
        "update",
        None,
        "now".into(),
        Some(&dll),
        |_| {},
    )
    .unwrap();
    let official_options = InstallOptions {
        overwrite: true,
        ..Default::default()
    };
    let restored = transaction::install(
        game.path(),
        payload.path(),
        &official_options,
        "official",
        None,
        "now".into(),
        |_| {},
    )
    .unwrap();
    assert!(!restored.extra.contains_key("fsr_runtime"));
    assert_eq!(
        std::fs::read(game.path().join(rdna2::DLL_NAME)).unwrap(),
        original
    );
    transaction::uninstall(game.path()).unwrap();
    assert_eq!(
        std::fs::read(game.path().join(rdna2::DLL_NAME)).unwrap(),
        b"foreign-original"
    );
}

#[test]
#[ignore = "requires RDNA2_ARCHIVE"]
fn pinned_cache_is_verified_and_corrupt_extracted_cache_is_rebuilt() {
    let archive = std::env::var("RDNA2_ARCHIVE").unwrap();
    let cache = tempfile::tempdir().unwrap();
    let isolated = cache.path().join("community-rdna2-4.1.1b");
    std::fs::create_dir_all(&isolated).unwrap();
    std::fs::copy(archive, isolated.join(rdna2::ASSET)).unwrap();
    std::fs::write(isolated.join(rdna2::DLL_NAME), b"corrupt stale cache").unwrap();
    let dll = rdna2::prepare(cache.path()).unwrap();
    assert_eq!(dll.parent().unwrap(), isolated);
    rdna2::validate(&dll).unwrap();
    assert!(!cache.path().join("extracted").exists());
}
