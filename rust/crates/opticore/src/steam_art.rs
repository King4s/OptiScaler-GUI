//! Steam's own locally cached artwork.
//!
//! Steam downloads library art into `<install>/appcache/librarycache/<appid>/`
//! and keeps it there, so a portrait is usually available offline for the exact
//! app id. Observed on a real install (485 cache entries): the portrait is
//! `library_600x900.jpg` (133 apps had it directly in their own directory, 18
//! more under a single content-addressed hash directory), while `header.jpg`
//! (225), `library_hero.jpg` (123) and `logo.png` (108) are landscape or logo
//! art. The flat `<appid>_library_600x900.jpg` spelling is retired: 0 of 485.
//!
//! Portrait and landscape lookups are separate: neither hands back the other
//! shape. Both only inspect the requested app id's directory.
//!
//! See `tasks/spec-steam-local-art.md`.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::scan::steam;

/// The one file name Steam writes for a library portrait.
const PORTRAIT_FILE: &str = "library_600x900.jpg";
const LANDSCAPE_FILES: [&str; 3] = ["header.jpg", "library_header.jpg", "library_hero.jpg"];
const MAX_APP_DIR_ENTRIES: usize = 512;

/// Largest local portrait we will read. The largest found in a real cache was
/// 575 KB, so this is about seven times the worst case.
const MAX_PORTRAIT_BYTES: u64 = 4 * 1024 * 1024;

/// Pixel and allocation limits for the decode. A portrait is 600x900; the bound
/// is generous for a sharper variant while still refusing a decompression bomb
/// hidden in a small file.
const MAX_PORTRAIT_DIMENSION: u32 = 4096;
const MAX_PORTRAIT_ALLOC: u64 = 64 * 1024 * 1024;

/// Every Steam install's local library cache on this machine.
pub fn library_caches() -> Vec<PathBuf> {
    library_caches_from(steam::steam_install_roots())
}

/// The library caches of the given install roots, one entry per installation.
///
/// `appcache` lives per installation rather than per library folder, so the
/// install roots are the input, not `libraryfolders.vdf`. One install can arrive
/// twice — the registry's forward-slash spelling and the built-in default's
/// backslash spelling — and `steam_install_roots` folds case only, so the
/// spellings are folded once more here: a second listing of the same directory is
/// the one cost this lookup must not pay per app id.
pub fn library_caches_from(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    roots
        .into_iter()
        .map(|root| root.join("appcache").join("librarycache"))
        .filter(|cache| cache.is_dir())
        .filter(|cache| seen.insert(folded(cache)))
        .collect()
}

/// A case- and separator-insensitive spelling of a path, for de-duplication
/// only. General path folding belongs to the game-key work (PR #33); this does
/// not have to agree with it, and nothing reads it as a key.
fn folded(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// The portrait Steam cached for this exact app id in one library cache, if any.
///
/// `<cache>/<appid>/library_600x900.jpg` first, then a single
/// `<cache>/<appid>/<entry>/library_600x900.jpg` for the apps whose assets the
/// client stores content-addressed. Two or more nested candidates are ambiguous,
/// so they yield nothing rather than an arbitrary pick, and nothing deeper than
/// that one level is considered.
pub fn portrait_file_in(library_cache: &Path, appid: u32) -> Option<PathBuf> {
    let app_dir = library_cache.join(appid.to_string());
    let direct = app_dir.join(PORTRAIT_FILE);
    if direct.is_file() {
        return Some(direct);
    }
    let mut nested = Vec::new();
    for entry in std::fs::read_dir(&app_dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join(PORTRAIT_FILE);
            if candidate.is_file() {
                nested.push(candidate);
            }
        }
    }
    match nested.len() {
        1 => nested.pop(),
        _ => None,
    }
}

/// The portrait for this app id in any of the given library caches.
pub fn portrait_in(library_caches: &[PathBuf], appid: u32) -> Option<PathBuf> {
    library_caches
        .iter()
        .find_map(|library_cache| portrait_file_in(library_cache, appid))
}

/// The portrait for this app id in any Steam installation on this machine.
pub fn portrait_file(appid: u32) -> Option<PathBuf> {
    portrait_in(&library_caches(), appid)
}

/// The first decodable wide image for this exact app id, preferring Steam's
/// header, then library header, then hero across the supplied installations.
pub fn landscape_in(library_caches: &[PathBuf], appid: u32) -> Option<PathBuf> {
    for filename in LANDSCAPE_FILES {
        for cache in library_caches {
            let app_dir = cache.join(appid.to_string());
            let direct = app_dir.join(filename);
            if valid_landscape(&direct) {
                return Some(direct);
            }
            if let Some(nested) = nested_landscape(&app_dir, filename) {
                return Some(nested);
            }
        }
    }
    None
}

fn valid_landscape(path: &Path) -> bool {
    // Do not follow a symlink out of the requested app id's cache directory.
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
        && decode_portrait(path).is_some_and(|image| image.width() > image.height())
}

fn nested_landscape(app_dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut candidate = None;
    let mut entries = 0;
    for entry in std::fs::read_dir(app_dir).ok()? {
        entries += 1;
        if entries > MAX_APP_DIR_ENTRIES {
            return None;
        }
        let entry = entry.ok()?;
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let path = entry.path().join(filename);
        if std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file()) {
            if candidate.is_some() {
                return None;
            }
            candidate = Some(path);
        }
    }
    candidate.filter(|path| valid_landscape(path))
}

/// Read and decode a candidate portrait under hard size and pixel bounds.
///
/// The byte bound is applied before the read, so an oversized file costs nothing.
/// A refusal is a miss, never an error: the caller falls through to the next
/// artwork source.
pub fn decode_portrait(path: &Path) -> Option<image::DynamicImage> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_PORTRAIT_BYTES {
        return None;
    }
    let bytes = read_bounded(path, MAX_PORTRAIT_BYTES)?;
    decode_portrait_bytes(&bytes)
}

/// Decode portrait bytes from an untrusted source, e.g. a download.
///
/// Same bounds as [`decode_portrait`], applied to bytes that are already in
/// memory: the byte bound first, then the pixel and allocation limits, so a
/// small body cannot expand into an unbounded allocation.
pub fn decode_portrait_bytes(bytes: &[u8]) -> Option<image::DynamicImage> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_PORTRAIT_BYTES {
        return None;
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_PORTRAIT_DIMENSION);
    limits.max_image_height = Some(MAX_PORTRAIT_DIMENSION);
    limits.max_alloc = Some(MAX_PORTRAIT_ALLOC);
    reader.limits(limits);
    reader.with_guessed_format().ok()?.decode().ok()
}

/// Read at most `max` bytes, refusing the file if it is longer.
fn read_bounded(path: &Path, max: u64) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > max {
        return None;
    }
    Some(bytes)
}
