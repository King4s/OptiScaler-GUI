//! Install / update / uninstall orchestration. Port of the Python
//! `OptiScalerManager` flow: download → verify → extract → payload copy →
//! uninstaller + config + manifest, with rollback on failure.

pub mod github;
pub mod manifest;
pub mod payload;

use crate::archive;
use github::ReleaseInfo;
use manifest::InstallManifest;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub target_filename: String,
    pub overwrite: bool,
    /// GPU type written to a freshly created OptiScaler.ini ("auto"/"nvidia"/"amd"/"intel")
    pub gpu_type: String,
    /// v0.7.9+ DLSS-inputs semantics: when the user answers "No" on an
    /// AMD/Intel setup, only Dxgi=false is written to the config.
    pub dlss_inputs: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            target_filename: "dxgi.dll".to_string(),
            overwrite: false,
            gpu_type: "auto".to_string(),
            dlss_inputs: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InstallStage {
    FetchingRelease,
    Downloading { done: u64, total: u64 },
    Extracting,
    CopyingPayload { done: usize, total: usize },
    Finalizing,
}

#[derive(Debug)]
pub enum InstallError {
    Download(github::DownloadError),
    Extraction(String),
    TargetExists(String),
    DllNotFound,
    Io(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Download(e) => write!(f, "download failed: {e}"),
            InstallError::Extraction(e) => write!(f, "extraction failed: {e}"),
            InstallError::TargetExists(name) => {
                write!(
                    f,
                    "target file {name} already exists (enable overwrite to update)"
                )
            }
            InstallError::DllNotFound => {
                write!(f, "OptiScaler.dll not found in the release payload")
            }
            InstallError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for InstallError {}

/// ISO-8601 local timestamp like Python's datetime.now().isoformat().
fn iso_now() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]"
    );
    now.format(&format)
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000000".to_string())
}

fn compact_now() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year][month][day]-[hour][minute][second]");
    now.format(&format)
        .unwrap_or_else(|_| "00000000-000000".to_string())
}

pub struct Installer {
    pub download_dir: PathBuf,
}

impl Installer {
    pub fn new(download_dir: &Path) -> Self {
        Self {
            download_dir: download_dir.to_path_buf(),
        }
    }

