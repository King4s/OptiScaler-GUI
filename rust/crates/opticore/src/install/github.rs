//! OptiScaler release download: latest-release lookup, archive asset pick,
//! streaming download with SHA256 digest verification, reuse of a valid
//! already-downloaded archive.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const RELEASES_LATEST_URL: &str =
    "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest";
const ARCHIVE_EXTENSIONS: &[&str] = &[".7z", ".zip"];

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseInfo {
    #[serde(default)]
    pub tag_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub assets: Vec<AssetInfo>,
}

impl ReleaseInfo {
    pub fn version_label(&self) -> String {
        self.tag_name
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// First asset with an archive extension (same rule as the Python app).
    pub fn archive_asset(&self) -> Option<&AssetInfo> {
        self.assets.iter().find(|a| {
            let lower = a.name.to_lowercase();
            ARCHIVE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
    /// "sha256:<hex>" when GitHub provides it
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug)]
pub enum DownloadError {
    Network(String),
    NoArchiveAsset,
    DigestMismatch,
    Io(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Network(e) => write!(f, "network error: {e}"),
            DownloadError::NoArchiveAsset => write!(f, "release has no archive asset"),
            DownloadError::DigestMismatch => {
                write!(f, "downloaded file failed SHA256 verification")
            }
            DownloadError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for DownloadError {}

pub fn fetch_latest_release() -> Result<ReleaseInfo, DownloadError> {
    fetch_release_from(RELEASES_LATEST_URL)
}

pub fn fetch_release_from(url: &str) -> Result<ReleaseInfo, DownloadError> {
    let mut resp = crate::images::http_agent()
        .get(url)
        .header("User-Agent", "OptiScaler-GUI")
        .call()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    serde_json::from_str(&body).map_err(|e| DownloadError::Network(format!("bad response: {e}")))
}

/// Verify a file against a GitHub "sha256:<hex>" digest. Files without a
/// digest pass (same policy as Python).
pub fn verify_digest(path: &Path, digest: Option<&str>) -> Result<(), DownloadError> {
    let Some(digest) = digest else {
        return Ok(());
    };
    let Some((algo, expected)) = digest.split_once(':') else {
        return Ok(()); // unsupported format — skip, like Python
    };
    if !algo.eq_ignore_ascii_case("sha256") || expected.is_empty() {
        return Ok(());
    }
    let mut file = std::fs::File::open(path).map_err(|e| DownloadError::Io(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hex_encode(&hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(DownloadError::DigestMismatch)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Download the release archive into `download_dir`, reusing an existing
/// digest-valid file. Progress callback receives (downloaded, total) bytes.
pub fn download_archive(
    release: &ReleaseInfo,
    download_dir: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, DownloadError> {
    let asset = release
        .archive_asset()
        .ok_or(DownloadError::NoArchiveAsset)?;
    std::fs::create_dir_all(download_dir).map_err(|e| DownloadError::Io(e.to_string()))?;
    let target = download_dir.join(&asset.name);

    // Reuse a valid existing download
    if target.exists() {
        let size_ok =
            asset.size == 0 || target.metadata().map(|m| m.len()).unwrap_or(0) == asset.size;
        if verify_digest(&target, asset.digest.as_deref()).is_ok() && size_ok {
            return Ok(target);
        }
        let _ = std::fs::remove_file(&target);
    }

    let mut resp = crate::images::http_agent()
        .get(&asset.browser_download_url)
        .header("User-Agent", "OptiScaler-GUI")
        .call()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    let total = asset.size;
    let mut out = std::fs::File::create(&target).map_err(|e| DownloadError::Io(e.to_string()))?;
    let mut reader = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| DownloadError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..n])
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        downloaded += n as u64;
        progress(downloaded, total);
    }
    drop(out);

    if total > 0 && std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0) != total {
        let _ = std::fs::remove_file(&target);
        return Err(DownloadError::Network("download size mismatch".into()));
    }
    if let Err(e) = verify_digest(&target, asset.digest.as_deref()) {
        let _ = std::fs::remove_file(&target);
        return Err(e);
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn picks_first_archive_asset() {
        let release: ReleaseInfo = serde_json::from_str(
            r#"{"tag_name":"v0.9.3","assets":[
                {"name":"notes.txt","browser_download_url":"u1"},
                {"name":"Optiscaler_0.9.3.7z","browser_download_url":"u2","size":10,
                 "digest":"sha256:abc"},
                {"name":"other.zip","browser_download_url":"u3"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(release.archive_asset().unwrap().name, "Optiscaler_0.9.3.7z");
        assert_eq!(release.version_label(), "v0.9.3");
    }

    #[test]
    fn digest_verification() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("x.bin");
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(b"hello").unwrap();
        drop(f);
        // sha256("hello")
        let good = "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_digest(&file, Some(good)).is_ok());
        assert!(matches!(
            verify_digest(&file, Some("sha256:deadbeef")),
            Err(DownloadError::DigestMismatch)
        ));
        // no digest / unsupported algo pass
        assert!(verify_digest(&file, None).is_ok());
        assert!(verify_digest(&file, Some("md5:abc")).is_ok());
    }
}
