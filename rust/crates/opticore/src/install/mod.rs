//! Install / update / uninstall orchestration: official download and extraction,
//! optional pinned community runtime, then checked transactional ownership.
//! Legacy manifests remain readable but cannot authorize destructive mutations.

pub mod github;
pub mod manifest;
pub mod payload;
pub mod rdna2;
pub mod transaction;

use crate::archive;
use github::ReleaseInfo;
use manifest::InstallManifest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const CACHE_INVENTORY: &str = ".optiscaler-cache-inventory.json";

#[derive(Serialize, Deserialize)]
struct CacheInventory {
    archive_sha256: String,
    files: std::collections::BTreeMap<String, String>,
}

fn file_sha256(path: &Path) -> Result<String, InstallError> {
    let mut file = std::fs::File::open(path).map_err(|e| InstallError::Io(e.to_string()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| InstallError::Io(e.to_string()))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(crate) fn reject_reparse(path: &Path) -> Result<(), InstallError> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| InstallError::Io(e.to_string()))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(InstallError::Extraction(
                "cache contains a reparse point".into(),
            ));
        }
    }
    #[cfg(not(windows))]
    if meta.file_type().is_symlink() {
        return Err(InstallError::Extraction("cache contains a symlink".into()));
    }
    Ok(())
}

fn cache_lock(path: &Path) -> Result<std::fs::File, InstallError> {
    if std::fs::symlink_metadata(path).is_ok() {
        reject_reparse(path)?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| InstallError::Io(e.to_string()))?;
    file.lock().map_err(|e| InstallError::Io(e.to_string()))?;
    Ok(file)
}

fn payload_inventory(
    root: &Path,
) -> Result<std::collections::BTreeMap<String, String>, InstallError> {
    fn visit(
        root: &Path,
        dir: &Path,
        files: &mut std::collections::BTreeMap<String, String>,
    ) -> Result<(), InstallError> {
        for entry in std::fs::read_dir(dir).map_err(|e| InstallError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| InstallError::Io(e.to_string()))?;
            let path = entry.path();
            reject_reparse(&path)?;
            let kind = entry
                .file_type()
                .map_err(|e| InstallError::Io(e.to_string()))?;
            if kind.is_dir() {
                visit(root, &path, files)?;
            } else if kind.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel != CACHE_INVENTORY {
                    files.insert(rel, file_sha256(&path)?);
                }
            } else {
                return Err(InstallError::Extraction(
                    "release contains a non-regular file".into(),
                ));
            }
        }
        Ok(())
    }
    let mut files = std::collections::BTreeMap::new();
    reject_reparse(root)?;
    visit(root, root, &mut files)?;
    Ok(files)
}

fn prepare_extraction(
    archive_path: &Path,
    extract_dir: &Path,
    extract: impl Fn(&Path) -> Result<(), InstallError>,
) -> Result<PathBuf, InstallError> {
    let parent = extract_dir
        .parent()
        .ok_or_else(|| InstallError::Io("cache has no parent".into()))?;
    std::fs::create_dir_all(parent).map_err(|e| InstallError::Io(e.to_string()))?;
    reject_reparse(parent)?;
    let lock_path = parent.join(format!(
        "{}.lock",
        extract_dir.file_name().unwrap().to_string_lossy()
    ));
    let _lock = cache_lock(&lock_path)?;
    reject_reparse(archive_path)?;
    let archive_sha256 = file_sha256(archive_path)?;
    if std::fs::symlink_metadata(extract_dir).is_ok() {
        reject_reparse(extract_dir)?;
    }
    if std::fs::symlink_metadata(extract_dir.join(CACHE_INVENTORY)).is_ok() {
        reject_reparse(&extract_dir.join(CACHE_INVENTORY))?;
    }
    let existing_files = if extract_dir.exists() {
        Some(payload_inventory(extract_dir)?)
    } else {
        None
    };
    if let Ok(bytes) = std::fs::read(extract_dir.join(CACHE_INVENTORY)) {
        if let Ok(saved) = serde_json::from_slice::<CacheInventory>(&bytes) {
            if saved.archive_sha256 == archive_sha256
                && saved.files.contains_key("OptiScaler.dll")
                && existing_files.as_ref() == Some(&saved.files)
            {
                return Ok(extract_dir.to_path_buf());
            }
        }
    }

    let stage = tempfile::Builder::new()
        .prefix(".extract-")
        .tempdir_in(parent)
        .map_err(|e| InstallError::Io(e.to_string()))?;
    extract(stage.path())?;
    let files = payload_inventory(stage.path())?;
    if !files.contains_key("OptiScaler.dll") {
        return Err(InstallError::DllNotFound);
    }
    let inventory = CacheInventory {
        archive_sha256,
        files,
    };
    let bytes = serde_json::to_vec(&inventory).map_err(|e| InstallError::Io(e.to_string()))?;
    std::fs::write(stage.path().join(CACHE_INVENTORY), bytes)
        .map_err(|e| InstallError::Io(e.to_string()))?;

    publish_stage(stage, extract_dir, |_, _| {})?;
    Ok(extract_dir.to_path_buf())
}

