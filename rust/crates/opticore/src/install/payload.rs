//! Payload installation: marker removal, stale legacy cleanup, config
//! backup, dynamic payload copy, uninstaller script, rollback.
//! Port of the corresponding OptiScalerManager methods.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Proxy DLL names OptiScaler can be installed as (UI order, dxgi.dll
/// default). Mirrors the upstream wiki's supported filename list.
pub const PROXY_FILENAMES: &[&str] = &[
    "dxgi.dll",
    "winmm.dll",
    "version.dll",
    "dbghelp.dll",
    "d3d12.dll",
    "wininet.dll",
    "winhttp.dll",
    "OptiScaler.asi",
];

/// Proxy names from older OptiScaler versions that upstream no longer
/// supports (a game only loads nvngx.dll if it calls the DLSS loader, so
/// OptiScaler silently never starts). Not offered for new installs; kept
/// so uninstall/cleanup still removes them and update migrates them.
pub const LEGACY_PROXY_FILENAMES: &[&str] = &["nvngx.dll"];

/// Files from older OptiScaler layouts removed before installing v0.9+ payloads.
const STALE_LEGACY_FILES: &[&str] = &["nvapi64.dll", "nvngx.dll"];

/// Release marker files never copied to the game dir.
const PAYLOAD_EXCLUDED_FILENAMES: &[&str] = &["OptiScaler.dll", super::manifest::MANIFEST_FILENAME];
const PAYLOAD_EXCLUDED_PREFIXES: &[&str] = &["!!"];

/// Directories the legacy (manifest-less) uninstall removes.
pub const LEGACY_UNINSTALL_DIRS: &[&str] = &["D3D12_Optiscaler", "DlssOverrides", "Licenses"];

/// Additional files the legacy uninstall removes (with proxies + basics).
pub const LEGACY_UNINSTALL_FILES: &[&str] = &[
    "OptiScaler.dll",
    "OptiScaler.ini",
    "OptiScaler.log",
    "Remove OptiScaler.bat",
    "Setup OptiScaler.bat",
    "OptiScaler.ini.backup",
    "amd_fidelityfx_dx12.dll",
    "amd_fidelityfx_vk.dll",
    "libxess_dx11.dll",
    "libxess.dll",
    "nvngx_dlss.dll",
    "amd_fidelityfx_dx12_v2.dll",
    "amd_fidelityfx_vk_v2.dll",
    "fakenvapi.dll",
    "fakenvapi.ini",
    "dlssg_to_fsr3_amd_is_better.dll",
    "libxell.dll",
    "libxess_fg.dll",
    "amd_fidelityfx_framegeneration_dx12.dll",
    "amd_fidelityfx_upscaler_dx12.dll",
    "setup_windows.bat",
    "setup_linux.sh",
];

/// Unreal Engine games install into Engine/Binaries/Win64, everything else
/// into the game root. Port of `_determine_install_directory`.
pub fn determine_install_directory(game_path: &Path) -> PathBuf {
    let unreal = game_path.join("Engine").join("Binaries").join("Win64");
    if unreal.is_dir() {
        unreal
    } else {
        game_path.to_path_buf()
    }
}

