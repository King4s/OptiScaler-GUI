use opticore::cover_art::{CoverCache, SteamGridDb};
use opticore::images::Fetcher;
use opticore::model::{Game, Platform, StoreIdentity};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn game(name: &str, path: &str, platform: Platform, id: &str) -> Game {
    let mut game = Game::new(name, PathBuf::from(path), platform);
    game.store_identity = Some(StoreIdentity::new(platform, id));
    game
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::new_rgb8(width, height)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

#[test]
fn keys_isolate_store_identity_and_install_path() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    let a = game("Same", "C:/one", Platform::Steam, "123");
    let b = game("Same", "C:/two", Platform::Steam, "123");
    let c = game("Same", "C:/one", Platform::Gog, "123");
    let file = dir.path().join("custom.png");
    std::fs::write(&file, png(20, 30)).unwrap();
    let selected = cache.set_override(&a, &file).unwrap();
    assert_eq!(cache.fetch(&a), Some(selected));
    assert_eq!(cache.fetch(&b), None);
    assert_eq!(cache.fetch(&c), None);
}

#[test]
fn override_survives_refresh_and_clear_does_not_delete_file() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    let game = game("My Game", "C:/one", Platform::Gog, "g1");
    let file = dir.path().join("custom.png");
    std::fs::write(&file, png(20, 30)).unwrap();
    let selected = cache.set_override(&game, &file).unwrap();
    assert_eq!(cache.refresh(&game), Some(selected.clone()));
    assert_eq!(cache.source(&game).as_deref(), Some("user override"));
    cache.clear_override(&game).unwrap();
    assert!(selected.exists());
    assert_eq!(cache.fetch(&game), None);
}

#[test]
fn legacy_landscape_is_upgraded_once_to_store_portrait() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("appid_620.jpg"), png(120, 60)).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let log = calls.clone();
    let fetcher: Fetcher = Arc::new(move |url| {
        log.lock().unwrap().push(url.to_string());
        if url.contains("GetItems") {
            Some(br#"{"response":{"store_items":[{"appid":620,"success":1,"assets":{"asset_url_format":"steam/apps/620/${FILENAME}","library_capsule":"library_600x900.jpg"}}]}}"#.to_vec())
        } else if url.ends_with("library_600x900.jpg") {
            Some(png(60, 90))
        } else {
            None
        }
    });
    let cache = CoverCache::with_sources(dir.path(), vec![], fetcher);
    let mut game = game("Same", "C:/one", Platform::Steam, "620");
    game.steam_appid = Some(620);
    let upgraded = cache.fetch(&game).unwrap();
    assert_ne!(upgraded, dir.path().join("appid_620.jpg"));
    assert_eq!(cache.source(&game).as_deref(), Some("steam store"));
    let count = calls.lock().unwrap().len();
    assert_eq!(cache.fetch(&game), Some(upgraded));
    assert_eq!(calls.lock().unwrap().len(), count);
    assert!(dir.path().join("appid_620.jpg").exists());
}

#[test]
fn supplied_steam_header_does_not_outrank_portrait() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("appid_620.jpg"), png(120, 60)).unwrap();
    let fetcher: Fetcher = Arc::new(move |url| {
        if url.contains("GetItems") {
            Some(br#"{"response":{"store_items":[{"appid":620,"success":1,"assets":{"asset_url_format":"steam/apps/620/${FILENAME}","library_capsule":"library_600x900.jpg"}}]}}"#.to_vec())
        } else if url.ends_with("library_600x900.jpg") {
            Some(png(60, 90))
        } else if url.ends_with("header.jpg") {
            Some(png(120, 60))
        } else {
            None
        }
    });
    let cache = CoverCache::with_sources(dir.path(), vec![], fetcher);
    let mut game = game("Same", "C:/one", Platform::Steam, "620");
    game.steam_appid = Some(620);
    game.art_url = Some("https://cdn.akamai.steamstatic.com/steam/apps/620/header.jpg".into());
    assert!(cache.fetch(&game).is_some());
    assert_eq!(cache.source(&game).as_deref(), Some("steam store"));
}

#[test]
fn untrusted_store_url_is_never_requested() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let log = calls.clone();
    let cache = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(move |url| {
            log.lock().unwrap().push(url.to_string());
            Some(png(60, 90))
        }),
    );
    let mut game = game("Same", "C:/one", Platform::Epic, "e1");
    game.art_url = Some("https://127.0.0.1/private".into());
    assert_eq!(cache.fetch(&game), None);
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn supplied_steam_url_cannot_name_a_different_app() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let log = calls.clone();
    let cache = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(move |url| {
            log.lock().unwrap().push(url.to_string());
            None
        }),
    );
    let mut game = game("Same", "C:/one", Platform::Steam, "620");
    game.steam_appid = Some(620);
    let wrong = "https://cdn.akamai.steamstatic.com/steam/apps/999/header.jpg";
    game.art_url = Some(wrong.into());
    assert_eq!(cache.fetch(&game), None);
    assert!(!calls.lock().unwrap().iter().any(|url| url == wrong));
}