fn publish_stage(
    stage: tempfile::TempDir,
    extract_dir: &Path,
    after_backup: impl FnOnce(&Path, &Path),
) -> Result<(), InstallError> {
    let parent = extract_dir.parent().unwrap();
    let backup = tempfile::Builder::new()
        .prefix(".replaced-")
        .tempdir_in(parent)
        .map_err(|e| InstallError::Io(e.to_string()))?;
    let old = backup.path().join("old");
    let had_old = std::fs::symlink_metadata(extract_dir).is_ok();
    if had_old {
        std::fs::rename(extract_dir, &old).map_err(|e| InstallError::Io(e.to_string()))?;
    }
    after_backup(&old, extract_dir);
    if let Err(error) = std::fs::rename(stage.path(), extract_dir) {
        if had_old {
            if let Err(restore_error) = std::fs::rename(&old, extract_dir) {
                let preserved = backup.keep();
                return Err(InstallError::Io(format!(
                    "cache publication failed: {error}; restore failed: {restore_error}; previous cache preserved at {}",
                    preserved.display()
                )));
            }
        }
        return Err(InstallError::Io(error.to_string()));
    }
    Ok(())
}

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
    /// Explicit community consent; never set by a recommendation.
    pub community_rdna2: bool,
    /// Explicit per-game rendering adapter from the current hardware snapshot.
    pub rendering_gpu: Option<crate::hardware::GpuProfile>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            target_filename: "dxgi.dll".to_string(),
            overwrite: false,
            confirmed_target: None,
            gpu_type: "auto".to_string(),
            dlss_inputs: true,
            community_rdna2: false,
            rendering_gpu: None,
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
    pinned_release: Option<ReleaseInfo>,
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
            pinned_release: None,
        }
    }

    /// Test a specific upstream release using the same upgrade path as the GUI.
    pub fn at_release(download_dir: &Path, release: ReleaseInfo) -> Self {
        Self {
            pinned_release: Some(release),
            ..Self::new(download_dir)
        }
    }

    /// Download (or reuse) the selected release archive, latest by default.
    /// Returns (extracted payload dir, release info).
    pub fn prepare_payload(
        &self,
        mut progress: impl FnMut(InstallStage),
    ) -> Result<(PathBuf, ReleaseInfo), InstallError> {
        let release = match &self.pinned_release {
            Some(release) => release.clone(),
            None => {
                progress(InstallStage::FetchingRelease);
                github::fetch_latest_release().map_err(InstallError::Download)?
            }
        };
        let extract_dir = self.prepare_release(&release, progress)?;
        Ok((extract_dir, release))
    }

    /// Prepare a specific official release and return its verified payload directory.
    pub fn prepare_release(
        &self,
        release: &github::ReleaseInfo,
        mut progress: impl FnMut(InstallStage),
    ) -> Result<PathBuf, InstallError> {
        let asset = release.archive_asset().ok_or(InstallError::Download(
            github::DownloadError::NoArchiveAsset,
        ))?;
        if Path::new(&asset.name)
            .file_name()
            .is_none_or(|name| name != asset.name.as_str())
        {
            return Err(InstallError::Extraction("unsafe archive asset name".into()));
        }
        std::fs::create_dir_all(&self.download_dir).map_err(|e| InstallError::Io(e.to_string()))?;
        reject_reparse(&self.download_dir)?;
        let _download_lock = cache_lock(
            &self
                .download_dir
                .join(format!("{}.download.lock", asset.name)),
        )?;
        let archive_file = self.download_dir.join(&asset.name);
        if std::fs::symlink_metadata(&archive_file).is_ok() {
            reject_reparse(&archive_file)?;
        }
        let archive_path = github::download_archive(release, &self.download_dir, |done, total| {
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
        let payload_dir = prepare_extraction(&archive_path, &extract_dir, |stage| {
            archive::extract_7z(&archive_path, stage)
                .map_err(|e| InstallError::Extraction(e.to_string()))
        })?;
        if let Some(tag) = release.tag_name.as_deref() {
            let _ = crate::release_defaults::remember(&self.download_dir, tag, &payload_dir);
        }
        Ok(payload_dir)
    }

    /// Full install into a game directory. Port of `install_optiscaler`.
    pub fn install(
        &self,
        game_path: &Path,
        options: &InstallOptions,
        mut progress: impl FnMut(InstallStage),
    ) -> Result<InstallManifest, InstallError> {
        if options.community_rdna2 && !rdna2::eligible(options.rendering_gpu.as_ref(), true) {
            return Err(InstallError::Io(
                "Choose the RX 6000 rendering GPU explicitly before opting in".into(),
            ));
        }
        let runtime = if options.community_rdna2 {
            Some(rdna2::prepare(&self.download_dir)?)
        } else {
            None
        };
        let (extracted, release) = self.prepare_payload(&mut progress)?;
        let dest_dir = confirmed_directory(game_path, options)?;
        transaction::install_with_runtime(
            &dest_dir,
            &extracted,
            options,
            &release.version_label(),
            release.html_url.clone(),
            iso_now(),
            runtime.as_deref(),
            progress,
        )
    }
}

/// Uninstall only with a complete owned v2 manifest; preserve legacy/foreign files.
/// Returns the removed files/dirs.
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
            community_rdna2: false,
            rendering_gpu: None,
        };
        let directory = crate::resolver::resolve(game_path)
            .map_err(InstallError::Io)?
            .directory;
        if manifest::read(&directory).is_some_and(|m| m.extra.contains_key("fsr_runtime")) {
            return Err(InstallError::Io("Community runtime requires a new explicit GPU choice and consent; use manual update".into()));
        }
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

    fn cache_fixture(root: &Path, extract: impl Fn(&Path) -> Result<(), InstallError>) -> PathBuf {
        let archive = root.join("release.7z");
        fs::write(&archive, b"archive identity").unwrap();
        prepare_extraction(&archive, &root.join("extracted/release"), extract).unwrap()
    }

    fn write_fixture_payload(dir: &Path) -> Result<(), InstallError> {
        fs::create_dir_all(dir).map_err(|e| InstallError::Io(e.to_string()))?;
        fs::write(dir.join("OptiScaler.dll"), b"main").unwrap();
        fs::write(dir.join("amd_fidelityfx_vk.dll"), b"vulkan").unwrap();
        Ok(())
    }

    #[test]
    fn extraction_cache_repairs_removed_and_modified_payload() {
        let tmp = tempfile::tempdir().unwrap();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let extract = |dir: &Path| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            write_fixture_payload(dir)
        };
        let cache = cache_fixture(tmp.path(), extract);
        cache_fixture(tmp.path(), extract);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        fs::remove_file(cache.join("amd_fidelityfx_vk.dll")).unwrap();
        cache_fixture(tmp.path(), extract);
        assert_eq!(
            fs::read(cache.join("amd_fidelityfx_vk.dll")).unwrap(),
            b"vulkan"
        );
        fs::write(cache.join("amd_fidelityfx_vk.dll"), b"changed").unwrap();
        cache_fixture(tmp.path(), extract);
        assert_eq!(
            fs::read(cache.join("amd_fidelityfx_vk.dll")).unwrap(),
            b"vulkan"
        );
        fs::write(cache.join("OptiScaler.dll"), b"changed").unwrap();
        cache_fixture(tmp.path(), extract);
        assert_eq!(fs::read(cache.join("OptiScaler.dll")).unwrap(), b"main");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 4);
    }

    #[test]
    fn failed_rebuild_does_not_publish_partial_payload() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = cache_fixture(tmp.path(), write_fixture_payload);
        fs::remove_file(cache.join("amd_fidelityfx_vk.dll")).unwrap();
        let archive = tmp.path().join("release.7z");
        let result = prepare_extraction(&archive, &cache, |stage| {
            fs::write(stage.join("partial.dll"), b"partial").unwrap();
            Err(InstallError::Extraction("rejected".into()))
        });
        assert!(result.is_err());
        assert!(!cache.join("partial.dll").exists());
        assert!(cache.join("OptiScaler.dll").exists());
    }

    #[test]
    fn failed_restore_preserves_the_only_old_cache_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("release");
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("sentinel"), b"old").unwrap();
        let stage = tempfile::tempdir_in(tmp.path()).unwrap();
        fs::write(stage.path().join("OptiScaler.dll"), b"new").unwrap();
        let error = publish_stage(stage, &cache, |_, target| {
            fs::create_dir(target).unwrap();
            fs::write(target.join("blocker"), b"foreign").unwrap();
        })
        .unwrap_err();
        assert!(error.to_string().contains("previous cache preserved at"));
        let backups: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".replaced-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(backups[0].join("old/sentinel")).unwrap(), b"old");
    }

    #[test]
    fn rejected_payload_does_not_create_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("release.7z");
        fs::write(&archive, b"archive").unwrap();
        let cache = tmp.path().join("extracted/release");
        let result = prepare_extraction(&archive, &cache, |stage| {
            fs::write(stage.join("amd_fidelityfx_vk.dll"), b"partial").unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(InstallError::DllNotFound)));
        assert!(!cache.exists());
    }

    #[test]
    fn concurrent_preparation_extracts_once() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let archive = root.join("release.7z");
        fs::write(&archive, b"archive identity").unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut threads = Vec::new();
        for _ in 0..4 {
            let root = root.clone();
            let calls = calls.clone();
            threads.push(std::thread::spawn(move || {
                prepare_extraction(
                    &root.join("release.7z"),
                    &root.join("extracted/release"),
                    |dir| {
                        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        write_fixture_payload(dir)
                    },
                )
                .unwrap()
            }));
        }
        let paths: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(paths.iter().all(|path| path == &paths[0]));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[cfg(windows)]
    #[test]
    fn reparse_cache_root_and_child_never_touch_foreign_tree() {
        fn junction(link: &Path, target: &Path) {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .status()
                .unwrap();
            assert!(status.success());
        }
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("release.7z");
        fs::write(&archive, b"archive").unwrap();
        let outside = tmp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("sentinel.txt"), b"keep").unwrap();
        let parent = tmp.path().join("extracted");
        fs::create_dir(&parent).unwrap();
        let cache = parent.join("release");
        junction(&cache, &outside);
        assert!(prepare_extraction(&archive, &cache, write_fixture_payload).is_err());
        fs::remove_dir(&cache).unwrap();
        fs::create_dir(&cache).unwrap();
        junction(&cache.join("nested"), &outside);
        assert!(prepare_extraction(&archive, &cache, write_fixture_payload).is_err());
        assert_eq!(fs::read(outside.join("sentinel.txt")).unwrap(), b"keep");
    }

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
