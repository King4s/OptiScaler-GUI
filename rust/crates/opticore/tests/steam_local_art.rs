//! Fixture tests for the local Steam portrait source (`opticore::steam_art`).
//!
//! Every input is built in a tempdir: no test reads this machine's Steam
//! directories, the registry or the network. The layout they build is the one
//! observed on a real install (see `tasks/spec-steam-local-art.md`).

use std::fs;
use std::path::{Path, PathBuf};

use opticore::images::{ArtRequest, ImageCache};
use opticore::steam_art::{decode_portrait, library_caches_from, portrait_file_in, portrait_in};

/// A real 6x9 PNG, one flat magenta plane: the pipeline rescales whatever it
/// caches to 300x450, so a colour no store image has is what tells the local
/// portrait apart from a CDN header or capsule.
const PORTRAIT_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x09, 0x08, 0x02, 0x00, 0x00, 0x00, 0x9e, 0xf8, 0xca,
    0xca, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xf0, 0x1f,
    0x0d, 0x31, 0x0c, 0x52, 0x21, 0x00, 0xf0, 0xa6, 0x6b, 0x95, 0x1a, 0xa4, 0x0b, 0xb6, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// A real, valid 5000x1 PNG: wider than the decode bound, so only the bound can
/// refuse it.
const OVER_BOUND_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x13, 0x88, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0xbd, 0x73, 0xd3,
    0xdf, 0x00, 0x00, 0x00, 0x27, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xc2, 0x01, 0x09, 0x00,
    0x00, 0x00, 0x02, 0xa0, 0xfe, 0x9f, 0xae, 0x09, 0x1d, 0x50, 0x4c, 0x53, 0x55, 0x55, 0x55, 0x55,
    0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0xbd, 0x07, 0xd6, 0x6b, 0xeb, 0x2b,
    0xce, 0x2b, 0xea, 0x92, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const APP_ID: u32 = 620;
const OTHER_APP_ID: u32 = 730;
const PORTRAIT_FILE: &str = "library_600x900.jpg";
/// An empty library cache, i.e. one Steam installation's
/// `appcache/librarycache`.
fn library_cache() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The app's own directory inside a library cache.
fn app_dir(cache: &Path, appid: u32) -> PathBuf {
    let dir = cache.join(appid.to_string());
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

fn request(name: &str, appid: Option<u32>) -> ArtRequest {
    ArtRequest {
        name: name.to_string(),
        appid,
        art_url: None,
        platform_is_gog: false,
        game_path: None,
    }
}

#[test]
fn the_exact_app_ids_portrait_is_returned() {
    let cache = library_cache();
    let portrait = app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE);
    write(&portrait, PORTRAIT_PNG);

    assert_eq!(portrait_file_in(cache.path(), APP_ID), Some(portrait));
}

#[test]
fn another_app_ids_portrait_is_not_returned() {
    let cache = library_cache();
    write(
        &app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE),
        PORTRAIT_PNG,
    );

    assert_eq!(portrait_file_in(cache.path(), OTHER_APP_ID), None);
}

#[test]
fn a_nested_content_addressed_portrait_is_accepted() {
    let cache = library_cache();
    let nested = app_dir(cache.path(), APP_ID)
        .join("58f55d2df14d0a1aae932baf05a0abca52da5352")
        .join(PORTRAIT_FILE);
    write(&nested, PORTRAIT_PNG);

    assert_eq!(portrait_file_in(cache.path(), APP_ID), Some(nested));
}

#[test]
fn two_nested_portraits_are_ambiguous_and_return_nothing() {
    let cache = library_cache();
    let dir = app_dir(cache.path(), APP_ID);
    write(&dir.join("aaaa").join(PORTRAIT_FILE), PORTRAIT_PNG);
    write(&dir.join("bbbb").join(PORTRAIT_FILE), PORTRAIT_PNG);

    assert_eq!(portrait_file_in(cache.path(), APP_ID), None);
}

#[test]
fn nothing_two_levels_below_the_app_directory_is_considered() {
    let cache = library_cache();
    let deep = app_dir(cache.path(), APP_ID)
        .join("a")
        .join("b")
        .join(PORTRAIT_FILE);
    write(&deep, PORTRAIT_PNG);

    assert_eq!(portrait_file_in(cache.path(), APP_ID), None);
}

