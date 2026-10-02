//! Bounded, read-only scanning of explicitly selected local library folders.

use super::{folder_facts, names};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 4;
const MAX_ENTRIES: usize = 4096;
const MAX_DIRECTORIES: usize = 256;
const MAX_WARNINGS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanRoot {
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct CustomGameEntry {
    pub path: PathBuf,
    pub name: String,
    pub facts: folder_facts::FolderFacts,
}

#[derive(Debug, Default)]
pub struct CustomScanResult {
    pub entries: Vec<CustomGameEntry>,
    pub warnings: Vec<String>,
}

/// Resolve one user-supplied local directory without following a link in any
/// component. The returned path is canonical and suitable for confinement.
pub fn validate_root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Custom root must be an absolute local path".into());
    }
    if path
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Custom root cannot contain parent traversal".into());
    }
    if is_network_path(path) {
        return Err("Network custom roots are not supported".into());
    }
    for component in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        if let Ok(metadata) = fs::symlink_metadata(component) {
            if is_link(&metadata) {
                return Err("Custom root cannot traverse a link or reparse point".into());
            }
        }
    }
    let canonical =
        fs::canonicalize(path).map_err(|e| format!("Custom root is unavailable: {e}"))?;
    if !canonical.is_dir() {
        return Err("Custom root must be a directory".into());
    }
    if canonical.parent().is_none() || is_protected_root(&canonical) {
        return Err("Custom root is a drive, profile, or system directory".into());
    }
    if is_network_path(&canonical) {
        return Err("Network custom roots are not supported".into());
    }
    Ok(canonical)
}

/// Return candidate game folders under enabled roots. No executable is
/// inferred; callers can merge the path/name with their usual game pipeline.
pub fn scan_roots(roots: &[ScanRoot]) -> CustomScanResult {
    let mut result = CustomScanResult::default();
    let mut seen = HashSet::new();
    let mut seen_roots = HashSet::new();
    for root in roots.iter().filter(|root| root.enabled) {
        let canonical = match validate_root(&root.path) {
            Ok(path) => path,
            Err(message) => {
                if result.warnings.len() < MAX_WARNINGS {
                    result
                        .warnings
                        .push(format!("{}: {message}", root.path.display()));
                }
                continue;
            }
        };
        if !seen_roots.insert(canonical.clone()) {
            continue;
        }
        let mut scanner = Scanner {
            root: &canonical,
            seen: &mut seen,
            result: &mut result,
            entries: 0,
            directories: 0,
            exhausted: false,
        };
        scanner.visit(&canonical, 0);
        if scanner.exhausted {
            scanner.warn(format!(
                "{}: custom scan work budget exhausted",
                canonical.display()
            ));
        }
    }
    result
}

struct Scanner<'a> {
    root: &'a Path,
    seen: &'a mut HashSet<PathBuf>,
    result: &'a mut CustomScanResult,
    entries: usize,
    directories: usize,
    exhausted: bool,
}

impl Scanner<'_> {
    fn warn(&mut self, message: String) {
        if self.result.warnings.len() < MAX_WARNINGS {
            self.result.warnings.push(message);
        }
    }

    fn spend_directory(&mut self) -> bool {
        self.directories += 1;
        if self.directories > MAX_DIRECTORIES {
            self.exhausted = true;
            return false;
        }
        true
    }

    fn spend_entry(&mut self) -> bool {
        self.entries += 1;
        if self.entries > MAX_ENTRIES {
            self.exhausted = true;
            return false;
        }
        true
    }

    fn visit(&mut self, dir: &Path, depth: usize) {
        if self.exhausted || !self.safe_directory(dir) {
            return;
        }
        let mut facts = folder_facts::FolderFacts {
            root: dir.to_path_buf(),
            ..Default::default()
        };
        let mut children = Vec::new();
        if self
            .inspect(
                dir,
                0,
                MAX_DEPTH.saturating_sub(depth).min(3),
                &mut facts,
                &mut children,
            )
            .is_err()
        {
            return;
        }
        let name = dir
            .file_name()
            .map(|n| names::folder_name_to_title(&n.to_string_lossy()));
        if let Some(name) = name {
            if !is_structural_folder(dir)
                && has_own_game_evidence(&facts)
                && folder_facts::is_game_folder(dir, &facts)
                && !names::is_launcher_entry(&name)
            {
                if self.seen.insert(dir.to_path_buf()) {
                    self.result.entries.push(CustomGameEntry {
                        path: dir.to_path_buf(),
                        name,
                        facts,
                    });
                }
                return;
            }
        }
        if depth < MAX_DEPTH {
            for child in children {
                self.visit(&child, depth + 1);
                if self.exhausted {
                    break;
                }
            }
        }
    }

    fn inspect(
        &mut self,
        dir: &Path,
        level: usize,
        max_level: usize,
        facts: &mut folder_facts::FolderFacts,
        children: &mut Vec<PathBuf>,
    ) -> Result<(), ()> {
        if !self.spend_directory() {
            return Err(());
        }
        let entries = fs::read_dir(dir).map_err(|e| {
            self.warn(format!("{}: cannot read directory: {e}", dir.display()));
        })?;
        let mut descend = Vec::new();
        for entry in entries {
            if !self.spend_entry() {
                return Err(());
            }
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if is_link(&metadata) {
                self.warn(format!("{}: linked entry skipped", path.display()));
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if metadata.is_dir() {
                if level == 0 {
                    facts.top_dirs.push(name);
                    children.push(path.clone());
                }
                if level < max_level && self.safe_directory(&path) {
                    descend.push(path);
                }
            } else if metadata.is_file() {
                facts.file_count += 1;
                if level == 0 {
                    facts.top_files.push(name.clone());
                } else if level == 1 {
                    facts.depth1_files.push(name.clone());
                }
                facts.found_exe |= name.ends_with(".exe");
                facts.found_game_content |= [".pak", ".uasset", ".dll", ".bin", ".unity3d"]
                    .iter()
                    .any(|suffix| name.ends_with(suffix));
            }
        }
        if level == 0 && !has_own_game_evidence(facts) {
            return Ok(());
        }
        for subdir in descend {
            self.inspect(&subdir, level + 1, max_level, facts, &mut Vec::new())?;
        }
        Ok(())
    }

    fn safe_directory(&self, path: &Path) -> bool {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return false;
        };
        if !metadata.is_dir() || is_link(&metadata) {
            return false;
        }
        fs::canonicalize(path)
            .map(|resolved| resolved.starts_with(self.root))
            .unwrap_or(false)
    }
}

