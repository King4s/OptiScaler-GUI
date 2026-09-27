//! Game artwork fetching with the Python-compatible disk cache layout
//! (`cache/game_images/appid_<id>.jpg`, name-keyed legacy stems, existing
//! caches carry over between the apps).
//!
//! Sources, in order: an already cached file, Steam's own local library cache
//! ([`crate::steam_art`]), the Steam store's portrait for the app id (GetItems),
//! Steam CDN header.jpg, Steam Store API (header_image / capsule_image), a
//! store-supplied art URL, GOG's public search, Xbox in-install logos, and finally
//! the executable's own icon. Downloads are resized to max 300×450 and saved as
//! JPEG q85, matching the Python pipeline. Every request this cache makes —
//! Steam, the store, GOG's search and every image download — goes through the
//! injectable [`Fetcher`], so failures and hostile bodies are testable without a
//! network, and every body it returns is bounded before it is parsed or decoded.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_SIZE: (u32, u32) = (300, 450);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10);
const CACHE_EXTENSIONS: &[&str] = &["jpg", "png", "jpeg", "webp"];

/// The HTTP fetch this cache uses, injectable so tests can supply mock responses,
/// failures and oversized bodies, and so the order tests need no network.
pub type Fetcher = std::sync::Arc<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>;

pub struct ImageCache {
    cache_dir: PathBuf,
    /// Steam's per-install `appcache/librarycache` directories, resolved once so
    /// artwork lookup does not re-read the registry for every game.
    steam_library_caches: Vec<PathBuf>,
    fetcher: Fetcher,
}

impl ImageCache {
    /// Uses the Steam installations found on this machine.
    /// [`ImageCache::with_sources`] is the seam callers and tests use.
    pub fn new(cache_dir: &Path) -> Self {
        Self::with_library_caches(cache_dir, crate::steam_art::library_caches())
    }

    pub fn with_library_caches(cache_dir: &Path, steam_library_caches: Vec<PathBuf>) -> Self {
        Self::with_sources(
            cache_dir,
            steam_library_caches,
            std::sync::Arc::new(http_get),
        )
    }