/// Remove "!! EXTRACT ALL FILES !!"-style marker files from the payload.
pub fn remove_setup_markers(extracted: &Path) {
    if let Ok(entries) = std::fs::read_dir(extracted) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("!!") && entry.path().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Remove files from older OptiScaler layouts (preserving the chosen proxy).
pub fn remove_stale_legacy_files(dest_dir: &Path, target_filename: &str) {
    for filename in STALE_LEGACY_FILES {
        if filename.eq_ignore_ascii_case(target_filename) {
            continue;
        }
        let path = dest_dir.join(filename);
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Timestamped backup of an existing OptiScaler.ini before overwrite installs.
pub fn backup_existing_config(dest_dir: &Path, timestamp: &str) -> Option<PathBuf> {
    let config = dest_dir.join("OptiScaler.ini");
    if !config.exists() {
        return None;
    }
    let backup = dest_dir.join(format!("OptiScaler.ini.{timestamp}.backup"));
    std::fs::copy(&config, &backup).ok()?;
    Some(backup)
}

fn should_copy_payload_file(file_name: &str) -> bool {
    if PAYLOAD_EXCLUDED_FILENAMES
        .iter()
        .any(|ex| ex.eq_ignore_ascii_case(file_name))
    {
        return false;
    }
    !PAYLOAD_EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| file_name.starts_with(prefix))
}

pub struct CopiedPayload {
    pub files: Vec<String>,
    pub directories: Vec<String>,
}

/// Copy the release payload dynamically (everything except OptiScaler.dll
/// and markers), preserving relative paths. Port of `_copy_release_payload`.
pub fn copy_release_payload(
    extracted: &Path,
    dest_dir: &Path,
    mut progress: impl FnMut(usize, usize),
) -> std::io::Result<CopiedPayload> {
    let mut files_to_copy: Vec<PathBuf> = Vec::new();
    collect_files(extracted, extracted, &mut files_to_copy)?;
    files_to_copy.retain(|rel| {
        rel.file_name()
            .map(|n| should_copy_payload_file(&n.to_string_lossy()))
            .unwrap_or(false)
    });

    let total = files_to_copy.len();
    let mut copied_files = Vec::new();
    let mut copied_dirs: BTreeSet<String> = BTreeSet::new();
    for (index, rel) in files_to_copy.iter().enumerate() {
        let src = extracted.join(rel);
        let dst = dest_dir.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, &dst)?;
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        for ancestor in rel.ancestors().skip(1) {
            let text = ancestor.to_string_lossy().replace('\\', "/");
            if !text.is_empty() && text != "." {
                copied_dirs.insert(text);
            }
        }
        copied_files.push(rel_text);
        progress(index + 1, total);
    }
    Ok(CopiedPayload {
        files: copied_files,
        directories: copied_dirs.into_iter().collect(),
    })
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

/// Locate OptiScaler.dll in the extracted payload (root first, then subdirs).
pub fn find_optiscaler_dll(extracted: &Path) -> Option<PathBuf> {
    let direct = extracted.join("OptiScaler.dll");
    if direct.exists() {
        return Some(direct);
    }
    let mut all = Vec::new();
    collect_files(extracted, extracted, &mut all).ok()?;
    all.into_iter()
        .find(|rel| {
            rel.file_name()
                .map(|n| n.eq_ignore_ascii_case("OptiScaler.dll"))
                .unwrap_or(false)
        })
        .map(|rel| extracted.join(rel))
}

/// Interactive "Remove OptiScaler.bat" — same shape as the Python generator.
pub fn create_uninstaller_script(
    install_dir: &Path,
    copied_files: &[String],
) -> std::io::Result<()> {
    let mut script = String::from(
        "@echo off\ncls\necho OptiScaler Uninstaller\necho ======================\necho.\n\
         echo This will remove OptiScaler from this game.\necho.\n\
         set /p removeChoice=\"Do you want to remove OptiScaler? [y/n]: \"\n\
         if \"%removeChoice%\"==\"y\" (\n    echo Removing OptiScaler files...\n",
    );
    for file in copied_files {
        let win_path = file.replace('/', "\\");
        script.push_str(&format!("    if exist \"{win_path}\" del \"{win_path}\"\n"));
    }
    for dir in LEGACY_UNINSTALL_DIRS {
        script.push_str(&format!("    if exist \"{dir}\" rd /s /q \"{dir}\"\n"));
    }
    script.push_str(
        "    if exist OptiScaler.log del OptiScaler.log\n    echo.\n\
         echo OptiScaler removed successfully!\n    echo.\n) else (\n    echo.\n\
         echo Operation cancelled.\n    echo.\n)\npause\nif \"%removeChoice%\"==\"y\" (\n    del \"%0\"\n)\n",
    );
    std::fs::write(install_dir.join("Remove OptiScaler.bat"), script)
}

/// Default OptiScaler.ini written when no config exists. Same content as the
/// Python `create_default_config`.
pub fn create_default_config(install_dir: &Path, gpu_type: &str) -> std::io::Result<()> {
    let content = format!(
        "[OptiScaler]\n; OptiScaler Configuration File\n; Generated by OptiScaler-GUI\n\n\
[GPU]\n; GPU type detection - auto, nvidia, amd, intel\nGPUType={gpu_type}\n\n\
[DLSS]\n; Enable DLSS support\nEnabled=auto\n; Path to DLSS library  \nLibraryPath=auto\n\
; DLSS feature path\nFeaturePath=auto\n; Path to NVNGX DLSS library\nNVNGX_DLSS_Path=auto\n\n\
[XeSS]\n; Enable Intel XeSS support\nEnabled=auto\n; XeSS library path\nLibraryPath=auto\n\
; XeSS DirectX 11 library path\nDx11LibraryPath=auto\n\n\
[FSR]\n; Enable AMD FSR support\nEnabled=auto\n\n\
[Spoofing]\n; Enable DXGI spoofing for AMD/Intel GPUs\nDxgi=auto\n; Enable Streamline spoofing\nStreamline=auto\n\n\
[Log]\n; Logging level (0=Error, 1=Warn, 2=Info, 3=Debug)\nLogLevel=2\n; Log to console\nLogToConsole=false\n\
; Log to file\nLogToFile=true\n; Log file name\nLogFile=auto\n\n\
[Overlay]\n; Enable performance overlay\nEnabled=true\n\n\
[Hotkeys]\n; Key to toggle overlay (Insert key)\nToggleOverlay=VK_INSERT\n"
    );
    std::fs::write(install_dir.join("OptiScaler.ini"), content)
}

/// Remove files/dirs copied during a failed install (path-traversal guarded).
pub fn rollback(install_dir: &Path, files: &[String], directories: &[String]) {
    let root = install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.to_path_buf());
    for rel in files {
        let path = install_dir.join(rel);
        if path_within(&path, &root) && path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
    let mut dirs: Vec<&String> = directories.iter().collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    for rel in dirs {
        let path = install_dir.join(rel);
        if path_within(&path, &root) && path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

pub(crate) fn path_within(path: &Path, root: &Path) -> bool {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    resolved
        .to_string_lossy()
        .to_lowercase()
        .starts_with(&root.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn install_dir_prefers_unreal_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Game");
        fs::create_dir_all(game.join("Engine").join("Binaries").join("Win64")).unwrap();
        assert!(determine_install_directory(&game).ends_with("Win64"));
        let plain = tmp.path().join("Plain");
        fs::create_dir_all(&plain).unwrap();
        assert_eq!(determine_install_directory(&plain), plain);
    }

    #[test]
    fn payload_copy_skips_markers_and_main_dll() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("extracted");
        let dst = tmp.path().join("game");
        fs::create_dir_all(src.join("Licenses")).unwrap();
        fs::create_dir_all(&dst).unwrap();
        for name in [
            "OptiScaler.dll",
            "!! README_EXTRACT ALL FILES TO GAME FOLDER !!.txt",
            "fakenvapi.dll",
            "OptiScaler.ini",
        ] {
            File::create(src.join(name)).unwrap();
        }
        File::create(src.join("Licenses").join("LICENSE.txt")).unwrap();

        let payload = copy_release_payload(&src, &dst, |_, _| {}).unwrap();
        assert!(payload.files.contains(&"fakenvapi.dll".to_string()));
        assert!(payload.files.contains(&"Licenses/LICENSE.txt".to_string()));
        assert!(!payload.files.iter().any(|f| f.contains("OptiScaler.dll")));
        assert!(!payload.files.iter().any(|f| f.starts_with("!!")));
        assert_eq!(payload.directories, vec!["Licenses"]);
        assert!(dst.join("fakenvapi.dll").exists());
        assert!(!dst.join("OptiScaler.dll").exists());
    }

    #[test]
    fn stale_legacy_cleanup_preserves_target() {
        let tmp = tempfile::tempdir().unwrap();
        File::create(tmp.path().join("nvapi64.dll")).unwrap();
        File::create(tmp.path().join("nvngx.dll")).unwrap();
        remove_stale_legacy_files(tmp.path(), "nvngx.dll");
        assert!(!tmp.path().join("nvapi64.dll").exists());
        assert!(tmp.path().join("nvngx.dll").exists()); // chosen proxy preserved
    }

    #[test]
    fn config_backup_and_rollback() {
        let tmp = tempfile::tempdir().unwrap();
        let mut f = File::create(tmp.path().join("OptiScaler.ini")).unwrap();
        f.write_all(b"[X]\nk=v\n").unwrap();
        drop(f);
        let backup = backup_existing_config(tmp.path(), "20260712-120000").unwrap();
        assert!(backup.exists());

        File::create(tmp.path().join("some.dll")).unwrap();
        fs::create_dir_all(tmp.path().join("Licenses")).unwrap();
        rollback(tmp.path(), &["some.dll".into()], &["Licenses".into()]);
        assert!(!tmp.path().join("some.dll").exists());
        assert!(!tmp.path().join("Licenses").exists());
        assert!(backup.exists()); // backups survive rollback
    }

    #[test]
    fn finds_optiscaler_dll_in_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("nested");
        fs::create_dir_all(&sub).unwrap();
        File::create(sub.join("OptiScaler.dll")).unwrap();
        let found = find_optiscaler_dll(tmp.path()).unwrap();
        assert!(found.ends_with(Path::new("nested").join("OptiScaler.dll")));
    }
}
