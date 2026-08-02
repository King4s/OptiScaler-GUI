//! GUI self-update: download the new exe from our releases, verify it
//! against the published SHA256SUMS.txt, and swap it in on restart.
//!
//! Unlike OptiScaler archives (where a missing digest passes), the checksum
//! is REQUIRED here — our own release pipeline always publishes sums, and a
//! new executable must never be applied unverified.
//!
//! Swap dance (Windows can't overwrite a running exe, but CAN rename it):
//! running exe → `<name>.old`, staged `<name>.update` → exe name, spawn the
//! new exe, caller exits. `cleanup_old` removes the leftover on next start.

use crate::install::github::{self, ReleaseInfo};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const GUI_RELEASES_LATEST: &str =
    "https://api.github.com/repos/King4s/OptiScaler-GUI/releases/latest";
const EXE_ASSET: &str = "OptiScaler-GUI.exe";
const SUMS_ASSET: &str = "SHA256SUMS.txt";

#[derive(Debug)]
pub enum SelfUpdateError {
    Network(String),
    MissingAsset(&'static str),
    MissingChecksum,
    DigestMismatch,
    Io(String),
}

impl std::fmt::Display for SelfUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelfUpdateError::Network(e) => write!(f, "network error: {e}"),
            SelfUpdateError::MissingAsset(name) => {
                write!(f, "release has no {name} asset")
            }
            SelfUpdateError::MissingChecksum => {
                write!(f, "release checksums do not cover the executable")
            }
            SelfUpdateError::DigestMismatch => {
                write!(f, "downloaded executable failed SHA256 verification")
            }
            SelfUpdateError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for SelfUpdateError {}

/// The staged-download path for an exe ("<exe>.update").
pub fn staged_path(current_exe: &Path) -> PathBuf {
    sibling(current_exe, "update")
}

fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "OptiScaler-GUI.exe".to_string());
    exe.with_file_name(format!("{name}.{suffix}"))
}

/// Extract the hex digest for `filename` from SHA256SUMS.txt content.
/// Lines are "<hex>  <name>" (sha256sum format; '*' binary marker tolerated).
pub fn checksum_for(sums: &str, filename: &str) -> Option<String> {
    for line in sums.lines() {
        let mut parts = line.split_whitespace();
        let (Some(hex), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if name.trim_start_matches('*').eq_ignore_ascii_case(filename) && hex.len() == 64 {
            return Some(hex.to_lowercase());
        }
    }
    None
}

fn fetch_text(url: &str) -> Result<String, SelfUpdateError> {
    let mut resp = crate::images::http_agent()
        .get(url)
        .header("User-Agent", "OptiScaler-GUI")
        .call()
        .map_err(|e| SelfUpdateError::Network(e.to_string()))?;
    resp.body_mut()
        .read_to_string()
        .map_err(|e| SelfUpdateError::Network(e.to_string()))
}

/// Download the release's exe next to `current_exe` as `<name>.update` and
/// verify it against the release's SHA256SUMS.txt. Returns the staged path.
pub fn download_update(
    release: &ReleaseInfo,
    current_exe: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, SelfUpdateError> {
    let exe_asset = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(EXE_ASSET))
        .ok_or(SelfUpdateError::MissingAsset(EXE_ASSET))?;
    let sums_asset = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(SUMS_ASSET))
        .ok_or(SelfUpdateError::MissingAsset(SUMS_ASSET))?;

    let sums = fetch_text(&sums_asset.browser_download_url)?;
    let expected = checksum_for(&sums, EXE_ASSET).ok_or(SelfUpdateError::MissingChecksum)?;

    let staged = staged_path(current_exe);
    let mut resp = crate::images::http_agent()
        .get(&exe_asset.browser_download_url)
        .header("User-Agent", "OptiScaler-GUI")
        .call()
        .map_err(|e| SelfUpdateError::Network(e.to_string()))?;
    let total = exe_asset.size;
    let mut out = std::fs::File::create(&staged).map_err(|e| SelfUpdateError::Io(e.to_string()))?;
    let mut reader = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| SelfUpdateError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..n])
            .map_err(|e| SelfUpdateError::Io(e.to_string()))?;
        downloaded += n as u64;
        progress(downloaded, total);
    }
    drop(out);

    let digest = format!("sha256:{expected}");
    if let Err(e) = github::verify_digest(&staged, Some(&digest)) {
        let _ = std::fs::remove_file(&staged);
        return Err(match e {
            github::DownloadError::DigestMismatch => SelfUpdateError::DigestMismatch,
            other => SelfUpdateError::Io(other.to_string()),
        });
    }
    Ok(staged)
}

