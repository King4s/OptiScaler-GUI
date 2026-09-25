use opticore::install::manifest::{self, InstallManifest, OwnedFile};
use opticore::install::transaction;
use opticore::install::{InstallOptions, InstallStage};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    game: PathBuf,
    payload: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let payload = temp.path().join("payload");
        fs::create_dir(&game).unwrap();
        fs::create_dir(&payload).unwrap();
        fs::write(payload.join("OptiScaler.dll"), b"proxy-v1").unwrap();
        Self {
            _temp: temp,
            game,
            payload,
        }
    }

    fn add_payload(&self, rel: &str, bytes: &[u8]) {
        let path = self.payload.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn install(&self, overwrite: bool) -> Result<InstallManifest, opticore::install::InstallError> {
        transaction::install(
            &self.game,
            &self.payload,
            &InstallOptions {
                overwrite,
                ..InstallOptions::default()
            },
            "v0.9.3",
            None,
            "2026-09-25T00:00:00".into(),
            |_| {},
        )
    }
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn v2_claim(game: &Path, files: &[&str]) {
    let names: Vec<String> = files.iter().map(|name| (*name).to_owned()).collect();
    let mut manifest = InstallManifest::new(
        "dxgi.dll",
        &names,
        &[],
        "v0.9.3",
        None,
        "2026-09-25T00:00:00".into(),
    );
    manifest.schema_version = 2;
    manifest.owned_files = names
        .into_iter()
        .map(|name| {
            (
                name,
                OwnedFile {
                    sha256: sha256(b"owned"),
                    original_backup: None,
                    original_sha256: None,
                    extra: BTreeMap::new(),
                },
            )
        })
        .collect();
    manifest::write(game, &manifest).unwrap();
}

#[test]
fn fresh_install_records_checksums_and_owned_paths() {
    let fixture = Fixture::new();
    fixture.add_payload("nested/plugin.dll", b"plugin");
    fixture.add_payload("OptiScaler.ini", b"[Spoofing]\nDxgi=auto\n");
    let installed = fixture.install(false).unwrap();

    assert_eq!(installed.schema_version, 2);
    assert_eq!(
        installed.owned_files["dxgi.dll"].sha256,
        sha256(b"proxy-v1")
    );
    assert_eq!(
        installed.owned_files["nested/plugin.dll"].sha256,
        sha256(b"plugin")
    );
    assert_eq!(
        installed.owned_files["OptiScaler.ini"].sha256,
        sha256(b"[Spoofing]\nDxgi=auto\n")
    );
    assert!(installed
        .owned_files
        .values()
        .all(|owned| owned.original_backup.is_none()));
    assert_eq!(
        fs::read(fixture.game.join("nested/plugin.dll")).unwrap(),
        b"plugin"
    );
    assert!(manifest::read(&fixture.game).unwrap().is_owned_v2());
}

#[test]
fn overwrite_backs_up_foreign_dll_and_uninstall_restores_it() {
    let fixture = Fixture::new();
    fs::write(fixture.game.join("dxgi.dll"), b"game-original").unwrap();
    let installed = fixture.install(true).unwrap();
    let owned = &installed.owned_files["dxgi.dll"];
    let backup = fixture.game.join(owned.original_backup.as_ref().unwrap());
    assert_eq!(
        owned.original_sha256.as_deref(),
        Some(sha256(b"game-original").as_str())
    );
    assert_eq!(fs::read(&backup).unwrap(), b"game-original");
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );

    let (removed, _) = transaction::uninstall(&fixture.game).unwrap();
    assert!(removed.contains(&"dxgi.dll".to_string()));
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"game-original"
    );
    assert!(!manifest::manifest_path(&fixture.game).exists());
}

