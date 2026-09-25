//! Install / update / uninstall orchestration. Port of the Python
//! `OptiScalerManager` flow: download → verify → extract → payload copy →
//! uninstaller + config + manifest, with rollback on failure.

pub mod github;
pub mod manifest;
pub mod payload;
pub mod transaction;

use crate::archive;
use github::ReleaseInfo;
use manifest::InstallManifest;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub target_filename: String,
    pub overwrite: bool,
    /// The directory and EXE the user approved before a potentially long download.
    pub confirmed_target: Option<crate::resolver::InstallTarget>,
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
            confirmed_target: None,
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

pub struct Installer {
    pub download_dir: PathBuf,
}

fn confirmed_directory(
    game_path: &Path,
    options: &InstallOptions,
) -> Result<PathBuf, InstallError> {
    let resolved = crate::resolver::resolve(game_path).map_err(InstallError::Io)?;
    if options.confirmed_target.as_ref().is_some_and(|confirmed| {
        confirmed.executable != resolved.executable || confirmed.directory != resolved.directory
    }) {
        return Err(InstallError::Io(
            "installation target changed after confirmation; select it again".into(),
        ));
    }
    Ok(resolved.directory)
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
        let dest_dir = confirmed_directory(game_path, options)?;
        transaction::install(
            &dest_dir,
            &extracted,
            options,
            &release.version_label(),
            release.html_url.clone(),
            iso_now(),
            progress,
        )
    }
}

/// Uninstall using the manifest when present, else the legacy known-file list.
/// Port of `uninstall_optiscaler`. Returns the removed files/dirs.
pub fn uninstall(game_path: &Path) -> Result<(Vec<String>, Vec<String>), InstallError> {
    let install_dir = crate::resolver::resolve(game_path)
        .map_err(InstallError::Io)?
        .directory;
    transaction::uninstall(&install_dir)
}

/// Version recorded in an existing install's manifest, if any.
pub fn installed_version(game_path: &Path) -> Option<String> {
    let install_dir = crate::resolver::resolve(game_path).ok()?.directory;
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
        let options = InstallOptions {
            target_filename: update_target_filename(game_path),
            overwrite: true,
            confirmed_target: None,
            gpu_type: gpu_type.to_string(),
            dlss_inputs: true,
        };
        self.install(game_path, &options, progress)
    }
}

/// Proxy filename an update should install with: the recorded one — except
/// legacy names upstream no longer supports (nvngx.dll), which migrate to
/// dxgi.dll; the stale-legacy cleanup inside install() removes the old file.
pub fn update_target_filename(game_path: &Path) -> String {
    installed_target_filename(game_path)
        .filter(|t| !payload::LEGACY_PROXY_FILENAMES.contains(&t.as_str()))
        .unwrap_or_else(|| "dxgi.dll".to_string())
}

/// Proxy filename recorded for an existing install (manifest first, then
/// probing known proxy names). Port of `get_installed_target_filename`.
pub fn installed_target_filename(game_path: &Path) -> Option<String> {
    let install_dir = match crate::resolver::resolve(game_path) {
        Ok(t) => t.directory,
        Err(_) => return None,
    };
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
    fn confirmed_target_rejects_changed_executable_choice() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
        let confirmed = crate::resolver::resolve(game).unwrap();
        fs::create_dir_all(game.join("bin/x64")).unwrap();
        let other = game.join("bin/x64/Other.exe");
        File::create(&other).unwrap();
        crate::resolver::remember(game, &other).unwrap();
        let options = InstallOptions {
            confirmed_target: Some(confirmed),
            ..InstallOptions::default()
        };
        assert!(confirmed_directory(game, &options).is_err());
    }

    #[test]
    fn uninstall_follows_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
        for f in ["dxgi.dll", "fakenvapi.dll", "Remove OptiScaler.bat"] {
            File::create(game.join(f)).unwrap();
        }
        fs::create_dir_all(game.join("Licenses")).unwrap();
        File::create(game.join("Licenses").join("L.txt")).unwrap();
        File::create(game.join("unrelated.txt")).unwrap();

        let mut m = InstallManifest::new(
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
        use sha2::{Digest, Sha256};
        m.schema_version = 2;
        for file in &m.files {
            m.owned_files.insert(
                file.clone(),
                manifest::OwnedFile {
                    sha256: hex::encode(Sha256::digest(fs::read(game.join(file)).unwrap())),
                    original_backup: None,
                    original_sha256: None,
                    extra: Default::default(),
                },
            );
        }
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
        // Legacy Python ownership is unknown: preserve every file.
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
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

        assert!(uninstall(game).unwrap_err().to_string().contains("legacy"));
        assert!(game.join("dxgi.dll").exists());
        assert!(game.join("D3D12_Optiscaler/plugin.dll").exists());
        assert_eq!(
            fs::read_to_string(game.join(manifest::MANIFEST_FILENAME)).unwrap(),
            python_manifest
        );
    }

    #[test]
    fn uninstall_legacy_without_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
        for f in ["dxgi.dll", "OptiScaler.ini", "libxess.dll"] {
            File::create(game.join(f)).unwrap();
        }
        fs::create_dir_all(game.join("D3D12_Optiscaler")).unwrap();
        assert!(uninstall(game)
            .unwrap_err()
            .to_string()
            .contains("no owned"));
        for f in [
            "dxgi.dll",
            "OptiScaler.ini",
            "libxess.dll",
            "D3D12_Optiscaler",
        ] {
            assert!(game.join(f).exists());
        }
    }

    #[test]
    fn installed_target_from_manifest_and_probe() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
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
    fn update_migrates_legacy_nvngx_target_to_dxgi() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        File::create(game.join("Game.exe")).unwrap();
        // No install at all → default
        assert_eq!(update_target_filename(game), "dxgi.dll");
        // Supported recorded target is reused as-is
        let m = InstallManifest::new(
            "winmm.dll",
            &["winmm.dll".into()],
            &[],
            "v0.9.2",
            None,
            "t".into(),
        );
        manifest::write(game, &m).unwrap();
        assert_eq!(update_target_filename(game), "winmm.dll");
        // nvngx.dll (unsupported upstream) migrates to the default proxy
        let m = InstallManifest::new(
            "nvngx.dll",
            &["nvngx.dll".into()],
            &[],
            "v0.9.2",
            None,
            "t".into(),
        );
        manifest::write(game, &m).unwrap();
        assert_eq!(update_target_filename(game), "dxgi.dll");
        // A manifest-less nvngx.dll on disk is not treated as the target
        std::fs::remove_file(game.join(manifest::MANIFEST_FILENAME)).unwrap();
        File::create(game.join("nvngx.dll")).unwrap();
        assert_eq!(installed_target_filename(game), None);
        assert_eq!(update_target_filename(game), "dxgi.dll");
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
