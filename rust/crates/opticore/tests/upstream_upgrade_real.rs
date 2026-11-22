//! Pin the tag under review even if another release lands during the run.
use opticore::{
    archive, ini,
    install::{self, github, transaction, InstallOptions, Installer},
    resolver,
};
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "needs v0.9.4 archive, reviewed tag and GitHub access"]
fn upgrade_from_v094_and_uninstall_from_nested_game() {
    let old_archive = PathBuf::from(env::var_os("OPTISCALER_V094_ARCHIVE").unwrap());
    let tag = env::var("OPTISCALER_EXPECTED_TAG").unwrap();
    let release = opticore::release_defaults::fetch_exact(&tag).unwrap();
    github::verify_digest(
        &old_archive,
        Some("sha256:575cb4df866116093df75af607e37fd70e10f5163e0f23fd5c804142e80ef0ad"),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let old_payload = temp.path().join("old");
    fs::create_dir(&old_payload).unwrap();
    archive::extract_7z(&old_archive, &old_payload).unwrap();
    let game = temp.path().join("Fixture");
    let bin = game.join("Fixture/Binaries/Win64");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("Fixture-Win64-Shipping.exe"), b"game").unwrap();
    fs::write(bin.join("winmm.dll"), b"original proxy").unwrap();
    fs::write(bin.join("other-mod.txt"), b"unrelated").unwrap();
    let target = resolver::resolve(&game).unwrap();
    assert_eq!(
        target.directory.canonicalize().unwrap(),
        bin.canonicalize().unwrap()
    );
    transaction::install(
        &target.directory,
        &old_payload,
        &InstallOptions {
            overwrite: true,
            target_filename: "winmm.dll".into(),
            ..Default::default()
        },
        "v0.9.4",
        None,
        "test".into(),
        |_| {},
    )
    .unwrap();
    let config = bin.join("OptiScaler.ini");
    let custom = format!(
        "{}\n; user customization\n",
        fs::read_to_string(&config).unwrap()
    );
    fs::write(&config, &custom).unwrap();
    let cache = temp.path().join("cache");
    let installer = Installer::at_release(&cache, release.clone());
    let updated = installer.update(&game, "auto", |_| {}).unwrap();
    assert_eq!(updated.optiscaler_version, tag);
    assert_eq!(
        install::installed_version(&game).as_deref(),
        Some(tag.as_str())
    );
    assert_eq!(updated.target_filename, "winmm.dll");
    assert!(!updated.extra.contains_key("fsr_runtime"));
    let payload = installer.prepare_release(&release, |_| {}).unwrap();
    assert_eq!(release.version_label(), tag);
    let snapshot = opticore::release_defaults::cached(&cache, &tag).unwrap();
    assert_eq!(
        ini::serialize(&snapshot),
        ini::serialize(&ini::read_file(&payload.join("OptiScaler.ini")).unwrap())
    );
    assert_eq!(
        fs::read(bin.join("winmm.dll")).unwrap(),
        fs::read(payload.join("OptiScaler.dll")).unwrap()
    );
    let vulkan = [
        "amd_fidelityfx_upscaler_vk.dll",
        "amd_fidelityfx_framegeneration_vk.dll",
    ];
    if tag == "v0.9.5" {
        // Simulate a partial extraction while keeping the primary DLL intact.
        fs::remove_file(payload.join(vulkan[0])).unwrap();
        let repaired = installer.prepare_release(&release, |_| {}).unwrap();
        assert_eq!(
            fs::read(repaired.join(vulkan[0])).unwrap(),
            fs::read(bin.join(vulkan[0])).unwrap()
        );
        let defaults = ini::read_file(&payload.join("OptiScaler.ini")).unwrap();
        assert!(defaults
            .sections
            .iter()
            .flat_map(|s| &s.entries)
            .any(|e| e.key == "DisableFlipMetering" && e.value == "auto"));
        for dll in vulkan {
            assert!(!old_payload.join(dll).exists());
            assert!(updated.owned_files.contains_key(dll));
            assert_eq!(
                fs::read(bin.join(dll)).unwrap(),
                fs::read(payload.join(dll)).unwrap()
            );
        }
    }
    assert_eq!(fs::read_to_string(&config).unwrap(), custom);
    install::uninstall(&game).unwrap();
    assert_eq!(install::installed_version(&game), None);
    assert_eq!(fs::read(bin.join("winmm.dll")).unwrap(), b"original proxy");
    assert_eq!(fs::read_to_string(&config).unwrap(), custom);
    assert_eq!(fs::read(bin.join("other-mod.txt")).unwrap(), b"unrelated");
    for rel in updated
        .owned_files
        .keys()
        .filter(|r| !r.eq_ignore_ascii_case("winmm.dll"))
    {
        assert!(!bin.join(rel).exists(), "owned file left behind: {rel}");
    }
    for dll in vulkan {
        assert!(!bin.join(dll).exists());
    }
}