    /// Full injection: the local library caches and the HTTP fetch.
    pub fn with_sources(
        cache_dir: &Path,
        steam_library_caches: Vec<PathBuf>,
        fetcher: Fetcher,
    ) -> Self {
        let _ = std::fs::create_dir_all(cache_dir);
        Self {
            cache_dir: cache_dir.to_path_buf(),
            steam_library_caches,
            fetcher,
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

        // 1. Steam's own library cache: offline, the right aspect ratio, and
        //    keyed by the exact app id. Tried before the CDN so a Steam game gets
        //    its portrait without a network round trip.
        if let Some(appid) = request.appid {
            if let Some(path) = self.local_steam_portrait(appid) {
                return Some(path);
            }
        }

        // 2. Steam CDN / Store API when we have an appid
        if let Some(appid) = request.appid {
            if let Some(path) = self.fetch_steam(appid) {
                return Some(path);
            }
        }

        // 3. Store-supplied art URL (Heroic library metadata)
        if let Some(url) = &request.art_url {
            if let Some(path) = self.download_and_cache(url, &self.name_stem(&request.name)) {
                return Some(path);
            }
        }

        // 4. GOG public search API for GOG installs
        if request.platform_is_gog {
            if let Some(url) = gog_search_image(&self.fetcher, &request.name) {
                if let Some(path) = self.download_and_cache(&url, &self.name_stem(&request.name)) {
                    return Some(path);
                }
            }
        }

        // 5. Xbox/Game Pass: the game ships its own store logos on disk
        if let Some(game_path) = &request.game_path {
            if let Some(local) = xbox_local_logo(game_path) {
                if let Some(path) = self.cache_local_image(&local, &self.name_stem(&request.name)) {
                    return Some(path);
                }
            }
        }

        // 6. Last resort: the game executable's own icon
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

    /// Steam's own locally cached portrait for this app id, stored in our cache
    /// layout so the next run finds it without touching Steam again.
    fn local_steam_portrait(&self, appid: u32) -> Option<PathBuf> {
        let source = crate::steam_art::portrait_in(&self.steam_library_caches, appid)?;
        let img = crate::steam_art::decode_portrait(&source)?;
        self.encode_to_cache(img, &format!("appid_{appid}"))
    }

    fn fetch_steam(&self, appid: u32) -> Option<PathBuf> {
        let appid_stem = format!("appid_{appid}");

        // Primary: the store's own portrait for this app id, when it has one
        if let Some(url) = self.store_portrait_url(appid) {
            if let Some(path) = self.download_and_cache(&url, &appid_stem) {
                return Some(path);
            }
        }

        // Fallback: Steam CDN header
        let cdn_url = format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg");
        if let Some(path) = self.download_and_cache(&cdn_url, &appid_stem) {
            return Some(path);
        }

        // Fallback: Store API for the actual hosted image URL.
        //
        // The rung answers with the artwork of the app id this game was looked up for, or
        // with nothing, and its own candidates are tried in order: a header whose bytes
        // cannot be downloaded still leaves the capsule beside it, which is what this rung
        // did before the parsing moved into `appdetails_image_urls`.
        let body = (self.fetcher)(&appdetails_url(appid))?;
        for url in appdetails_image_urls(&body, appid) {
            if let Some(path) = self.download_and_cache(&url, &appid_stem) {
                return Some(path);
            }
        }
        None
    }

    /// The store's portrait URL for this app id, when the store has one.
    fn store_portrait_url(&self, appid: u32) -> Option<String> {
        let body = (self.fetcher)(&store_items_url(appid))?;
        store_item_portrait_url(&body, appid)
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
        let bytes = (self.fetcher)(&url)?;
        let img = crate::steam_art::decode_portrait_bytes(&bytes)?;
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
fn gog_search_image(fetcher: &Fetcher, name: &str) -> Option<String> {
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
    let body = fetcher(&url)?;
    if !within_response_bound(&body) {
        return None;
    }
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
/// Shared with `crate::launch` as the direct-start fallback.
pub(crate) fn largest_exe(game_path: &Path) -> Option<PathBuf> {
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
        // No redirects. A redirect is the one way a file that was fetched for one artwork
        // URL can belong to an app id other than the one in that URL, and nothing here can
        // see where the request ended up, so a redirected asset is a miss instead of a
        // cover. Measured on 2026-09-27: the live store API, the GetItems API and 84 real
        // asset URLs all answer 200 directly, so this costs nothing today.
        .max_redirects(0)
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .into()
}

/// A store response this cache will parse. The bodies involved are kilobytes —
/// the captured GetItems response for one app is 1.8 KB — so this is far above
/// anything real, and it is the bound the specification promises for a response,
/// separate from the bound on decoded image bytes.
const MAX_STORE_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

/// Whether a response body is small enough to parse at all.
fn within_response_bound(body: &[u8]) -> bool {
    body.len() as u64 <= MAX_STORE_RESPONSE_BYTES
}

/// Where a store asset path is served from. `asset_url_format` in a GetItems
/// response is host-less, and this is the host the existing header fetch uses.
const STORE_ASSET_BASE: &str = "https://cdn.akamai.steamstatic.com/";

/// The GetItems request for one app id.
///
/// Verified live for app 620 on 2026-09-26: this encoded form answers
/// `success: 1` with `assets.library_capsule = "library_600x900.jpg"`, and the
/// resulting portrait URL answers 200 with `image/jpeg`.
pub fn store_items_url(appid: u32) -> String {
    let input = format!(
        r#"{{"ids":[{{"appid":{appid}}}],"context":{{"language":"english","country_code":"US"}},"data_request":{{"include_assets":true}}}}"#
    );
    format!(
        "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}&format=json",
        percent_encode(&input)
    )
}

/// Percent-encodes a query-string value: everything outside the unreserved set.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 3);
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Whether an item reports itself as resolved.
///
/// `success` is documented as a flag but is an integer bitfield in practice:
/// measured live, 1 for app 620 and 15 for an app id that does not exist (whose
/// item also carries `appid: 0`, which is what the equality check in
/// [`store_item_portrait_url`] rejects). What counts as reported: a non-zero
/// integer, or a boolean `true`. What does not: a missing field, `false`, `0`, a
/// float such as `1.0` (a number is read as an `i64`), and a string. Both the
/// boolean arm and the numeric boundary are pinned by a test.
fn reports_success(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(flag)) => *flag,
        Some(serde_json::Value::Number(number)) => number.as_i64().is_some_and(|n| n != 0),
        _ => false,
    }
}

/// The portrait URL for `appid`, read out of a GetItems response body.
///
/// The body is untrusted. The item must be the app id we asked about and must
/// report success, and both the format string and the portrait file name must be
/// present, before `${FILENAME}` in the format is replaced. Anything else yields
/// nothing: no neighbouring item, no sibling asset key, no format string that
/// does not ask for a file name.
pub fn store_item_portrait_url(body: &[u8], appid: u32) -> Option<String> {
    if !within_response_bound(body) {
        return None;
    }
    let data: serde_json::Value = serde_json::from_slice(body).ok()?;
    let items = data.get("response")?.get("store_items")?.as_array()?;
    let item = items.iter().find(|item| {
        item.get("appid").and_then(serde_json::Value::as_u64) == Some(u64::from(appid))
    })?;
    if !reports_success(item.get("success")) {
        return None;
    }
    let assets = item.get("assets")?;
    let format = assets
        .get("asset_url_format")
        .and_then(serde_json::Value::as_str)?;
    let portrait = assets
        .get("library_capsule")
        .and_then(serde_json::Value::as_str)?;
    if format.is_empty() || portrait.is_empty() {
        return None;
    }
    let filled = format.replace("${FILENAME}", portrait);
    if filled == format {
        return None;
    }
    Some(format!("{STORE_ASSET_BASE}{filled}"))
}

/// The appdetails request for one app id.
///
/// The same request the rung made inline before this was extracted, apart from the
/// app id itself: `filters=basic` already carries `steam_appid`, `header_image` and
/// `capsule_image`, which is everything [`appdetails_image_url`] reads.
pub fn appdetails_url(appid: u32) -> String {
    format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic")
}

/// The image URLs for `appid`, in the order the rung should try them.
///
/// The body is untrusted, and the response is keyed the way the store chooses
/// rather than by the id that was asked for: measured live on 2026-09-27,
/// `appids=620` answers under the key `323180` with Portal 2's payload, so reading
/// it by `appid.to_string()` finds nothing for most games. The entry key is
/// therefore ignored and every entry is considered.
///
/// Identity is required twice before a candidate is used, because the key is not
/// evidence: `data.steam_appid` must equal `appid` (the store's own label for the
/// payload) *and* the URL's path must hold `appid` directly after an `apps` segment
/// (the path the bytes are served from, and the app id the result is cached under).
/// The second measured shape is why both are needed: `appids=100` answers under the
/// key `100` with `steam_appid` 80, app 80's name and app 80's header and capsule
/// images. Either check alone rejects that body; neither may be dropped, so that a
/// label and a path which disagree can never hand one game another game's cover.
///
/// `header_image` comes before `capsule_image`; a missing, non-string or unusable
/// field contributes nothing and the next one is tried. The list is what lets a
/// caller keep the rung's own fallback: when the header cannot be downloaded, the
/// capsule beside it is still worth a try.
pub fn appdetails_image_urls(body: &[u8], appid: u32) -> Vec<String> {
    let mut urls = Vec::new();
    if !within_response_bound(body) {
        return urls;
    }
    let Ok(data) = serde_json::from_slice::<serde_json::Value>(body) else {
        return urls;
    };
    let Some(entries) = data.as_object() else {
        return urls;
    };
    for entry in entries.values() {
        if !reports_success(entry.get("success")) {
            continue;
        }
        let Some(payload) = entry.get("data") else {
            continue;
        };
        if payload
            .get("steam_appid")
            .and_then(serde_json::Value::as_u64)
            != Some(u64::from(appid))
        {
            continue;
        }
        for key in ["header_image", "capsule_image"] {
            let Some(url) = payload.get(key).and_then(serde_json::Value::as_str) else {
                continue;
            };
            if steam_app_path_names(url, appid) && !urls.iter().any(|seen| seen == url) {
                urls.push(url.to_string());
            }
        }
    }
    urls
}

/// The first image URL for `appid`: its `header_image` when that is usable, otherwise the
/// `capsule_image` beside it, otherwise nothing.
///
/// See [`appdetails_image_urls`] for the rules the candidate has to satisfy and for the
/// two live shapes they are built on.
pub fn appdetails_image_url(body: &[u8], appid: u32) -> Option<String> {
    appdetails_image_urls(body, appid).into_iter().next()
}

/// Whether an image URL is an absolute https URL whose path holds `appid` as a whole
/// segment directly after an `apps` segment.
///
/// The app id is looked for in the **path only**, after the authority has been taken
/// off, so a host that happens to be spelled `apps` cannot stand in for a path
/// segment. The URL has to be absolute and its scheme `https`, compared
/// case-insensitively — schemes are case-insensitive, and `HTTPS://…` from the store
/// is still the store. Everything the URI carries is read literally, so the path may
/// only consist of the plain, non-empty segments Steam's asset paths actually use: a
/// `.` or `..` segment, a percent-encoded one, an empty one between double slashes,
/// a space or a userinfo part in the authority are all refused rather than resolved.
/// `…/apps/620/../80/header.jpg` is the reason: an HTTP client normalises it to app
/// 80's artwork, which would be cached under this game's id.
fn steam_app_path_names(url: &str, appid: u32) -> bool {
    // The whole URI is read literally, query and fragment included, and every byte must be
    // one a URI may contain (RFC 3986's unreserved, reserved and percent characters). A
    // space, a `<`, a `>`, a quote, a backslash, a backtick, a brace or any control or
    // non-ASCII byte is not part of a URI, and the locked downloader refuses such a URL
    // before the request leaves the process.
    if !url.bytes().all(is_uri_character) {
        return false;
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if !authority_is_plain(authority) {
        return false;
    }
    // The path says which app id a URL claims to serve; it says nothing about who serves it.
    // Shape alone cannot tell the store's CDN from anyone else's `…/steam/apps/620/…`, so
    // the host is pinned to the store's own artwork domain — otherwise the bytes of whatever
    // host the response names would be written as this game's cover.
    if !host_is_the_stores_artwork_cdn(authority) {
        return false;
    }
    let Some(after_authority) = rest[authority.len()..].strip_prefix('/') else {
        return false;
    };
    let path = after_authority.split(['?', '#']).next().unwrap_or_default();
    if path.is_empty() || !path_is_plain(path) {
        return false;
    }
    let wanted = appid.to_string();
    let mut segments = path.split('/');
    let mut previous = segments.next().unwrap_or_default();
    for segment in segments {
        if previous == "apps" && segment == wanted {
            return true;
        }
        previous = segment;
    }
    false
}

/// Whether an authority is one of the store's own artwork hosts.
///
/// The path rule says which app id a URL claims to serve; it says nothing about who serves
/// it. Without this check a response naming any host would have that host's bytes written
/// under this game's id — `https://evil.example/steam/apps/620/header.jpg` is exactly that
/// — so the authority must be the store's own domain: `steamstatic.com` itself or a
/// subdomain of it, which is where every piece of artwork this pipeline has ever fetched
/// lives (`shared.akamai.steamstatic.com` in the live corpus,
/// `cdn.akamai.steamstatic.com` in the URL the CDN rung builds). A lookalike such as
/// `steamstatic.com.evil.example` or `notsteamstatic.com` does not end with that and is
/// refused.
fn host_is_the_stores_artwork_cdn(authority: &str) -> bool {
    let host = authority.split(':').next().unwrap_or_default();
    // Host names are case-insensitive — `Shared.Akamai.Steamstatic.com` is the same host as
    // the lowercase spelling, and the downloader fetches it — so the comparison is too, just
    // like the scheme's. A lookalike written in any case still fails it.
    let host = host.to_ascii_lowercase();
    host == "steamstatic.com" || host.ends_with(".steamstatic.com")
}

/// Whether the authority is a host name with an optional port — and nothing else.
///
/// A character check is not enough: `https://:/path` has an empty host and
/// `https://host:abc/path` has a port that is not a number, and neither is a destination
/// this pipeline may follow. The host must be dotted labels of letters, digits and
/// hyphens with no empty or hyphen-edged label (`shared.akamai.steamstatic.com` is the
/// shape the store serves its artwork from), the port at most five digits. Brackets,
/// userinfo, an underscore and an IPv6 literal are all refused: Steam serves no artwork
/// from any of them, and a URL this code cannot read literally is a miss, never a guess.
fn authority_is_plain(authority: &str) -> bool {
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    // A host of nothing but digits and dots is an address literal, and the store serves its
    // artwork from names. Refusing it keeps the question of whether the address is valid —
    // `999.999.999.999` is a host this pipeline would otherwise follow — out of this code.
    if host
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return false;
    }
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == '-')
        })
        && port.is_none_or(|port| {
            port.chars().all(|byte| byte.is_ascii_digit()) && port.parse::<u16>().is_ok()
        })
}

