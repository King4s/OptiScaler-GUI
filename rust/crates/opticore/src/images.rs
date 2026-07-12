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

    /// Fetch (or return cached) artwork for a game via every source we have,
    /// in order of quality. Blocking — run on a worker thread. The exe-icon
    /// last resort means every game gets *something*.
    pub fn fetch(&self, request: &ArtRequest) -> Option<PathBuf> {
        if let Some(cached) = self.cached_path(&request.name, request.appid) {
            return Some(cached);
        }

        // 1. Steam CDN / Store API when we have an appid
        if let Some(appid) = request.appid {
            if let Some(path) = self.fetch_steam(appid) {
                return Some(path);
            }
        }

        // 2. Store-supplied art URL (Heroic library metadata)
        if let Some(url) = &request.art_url {
            if let Some(path) = self.download_and_cache(url, &self.name_stem(&request.name)) {
                return Some(path);
            }
        }

        // 3. GOG public search API for GOG installs
        if request.platform_is_gog {
            if let Some(url) = gog_search_image(&request.name) {
                if let Some(path) = self.download_and_cache(&url, &self.name_stem(&request.name)) {
                    return Some(path);
                }
            }
        }

        // 4. Xbox/Game Pass: the game ships its own store logos on disk
        if let Some(game_path) = &request.game_path {
            if let Some(local) = xbox_local_logo(game_path) {
                if let Some(path) = self.cache_local_image(&local, &self.name_stem(&request.name)) {
                    return Some(path);
                }
            }
        }

        // 5. Last resort: the game executable's own icon
        if let Some(game_path) = &request.game_path {
            if let Some(icon) = exe_icon_image(game_path) {
                let out_path = self
                    .cache_dir
                    .join(format!("{}.png", self.name_stem(&request.name)));
                if icon.save(&out_path).is_ok() {
                    return Some(out_path);
                }
            }
        }

        None
    }

    fn name_stem(&self, game_name: &str) -> String {
        Self::safe_name(game_name)
    }

    fn fetch_steam(&self, appid: u32) -> Option<PathBuf> {
        // Primary: Steam CDN header
        let cdn_url = format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg");
        let appid_stem = format!("appid_{appid}");
        if let Some(path) = self.download_and_cache(&cdn_url, &appid_stem) {
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
                if let Some(path) = self.download_and_cache(url, &appid_stem) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn encode_to_cache(&self, img: image::DynamicImage, stem: &str) -> Option<PathBuf> {
        let img = img.thumbnail(MAX_SIZE.0, MAX_SIZE.1);
        let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
        let out_path = self.cache_dir.join(format!("{stem}.jpg"));
        let mut out = std::fs::File::create(&out_path).ok()?;
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
        rgb.write_with_encoder(encoder).ok()?;
        Some(out_path)
    }

    fn download_and_cache(&self, url: &str, stem: &str) -> Option<PathBuf> {
        // Heroic/GOG URLs are sometimes protocol-relative
        let url = if url.starts_with("//") {
            format!("https:{url}")
        } else {
            url.to_string()
        };
        let bytes = http_get(&url)?;
        let img = image::load_from_memory(&bytes).ok()?;
        self.encode_to_cache(img, stem)
    }

    fn cache_local_image(&self, source: &Path, stem: &str) -> Option<PathBuf> {
        let bytes = std::fs::read(source).ok()?;
        let img = image::load_from_memory(&bytes).ok()?;
        self.encode_to_cache(img, stem)
    }
}

/// Everything the artwork pipeline can use for one game.
pub struct ArtRequest {
    pub name: String,
    pub appid: Option<u32>,
    pub art_url: Option<String>,
    pub platform_is_gog: bool,
    pub game_path: Option<PathBuf>,
}

/// GOG's public catalogue search (no auth). Name-verified like the Steam
/// store search so a wrong game's art can't win.
fn gog_search_image(name: &str) -> Option<String> {
    fn norm(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    let encoded: String = name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    let url = format!("https://embed.gog.com/games/ajax/filtered?mediaType=game&search={encoded}");
    let body = http_get(&url)?;
    let data: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let query_norm = norm(name);
    for product in data.get("products").and_then(serde_json::Value::as_array)? {
        let title = product
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if norm(title) != query_norm {
            continue;
        }
        if let Some(image) = product.get("image").and_then(serde_json::Value::as_str) {
            return Some(format!("{image}.jpg"));
        }
    }
    None
}

/// Xbox/Game Pass titles ship their store logos inside the install:
/// MicrosoftGame.config points at Square480x480Logo/SplashScreen/StoreLogo
/// assets. Search the config first (targeted — game content dirs are full of
/// unrelated PNGs), preferring the larger art.
fn xbox_local_logo(game_path: &Path) -> Option<PathBuf> {
    let config_path = find_shallow(game_path, "MicrosoftGame.config", 2)?;
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config_dir = config_path.parent()?;
    for attribute in [
        "Square480x480Logo",
        "SplashScreenImage",
        "StoreLogo",
        "Square150x150Logo",
    ] {
        if let Some(value) = xml_attr_or_tag(&content, attribute) {
            let candidate = config_dir.join(value.replace('\\', "/"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Pull `Name="value"` attribute or `<Name>value</Name>` element text.
fn xml_attr_or_tag(content: &str, name: &str) -> Option<String> {
    // attribute form
    let attr_needle = format!("{name}=\"");
    if let Some(pos) = content.find(&attr_needle) {
        let rest = &content[pos + attr_needle.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    // element form
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].trim().to_string())
}

/// Find a file by exact name within `max_depth` directory levels.
fn find_shallow(root: &Path, file_name: &str, max_depth: usize) -> Option<PathBuf> {
    let direct = root.join(file_name);
    if direct.is_file() {
        return Some(direct);
    }
    if max_depth == 0 {
        return None;
    }
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_shallow(&path, file_name, max_depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Extract the largest icon from the game's main executable (PE resources).
/// The universal fallback: every game has an exe, every exe has an icon.
fn exe_icon_image(game_path: &Path) -> Option<image::DynamicImage> {
    let exe = largest_exe(game_path)?;
    let bytes = std::fs::read(&exe).ok()?;
    let file = pelite::PeFile::from_bytes(&bytes).ok()?;
    let resources = file.resources().ok()?;
    // First RT_GROUP_ICON entry, assembled into a .ico blob
    for (_, group) in resources.icons().flatten() {
        let mut ico = Vec::new();
        group.write(&mut ico).ok()?;
        if let Ok(img) = image::load_from_memory_with_format(&ico, image::ImageFormat::Ico) {
            return Some(img);
        }
    }
    None
}

/// The game's main exe: largest top-level exe, else largest within 3 levels.
fn largest_exe(game_path: &Path) -> Option<PathBuf> {
    fn collect(dir: &Path, depth: usize, out: &mut Vec<(u64, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth > 0 {
                    collect(&path, depth - 1, out);
                }
            } else if path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
            {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((size, path));
            }
        }
    }
    let mut exes = Vec::new();
    collect(game_path, 3, &mut exes);
    exes.sort();
    exes.pop().map(|(_, path)| path)
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