/// Rename the running exe aside and move the staged update into its place.
/// Split from [`apply_and_restart`] so the file dance is unit-testable.
pub fn swap_staged(current_exe: &Path) -> Result<(), SelfUpdateError> {
    let staged = staged_path(current_exe);
    if !staged.is_file() {
        return Err(SelfUpdateError::Io(format!(
            "no staged update at {}",
            staged.display()
        )));
    }
    let old = sibling(current_exe, "old");
    if old.exists() {
        std::fs::remove_file(&old).map_err(|e| SelfUpdateError::Io(e.to_string()))?;
    }
    std::fs::rename(current_exe, &old).map_err(|e| SelfUpdateError::Io(e.to_string()))?;
    if let Err(e) = std::fs::rename(&staged, current_exe) {
        // Restore the original so the app keeps working
        let _ = std::fs::rename(&old, current_exe);
        return Err(SelfUpdateError::Io(e.to_string()));
    }
    Ok(())
}

/// Swap the staged update in and start the new executable. On Ok the caller
/// must exit promptly; the new instance is already running.
pub fn apply_and_restart(current_exe: &Path) -> Result<(), SelfUpdateError> {
    swap_staged(current_exe)?;
    std::process::Command::new(current_exe)
        .spawn()
        .map_err(|e| SelfUpdateError::Io(e.to_string()))?;
    Ok(())
}

/// Startup cleanup: remove `<exe>.old` from a completed update and any
/// `<exe>.update` left by an interrupted download.
pub fn cleanup_old(current_exe: &Path) {
    let _ = std::fs::remove_file(sibling(current_exe, "old"));
    let _ = std::fs::remove_file(sibling(current_exe, "update"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_parsing() {
        let hex = "a".repeat(64);
        let sums = format!("{hex}  OptiScaler-GUI.exe\n{}  other.txt\n", "b".repeat(64));
        assert_eq!(checksum_for(&sums, "OptiScaler-GUI.exe"), Some(hex.clone()));
        // '*' binary marker and case-insensitive name
        let starred = format!("{hex} *optiscaler-gui.exe\n");
        assert_eq!(checksum_for(&starred, "OptiScaler-GUI.exe"), Some(hex));
        assert_eq!(checksum_for("garbage\n", "OptiScaler-GUI.exe"), None);
        // Truncated hash is rejected
        assert_eq!(
            checksum_for("abc  OptiScaler-GUI.exe\n", "OptiScaler-GUI.exe"),
            None
        );
    }

    #[test]
    fn swap_dance_and_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("app.exe");
        std::fs::write(&exe, b"old-version").unwrap();
        std::fs::write(staged_path(&exe), b"new-version").unwrap();

        swap_staged(&exe).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new-version");
        assert_eq!(
            std::fs::read(tmp.path().join("app.exe.old")).unwrap(),
            b"old-version"
        );
        assert!(!staged_path(&exe).exists());

        cleanup_old(&exe);
        assert!(!tmp.path().join("app.exe.old").exists());
        assert!(exe.exists()); // the real exe is never touched by cleanup
    }

    #[test]
    fn swap_without_staged_file_fails_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("app.exe");
        std::fs::write(&exe, b"v1").unwrap();
        assert!(swap_staged(&exe).is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"v1");
    }
}