#[test]
fn landscape_and_logo_art_are_not_portraits() {
    let cache = library_cache();
    let dir = app_dir(cache.path(), APP_ID);
    for name in ["header.jpg", "library_hero.jpg", "logo.png"] {
        write(&dir.join(name), PORTRAIT_PNG);
    }

    assert_eq!(portrait_file_in(cache.path(), APP_ID), None);
}

#[test]
fn the_portrait_is_found_in_whichever_install_root_has_it() {
    let first = library_cache();
    let second = library_cache();
    let portrait = app_dir(second.path(), APP_ID).join(PORTRAIT_FILE);
    write(&portrait, PORTRAIT_PNG);

    let caches = vec![first.path().to_path_buf(), second.path().to_path_buf()];

    assert_eq!(portrait_in(&caches, APP_ID), Some(portrait));
}

#[test]
fn a_valid_portrait_decodes() {
    let cache = library_cache();
    let portrait = app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE);
    write(&portrait, PORTRAIT_PNG);

    let image = decode_portrait(&portrait).expect("a real PNG must decode");
    assert_eq!((image.width(), image.height()), (6, 9));
}

#[test]
fn malformed_bytes_are_refused() {
    let cache = library_cache();
    let portrait = app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE);
    write(&portrait, b"not an image at all");

    assert!(decode_portrait(&portrait).is_none());
}

#[test]
fn a_file_over_the_byte_bound_is_refused() {
    let cache = library_cache();
    let portrait = app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE);
    // A valid image with trailing padding: readable in itself, over the bound.
    let mut oversized = PORTRAIT_PNG.to_vec();
    oversized.resize(5 * 1024 * 1024, 0);
    write(&portrait, &oversized);

    assert!(decode_portrait(&portrait).is_none());
}

#[test]
fn an_image_declaring_pixels_over_the_bound_is_refused() {
    let cache = library_cache();
    let portrait = app_dir(cache.path(), APP_ID).join(PORTRAIT_FILE);
    write(&portrait, OVER_BOUND_PNG);

    assert!(decode_portrait(&portrait).is_none());
}

/// The order is what this test is about: with a real app id the CDN and the store
/// API both answer, so a source that ran before the local lookup leaves *their*
/// art in the cache. The local fixture is a flat magenta plane, which no store
/// header or capsule is, so the cached pixels say which source produced the file.
/// (With the CDN unreachable the fallback chain ends at the local portrait either
/// way, so the check bites whenever the network answers at all.)
#[test]
fn fetch_uses_the_local_portrait_before_any_network_source() {
    let steam_cache = library_cache();
    write(
        &app_dir(steam_cache.path(), APP_ID).join(PORTRAIT_FILE),
        PORTRAIT_PNG,
    );

    let cache_dir = library_cache();
    let images = ImageCache::with_library_caches(
        &cache_dir.path().join("art"),
        vec![steam_cache.path().to_path_buf()],
    );

    let found = images
        .fetch(&request("Portal 2", Some(APP_ID)))
        .expect("the local portrait must be used");

    assert_eq!(
        found,
        cache_dir
            .path()
            .join("art")
            .join(format!("appid_{APP_ID}.jpg"))
    );
    let cached = decode_portrait(&found).expect("the cached portrait must decode");
    let pixel = cached
        .to_rgba8()
        .get_pixel(cached.width() / 2, cached.height() / 2)
        .0;
    assert!(
        pixel[0] > 230 && pixel[1] < 25 && pixel[2] > 230,
        "a CDN header or store capsule was cached instead of the local magenta portrait: pixel {pixel:?}"
    );
}

#[test]
fn fetch_reports_nothing_when_no_source_applies() {
    let cache_dir = library_cache();
    let images = ImageCache::with_library_caches(&cache_dir.path().join("art"), Vec::new());

    assert!(images.fetch(&request("Nothing", None)).is_none());
}

#[test]
fn one_install_spelled_twice_yields_one_library_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("appcache").join("librarycache");
    fs::create_dir_all(&cache).unwrap();

    // The registry reports one spelling and the built-in default another; both
    // resolve to the same installation and must not be listed twice.
    let root_path = root.path().to_path_buf();
    let forward = PathBuf::from(root_path.to_string_lossy().replace('\\', "/"));
    let caches = library_caches_from(vec![root_path, forward]);

    assert_eq!(caches, vec![cache], "one installation was listed twice");

    // An install root without an appcache directory contributes nothing.
    let absent = root.path().join("not-an-install");
    assert!(library_caches_from(vec![absent]).is_empty());
}