/// Whether every path segment is one this pipeline will follow: non-empty, not a dot
/// segment (the HTTP client would normalise those away and could land on another app's
/// file), built from the characters Steam's asset paths use, and free of percent escapes
/// that could become something else.
fn path_is_plain(path: &str) -> bool {
    path.split('/').all(segment_is_plain)
}

/// Whether one path segment is one this pipeline will follow.
///
/// A `%` escape is judged by what it stands for, not by the fact that it is written out:
/// `%2e` is a dot, `%2f` a slash, `%5c` a backslash and `%25` a percent sign, so a segment
/// holding one of those can be normalised — or decoded a second time by whoever serves it —
/// into a different path than the one that was checked, which is the one thing these rules
/// exist to prevent. Any other escape is just a byte of a file name: `%68eader.jpg` *is*
/// `header.jpg`, the downloader fetches it like any other URL, and refusing it would cost a
/// cover for nothing. A truncated or non-hex escape is refused, because it is not a URL a
/// client can send at all.
fn segment_is_plain(segment: &str) -> bool {
    if segment.is_empty() || segment == "." || segment == ".." {
        return false;
    }
    let bytes = segment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let Some(escape) = bytes.get(index + 1..index + 3) else {
                return false;
            };
            if !escape.iter().all(|byte| byte.is_ascii_hexdigit()) {
                return false;
            }
            let (first, second) = (
                escape[0].to_ascii_lowercase(),
                escape[1].to_ascii_lowercase(),
            );
            if matches!(
                (first, second),
                (b'2', b'e' | b'f') | (b'5', b'c') | (b'2', b'5')
            ) {
                return false;
            }
            index += 3;
            continue;
        }
        if !(bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b'_' | b'.')) {
            return false;
        }
        index += 1;
    }
    true
}

/// Whether a byte is one a URI may contain: RFC 3986's unreserved and reserved characters
/// plus the percent sign that introduces an escape. Everything else — a space, `<`, `>`,
/// `"`, `\`, `^`, a backtick, `{`, `|`, `}`, a DEL, a control byte, any non-ASCII byte — is
/// not, which is what the downloader's URI parser enforces too.
fn is_uri_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b':'
                | b'/'
                | b'?'
                | b'#'
                | b'['
                | b']'
                | b'@'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'%'
        )
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
        let cache = ImageCache::with_library_caches(tmp.path(), Vec::new());
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
    fn the_artwork_downloader_follows_no_redirects() {
        // A redirect is invisible to every caller here: whatever the response holds would be
        // cached under the app id of the URL that was checked, not the app id it came from.
        // The rule therefore lives in the agent's configuration rather than in a check on a
        // response that cannot see where it came from — with no redirects followed, a 3xx is
        // returned as-is and `http_get` accepts only 200.
        assert_eq!(http_agent().config().max_redirects(), 0);
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
        let cache = ImageCache::with_library_caches(tmp.path(), Vec::new());
        assert!(cache.cached_path("Nothing", Some(1)).is_none());
    }
}