#[test]
fn steamgriddb_requires_token_and_explicit_search() {
    assert!(SteamGridDb::new("").is_err());
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let log = calls.clone();
    let cache = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(move |url| {
            log.lock().unwrap().push(url.to_string());
            None
        }),
    );
    let game = game("Name", "C:/one", Platform::Epic, "e1");
    assert_eq!(cache.fetch(&game), None);
    assert!(!calls
        .lock()
        .unwrap()
        .iter()
        .any(|url| url.contains("steamgriddb")));
}

#[test]
fn failed_lookup_is_not_repeated_until_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(0));
    let count = calls.clone();
    let cache = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(move |_| {
            *count.lock().unwrap() += 1;
            None
        }),
    );
    let mut game = game("Name", "C:/one", Platform::Steam, "620");
    game.steam_appid = Some(620);
    assert_eq!(cache.fetch(&game), None);
    let first = *calls.lock().unwrap();
    assert!(first > 0);
    assert_eq!(cache.fetch(&game), None);
    assert_eq!(*calls.lock().unwrap(), first);
    assert_eq!(cache.refresh(&game), None);
    assert!(*calls.lock().unwrap() > first);
}

#[test]
fn refresh_offline_preserves_last_good_cover_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut game = game("Cover", "C:/cover", Platform::Epic, "cover");
    game.art_url = Some("https://cdn1.epicgames.com/cover.jpg".into());
    let online = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| Some(png(60, 90))));
    let original = online.fetch(&game).unwrap();
    let offline = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    assert_eq!(offline.refresh(&game), Some(original.clone()));
    assert_eq!(offline.source(&game).as_deref(), Some("store supplied"));
    let restarted = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(|_| panic!("cached cover must not fetch")),
    );
    assert_eq!(restarted.fetch(&game), Some(original));
    assert_eq!(restarted.source(&game).as_deref(), Some("store supplied"));
}

#[test]
fn refresh_rejects_inferior_header_and_preserves_portrait_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut game = game("Cover", "C:/cover", Platform::Steam, "620");
    game.steam_appid = Some(620);
    let portrait = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(|url| {
            if url.contains("GetItems") {
                Some(br#"{"response":{"store_items":[{"appid":620,"success":1,"assets":{"asset_url_format":"steam/apps/620/${FILENAME}","library_capsule":"library_600x900.jpg"}}]}}"#.to_vec())
            } else if url.ends_with("library_600x900.jpg") {
                Some(png(60, 90))
            } else {
                None
            }
        }),
    );
    let original = portrait.fetch(&game).unwrap();
    let fallback = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(|url| url.ends_with("header.jpg").then(|| png(120, 60))),
    );
    assert_eq!(fallback.refresh(&game), Some(original.clone()));
    let restarted = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    assert_eq!(restarted.fetch(&game), Some(original));
    assert_eq!(restarted.source(&game).as_deref(), Some("steam store"));
}

#[test]
fn refresh_can_improve_a_cached_landscape() {
    let dir = tempfile::tempdir().unwrap();
    let mut game = game("Cover", "C:/cover", Platform::Epic, "cover");
    game.art_url = Some("https://cdn1.epicgames.com/cover.jpg".into());
    let landscape = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| Some(png(120, 60))));
    let original = landscape.fetch(&game).unwrap();
    let portrait = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| Some(png(60, 90))));
    let improved = portrait.refresh(&game).unwrap();
    assert_ne!(improved, original);
    assert!(original.exists());
    let restarted = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    assert_eq!(restarted.fetch(&game), Some(improved));
}