    /// Download (or reuse) the latest release archive and extract it.
    /// Returns (extracted payload dir, release info).
    pub fn prepare_payload(
        &self,
        mut progress: impl FnMut(InstallStage),
    ) -> Result<(PathBuf, ReleaseInfo), InstallError> {
        progress(InstallStage::FetchingRelease);
        let release = github::fetch_latest_release().map_err(InstallError::Download)?;
        let archive_path = github::download_archive(&release, &self.download_dir, |done, total| {
            progress(InstallStage::Downloading { done, total })
        })
        .map_err(InstallError::Download)?;

        progress(InstallStage::Extracting);
        let extract_dir = self.download_dir.join("extracted").join(
            archive_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        // Fresh extraction dir per archive version; reuse if already populated
        if !extract_dir.join("OptiScaler.dll").exists() {
            std::fs::create_dir_all(&extract_dir).map_err(|e| InstallError::Io(e.to_string()))?;
            let absolute = extract_dir
                .canonicalize()
                .map_err(|e| InstallError::Io(e.to_string()))?;
            archive::extract_7z(&archive_path, &absolute)
                .map_err(|e| InstallError::Extraction(e.to_string()))?;
        }
        Ok((extract_dir, release))
    }

    /// Full install into a game directory. Port of `install_optiscaler`.
    pub fn install(
        &self,
        game_path: &Path,
        options: &InstallOptions,
        mut progress: impl FnMut(InstallStage),
    ) -> Result<InstallManifest, InstallError> {
        let (extracted, release) = self.prepare_payload(&mut progress)?;
        payload::remove_setup_markers(&extracted);

        let dest_dir = payload::determine_install_directory(game_path);
        std::fs::create_dir_all(&dest_dir).map_err(|e| InstallError::Io(e.to_string()))?;

        let target_path = dest_dir.join(&options.target_filename);
        if target_path.exists() {
            if !options.overwrite {
                return Err(InstallError::TargetExists(options.target_filename.clone()));
            }
            std::fs::remove_file(&target_path).map_err(|e| InstallError::Io(e.to_string()))?;
        }
        if options.overwrite {
            payload::remove_stale_legacy_files(&dest_dir, &options.target_filename);
            payload::backup_existing_config(&dest_dir, &compact_now());
        }

        let mut op_files: Vec<String> = Vec::new();
        let mut op_dirs: Vec<String> = Vec::new();
        let result = (|| {
            let dll = payload::find_optiscaler_dll(&extracted).ok_or(InstallError::DllNotFound)?;
            std::fs::copy(&dll, &target_path).map_err(|e| InstallError::Io(e.to_string()))?;
            op_files.push(options.target_filename.clone());

            let copied = payload::copy_release_payload(&extracted, &dest_dir, |done, total| {
                progress(InstallStage::CopyingPayload { done, total })
            })
            .map_err(|e| InstallError::Io(e.to_string()))?;
            op_files.extend(copied.files.iter().cloned());
            op_dirs.extend(copied.directories.iter().cloned());

            progress(InstallStage::Finalizing);
            payload::create_uninstaller_script(&dest_dir, &op_files)
                .map_err(|e| InstallError::Io(e.to_string()))?;
            op_files.push("Remove OptiScaler.bat".to_string());

            if !dest_dir.join("OptiScaler.ini").exists() {
                payload::create_default_config(&dest_dir, &options.gpu_type)
                    .map_err(|e| InstallError::Io(e.to_string()))?;
                op_files.push("OptiScaler.ini".to_string());
            }
            // v0.7.9 DLSS-inputs semantics: "No" → force Dxgi=false in the config
            if !options.dlss_inputs {
                set_ini_value(
                    &dest_dir.join("OptiScaler.ini"),
                    "Spoofing",
                    "Dxgi",
                    "false",
                )
                .map_err(|e| InstallError::Io(e.to_string()))?;
            }

            let manifest = InstallManifest::new(
                &options.target_filename,
                &op_files,
                &op_dirs,
                &release.version_label(),
                release.html_url.clone(),
                iso_now(),
            );
            manifest::write(&dest_dir, &manifest).map_err(|e| InstallError::Io(e.to_string()))?;
            Ok(manifest)
        })();

        if result.is_err() {
            payload::rollback(&dest_dir, &op_files, &op_dirs);
        }
        result
    }
}

/// Uninstall using the manifest when present, else the legacy known-file list.
/// Port of `uninstall_optiscaler`. Returns the removed files/dirs.
pub fn uninstall(game_path: &Path) -> Result<(Vec<String>, Vec<String>), InstallError> {
    let install_dir = payload::determine_install_directory(game_path);
    let root = install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.clone());
    let mut removed_files = Vec::new();
    let mut removed_dirs = Vec::new();

    if let Some(m) = manifest::read(&install_dir) {
        let mut files: Vec<String> = m.files.clone();
        files.extend([
            manifest::MANIFEST_FILENAME.to_string(),
            "Remove OptiScaler.bat".to_string(),
            "OptiScaler.log".to_string(),
        ]);
        files.sort();
        files.dedup();
        for rel in &files {
            let path = install_dir.join(rel);
            if payload::path_within(&path, &root)
                && path.is_file()
                && std::fs::remove_file(&path).is_ok()
            {
                removed_files.push(rel.clone());
            }
        }
        let mut dirs = m.directories.clone();
        dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
        for rel in &dirs {
            let path = install_dir.join(rel);
            if payload::path_within(&path, &root)
                && path.is_dir()
                && std::fs::remove_dir_all(&path).is_ok()
            {
                removed_dirs.push(rel.clone());
            }
        }
        if !removed_files.is_empty() || !removed_dirs.is_empty() {
            return Ok((removed_files, removed_dirs));
        }
    }

