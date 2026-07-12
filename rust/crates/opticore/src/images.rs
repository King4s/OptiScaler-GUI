//! Game artwork fetching with the Python-compatible disk cache layout
//! (`cache/game_images/appid_<id>.jpg`, name-keyed legacy stems, existing
//! caches carry over between the apps).
//!
//! Sources, in order: Steam CDN header.jpg → Steam Store API
//! (header_image / capsule_image). Downloads are resized to max 300×450 and
//! saved as JPEG q85, matching the Python pipeline.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_SIZE: (u32, u32) = (300, 450);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10);
const CACHE_EXTENSIONS: &[&str] = &["jpg", "png", "jpeg", "webp"];

pub struct ImageCache {
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new(cache_dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(cache_dir);
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Sanitize a game name the way Python does for legacy name-keyed files.
    fn safe_name(game_name: &str) -> String {
        game_name
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                other => other,
            })
            .collect()
    }

    /// Cached image path if one exists (appid stem preferred, then name stem).
    pub fn cached_path(&self, game_name: &str, appid: Option<u32>) -> Option<PathBuf> {
        let mut stems = Vec::new();
        if let Some(appid) = appid {
            stems.push(format!("appid_{appid}"));
        }
        stems.push(Self::safe_name(game_name));
        for stem in stems {
            for ext in CACHE_EXTENSIONS {
                let candidate = self.cache_dir.join(format!("{stem}.{ext}"));
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Fetch (or return cached) artwork for a game. Blocking — run on a
    /// worker thread. Returns None when no appid or all sources fail.
    pub fn fetch(&self, game_name: &str, appid: Option<u32>) -> Option<PathBuf> {
        if let Some(cached) = self.cached_path(game_name, appid) {
            return Some(cached);
        }
        let appid = appid?;

        // Primary: Steam CDN header
        let cdn_url = format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg");
        if let Some(path) = self.download_and_cache(&cdn_url, appid) {
            return Some(path);
        }

        // Fallback: Store API for the actual hosted image URL
        let store_url =
            format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic");
        let body = http_get(&store_url)?;
        let data: serde_json::Value = serde_json::from_slice(&body).ok()?;
        let app_info = data.get(appid.to_string())?;
        if app_info.get("success") != Some(&serde_json::Value::Bool(true)) {
            return None;
        }
        for key in ["header_image", "capsule_image"] {
            if let Some(url) = app_info
                .get("data")
                .and_then(|d| d.get(key))
                .and_then(serde_json::Value::as_str)
            {
                if let Some(path) = self.download_and_cache(url, appid) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn download_and_cache(&self, url: &str, appid: u32) -> Option<PathBuf> {
        let bytes = http_get(url)?;
        let img = image::load_from_memory(&bytes).ok()?;
        let img = img.thumbnail(MAX_SIZE.0, MAX_SIZE.1);
        let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
        let out_path = self.cache_dir.join(format!("appid_{appid}.jpg"));
        let mut out = std::fs::File::create(&out_path).ok()?;
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
        rgb.write_with_encoder(encoder).ok()?;
        Some(out_path)
    }
}

/// Shared HTTP agent. ureq 3 defaults to the Rustls provider even when built
/// with the native-tls feature — the provider must be selected explicitly or
/// every https call panics at runtime.
pub(crate) fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .into()
}

fn http_get(url: &str) -> Option<Vec<u8>> {
    let mut resp = http_agent().get(url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }
    let mut bytes = Vec::new();
    resp.body_mut()
        .as_reader()
        .take(20 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_path_prefers_appid_stem() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ImageCache::new(tmp.path());
        std::fs::write(tmp.path().join("appid_123.jpg"), b"x").unwrap();
        std::fs::write(tmp.path().join("Some Game.png"), b"x").unwrap();
        assert_eq!(
            cache.cached_path("Some Game", Some(123)).unwrap(),
            tmp.path().join("appid_123.jpg")
        );
        assert_eq!(
            cache.cached_path("Some Game", None).unwrap(),
            tmp.path().join("Some Game.png")
        );
    }

    #[test]
    fn sanitizes_name_stems() {
        assert_eq!(
            ImageCache::safe_name("The Witcher 3: Wild Hunt"),
            "The Witcher 3_ Wild Hunt"
        );
    }

    #[test]
    fn missing_cache_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ImageCache::new(tmp.path());
        assert!(cache.cached_path("Nothing", Some(1)).is_none());
    }
}