#[test]
fn uninstall_preserves_foreign_nested_mod_files_and_nonempty_directory() {
    let fixture = Fixture::new();
    fixture.add_payload("mods/owned.dll", b"owned");
    fixture.install(false).unwrap();
    fs::write(fixture.game.join("mods/other-mod.dll"), b"foreign").unwrap();
    fs::write(fixture.game.join("unrelated.txt"), b"foreign-root").unwrap();

    transaction::uninstall(&fixture.game).unwrap();
    assert!(!fixture.game.join("mods/owned.dll").exists());
    assert_eq!(
        fs::read(fixture.game.join("mods/other-mod.dll")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(fixture.game.join("unrelated.txt")).unwrap(),
        b"foreign-root"
    );
    assert!(fixture.game.join("mods").is_dir());
}

#[test]
fn changed_owned_file_blocks_update_and_uninstall_without_mutation() {
    let fixture = Fixture::new();
    fixture.install(false).unwrap();
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
    fs::write(fixture.game.join("dxgi.dll"), b"user-modified").unwrap();
    fs::write(fixture.payload.join("OptiScaler.dll"), b"proxy-v2").unwrap();

    assert!(fixture.install(true).is_err());
    assert!(transaction::uninstall(&fixture.game).is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"user-modified"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
}

#[test]
fn edited_installed_ini_is_preserved_during_update_and_uninstall() {
    let fixture = Fixture::new();
    fixture.add_payload("OptiScaler.ini", b"[Spoofing]\nDxgi=auto\n");
    fixture.install(false).unwrap();
    let edited = b"[Spoofing]\nDxgi=false\n; user choice\n";
    fs::write(fixture.game.join("OptiScaler.ini"), edited).unwrap();
    fs::write(fixture.payload.join("OptiScaler.dll"), b"proxy-v2").unwrap();

    fixture
        .install(true)
        .expect("edited config should not block a safe binary update");
    assert_eq!(
        fs::read(fixture.game.join("OptiScaler.ini")).unwrap(),
        edited
    );
    transaction::uninstall(&fixture.game).expect("edited config should remain after uninstall");
    assert_eq!(
        fs::read(fixture.game.join("OptiScaler.ini")).unwrap(),
        edited
    );
    assert!(!fixture.game.join("dxgi.dll").exists());
}

#[test]
fn update_refuses_new_foreign_payload_collision() {
    let fixture = Fixture::new();
    fixture.install(false).unwrap();
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
    fixture.add_payload("extra.dll", b"new-payload");
    fs::write(fixture.game.join("extra.dll"), b"other-mod").unwrap();
    assert!(fixture.install(true).is_err());
    assert_eq!(
        fs::read(fixture.game.join("extra.dll")).unwrap(),
        b"other-mod"
    );
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
}

#[cfg(windows)]
#[test]
fn install_does_not_write_through_hard_link_outside_game() {
    let fixture = Fixture::new();
    let outside = fixture._temp.path().join("outside.dll");
    fs::write(&outside, b"outside-original").unwrap();
    fs::hard_link(&outside, fixture.game.join("dxgi.dll")).unwrap();
    let _ = fixture.install(true);
    assert_eq!(fs::read(&outside).unwrap(), b"outside-original");
}

#[test]
fn legacy_manifest_refuses_destructive_update_and_uninstall() {
    let fixture = Fixture::new();
    let legacy = InstallManifest::new(
        "dxgi.dll",
        &["dxgi.dll".into()],
        &[],
        "v0.9.2",
        None,
        "old".into(),
    );
    manifest::write(&fixture.game, &legacy).unwrap();
    fs::write(fixture.game.join("dxgi.dll"), b"legacy-or-foreign").unwrap();
    let before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();

    assert!(fixture.install(true).is_err());
    assert!(transaction::uninstall(&fixture.game).is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"legacy-or-foreign"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        before
    );
}

#[test]
fn foreign_tool_v2_manifest_cannot_authorize_update_or_uninstall() {
    for update in [true, false] {
        let fixture = Fixture::new();
        v2_claim(&fixture.game, &["dxgi.dll"]);
        let mut claim = manifest::read(&fixture.game).unwrap();
        claim.installed_by = "Another tool".into();
        manifest::write(&fixture.game, &claim).unwrap();
        fs::write(fixture.game.join("dxgi.dll"), b"owned").unwrap();
        let before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();

        let result = if update {
            fixture.install(true).map(|_| ())
        } else {
            transaction::uninstall(&fixture.game).map(|_| ())
        };
        assert!(
            result.is_err(),
            "accepted another tool's manifest (update={update})"
        );
        assert_eq!(fs::read(fixture.game.join("dxgi.dll")).unwrap(), b"owned");
        assert_eq!(
            fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
            before
        );
    }
}

#[test]
fn payload_scripts_and_setup_markers_are_never_installed() {
    let fixture = Fixture::new();
    for name in [
        "Remove OptiScaler.bat",
        "setup.cmd",
        "nested/install.ps1",
        "nested/setup.sh",
        "!! EXTRACT FIRST !!.txt",
    ] {
        fixture.add_payload(name, b"dangerous script");
    }
    let installed = fixture.install(false).unwrap();
    assert_eq!(installed.files, vec!["dxgi.dll"]);
    for name in [
        "Remove OptiScaler.bat",
        "setup.cmd",
        "nested/install.ps1",
        "nested/setup.sh",
        "!! EXTRACT FIRST !!.txt",
    ] {
        assert!(!fixture.game.join(name).exists(), "copied {name}");
    }
}

#[test]
fn malformed_v2_claims_cannot_escape_or_alias_file_ownership() {
    for claims in [
        vec!["../outside.dll"],
        vec!["dxgi.dll", "DXGI.dll"],
        vec!["CON.txt"],
        vec![".optiscaler-gui-backups/stolen.dll"],
        vec![".optiscaler-gui-target.json"],
    ] {
        let fixture = Fixture::new();
        v2_claim(&fixture.game, &claims);
        fs::write(fixture.game.join("dxgi.dll"), b"foreign").unwrap();
        let before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
        assert!(
            transaction::uninstall(&fixture.game).is_err(),
            "accepted {claims:?}"
        );
        assert_eq!(fs::read(fixture.game.join("dxgi.dll")).unwrap(), b"foreign");
        assert_eq!(
            fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
            before
        );
    }
}

#[test]
fn late_copy_failure_rolls_back_replaced_dll_and_preserves_manifest() {
    let fixture = Fixture::new();
    fixture.install(false).unwrap();
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
    fs::write(fixture.payload.join("OptiScaler.dll"), b"proxy-v2").unwrap();
    fixture.add_payload("z.dll", b"new-later-file");
    let mut injected = false;
    let result = transaction::install(
        &fixture.game,
        &fixture.payload,
        &InstallOptions {
            overwrite: true,
            ..InstallOptions::default()
        },
        "v0.9.4",
        None,
        "2026-09-25T01:00:00".into(),
        |stage| {
            if matches!(stage, InstallStage::CopyingPayload { done: 1, .. }) && !injected {
                fs::create_dir(fixture.game.join("z.dll")).unwrap();
                injected = true;
            }
        },
    );
    assert!(injected);
    assert!(result.is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
    assert!(fixture.game.join("z.dll").is_dir());
}

#[test]
fn foreign_file_appearing_after_preflight_is_not_overwritten() {
    let fixture = Fixture::new();
    fixture.install(false).unwrap();
    fixture.add_payload("z.dll", b"new-payload");
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
    let result = transaction::install(
        &fixture.game,
        &fixture.payload,
        &InstallOptions {
            overwrite: true,
            ..InstallOptions::default()
        },
        "v0.9.4",
        None,
        "2026-09-25T01:00:00".into(),
        |stage| {
            if matches!(stage, InstallStage::CopyingPayload { done: 1, .. }) {
                fs::write(fixture.game.join("z.dll"), b"late-foreign").unwrap();
            }
        },
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(fixture.game.join("z.dll")).unwrap(),
        b"late-foreign"
    );
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
}

#[test]
fn rollback_preserves_file_changed_after_write() {
    let fixture = Fixture::new();
    fixture.install(false).unwrap();
    fixture.add_payload("z.dll", b"new-payload");
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();
    let result = transaction::install(
        &fixture.game,
        &fixture.payload,
        &InstallOptions {
            overwrite: true,
            ..InstallOptions::default()
        },
        "v0.9.4",
        None,
        "2026-09-25T01:00:00".into(),
        |stage| {
            if matches!(stage, InstallStage::CopyingPayload { done: 1, .. }) {
                fs::write(fixture.game.join("dxgi.dll"), b"late-user-change").unwrap();
                fs::create_dir(fixture.game.join("z.dll")).unwrap();
            }
        },
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"late-user-change"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
}

#[test]
fn changed_original_backup_blocks_restore_without_mutation() {
    let fixture = Fixture::new();
    fs::write(fixture.game.join("dxgi.dll"), b"original").unwrap();
    let installed = fixture.install(true).unwrap();
    let backup = fixture.game.join(
        installed.owned_files["dxgi.dll"]
            .original_backup
            .as_ref()
            .unwrap(),
    );
    fs::write(&backup, b"tampered").unwrap();
    let manifest_before = fs::read(manifest::manifest_path(&fixture.game)).unwrap();

    assert!(transaction::uninstall(&fixture.game).is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );
    assert_eq!(
        fs::read(manifest::manifest_path(&fixture.game)).unwrap(),
        manifest_before
    );
    assert_eq!(fs::read(backup).unwrap(), b"tampered");
}

#[cfg(windows)]
#[test]
fn locked_foreign_dll_causes_no_loss() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    fs::write(fixture.game.join("dxgi.dll"), b"game-original").unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(fixture.game.join("dxgi.dll"))
        .unwrap();
    assert!(fixture.install(true).is_err());
    drop(lock);
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"game-original"
    );
    assert!(!manifest::manifest_path(&fixture.game).exists());
}

#[cfg(windows)]
#[test]
fn backup_junction_is_refused_without_writing_through_it() {
    use std::process::Command;
    let fixture = Fixture::new();
    let outside = fixture._temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"keep").unwrap();
    let junction = fixture.game.join(manifest::BACKUP_DIRECTORY);
    let status = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&junction)
        .arg(&outside)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "junction fixture creation must succeed for this safety check"
    );
    assert!(fixture.install(false).is_err());
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert!(!fixture.game.join("dxgi.dll").exists());
    assert!(!manifest::manifest_path(&fixture.game).exists());
    fs::remove_dir(&junction).unwrap();
    fixture.install(false).unwrap();
    if junction.exists() {
        fs::remove_dir(&junction).unwrap();
    }
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(transaction::uninstall(&fixture.game).is_err());
    assert_eq!(
        fs::read(fixture.game.join("dxgi.dll")).unwrap(),
        b"proxy-v1"
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert!(manifest::manifest_path(&fixture.game).exists());
    fs::remove_dir(&junction).unwrap();
}