    // Legacy fallback: known file list + proxy names
    let mut legacy: Vec<&str> = payload::LEGACY_UNINSTALL_FILES.to_vec();
    legacy.extend(payload::PROXY_FILENAMES);
    legacy.sort();
    legacy.dedup();
    for filename in legacy {
        let path = install_dir.join(filename);
        if path.is_file() && std::fs::remove_file(&path).is_ok() {
            removed_files.push(filename.to_string());
        }
    }
    for dirname in payload::LEGACY_UNINSTALL_DIRS {
        let path = install_dir.join(dirname);
        if path.is_dir() && std::fs::remove_dir_all(&path).is_ok() {
            removed_dirs.push(dirname.to_string());
        }
    }
    if removed_files.is_empty() && removed_dirs.is_empty() {
        return Err(InstallError::Io(
            "no OptiScaler files found to remove".into(),
        ));
    }
    Ok((removed_files, removed_dirs))
}

/// Version recorded in an existing install's manifest, if any.
pub fn installed_version(game_path: &Path) -> Option<String> {
    let install_dir = payload::determine_install_directory(game_path);
    manifest::read(&install_dir).map(|m| m.optiscaler_version)
}

/// Compare an installed version against the latest release tag.
/// Tags are "v0.9.3"-style; compares numeric components, tolerant of suffixes.
pub fn is_update_available(installed: &str, latest: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.trim_start_matches(['v', 'V'])
            .split(['.', '-', 'a', 'b'])
            .map_while(|p| p.parse::<u64>().ok())
            .collect()
    }
    let installed_parts = parts(installed);
    let latest_parts = parts(latest);
    if installed_parts.is_empty() || latest_parts.is_empty() {
        // Unknown versions ("Unknown") — offer update only if labels differ
        return !installed.eq_ignore_ascii_case(latest);
    }
    latest_parts > installed_parts
}

impl Installer {
    /// Update an existing install: overwrite with the recorded proxy filename
    /// (falls back to the given default), config backup + stale cleanup happen
    /// inside install(). Port of the Python update flow.
    pub fn update(
        &self,
        game_path: &Path,
        gpu_type: &str,
        progress: impl FnMut(InstallStage),
    ) -> Result<InstallManifest, InstallError> {
        let target = installed_target_filename(game_path).unwrap_or_else(|| "dxgi.dll".to_string());
        let options = InstallOptions {
            target_filename: target,
            overwrite: true,
            gpu_type: gpu_type.to_string(),
            dlss_inputs: true,
        };
        self.install(game_path, &options, progress)
    }
}

/// Proxy filename recorded for an existing install (manifest first, then
/// probing known proxy names). Port of `get_installed_target_filename`.
pub fn installed_target_filename(game_path: &Path) -> Option<String> {
    let install_dir = payload::determine_install_directory(game_path);
    if let Some(m) = manifest::read(&install_dir) {
        if !m.target_filename.is_empty() {
            return Some(m.target_filename);
        }
    }
    payload::PROXY_FILENAMES
        .iter()
        .find(|f| install_dir.join(f).exists())
        .map(|f| f.to_string())
}