#[test]
fn appdetails_capsule_survives_failed_header_download() {
    let dir = tempfile::tempdir().unwrap();
    let fetcher: Fetcher = Arc::new(|url| {
        if url.contains("GetItems") {
            None
        } else if url.contains("appdetails") {
            Some(br#"{"620":{"success":true,"data":{"steam_appid":620,"header_image":"https://cdn.akamai.steamstatic.com/steam/apps/620/bad.jpg","capsule_image":"https://cdn.akamai.steamstatic.com/steam/apps/620/capsule.jpg"}}}"#.to_vec())
        } else if url.ends_with("capsule.jpg") {
            Some(png(60, 90))
        } else {
            None
        }
    });
    let cache = CoverCache::with_sources(dir.path(), vec![], fetcher);
    let mut game = game("Same", "C:/one", Platform::Steam, "620");
    game.steam_appid = Some(620);
    assert!(cache.fetch(&game).is_some());
    assert_eq!(cache.source(&game).as_deref(), Some("steam appdetails"));
}

#[test]
fn xbox_local_logo_is_available_without_network() {
    let dir = tempfile::tempdir().unwrap();
    let install = dir.path().join("install");
    std::fs::create_dir(&install).unwrap();
    std::fs::write(
        install.join("MicrosoftGame.config"),
        r#"<Game Square480x480Logo="StoreLogo.png"/>"#,
    )
    .unwrap();
    std::fs::write(install.join("StoreLogo.png"), png(60, 90)).unwrap();
    let cache = CoverCache::with_sources(&dir.path().join("cache"), vec![], Arc::new(|_| None));
    let game = Game::new("Xbox title", install, Platform::Xbox);
    assert!(cache.fetch(&game).is_some());
    assert_eq!(cache.source(&game).as_deref(), Some("xbox local"));
}

#[test]
fn clear_automatic_cache_keeps_selected_images_and_legacy_root() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(|url| url.contains("epicgames.com").then(|| png(60, 90))),
    );
    let mut automatic = game("Auto", "C:/auto", Platform::Epic, "e1");
    automatic.art_url = Some("https://cdn1.epicgames.com/auto.jpg".into());
    let automatic_path = cache.fetch(&automatic).unwrap();
    let selected = game("Selected", "C:/selected", Platform::Gog, "g1");
    let custom = dir.path().join("custom.png");
    std::fs::write(&custom, png(60, 90)).unwrap();
    let selected_path = cache.set_override(&selected, &custom).unwrap();
    let legacy = dir.path().join("Old Game.png");
    std::fs::write(&legacy, png(60, 90)).unwrap();
    cache.clear_automatic_cache().unwrap();
    assert!(!automatic_path.exists());
    assert!(selected_path.exists());
    assert_eq!(cache.fetch(&selected), Some(selected_path));
    assert!(legacy.exists());
}

#[test]
fn another_games_override_is_not_blocked_by_network() {
    use std::sync::mpsc;
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let cache = Arc::new(CoverCache::with_sources(
        dir.path(),
        vec![],
        Arc::new(move |_| {
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            None
        }),
    ));
    let mut remote = game("Remote", "C:/remote", Platform::Epic, "remote");
    remote.art_url = Some("https://cdn1.epicgames.com/remote.jpg".into());
    let remote_cache = cache.clone();
    let fetch = std::thread::spawn(move || remote_cache.fetch(&remote));
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let custom = dir.path().join("custom.png");
    std::fs::write(&custom, png(20, 30)).unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    let local_cache = cache.clone();
    let selected = game("Local", "C:/local", Platform::Manual, "local");
    let writer = std::thread::spawn(move || {
        done_tx
            .send(local_cache.set_override(&selected, &custom))
            .unwrap();
    });
    let result = done_rx.recv_timeout(Duration::from_secs(5));
    release_tx.send(()).unwrap();
    fetch.join().unwrap();
    writer.join().unwrap();
    assert!(result
        .expect("unrelated override was blocked by HTTP")
        .is_ok());
}

#[test]
fn generated_looking_foreign_file_is_preserved_without_state_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    let entry = dir.path().join("cover-v2").join("a".repeat(64));
    std::fs::create_dir_all(&entry).unwrap();
    let foreign = entry.join(format!("image-{}-{}.jpg", "0".repeat(32), "0".repeat(16)));
    std::fs::write(&foreign, b"foreign file").unwrap();
    let state = entry.join(format!("state-{}-{}.json", "0".repeat(32), "1".repeat(16)));
    std::fs::write(state, br#"{"source":"miss","file":null}"#).unwrap();
    cache.clear_automatic_cache().unwrap();
    assert_eq!(std::fs::read(foreign).unwrap(), b"foreign file");
}

#[cfg(windows)]
#[test]
fn cache_clear_never_traverses_a_windows_junction() {
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let root = dir.path().join("cover-v2");
    std::fs::create_dir(&root).unwrap();
    let link = root.join("b".repeat(64));
    let image_name = format!("image-{}-{}.jpg", "0".repeat(32), "0".repeat(16));
    let external_image = external.path().join(&image_name);
    std::fs::write(&external_image, b"external artwork").unwrap();
    std::fs::write(
        external
            .path()
            .join(format!("state-{}-{}.json", "0".repeat(32), "1".repeat(16))),
        serde_json::to_vec(&serde_json::json!({"source":"steam store", "file":image_name}))
            .unwrap(),
    )
    .unwrap();
    let quote = |path: &std::path::Path| path.to_string_lossy().replace('\'', "''");
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(format!(
            "New-Item -ItemType Junction -Path '{}' -Target '{}' -ErrorAction Stop | Out-Null",
            quote(&link),
            quote(external.path())
        ))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction fixture creation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cache = CoverCache::with_sources(dir.path(), vec![], Arc::new(|_| None));
    let result = cache.clear_automatic_cache();
    // Remove only the junction itself before TempDir cleanup.
    std::fs::remove_dir(&link).unwrap();
    result.unwrap();
    assert_eq!(std::fs::read(external_image).unwrap(), b"external artwork");
}
