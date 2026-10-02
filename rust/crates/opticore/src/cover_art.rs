//! Per-install cover art with explicit provenance and user-owned overrides.
//! Blocking operations belong on a worker thread, never in a paint/render path.

use crate::images::{self, Fetcher, ImageCache};
use crate::model::{Game, Platform};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
struct State {
    source: String,
    file: Option<String>,
}

/// An append-only per-install cache. Old Python cache entries remain readable.
pub struct CoverCache {
    cache_dir: PathBuf,
    steam_library_caches: Vec<PathBuf>,
    fetcher: Fetcher,
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl CoverCache {
    pub fn new(cache_dir: &Path) -> Self {
        Self::with_sources(
            cache_dir,
            crate::steam_art::library_caches(),
            std::sync::Arc::new(images::http_get),
        )
    }

    /// Inject both local Steam roots and transport for offline tests.
    pub fn with_sources(
        cache_dir: &Path,
        steam_library_caches: Vec<PathBuf>,
        fetcher: Fetcher,
    ) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            steam_library_caches,
            fetcher,
            locks: Mutex::new(HashMap::new()),
        }
    }

    /// Returns cached art or performs one bounded resolution attempt. Blocking.
    pub fn fetch(&self, game: &Game) -> Option<PathBuf> {
        let dir = self.game_dir(game);
        let lock = self.entry_lock(&dir).ok()?;
        let _guard = lock.lock().ok()?;
        if let Some(state) = read_state(&dir) {
            if state.source != "cleared" {
                if let Some(path) = self.state_path(game, &dir, &state) {
                    return Some(path);
                }
                if state.source == "miss" {
                    return None;
                }
            }
        }
        self.resolve(game, &dir)
    }

    /// Explicitly retries automatic sources. Selected images and last-good
    /// automatic covers are retained unless an automatic candidate improves them.
    pub fn refresh(&self, game: &Game) -> Option<PathBuf> {
        let dir = self.game_dir(game);
        let lock = self.entry_lock(&dir).ok()?;
        let _guard = lock.lock().ok()?;
        if let Some(state) = read_state(&dir) {
            if matches!(state.source.as_str(), "user override" | "steamgriddb") {
                if let Some(path) = self.state_path(game, &dir, &state) {
                    return Some(path);
                }
            }
        }
        self.resolve(game, &dir)
    }

    pub fn source(&self, game: &Game) -> Option<String> {
        let dir = self.game_dir(game);
        if let Some(state) = read_state(&dir) {
            return self.state_path(game, &dir, &state).map(|_| state.source);
        }
        self.legacy_path(game).map(|_| "legacy cache".to_string())
    }

    /// Copies and validates a user-selected image; the original is untouched.
    pub fn set_override(&self, game: &Game, path: &Path) -> Result<PathBuf, String> {
        let lock = self.entry_lock(&self.game_dir(game))?;
        let _guard = lock.lock().map_err(|_| "cover lock poisoned".to_string())?;
        let metadata = fs::metadata(path).map_err(|_| "cover file unavailable".to_string())?;
        if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES as u64 {
            return Err("invalid cover size".into());
        }
        let bytes = read_image_file(path).map_err(|_| "cover file unavailable".to_string())?;
        self.save_selected(game, &bytes, "user override")
            .map_err(|_| "could not save cover".to_string())
    }

    /// Clears selection without deleting any cached file.
    pub fn clear_override(&self, game: &Game) -> Result<(), String> {
        let lock = self.entry_lock(&self.game_dir(game))?;
        let _guard = lock.lock().map_err(|_| "cover lock poisoned".to_string())?;
        let dir = self.game_dir(game);
        if read_state(&dir)
            .is_some_and(|s| matches!(s.source.as_str(), "user override" | "steamgriddb"))
        {
            write_state(
                &dir,
                &State {
                    source: "cleared".into(),
                    file: None,
                },
            )
            .map_err(|_| "could not clear cover selection".to_string())?;
        }
        Ok(())
    }

    /// Clears only recognized automatic v2 entries. Legacy root files and any
    /// directory with a selected-art record (even a historical one) are retained.
    pub fn clear_automatic_cache(&self) -> Result<(), String> {
        let root = self.cache_dir.join("cover-v2");
        for directory in [&self.cache_dir, &root] {
            match fs::symlink_metadata(directory) {
                Ok(metadata) if metadata.is_dir() && !is_reparse_or_symlink(&metadata) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                _ => return Err("refusing linked or inaccessible cover cache".into()),
            }
        }
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err("could not read cover cache".into()),
        };
        for entry in entries.flatten() {
            let metadata =
                fs::symlink_metadata(entry.path()).map_err(|_| "could not inspect cover cache")?;
            if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            let lock = self.entry_lock(&entry.path())?;
            let _guard = lock.lock().map_err(|_| "cover lock poisoned")?;
            let mut files = Vec::new();
            let mut owned_images = HashSet::new();
            let mut state_count = 0;
            let mut preserve = false;
            for file in fs::read_dir(entry.path()).map_err(|_| "could not read cover entry")? {
                let file = file.map_err(|_| "could not read cover entry")?;
                let metadata = fs::symlink_metadata(file.path())
                    .map_err(|_| "could not inspect cover entry")?;
                if !metadata.is_file() || is_reparse_or_symlink(&metadata) {
                    preserve = true;
                    break;
                }
                let name = file.file_name().to_string_lossy().into_owned();
                if generated_name(&name, "state", "json") {
                    let bytes = read_bounded(&file.path(), 4096)
                        .map_err(|_| "could not read cover state")?;
                    let Ok(state) = serde_json::from_slice::<State>(&bytes) else {
                        preserve = true;
                        break;
                    };
                    if matches!(state.source.as_str(), "user override" | "steamgriddb") {
                        preserve = true;
                        break;
                    }
                    if !matches!(
                        state.source.as_str(),
                        "cleared"
                            | "legacy cache"
                            | "miss"
                            | "steam local"
                            | "store supplied"
                            | "steam store"
                            | "steam header"
                            | "steam appdetails"
                            | "xbox local"
                            | "exe icon"
                    ) {
                        preserve = true;
                        break;
                    }
                    if let Some(image) = state.file {
                        if !generated_name(&image, "image", "jpg") {
                            preserve = true;
                            break;
                        }
                        owned_images.insert(image);
                    }
                    state_count += 1;
                } else if !generated_name(&name, "image", "jpg") {
                    preserve = true;
                    break;
                }
                files.push(file.path());
            }
            if files.iter().any(|path| {
                let name = path.file_name().unwrap().to_string_lossy();
                generated_name(&name, "image", "jpg") && !owned_images.contains(name.as_ref())
            }) {
                preserve = true;
            }
            if !preserve && state_count > 0 {
                for file in files {
                    // Recheck the directory and file immediately before deletion.
                    for path in [&self.cache_dir, &root, &entry.path(), &file] {
                        let metadata = fs::symlink_metadata(path)
                            .map_err(|_| "cover cache changed while clearing")?;
                        if is_reparse_or_symlink(&metadata) {
                            return Err("refusing linked cover cache entry".into());
                        }
                    }
                    fs::remove_file(file).map_err(|_| "could not clear cover cache")?;
                }
            }
        }
        Ok(())
    }

    fn entry_lock(&self, dir: &Path) -> Result<Arc<Mutex<()>>, String> {
        let mut locks = self.locks.lock().map_err(|_| "cover lock poisoned")?;
        Ok(locks
            .entry(dir.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    fn save_selected(&self, game: &Game, bytes: &[u8], source: &str) -> io::Result<PathBuf> {
        let dir = self.game_dir(game);
        let path = save_image(&dir, bytes)?;
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        write_state(
            &dir,
            &State {
                source: source.into(),
                file: Some(file),
            },
        )?;
        Ok(path)
    }

    fn resolve(&self, game: &Game, dir: &Path) -> Option<PathBuf> {
        let legacy = self.legacy_path(game);
        if legacy.as_ref().is_some_and(|path| is_portrait_file(path)) {
            return publish_automatic(
                dir,
                State {
                    source: "legacy cache".into(),
                    file: None,
                },
                legacy,
            );
        }

        if let Some(appid) = verified_steam_appid(game) {
            if let Some(path) = crate::steam_art::portrait_in(&self.steam_library_caches, appid) {
                if let Some(result) = self.save_from_path(dir, &path, "steam local") {
                    return Some(result);
                }
            }
        }
        if game.platform != Platform::Steam {
            if let Some(result) = self.store_supplied(game, dir) {
                return Some(result);
            }
        }
        if let Some(appid) = verified_steam_appid(game) {
            let body = (self.fetcher)(&images::store_items_url(appid));
            if let Some(url) = body
                .as_deref()
                .and_then(|body| images::store_item_portrait_url(body, appid))
            {
                if let Some(result) = self.download(dir, &url, "steam store") {
                    return Some(result);
                }
            }
            if let Some(result) = self.store_supplied(game, dir) {
                return Some(result);
            }
            let header =
                format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg");
            if let Some(result) = self.download(dir, &header, "steam header") {
                return Some(result);
            }
            if let Some(body) = (self.fetcher)(&images::appdetails_url(appid)) {
                for url in images::appdetails_image_urls(&body, appid) {
                    if let Some(result) = self.download(dir, &url, "steam appdetails") {
                        return Some(result);
                    }
                }
            }
        }
        if game.platform == Platform::Xbox {
            if let Some(path) = images::xbox_local_logo(&game.path) {
                if let Some(result) = self.save_from_path(dir, &path, "xbox local") {
                    return Some(result);
                }
            }
        }
        if let Some(icon) = images::exe_icon_image(&game.path) {
            if let Ok(path) = save_dynamic_image(dir, icon) {
                let file = path.file_name()?.to_string_lossy().into_owned();
                return publish_automatic(
                    dir,
                    State {
                        source: "exe icon".into(),
                        file: Some(file),
                    },
                    Some(path),
                );
            }
        }
        let source = if legacy.is_some() {
            "legacy cache"
        } else {
            "miss"
        };
        publish_automatic(
            dir,
            State {
                source: source.into(),
                file: None,
            },
            legacy,
        )
    }

    fn store_supplied(&self, game: &Game, dir: &Path) -> Option<PathBuf> {
        let url = game.art_url.as_deref()?;
        let url = if url.starts_with("//") {
            format!("https:{url}")
        } else {
            url.to_string()
        };
        supplied_url_matches_game(game, &url)
            .then(|| self.download(dir, &url, "store supplied"))
            .flatten()
    }

    fn save_from_path(&self, dir: &Path, source: &Path, label: &str) -> Option<PathBuf> {
        let metadata = fs::metadata(source).ok()?;
        if metadata.len() > MAX_IMAGE_BYTES as u64 {
            return None;
        }
        let bytes = read_image_file(source).ok()?;
        self.save_bytes(dir, &bytes, label)
    }

    fn download(&self, dir: &Path, url: &str, label: &str) -> Option<PathBuf> {
        if !safe_store_url(url) {
            return None;
        }
        let bytes = (self.fetcher)(url)?;
        self.save_bytes(dir, &bytes, label)
    }

    fn save_bytes(&self, dir: &Path, bytes: &[u8], label: &str) -> Option<PathBuf> {
        let path = save_image(dir, bytes).ok()?;
        let file = path.file_name()?.to_string_lossy().into_owned();
        publish_automatic(
            dir,
            State {
                source: label.into(),
                file: Some(file),
            },
            Some(path),
        )
    }

    fn game_dir(&self, game: &Game) -> PathBuf {
        let mut hash = Sha256::new();
        hash.update(format!("{:?}", game.platform));
        hash.update([0]);
        if let Some(identity) = &game.store_identity {
            hash.update(format!("{:?}", identity.platform));
            hash.update([0]);
            hash.update(identity.store_id.as_bytes());
        }
        hash.update([0]);
        hash.update(crate::model::GameKey::path_key(&game.path).as_bytes());
        let key: String = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        self.cache_dir.join("cover-v2").join(key)
    }

    fn legacy_path(&self, game: &Game) -> Option<PathBuf> {
        let appid = verified_steam_appid(game);
        ImageCache::with_sources(&self.cache_dir, vec![], std::sync::Arc::new(|_| None))
            .cached_path(&game.name, appid)
    }

    fn state_path(&self, game: &Game, dir: &Path, state: &State) -> Option<PathBuf> {
        match state.source.as_str() {
            "miss" | "cleared" => None,
            "legacy cache" => self.legacy_path(game),
            _ => {
                let name = state.file.as_deref()?;
                if !generated_name(name, "image", "jpg") {
                    return None;
                }
                let path = dir.join(name);
                path.is_file().then_some(path)
            }
        }
    }
}

fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

/// Decide before publishing so misses and inferior fallbacks cannot hide the
/// last-good image, even if the process stops during a refresh.
fn publish_automatic(dir: &Path, candidate: State, path: Option<PathBuf>) -> Option<PathBuf> {
    let previous = read_state(dir).and_then(|state| {
        let name = state.file.as_deref()?;
        if !generated_name(name, "image", "jpg") {
            return None;
        }
        let previous_path = dir.join(name);
        let quality = automatic_quality(&previous_path, &state.source)?;
        Some((state, previous_path, quality))
    });
    if let Some((state, previous_path, quality)) = &previous {
        let candidate_quality = path
            .as_deref()
            .and_then(|path| automatic_quality(path, &candidate.source));
        if matches!(state.source.as_str(), "user override" | "steamgriddb")
            || Some(*quality) >= candidate_quality
        {
            return Some(previous_path.clone());
        }
    }
    if write_state(dir, &candidate).is_err() {
        return previous.map(|(_, path, _)| path);
    }
    path
}

fn automatic_quality(path: &Path, source: &str) -> Option<(u8, bool, u64)> {
    let bytes = read_image_file(path).ok()?;
    let image = crate::steam_art::decode_portrait_bytes(&bytes)?;
    let (width, height) = image.dimensions();
    // Actual cover artwork outranks logos/icons, then prefer portrait shape and
    // usable pixel area. Equal-quality candidates leave the existing state alone.
    let kind = match source {
        "exe icon" => 0,
        "xbox local" => 1,
        _ => 2,
    };
    Some((kind, height > width, u64::from(width) * u64::from(height)))
}

fn verified_steam_appid(game: &Game) -> Option<u32> {
    if game.platform != Platform::Steam {
        return None;
    }
    let appid = game.steam_appid?;
    let identity = game.store_identity.as_ref()?;
    (identity.platform == Platform::Steam && identity.store_id == appid.to_string())
        .then_some(appid)
}

fn is_portrait_file(path: &Path) -> bool {
    fs::metadata(path)
        .ok()
        .is_some_and(|m| m.len() <= MAX_IMAGE_BYTES as u64)
        && read_image_file(path)
            .ok()
            .and_then(|b| crate::steam_art::decode_portrait_bytes(&b))
            .is_some_and(|i| {
                let (w, h) = i.dimensions();
                h > w
            })
}

fn save_image(dir: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let img = crate::steam_art::decode_portrait_bytes(bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid cover image"))?;
    save_dynamic_image(dir, img)
}

fn read_image_file(path: &Path) -> io::Result<Vec<u8>> {
    read_bounded(path, MAX_IMAGE_BYTES as u64)
}

fn read_bounded(path: &Path, max: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(max + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(bytes)
}

fn generated_name(name: &str, prefix: &str, ext: &str) -> bool {
    let Some(rest) = name.strip_prefix(&format!("{prefix}-")) else {
        return false;
    };
    let Some(stem) = rest.strip_suffix(&format!(".{ext}")) else {
        return false;
    };
    let Some((timestamp, sequence)) = stem.split_once('-') else {
        return false;
    };
    timestamp.len() == 32
        && sequence.len() == 16
        && timestamp
            .bytes()
            .chain(sequence.bytes())
            .all(|b| b.is_ascii_hexdigit())
}

fn save_dynamic_image(dir: &Path, img: image::DynamicImage) -> io::Result<PathBuf> {
    let (width, height) = img.dimensions();
    if width > 4096 || height > 4096 || u64::from(width) * u64::from(height) > 16_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cover dimensions too large",
        ));
    }
    let img = image::DynamicImage::ImageRgb8(img.thumbnail(300, 450).to_rgb8());
    let mut encoded = Vec::new();
    img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut encoded,
        85,
    ))
    .map_err(io::Error::other)?;
    atomic_append(dir, "image", "jpg", &encoded)
}

