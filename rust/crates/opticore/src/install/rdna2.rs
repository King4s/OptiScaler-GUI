//! Separate, opt-in community runtime; never changes the official extraction cache.
use super::{github, InstallError};
use crate::hardware::GpuProfile;
use std::path::{Path, PathBuf};
pub const RELEASE_URL: &str = "https://github.com/the3rdparty1917/fsr4xyz/releases/tag/4.1.1b";
pub const ASSET: &str = "FSR_4.1.1b_INT8_with_RDNA2_fix.7z";
pub const ARCHIVE_SHA256: &str = "66e9a818e0c914def7712c8dac06b08e64a64dbcfe77f3162d43ea6de93869ff";
pub const DLL_SHA256: &str = "0dd77d9c78d1ef9bc330cf4697ab3ffe24bc1aa7850e4130263dc922107fbd75";
pub const DLL_SIZE: u64 = 34013696;
pub const ARCHIVE_MEMBER: &str = "4.1.1b/amd_fidelityfx_upscaler_dx12.dll";
pub const DLL_NAME: &str = "amd_fidelityfx_upscaler_dx12.dll";
pub fn eligible(gpu: Option<&GpuProfile>, consent: bool) -> bool {
    consent && gpu.is_some_and(|g| g.recommendation_key() == "hardware.rdna2")
}
pub fn validate(path: &Path) -> Result<(), InstallError> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| InstallError::Io(e.to_string()))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() != DLL_SIZE {
        return Err(InstallError::Io(
            "RDNA2 runtime size/type verification failed".into(),
        ));
    }
    github::verify_digest(path, Some(&format!("sha256:{DLL_SHA256}")))
        .map_err(InstallError::Download)
}
pub fn prepare(cache: &Path) -> Result<PathBuf, InstallError> {
    let cache = cache.join("community-rdna2-4.1.1b");
    let release = github::ReleaseInfo {
        tag_name: Some("4.1.1b".into()),
        name: None,
        html_url: Some(RELEASE_URL.into()),
        assets: vec![github::AssetInfo {
            name: ASSET.into(),
            browser_download_url: format!(
                "https://github.com/the3rdparty1917/fsr4xyz/releases/download/4.1.1b/{ASSET}"
            ),
            size: 3456158,
            digest: Some(format!("sha256:{ARCHIVE_SHA256}")),
        }],
    };
    let archive =
        github::download_archive(&release, &cache, |_, _| {}).map_err(InstallError::Download)?;
    // Fresh private extraction, including when an old cache is damaged.
    let staging = tempfile::tempdir_in(&cache).map_err(|e| InstallError::Io(e.to_string()))?;
    crate::archive::extract_7z(&archive, staging.path())
        .map_err(|e| InstallError::Extraction(e.to_string()))?;
    let dll = staging.path().join(ARCHIVE_MEMBER);
    validate(&dll)?;
    let target = cache.join(DLL_NAME);
    super::transaction::atomic_replace(&dll, &target)
        .map_err(|e| InstallError::Io(e.to_string()))?;
    validate(&target)?;
    Ok(target)
}
