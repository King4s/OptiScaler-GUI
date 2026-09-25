//! Checked file mutations for v2 installs. No legacy manifest authorizes deletion.

use super::{manifest, payload, InstallError, InstallOptions, InstallStage};
use manifest::{InstallManifest, OwnedFile};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

struct Lock(PathBuf);
impl Lock {
    fn acquire(root: &Path) -> Result<Self, InstallError> {
        let path = root.join(".optiscaler-gui.lock");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| err(format!("installation is locked: {e}")))?;
        Ok(Self(path))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn err(message: impl Into<String>) -> InstallError {
    InstallError::Io(message.into())
}

fn valid_relative(rel: &str) -> bool {
    if rel.is_empty() || rel.contains('\\') || rel.contains(':') || rel.contains('\0') {
        return false;
    }
    let path = Path::new(rel);
    !path.is_absolute()
        && path.components().all(|c| match c {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
                !s.is_empty()
                    && !s.ends_with('.')
                    && !s.ends_with(' ')
                    && !s.chars().any(|c| c.is_control() || "<>\"|?*".contains(c))
                    && ![
                        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                        "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
                        "LPT7", "LPT8", "LPT9",
                    ]
                    .contains(&stem.as_str())
            }
            _ => false,
        })
        && !rel
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
}

fn checked(root: &Path, rel: &str) -> Result<PathBuf, InstallError> {
    if !valid_relative(rel) {
        return Err(err(format!("unsafe relative path: {rel}")));
    }
    let mut path = root.to_path_buf();
    for component in Path::new(rel).components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if is_link(&meta) {
                    return Err(err(format!("link in install path: {rel}")));
                }
                let actual = path.canonicalize().map_err(|e| err(e.to_string()))?;
                if !actual.starts_with(root) {
                    return Err(err(format!("path leaves install directory: {rel}")));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(err(format!("cannot validate install path {rel}: {e}")));
            }
        }
    }
    if !payload::path_within(&path, root) {
        return Err(err(format!("path leaves install directory: {rel}")));
    }
    Ok(path)
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reject all reparse points, including junctions, not only symbolic links.
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

fn reserved(rel: &str) -> bool {
    let top = rel.split('/').next().unwrap_or("");
    top.eq_ignore_ascii_case(manifest::BACKUP_DIRECTORY)
        || top.eq_ignore_ascii_case(manifest::MANIFEST_FILENAME)
        || top.eq_ignore_ascii_case(".optiscaler-gui.lock")
        || top.eq_ignore_ascii_case(".optiscaler-gui-target.json")
}

fn mutable_config(rel: &str) -> bool {
    rel.eq_ignore_ascii_case("OptiScaler.ini")
}

fn still_owned(root: &Path, rel: &str, owned: &OwnedFile) -> Result<bool, InstallError> {
    let path = checked(root, rel)?;
    Ok(path.is_file() && digest(&path)? == owned.sha256)
}

fn digest(path: &Path) -> Result<String, InstallError> {
    let mut file = File::open(path).map_err(|e| err(e.to_string()))?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let count = file.read(&mut buf).map_err(|e| err(e.to_string()))?;
        if count == 0 {
            break;
        }
        hash.update(&buf[..count]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(""))
}

// Never open an existing destination for writing: it may be a hard link to a
// file outside the game tree, even when the pathname itself is in the tree.
fn replace_file(src: &Path, dst: &Path) -> io::Result<()> {
    if dst.exists() {
        fs::remove_file(dst)?;
    }
    let mut input = File::open(src)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(dst)?;
    if let Err(error) = io::copy(&mut input, &mut output) {
        drop(output);
        let _ = fs::remove_file(dst);
        return Err(error);
    }
    Ok(())
}