fn unique_name(prefix: &str, ext: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{prefix}-{nanos:032x}-{:016x}.{ext}",
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    )
}

fn atomic_append(dir: &Path, prefix: &str, ext: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let final_path = dir.join(unique_name(prefix, ext));
    let temp = dir.join(unique_name("partial", "tmp"));
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    out.write_all(bytes)?;
    out.sync_all()?;
    drop(out);
    fs::rename(&temp, &final_path)?;
    Ok(final_path)
}

fn write_state(dir: &Path, state: &State) -> io::Result<()> {
    let bytes = serde_json::to_vec(state).map_err(io::Error::other)?;
    atomic_append(dir, "state", "json", &bytes).map(|_| ())
}

fn read_state(dir: &Path) -> Option<State> {
    let latest = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            generated_name(&name, "state", "json").then_some((name, e.path()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))?
        .1;
    let bytes = read_bounded(&latest, 4096).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn safe_store_url(url: &str) -> bool {
    if url.len() > 2048 {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let Some((host, path)) = rest.split_once('/') else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    let allowed = [
        "steamstatic.com",
        "gog-statics.com",
        "gog.com",
        "epicgames.com",
    ];
    allowed
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        && !path.is_empty()
        && !path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == ".." || s.contains('%'))
        && !url
            .bytes()
            .any(|b| b <= 0x20 || b >= 0x7f || matches!(b, b'@' | b'\\' | b'#'))
}

fn supplied_url_matches_game(game: &Game, url: &str) -> bool {
    if !safe_store_url(url) {
        return false;
    }
    let host = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default();
    match game.platform {
        Platform::Steam => {
            verified_steam_appid(game).is_some_and(|appid| images::steam_app_path_names(url, appid))
        }
        Platform::Gog => {
            host == "gog.com"
                || host.ends_with(".gog.com")
                || host == "gog-statics.com"
                || host.ends_with(".gog-statics.com")
        }
        Platform::Epic => host == "epicgames.com" || host.ends_with(".epicgames.com"),
        Platform::Heroic => {
            host == "epicgames.com"
                || host.ends_with(".epicgames.com")
                || host == "gog.com"
                || host.ends_with(".gog.com")
                || host == "gog-statics.com"
                || host.ends_with(".gog-statics.com")
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GridGame {
    pub id: u64,
    pub name: String,
}

pub type SearchGame = GridGame;

#[derive(Debug, Clone, Deserialize)]
pub struct GridImage {
    pub id: u64,
    pub url: String,
    pub thumb: Option<String>,
}

/// Explicit, token-scoped SteamGridDB browser. Never called by CoverCache::fetch.
pub struct SteamGridDb {
    token: String,
}

impl SteamGridDb {
    pub fn new(token: impl Into<String>) -> Result<Self, String> {
        let token = token.into();
        if token.is_empty() || token.len() > 512 || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("invalid SteamGridDB token".into());
        }
        Ok(Self { token })
    }

    pub fn search(&self, title: &str) -> Result<Vec<GridGame>, String> {
        if title.trim().is_empty() || title.len() > 200 {
            return Err("invalid title".into());
        }
        let encoded = percent_encode(title);
        let body = self.get_json(&format!(
            "https://www.steamgriddb.com/api/v2/search/autocomplete/{encoded}"
        ))?;
        parse_data(body)
    }

    /// Explicit lookup: prefer the documented Steam external-ID mapping, then
    /// return title candidates for the user to confirm. Never called by fetch.
    pub fn search_game(&self, game: &Game) -> Result<Vec<GridGame>, String> {
        if let Some(url) = exact_steam_url(game) {
            if let Ok(body) = self.get_json(&url) {
                if let Some(candidate) = parse_exact_game(&body) {
                    return Ok(vec![candidate]);
                }
            }
        }
        self.search(&game.name)
    }

    pub fn grids(&self, game_id: u64) -> Result<Vec<GridImage>, String> {
        if game_id == 0 {
            return Err("invalid game id".into());
        }
        let body = self.get_json(&format!(
            "https://www.steamgriddb.com/api/v2/grids/game/{game_id}"
        ))?;
        let images: Vec<GridImage> = parse_data(body)?;
        Ok(images
            .into_iter()
            .filter(|image| safe_grid_url(&image.url))
            .map(|mut image| {
                image.thumb = image.thumb.filter(|url| safe_grid_thumb_url(url));
                image
            })
            .take(100)
            .collect())
    }

    /// Downloads only the image the user selected after reviewing a grid list.
    pub fn select(
        &self,
        cache: &CoverCache,
        game: &Game,
        image: &GridImage,
    ) -> io::Result<PathBuf> {
        if !safe_grid_url(&image.url) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid grid URL",
            ));
        }
        let bytes =
            images::http_get(&image.url).ok_or_else(|| io::Error::other("grid download failed"))?;
        let lock = cache
            .entry_lock(&cache.game_dir(game))
            .map_err(io::Error::other)?;
        let _guard = lock
            .lock()
            .map_err(|_| io::Error::other("cover lock poisoned"))?;
        cache.save_selected(game, &bytes, "steamgriddb")
    }

    fn get_json(&self, url: &str) -> Result<Vec<u8>, String> {
        if !url.starts_with("https://www.steamgriddb.com/api/v2/") {
            return Err("invalid API URL".into());
        }
        let mut response = images::artwork_agent()
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .call()
            .map_err(|_| "SteamGridDB request failed".to_string())?;
        if response.status() != 200 {
            return Err("SteamGridDB request failed".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_JSON_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "SteamGridDB response failed".to_string())?;
        if bytes.len() as u64 > MAX_JSON_BYTES {
            return Err("SteamGridDB response too large".into());
        }
        Ok(bytes)
    }
}

fn parse_data<T: serde::de::DeserializeOwned>(body: Vec<u8>) -> Result<Vec<T>, String> {
    #[derive(Deserialize)]
    struct Envelope<T> {
        success: bool,
        data: Vec<T>,
    }
    let parsed: Envelope<T> =
        serde_json::from_slice(&body).map_err(|_| "invalid SteamGridDB response".to_string())?;
    if !parsed.success {
        return Err("SteamGridDB request failed".into());
    }
    Ok(parsed.data.into_iter().take(100).collect())
}

fn exact_steam_url(game: &Game) -> Option<String> {
    verified_steam_appid(game)
        .map(|appid| format!("https://www.steamgriddb.com/api/v2/games/steam/{appid}"))
}

fn parse_exact_game(body: &[u8]) -> Option<GridGame> {
    if body.len() as u64 > MAX_JSON_BYTES {
        return None;
    }
    #[derive(Deserialize)]
    struct Envelope {
        success: bool,
        data: GridGame,
    }
    let parsed: Envelope = serde_json::from_slice(body).ok()?;
    (parsed.success && parsed.data.id != 0 && !parsed.data.name.trim().is_empty())
        .then_some(parsed.data)
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn safe_grid_url(url: &str) -> bool {
    safe_grid_asset(url, "grid")
}

fn safe_grid_thumb_url(url: &str) -> bool {
    safe_grid_asset(url, "thumb")
}

fn safe_grid_asset(url: &str, kind: &str) -> bool {
    let prefix = format!("https://s3.amazonaws.com/steamgriddb/{kind}/");
    let current_prefix = format!("https://cdn2.steamgriddb.com/{kind}/");
    let Some(path) = url
        .strip_prefix(&prefix)
        .or_else(|| url.strip_prefix(&current_prefix))
    else {
        return false;
    };
    !path.is_empty()
        && path != "."
        && path != ".."
        && path.len() <= 160
        && !path.contains('/')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod exact_id_tests {
    use super::*;
    use crate::model::StoreIdentity;

    #[test]
    fn refresh_icon_cannot_replace_last_good_automatic_artwork() {
        let dir = tempfile::tempdir().unwrap();
        let original =
            save_dynamic_image(dir.path(), image::DynamicImage::new_rgb8(120, 60)).unwrap();
        let original_state = State {
            source: "steam header".into(),
            file: Some(original.file_name().unwrap().to_string_lossy().into_owned()),
        };
        write_state(dir.path(), &original_state).unwrap();
        let icon = save_dynamic_image(dir.path(), image::DynamicImage::new_rgb8(200, 300)).unwrap();
        let result = publish_automatic(
            dir.path(),
            State {
                source: "exe icon".into(),
                file: Some(icon.file_name().unwrap().to_string_lossy().into_owned()),
            },
            Some(icon),
        );
        assert_eq!(result, Some(original));
        let persisted = read_state(dir.path()).unwrap();
        assert_eq!(persisted.source, original_state.source);
        assert_eq!(persisted.file, original_state.file);
    }

    #[test]
    fn exact_lookup_requires_verified_steam_identity() {
        let mut game = Game::new(
            "Portal 2",
            PathBuf::from("C:/games/portal2"),
            Platform::Steam,
        );
        game.steam_appid = Some(620);
        assert!(exact_steam_url(&game).is_none());
        game.store_identity = Some(StoreIdentity::steam(620));
        assert_eq!(
            exact_steam_url(&game).as_deref(),
            Some("https://www.steamgriddb.com/api/v2/games/steam/620")
        );
        game.store_identity = Some(StoreIdentity::steam(999));
        assert!(exact_steam_url(&game).is_none());
        game.platform = Platform::Gog;
        game.store_identity = Some(StoreIdentity::new(Platform::Gog, "620"));
        assert!(exact_steam_url(&game).is_none());
    }

    #[test]
    fn exact_response_is_one_game_not_search_array() {
        let candidate =
            parse_exact_game(br#"{"success":true,"data":{"id":123,"name":"Portal 2"}}"#).unwrap();
        assert_eq!(candidate.id, 123);
        assert!(
            parse_exact_game(br#"{"success":false,"data":{"id":123,"name":"Portal 2"}}"#).is_none()
        );
        assert!(parse_exact_game(br#"{"success":true,"data":[]}"#).is_none());
    }

    #[test]
    fn grid_allowlist_accepts_verified_current_cdn_and_legacy_s3() {
        assert!(safe_grid_url(
            "https://cdn2.steamgriddb.com/grid/aa6488ad256e6d0242bc786fcc377e64.png"
        ));
        assert!(safe_grid_thumb_url(
            "https://cdn2.steamgriddb.com/thumb/58df6629cddf7595b29c6069414583a1.jpg"
        ));
        assert!(safe_grid_url(
            "https://s3.amazonaws.com/steamgriddb/grid/example.png"
        ));
        for url in [
            "http://cdn2.steamgriddb.com/grid/example.png",
            "https://cdn2.steamgriddb.com.evil.example/grid/example.png",
            "https://cdn2.steamgriddb.com@evil.example/grid/example.png",
            "https://cdn2.steamgriddb.com/grid/../private.png",
            "https://cdn2.steamgriddb.com/grid/%2e%2e/private.png",
            "https://cdn2.steamgriddb.com:8443/grid/example.png",
            "https://cdn2.steamgriddb.com/grid/example.png?redirect=1",
            "https://cdn3.steamgriddb.com/grid/example.png",
        ] {
            assert!(!safe_grid_url(url), "accepted {url}");
        }
    }
}
