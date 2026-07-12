//! Install manifest — schema v1, byte-compatible with the Python app's
//! `.optiscaler-gui-install.json` so either app can update/uninstall
//! installations made by the other. THE cross-version contract.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILENAME: &str = ".optiscaler-gui-install.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    pub schema_version: u32,
    pub installed_by: String,
    /// ISO 8601 local time, matching Python's datetime.now().isoformat()
    pub installed_at: String,
    pub target_filename: String,
    pub optiscaler_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_url: Option<String>,
    /// Forward-slash relative paths, sorted & deduped
    pub files: Vec<String>,
    pub directories: Vec<String>,
}

impl InstallManifest {
    pub fn new(
        target_filename: &str,
        files: &[String],
        directories: &[String],
        version: &str,
        release_url: Option<String>,
        installed_at: String,
    ) -> Self {
        let files: BTreeSet<String> = files
            .iter()
            .filter(|f| !f.is_empty())
            .map(|f| f.replace('\\', "/"))
            .collect();
        let directories: BTreeSet<String> = directories
            .iter()
            .filter(|d| !d.is_empty())
            .map(|d| d.replace('\\', "/"))
            .collect();
        Self {
            schema_version: 1,
            installed_by: "OptiScaler-GUI".to_string(),
            installed_at,
            target_filename: target_filename.to_string(),
            optiscaler_version: version.to_string(),
            release_url,
            files: files.into_iter().collect(),
            directories: directories.into_iter().collect(),
        }
    }
}

pub fn manifest_path(install_dir: &Path) -> PathBuf {
    install_dir.join(MANIFEST_FILENAME)
}

pub fn read(install_dir: &Path) -> Option<InstallManifest> {
    let content = std::fs::read_to_string(manifest_path(install_dir)).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write(install_dir: &Path, manifest: &InstallManifest) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(manifest_path(install_dir), json)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture produced by the actual Python `_write_install_manifest`.
    const PYTHON_MANIFEST: &str = include_str!("../../tests/fixtures/python_manifest.json");

    #[test]
    fn reads_python_written_manifest() {
        let m: InstallManifest = serde_json::from_str(PYTHON_MANIFEST).expect("parse");
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.installed_by, "OptiScaler-GUI");
        assert_eq!(m.target_filename, "dxgi.dll");
        assert_eq!(m.optiscaler_version, "v0.9.3");
        assert!(m.files.contains(&"D3D12_Optiscaler/plugin.dll".to_string()));
        assert_eq!(m.directories, vec!["D3D12_Optiscaler", "Licenses"]);
    }

    #[test]
    fn writes_python_compatible_fields() {
        let m = InstallManifest::new(
            "dxgi.dll",
            &["dxgi.dll".into(), "sub\\file.dll".into(), "".into()],
            &["Licenses".into()],
            "v0.9.3",
            Some("https://example.com".into()),
            "2026-07-12T12:00:00".into(),
        );
        let json = serde_json::to_string_pretty(&m).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        // exact key set the Python reader expects
        for key in [
            "schema_version",
            "installed_by",
            "installed_at",
            "target_filename",
            "optiscaler_version",
            "release_url",
            "files",
            "directories",
        ] {
            assert!(value.get(key).is_some(), "missing key {key}");
        }
        // backslashes normalized, empty entries dropped, sorted
        assert_eq!(m.files, vec!["dxgi.dll", "sub/file.dll"]);
    }

    #[test]
    fn roundtrip_via_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let m = InstallManifest::new(
            "winmm.dll",
            &["winmm.dll".into()],
            &[],
            "v0.9.3",
            None,
            "2026-07-12T12:00:00".into(),
        );
        write(tmp.path(), &m).unwrap();
        let back = read(tmp.path()).unwrap();
        assert_eq!(back.target_filename, "winmm.dll");
        assert_eq!(back.files, vec!["winmm.dll"]);
    }
}
