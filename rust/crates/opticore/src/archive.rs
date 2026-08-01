//! OptiScaler release archive extraction.
//!
//! Pure-Rust 7z extraction via sevenz-rust2, which decodes the BCJ2 filter
//! OptiScaler's archives use (verified byte-identical to 7z.exe against the
//! real 0.9.3 release in the M0 spike). No bundled 7z.exe required.

use std::path::Path;

#[derive(Debug)]
pub enum ArchiveError {
    Open(String),
    Extract(String),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveError::Open(e) => write!(f, "failed to open archive: {e}"),
            ArchiveError::Extract(e) => write!(f, "failed to extract archive: {e}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

/// List entry names and sizes without extracting.
pub fn list_7z(archive: &Path) -> Result<Vec<(String, u64)>, ArchiveError> {
    let reader = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
        .map_err(|e| ArchiveError::Open(e.to_string()))?;
    Ok(reader
        .archive()
        .files
        .iter()
        .map(|f| (f.name().to_string(), f.size()))
        .collect())
}

/// True when an archive entry name cannot escape the extraction directory
/// (zip-slip): no absolute paths, drive letters, or `..` components.
fn entry_name_is_safe(name: &str) -> bool {
    !name.starts_with(['/', '\\'])
        && !name.contains(':')
        && !name.split(['/', '\\']).any(|part| part == "..")
}

/// Extract the full archive into `out_dir` (must be an absolute path —
/// sevenz-rust2 rejects destinations containing parent-directory components).
/// Entry names are validated against path traversal before extraction.
pub fn extract_7z(archive: &Path, out_dir: &Path) -> Result<(), ArchiveError> {
    for (name, _) in list_7z(archive)? {
        if !entry_name_is_safe(&name) {
            return Err(ArchiveError::Extract(format!(
                "unsafe entry path in archive: {name}"
            )));
        }
    }
    sevenz_rust2::decompress_file(archive, out_dir)
        .map_err(|e| ArchiveError::Extract(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn entry_name_safety() {
        for good in [
            "OptiScaler.dll",
            "Licenses/L.txt",
            "D3D12_Optiscaler\\D3D12Core.dll",
        ] {
            assert!(entry_name_is_safe(good), "{good}");
        }
        for bad in [
            "../evil.dll",
            "..\\evil.dll",
            "Licenses/../../evil.dll",
            "/abs/path.dll",
            "\\abs\\path.dll",
            "C:\\Windows\\evil.dll",
        ] {
            assert!(!entry_name_is_safe(bad), "{bad}");
        }
    }

    #[test]
    fn open_error_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("not-an-archive.7z");
        let mut f = std::fs::File::create(&bogus).unwrap();
        f.write_all(b"definitely not a 7z file").unwrap();
        assert!(matches!(list_7z(&bogus), Err(ArchiveError::Open(_))));
    }

    /// BCJ2 canary: run manually / in scheduled CI against a real release:
    /// `OPTISCALER_ARCHIVE=path\to\Optiscaler_x.y.z.7z cargo test -- --ignored`
    #[test]
    #[ignore]
    fn extracts_real_optiscaler_archive() {
        let archive = std::env::var("OPTISCALER_ARCHIVE").expect("set OPTISCALER_ARCHIVE");
        let out = tempfile::tempdir().unwrap();
        let entries = list_7z(Path::new(&archive)).unwrap();
        assert!(!entries.is_empty());
        extract_7z(Path::new(&archive), out.path()).unwrap();
        assert!(out.path().join("OptiScaler.dll").exists());
    }
}