fn existing_manifest(root: &Path) -> Result<Option<InstallManifest>, InstallError> {
    let path = checked(root, manifest::MANIFEST_FILENAME)?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| err(e.to_string()))?;
    let m: InstallManifest =
        serde_json::from_str(&text).map_err(|e| err(format!("invalid install manifest: {e}")))?;
    if !m.is_owned_v2() {
        return Err(err(
            "legacy or incomplete manifest cannot authorize update/uninstall",
        ));
    }
    let mut unique = BTreeSet::new();
    for (rel, owned) in &m.owned_files {
        if !unique.insert(rel.to_ascii_lowercase()) {
            return Err(err("manifest contains case-aliased files"));
        }
        if reserved(rel) {
            return Err(err("manifest claims an internal control file"));
        }
        checked(root, rel)?;
        if let Some(backup) = &owned.original_backup {
            if owned.original_sha256.is_none() {
                return Err(err("manifest backup has no original checksum"));
            }
            if !backup.starts_with(&format!("{}/", manifest::BACKUP_DIRECTORY)) {
                return Err(err("backup path is outside private backup directory"));
            }
            checked(root, backup)?;
        }
    }
    for rel in &m.directories {
        checked(root, rel)?;
    }
    Ok(Some(m))
}

fn collect(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, PathBuf>,
) -> Result<(), InstallError> {
    for entry in fs::read_dir(dir).map_err(|e| err(e.to_string()))? {
        let entry = entry.map_err(|e| err(e.to_string()))?;
        let kind = entry.file_type().map_err(|e| err(e.to_string()))?;
        if kind.is_symlink() {
            return Err(err("payload contains a link"));
        }
        if kind.is_dir() {
            collect(root, &entry.path(), out)?;
            continue;
        }
        if !kind.is_file() {
            return Err(err("payload contains a non-file entry"));
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| err(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("!!")
            || name.eq_ignore_ascii_case("OptiScaler.dll")
            || name.eq_ignore_ascii_case(manifest::MANIFEST_FILENAME)
            || name.to_ascii_lowercase().ends_with(".bat")
            || name.to_ascii_lowercase().ends_with(".cmd")
            || name.to_ascii_lowercase().ends_with(".ps1")
            || name.to_ascii_lowercase().ends_with(".sh")
        {
            continue;
        }
        if !valid_relative(&rel) {
            return Err(err(format!("unsafe payload path: {rel}")));
        }
        if reserved(&rel) {
            return Err(err("payload contains an internal control path"));
        }
        out.insert(rel, entry.path());
    }
    Ok(())
}

fn mkdir_parents(
    root: &Path,
    path: &Path,
    created: &mut BTreeSet<PathBuf>,
) -> Result<(), InstallError> {
    let parent = path.parent().ok_or_else(|| err("missing parent"))?;
    let rel = parent.strip_prefix(root).map_err(|e| err(e.to_string()))?;
    let mut dir = root.to_path_buf();
    for part in rel.components() {
        dir.push(part);
        if dir.exists() {
            if !dir.is_dir() || fs::symlink_metadata(&dir).is_ok_and(|m| is_link(&m)) {
                return Err(err("unsafe destination directory"));
            }
        } else {
            fs::create_dir(&dir).map_err(|e| err(e.to_string()))?;
            created.insert(dir.clone());
        }
    }
    Ok(())
}

fn remove_empty_dirs(dirs: &BTreeSet<PathBuf>) {
    let mut dirs: Vec<_> = dirs.iter().collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in dirs {
        let _ = fs::remove_dir(dir);
    }
}

fn create_staging(root: &Path, staging: &Path) -> Result<(), InstallError> {
    let backup_root = checked(root, manifest::BACKUP_DIRECTORY)?;
    if !backup_root.exists() {
        fs::create_dir(&backup_root).map_err(|e| err(e.to_string()))?;
    }
    // A pre-existing staging path is never reused or trusted.
    fs::create_dir(staging).map_err(|e| err(e.to_string()))
}

pub fn install(
    root: &Path,
    extracted: &Path,
    options: &InstallOptions,
    version: &str,
    release_url: Option<String>,
    installed_at: String,
    mut progress: impl FnMut(InstallStage),
) -> Result<InstallManifest, InstallError> {
    let root = root.canonicalize().map_err(|e| err(e.to_string()))?;
    let _lock = Lock::acquire(&root)?;
    let previous = existing_manifest(&root)?;
    if previous.is_some() && !options.overwrite {
        return Err(InstallError::TargetExists(options.target_filename.clone()));
    }
    if !payload::PROXY_FILENAMES.contains(&options.target_filename.as_str()) {
        return Err(err("unsupported proxy filename"));
    }
    let dll = payload::find_optiscaler_dll(extracted).ok_or(InstallError::DllNotFound)?;
    if fs::symlink_metadata(&dll).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err(err("unsafe OptiScaler.dll source"));
    }
    let mut sources = BTreeMap::new();
    collect(extracted, extracted, &mut sources)?;
    sources.insert(options.target_filename.clone(), dll);
    let mut unique = BTreeSet::new();
    for rel in sources.keys() {
        if !unique.insert(rel.to_ascii_lowercase()) {
            return Err(err("payload contains case-aliased files"));
        }
    }
    // A pre-existing config is user-owned; a fresh config comes from this release.
    if root.join("OptiScaler.ini").exists()
        || previous
            .as_ref()
            .is_some_and(|m| m.owned_files.keys().any(|rel| mutable_config(rel)))
    {
        sources.retain(|rel, _| !rel.eq_ignore_ascii_case("OptiScaler.ini"));
    }
    let mut released_config = BTreeSet::new();
    if let Some(old) = &previous {
        for (rel, owned) in &old.owned_files {
            if !still_owned(&root, rel, owned)? {
                if mutable_config(rel) {
                    released_config.insert(rel.clone());
                } else {
                    return Err(err(format!("owned file was modified or removed: {rel}")));
                }
            }
        }
    }
    let mut expected_destinations = BTreeMap::new();
    for rel in sources.keys() {
        let dst = checked(&root, rel)?;
        if dst.exists() {
            if !dst.is_file() {
                return Err(err(format!("destination is not a file: {rel}")));
            }
            if let Some(old) = previous.as_ref().and_then(|m| m.owned_files.get(rel)) {
                if digest(&dst)? != old.sha256 {
                    return Err(err(format!("owned file was modified: {rel}")));
                }
            } else if previous.is_some() || !options.overwrite {
                return Err(InstallError::TargetExists(rel.clone()));
            }
            expected_destinations.insert(rel.clone(), Some(digest(&dst)?));
        } else if previous
            .as_ref()
            .is_some_and(|m| m.owned_files.contains_key(rel))
        {
            return Err(err(format!("owned file is missing: {rel}")));
        } else {
            expected_destinations.insert(rel.clone(), None);
        }
    }
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let staging_rel = format!("{}/{}", manifest::BACKUP_DIRECTORY, nonce);
    let staging = checked(&root, &staging_rel)?;
    create_staging(&root, &staging)?;
    let mut created_dirs = BTreeSet::new();
    let mut snapshots: BTreeMap<String, Option<PathBuf>> = BTreeMap::new();
    let mut originals = Vec::new();
    let mut written = BTreeSet::new();
    let mut written_hashes = BTreeMap::new();
    let operation = (|| {
        let mut owned = previous
            .as_ref()
            .map_or_else(BTreeMap::new, |m| m.owned_files.clone());
        owned.retain(|rel, _| !released_config.contains(rel));
        for (index, (rel, src)) in sources.iter().enumerate() {
            let dst = checked(&root, rel)?;
            let current = if dst.is_file() {
                Some(digest(&dst)?)
            } else if dst.exists() {
                return Err(err(format!("destination changed after preflight: {rel}")));
            } else {
                None
            };
            if current != expected_destinations[rel] {
                return Err(err(format!("destination changed after preflight: {rel}")));
            }
            let original = if dst.exists() {
                let snapshot = staging.join("rollback").join(rel);
                mkdir_parents(&root, &snapshot, &mut created_dirs)?;
                fs::copy(&dst, &snapshot).map_err(|e| err(e.to_string()))?;
                if Some(digest(&snapshot)?) != expected_destinations[rel] {
                    return Err(err(format!("snapshot differs from preflight: {rel}")));
                }
                Some(snapshot)
            } else {
                None
            };
            let current = if dst.is_file() {
                Some(digest(&dst)?)
            } else if dst.exists() {
                return Err(err(format!("destination changed after snapshot: {rel}")));
            } else {
                None
            };
            if current != expected_destinations[rel] {
                return Err(err(format!("destination changed after snapshot: {rel}")));
            }
            snapshots.insert(rel.clone(), original.clone());
            let backup = if let Some(old) = owned.get(rel) {
                old.original_backup.clone()
            } else if let Some(snapshot) = &original {
                let backup_rel = format!("{staging_rel}/original/{rel}");
                let backup_dst = checked(&root, &backup_rel)?;
                mkdir_parents(&root, &backup_dst, &mut created_dirs)?;
                fs::copy(snapshot, &backup_dst).map_err(|e| err(e.to_string()))?;
                originals.push(backup_dst);
                Some(backup_rel)
            } else {
                None
            };
            mkdir_parents(&root, &dst, &mut created_dirs)?;
            let current = if dst.is_file() {
                Some(digest(&dst)?)
            } else if dst.exists() {
                return Err(err(format!("destination changed before copy: {rel}")));
            } else {
                None
            };
            if current != expected_destinations[rel] {
                return Err(err(format!("destination changed before copy: {rel}")));
            }
            written.insert(rel.clone());
            replace_file(src, &dst).map_err(|e| err(e.to_string()))?;
            if rel == "OptiScaler.ini" && !options.dlss_inputs {
                super::set_ini_value(&dst, "Spoofing", "Dxgi", "false")
                    .map_err(|e| err(e.to_string()))?;
            }
            let original_sha256 = if let Some(old) = owned.get(rel) {
                old.original_sha256.clone()
            } else if let Some(snapshot) = &original {
                Some(digest(snapshot)?)
            } else {
                None
            };
            let extra = owned
                .get(rel)
                .map_or_else(BTreeMap::new, |old| old.extra.clone());
            let installed_hash = digest(&dst)?;
            written_hashes.insert(rel.clone(), installed_hash.clone());
            owned.insert(
                rel.clone(),
                OwnedFile {
                    sha256: installed_hash,
                    original_backup: backup,
                    original_sha256,
                    extra,
                },
            );
            progress(InstallStage::CopyingPayload {
                done: index + 1,
                total: sources.len(),
            });
        }
        // Existing configs are left unchanged. No synthetic fallback INI is generated.
        progress(InstallStage::Finalizing);
        let mut manifest = InstallManifest::new(
            &options.target_filename,
            &[],
            &[],
            version,
            release_url,
            installed_at,
        );
        manifest.schema_version = 2;
        manifest.owned_files = owned;
        manifest.files = manifest.owned_files.keys().cloned().collect();
        manifest.directories = manifest
            .files
            .iter()
            .filter_map(|f| Path::new(f).parent())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|p| !p.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if let Some(old) = previous {
            manifest.extra = old.extra;
        }
        let tmp = staging.join("manifest.json");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&manifest).map_err(|e| err(e.to_string()))?,
        )
        .map_err(|e| err(e.to_string()))?;
        fs::rename(&tmp, root.join(manifest::MANIFEST_FILENAME)).map_err(|e| err(e.to_string()))?;
        Ok(manifest)
    })();
    let mut rollback_errors = Vec::new();
    if operation.is_err() {
        for rel in written.iter().rev() {
            if let Ok(dst) = checked(&root, rel) {
                let current = if dst.is_file() {
                    digest(&dst).ok()
                } else {
                    None
                };
                let original = snapshots.get(rel).and_then(|value| value.as_ref());
                let safe = if let Some(expected) = written_hashes.get(rel) {
                    current.as_ref() == Some(expected)
                } else if let Some(snapshot) = original {
                    !dst.exists() || current == digest(snapshot).ok()
                } else {
                    !dst.exists()
                };
                if !safe {
                    rollback_errors.push(format!("{rel}: destination changed during rollback"));
                    continue;
                }
                if let Some(Some(snapshot)) = snapshots.get(rel) {
                    if let Err(e) = replace_file(snapshot, &dst) {
                        rollback_errors.push(format!("{rel}: {e}"));
                    }
                } else if dst.exists() {
                    if let Err(e) = fs::remove_file(dst) {
                        rollback_errors.push(format!("{rel}: {e}"));
                    }
                }
            } else {
                rollback_errors.push(format!("{rel}: path changed during rollback"));
            }
        }
    }
    if !rollback_errors.is_empty() {
        return Err(err(format!(
            "rollback incomplete ({}); recovery snapshots retained at {}",
            rollback_errors.join(", "),
            staging.display()
        )));
    }
    for snapshot in snapshots.values().flatten() {
        let _ = fs::remove_file(snapshot);
    }
    let _ = fs::remove_file(staging.join("manifest.json"));
    if operation.is_err() {
        for original in originals {
            let _ = fs::remove_file(original);
        }
        remove_empty_dirs(&created_dirs);
    }
    prune_empty(&staging);
    operation
}