fn has_own_game_evidence(facts: &folder_facts::FolderFacts) -> bool {
    facts.top_files.iter().any(|name| {
        [".exe", ".pak", ".uasset", ".dll", ".bin", ".unity3d"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
    }) || facts.top_dirs.iter().any(|name| {
        matches!(
            name.as_str(),
            "engine" | "binaries" | "content" | "assets" | "bin" | "win64" | "x64"
        ) || name.ends_with("_data")
    })
}

fn is_structural_folder(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    matches!(
        name.to_string_lossy().to_ascii_lowercase().as_str(),
        "binaries" | "bin" | "win64" | "win32" | "x64" | "x86" | "engine" | "content" | "assets"
    )
}

#[cfg(windows)]
fn is_link(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn is_network_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => drive_is_remote(letter),
            _ => true,
        },
        _ => true,
    }
}

#[cfg(windows)]
fn drive_is_remote(letter: u8) -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    }
    let drive = [letter as u16, b':' as u16, b'\\' as u16, 0];
    // Unknown and remote drives are both unsuitable for bounded local scans.
    matches!(unsafe { GetDriveTypeW(drive.as_ptr()) }, 0 | 1 | 4)
}

#[cfg(not(windows))]
fn is_network_path(_path: &Path) -> bool {
    false
}

fn is_protected_root(path: &Path) -> bool {
    #[cfg(windows)]
    if let Some(profile) = std::env::var_os("USERPROFILE").and_then(|p| fs::canonicalize(p).ok()) {
        if let Some(container) = profile.parent() {
            if same_path(path, container)
                || path
                    .parent()
                    .is_some_and(|parent| same_path(parent, container))
            {
                return true;
            }
        }
    }
    let exact = if cfg!(windows) {
        vec![
            "USERPROFILE",
            "PUBLIC",
            "APPDATA",
            "LOCALAPPDATA",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramData",
            "WINDIR",
            "SystemRoot",
        ]
    } else {
        vec!["HOME"]
    };
    if exact
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .filter_map(|p| fs::canonicalize(p).ok())
        .any(|p| same_path(path, &p))
    {
        return true;
    }
    #[cfg(windows)]
    {
        for key in ["WINDIR", "SystemRoot"] {
            if let Some(system) = std::env::var_os(key).and_then(|p| fs::canonicalize(p).ok()) {
                if within_path(path, &system) {
                    return true;
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        if path.parent().is_some_and(|parent| {
            ["/home", "/Users"]
                .iter()
                .any(|base| parent == Path::new(base))
        }) {
            return true;
        }
        for system in [
            "/etc", "/usr", "/var", "/bin", "/sbin", "/System", "/proc", "/sys", "/dev",
        ] {
            if path.starts_with(system) {
                return true;
            }
        }
        if ["/home", "/Users", "/tmp", "/var/tmp"]
            .iter()
            .any(|p| path == Path::new(p))
        {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy()
        .eq_ignore_ascii_case(&b.to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(a: &Path, b: &Path) -> bool {
    a == b
}

#[cfg(windows)]
fn within_path(path: &Path, base: &Path) -> bool {
    path.to_string_lossy()
        .to_lowercase()
        .starts_with(&format!("{}\\", base.to_string_lossy().to_lowercase()))
        || same_path(path, base)
}