/// Minimal INI value setter preserving all other lines/comments.
fn set_ini_value(ini_path: &Path, section: &str, key: &str, value: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(ini_path).unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut replaced = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_section && !replaced {
                out.push(format!("{key}={value}"));
                replaced = true;
            }
            in_section = trimmed[1..trimmed.len() - 1].eq_ignore_ascii_case(section);
            out.push(line.to_string());
            continue;
        }
        if in_section && !replaced {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    out.push(format!("{key}={value}"));
                    replaced = true;
                    continue;
                }
            }
        }
        out.push(line.to_string());
    }
    if !replaced {
        if !in_section {
            out.push(format!("[{section}]"));
        }
        out.push(format!("{key}={value}"));
    }
    std::fs::write(ini_path, out.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn uninstall_follows_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        for f in ["dxgi.dll", "fakenvapi.dll", "Remove OptiScaler.bat"] {
            File::create(game.join(f)).unwrap();
        }
        fs::create_dir_all(game.join("Licenses")).unwrap();
        File::create(game.join("Licenses").join("L.txt")).unwrap();
        File::create(game.join("unrelated.txt")).unwrap();

        let m = InstallManifest::new(
            "dxgi.dll",
            &[
                "dxgi.dll".into(),
                "fakenvapi.dll".into(),
                "Licenses/L.txt".into(),
            ],
            &["Licenses".into()],
            "v0.9.3",
            None,
            "2026-07-12T12:00:00".into(),
        );
        manifest::write(game, &m).unwrap();

        let (files, dirs) = uninstall(game).unwrap();
        assert!(files.contains(&"dxgi.dll".to_string()));
        assert!(dirs.contains(&"Licenses".to_string()));
        assert!(game.join("unrelated.txt").exists()); // untouched
        assert!(!game.join(manifest::MANIFEST_FILENAME).exists());
        assert!(!game.join("Licenses").exists());
    }

    #[test]
    fn uninstall_python_made_install() {
        // A manifest written by the actual Python app must drive uninstall
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        let python_manifest = include_str!("../../tests/fixtures/python_manifest.json");
        fs::write(game.join(manifest::MANIFEST_FILENAME), python_manifest).unwrap();
        fs::create_dir_all(game.join("D3D12_Optiscaler")).unwrap();
        for f in [
            "dxgi.dll",
            "OptiScaler.ini",
            "fakenvapi.dll",
            "Remove OptiScaler.bat",
        ] {
            File::create(game.join(f)).unwrap();
        }
        File::create(game.join("D3D12_Optiscaler").join("plugin.dll")).unwrap();

        let (files, dirs) = uninstall(game).unwrap();
        assert!(files.contains(&"dxgi.dll".to_string()));
        assert!(files.contains(&"D3D12_Optiscaler/plugin.dll".to_string()));
        assert!(dirs.contains(&"D3D12_Optiscaler".to_string()));
        assert!(!game.join("dxgi.dll").exists());
    }

    #[test]
    fn uninstall_legacy_without_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        for f in ["dxgi.dll", "OptiScaler.ini", "libxess.dll"] {
            File::create(game.join(f)).unwrap();
        }
        fs::create_dir_all(game.join("D3D12_Optiscaler")).unwrap();
        let (files, dirs) = uninstall(game).unwrap();
        assert!(files.contains(&"dxgi.dll".to_string()));
        assert!(files.contains(&"libxess.dll".to_string()));
        assert!(dirs.contains(&"D3D12_Optiscaler".to_string()));
    }

    #[test]
    fn installed_target_from_manifest_and_probe() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        assert_eq!(installed_target_filename(game), None);
        File::create(game.join("winmm.dll")).unwrap();
        assert_eq!(
            installed_target_filename(game).as_deref(),
            Some("winmm.dll")
        );
        let m = InstallManifest::new(
            "d3d12.dll",
            &["d3d12.dll".into()],
            &[],
            "v0.9.3",
            None,
            "t".into(),
        );
        manifest::write(game, &m).unwrap();
        assert_eq!(
            installed_target_filename(game).as_deref(),
            Some("d3d12.dll")
        );
    }

    #[test]
    fn update_availability_comparison() {
        assert!(is_update_available("v0.9.2", "v0.9.3"));
        assert!(is_update_available("v0.9.3", "v0.10.0"));
        assert!(!is_update_available("v0.9.3", "v0.9.3"));
        assert!(!is_update_available("v0.10.0", "v0.9.3"));
        // unknown installed version: differ → update offered
        assert!(is_update_available("Unknown", "v0.9.3"));
        assert!(!is_update_available("Unknown", "Unknown"));
    }

    #[test]
    fn ini_value_setter_preserves_content() {
        let tmp = tempfile::tempdir().unwrap();
        let ini = tmp.path().join("OptiScaler.ini");
        let mut f = File::create(&ini).unwrap();
        f.write_all(b"[Spoofing]\n; comment\nDxgi=auto\nStreamline=auto\n")
            .unwrap();
        drop(f);
        set_ini_value(&ini, "Spoofing", "Dxgi", "false").unwrap();
        let content = fs::read_to_string(&ini).unwrap();
        assert!(content.contains("Dxgi=false"));
        assert!(content.contains("; comment"));
        assert!(content.contains("Streamline=auto"));
    }
}
