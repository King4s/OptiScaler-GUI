use opticore::resolver;
use std::fs;
use std::path::{Path, PathBuf};

fn file(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"fixture").unwrap();
    path
}

#[test]
fn fatekeeper_uses_project_shipping_not_engine_or_bootstrap() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = file(
        tmp.path(),
        "SLASHER/Binaries/Win64/SLASHER-Win64-Shipping.exe",
    );
    file(tmp.path(), "Fatekeeper.exe");
    file(tmp.path(), "Engine/Binaries/Win64/CrashReportClient.exe");
    let target = resolver::resolve(tmp.path()).unwrap();
    assert_eq!(target.executable, exe.canonicalize().unwrap());
    assert_eq!(
        target.directory,
        exe.parent().unwrap().canonicalize().unwrap()
    );
}

#[test]
fn ordinary_nested_layouts_are_resolved_without_root_guess() {
    for dir in [
        "bin/x64",
        "Retail",
        "binaries",
        "bin/x64_dx12",
        "Project/Binaries/WinGDK",
        "Content/Project/Binaries/WinGDK",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let exe = file(tmp.path(), &format!("{dir}/Game.exe"));
        file(tmp.path(), "Launcher.exe");
        file(tmp.path(), "setup.exe");
        let target = resolver::resolve(tmp.path()).unwrap();
        assert_eq!(target.executable, exe.canonicalize().unwrap(), "{dir}");
        assert_eq!(
            target.directory,
            exe.parent().unwrap().canonicalize().unwrap(),
            "{dir}"
        );
    }
}

#[test]
fn satisfactory_keeps_actual_exe_but_uses_documented_engine_target() {
    for folder in ["Satisfactory", "Renamed Steam Library Entry"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(folder);
        let exe = file(
            &root,
            "FactoryGame/Binaries/Win64/FactoryGameSteam-Win64-Shipping.exe",
        );
        let expected = root.join("Engine/Binaries/Win64");
        fs::create_dir_all(&expected).unwrap();
        let target = resolver::resolve(&root).unwrap();
        assert_eq!(target.executable, exe.canonicalize().unwrap());
        assert_eq!(target.directory, expected.canonicalize().unwrap());
        resolver::remember(&root, &exe).unwrap();
        assert_eq!(
            resolver::resolve(&root).unwrap().directory,
            expected.canonicalize().unwrap()
        );
    }
}

#[test]
fn satisfactory_missing_documented_target_never_falls_back_to_exe_folder() {
    for folder in ["Satisfactory", "Renamed Steam Library Entry"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(folder);
        let exe = file(
            &root,
            "FactoryGame/Binaries/Win64/FactoryGameSteam-Win64-Shipping.exe",
        );
        file(&root, "Satisfactory.exe");
        assert!(resolver::resolve(&root).is_err());
        assert!(resolver::remember(&root, &exe).is_err());
        assert!(!root.join(".optiscaler-gui-target.json").exists());
    }
}

#[test]
fn satisfactory_multiple_shipping_exes_require_an_explicit_choice() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Satisfactory");
    let selected = file(
        &root,
        "FactoryGame/Binaries/Win64/FactoryGameSteam-Win64-Shipping.exe",
    );
    file(
        &root,
        "FactoryGame/Binaries/Win64/FactoryGameEGS-Win64-Shipping.exe",
    );
    let engine = root.join("Engine/Binaries/Win64");
    fs::create_dir_all(&engine).unwrap();
    assert!(resolver::resolve(&root).is_err());
    resolver::remember(&root, &selected).unwrap();
    let target = resolver::resolve(&root).unwrap();
    assert_eq!(target.executable, selected.canonicalize().unwrap());
    assert_eq!(target.directory, engine.canonicalize().unwrap());
}

#[test]
fn ambiguous_targets_require_explicit_choice_and_remember_relative_path() {
    let tmp = tempfile::tempdir().unwrap();
    file(tmp.path(), "bin/x64/GameDX11.exe");
    let selected = file(tmp.path(), "bin/x64_dx12/GameDX12.exe");
    assert!(resolver::resolve(tmp.path()).is_err());
    resolver::remember(tmp.path(), &selected).unwrap();
    assert_eq!(
        resolver::resolve(tmp.path()).unwrap().executable,
        selected.canonicalize().unwrap()
    );
    let saved = fs::read_to_string(tmp.path().join(".optiscaler-gui-target.json")).unwrap();
    assert!(!saved.contains(&tmp.path().to_string_lossy().to_string()));
}

#[test]
fn missing_exe_and_engine_tools_do_not_resolve() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(resolver::resolve(tmp.path()).is_err());
    for name in [
        "Engine/Binaries/Win64/CrashReportClient.exe",
        "Launcher.exe",
        "setup.exe",
        "uninstall.exe",
    ] {
        file(tmp.path(), name);
    }
    assert!(resolver::resolve(tmp.path()).is_err());
}

#[test]
fn explicit_and_persisted_escape_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Game");
    fs::create_dir(&root).unwrap();
    let outside = file(tmp.path(), "Other.exe");
    assert!(resolver::choose(&root, &outside).is_err());
    fs::write(
        root.join(".optiscaler-gui-target.json"),
        r#"{"executable":"../Other.exe"}"#,
    )
    .unwrap();
    assert!(resolver::resolve(&root).is_err());
    assert_eq!(fs::read(outside).unwrap(), b"fixture");
}

#[test]
fn explicit_choice_rejects_tools_and_removed_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = file(tmp.path(), "CrashReportClient.exe");
    assert!(resolver::remember(tmp.path(), &tool).is_err());
    let exe = file(tmp.path(), "Game.exe");
    resolver::remember(tmp.path(), &exe).unwrap();
    fs::remove_file(exe).unwrap();
    assert!(resolver::resolve(tmp.path()).is_err());
}

#[test]
fn scanner_does_not_mistake_native_dlss_for_optiscaler_installation() {
    let tmp = tempfile::tempdir().unwrap();
    file(tmp.path(), "Game.exe");
    file(tmp.path(), "nvngx_dlss.dll");
    file(tmp.path(), "nvngx_dlssg.dll");
    let facts = opticore::scan::folder_facts::collect(tmp.path()).unwrap();
    assert!(!opticore::scan::folder_facts::detect_optiscaler(
        tmp.path(),
        &facts
    ));
}

#[test]
fn scanner_uses_same_shipping_target_as_installer_not_engine_leftovers() {
    let tmp = tempfile::tempdir().unwrap();
    file(
        tmp.path(),
        "SLASHER/Binaries/Win64/SLASHER-Win64-Shipping.exe",
    );
    file(tmp.path(), "Engine/Binaries/Win64/OptiScaler.dll");
    let facts = opticore::scan::folder_facts::collect(tmp.path()).unwrap();
    assert!(!opticore::scan::folder_facts::detect_optiscaler(
        tmp.path(),
        &facts
    ));
}