pub fn uninstall(root: &Path) -> Result<(Vec<String>, Vec<String>), InstallError> {
    let root = root.canonicalize().map_err(|e| err(e.to_string()))?;
    let _lock = Lock::acquire(&root)?;
    let manifest =
        existing_manifest(&root)?.ok_or_else(|| err("no owned v2 installation manifest"))?;
    let mut safe = Vec::new();
    for (rel, owned) in &manifest.owned_files {
        let dst = checked(&root, rel)?;
        if !dst.exists() || !dst.is_file() || digest(&dst)? != owned.sha256 {
            continue;
        }
        if let Some(backup) = &owned.original_backup {
            let backup = checked(&root, backup)?;
            if !backup.is_file() {
                return Err(err(format!("original backup missing: {rel}")));
            }
            if owned
                .original_sha256
                .as_ref()
                .is_some_and(|sum| digest(&backup).ok().as_ref() != Some(sum))
            {
                return Err(err(format!("original backup changed: {rel}")));
            }
        }
        safe.push(rel.clone());
    }
    if manifest
        .owned_files
        .keys()
        .any(|rel| !safe.contains(rel) && !mutable_config(rel))
    {
        return Err(err(
            "owned files changed or disappeared; uninstall refused to preserve them",
        ));
    }
    let staging = checked(
        &root,
        &format!(
            "{}/uninstall-{}-{}",
            manifest::BACKUP_DIRECTORY,
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ),
    )?;
    // Validate cleanup paths before committing changes or removing the manifest.
    let dirs: BTreeSet<_> = manifest
        .directories
        .iter()
        .map(|rel| checked(&root, rel))
        .collect::<Result<_, _>>()?;
    let backups: Vec<_> = manifest
        .owned_files
        .iter()
        .filter(|(rel, _)| safe.contains(rel))
        .filter_map(|(_, owned)| owned.original_backup.as_ref())
        .map(|rel| checked(&root, rel))
        .collect::<Result<_, _>>()?;
    let backup_root = checked(&root, manifest::BACKUP_DIRECTORY)?;
    create_staging(&root, &staging)?;
    let mut snapshots = BTreeMap::new();
    let mut touched = Vec::new();
    let mut post_uninstall = BTreeMap::new();
    let operation = (|| {
        for rel in &safe {
            let dst = checked(&root, rel)?;
            let snapshot = staging.join(rel);
            if let Some(parent) = snapshot.parent() {
                fs::create_dir_all(parent).map_err(|e| err(e.to_string()))?;
            }
            fs::copy(&dst, &snapshot).map_err(|e| err(e.to_string()))?;
            if digest(&snapshot)? != manifest.owned_files[rel].sha256 {
                return Err(err(format!(
                    "uninstall snapshot differs from manifest: {rel}"
                )));
            }
            snapshots.insert(rel.clone(), snapshot);
        }
        for rel in &safe {
            let dst = checked(&root, rel)?;
            if digest(&dst)? != manifest.owned_files[rel].sha256 {
                return Err(err(format!("owned file changed before uninstall: {rel}")));
            }
            touched.push(rel.clone());
            if let Some(backup) = &manifest.owned_files[rel].original_backup {
                replace_file(&checked(&root, backup)?, &dst).map_err(|e| err(e.to_string()))?;
                post_uninstall.insert(rel.clone(), Some(digest(&dst)?));
            } else {
                fs::remove_file(&dst).map_err(|e| err(e.to_string()))?;
                post_uninstall.insert(rel.clone(), None);
            }
        }
        fs::remove_file(root.join(manifest::MANIFEST_FILENAME)).map_err(|e| err(e.to_string()))?;
        Ok::<(), InstallError>(())
    })();
    let mut rollback_errors = Vec::new();
    if operation.is_err() {
        for rel in &touched {
            if let (Ok(dst), Some(snapshot)) = (checked(&root, rel), snapshots.get(rel)) {
                let current = if dst.is_file() {
                    digest(&dst).ok()
                } else {
                    None
                };
                let safe = match post_uninstall.get(rel) {
                    Some(Some(expected)) => current.as_ref() == Some(expected),
                    Some(None) => !dst.exists(),
                    None => !dst.exists() || current == digest(snapshot).ok(),
                };
                if !safe {
                    rollback_errors.push(format!("{rel}: destination changed during rollback"));
                    continue;
                }
                if let Err(e) = replace_file(snapshot, &dst) {
                    rollback_errors.push(format!("{rel}: {e}"));
                }
            } else {
                rollback_errors.push(format!("{rel}: path changed during rollback"));
            }
        }
    }
    if !rollback_errors.is_empty() {
        return Err(err(format!(
            "uninstall rollback incomplete ({}); recovery snapshots retained at {}",
            rollback_errors.join(", "),
            staging.display()
        )));
    }
    for snapshot in snapshots.values() {
        let _ = fs::remove_file(snapshot);
    }
    prune_empty(&staging);
    operation?;
    let mut removed_dirs = Vec::new();
    let mut sorted: Vec<_> = dirs.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in sorted {
        if fs::remove_dir(dir).is_ok() {
            if let Ok(rel) = dir.strip_prefix(&root) {
                removed_dirs.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    for backup in backups {
        let _ = fs::remove_file(backup);
    }
    // Backup directories are removed only if empty; foreign files remain untouched.
    if backup_root.exists() {
        prune_empty(&backup_root);
    }
    Ok((safe, removed_dirs))
}

/// Gate for destructive proxy toggles; v1 manifests and modified proxies fail closed.
pub fn verify_owned_proxy(root: &Path, proxy: &str) -> Result<(), InstallError> {
    let root = root.canonicalize().map_err(|e| err(e.to_string()))?;
    let manifest = existing_manifest(&root)?.ok_or_else(|| err("no owned v2 installation"))?;
    if manifest.target_filename != proxy {
        return Err(err("proxy differs from manifest"));
    }
    let owned = manifest
        .owned_files
        .get(proxy)
        .ok_or_else(|| err("proxy not owned"))?;
    let path = checked(&root, proxy)?;
    if !path.is_file() || digest(&path)? != owned.sha256 {
        return Err(err("proxy was modified or missing"));
    }
    Ok(())
}

fn prune_empty(dir: &Path) {
    if fs::symlink_metadata(dir).map_or(true, |m| is_link(&m)) {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry
                .file_type()
                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
            {
                prune_empty(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}
