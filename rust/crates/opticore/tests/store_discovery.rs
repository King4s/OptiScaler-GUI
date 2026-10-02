use opticore::model::{DiscoverySource, Platform, TitleSource};
use opticore::scan::stores::{
    scan_amazon_db, scan_battlenet_db, scan_ea_records, scan_ea_root, scan_ubisoft_records,
    RegistryRecord,
};
use std::path::Path;

fn game_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..6 {
        std::fs::write(dir.join(format!("part{i}.bin")), b"x").unwrap();
    }
    std::fs::write(dir.join("game.exe"), b"x").unwrap();
    dir
}

fn field(number: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 128);
    let mut out = vec![(number << 3) | 2, payload.len() as u8];
    out.extend_from_slice(payload);
    out
}

fn product(code: &str, install_path: &Path) -> Vec<u8> {
    let mut product = field(2, code.as_bytes());
    let settings = field(1, install_path.to_string_lossy().as_bytes());
    product.extend(field(3, &settings));
    field(1, &product)
}

#[test]
fn amazon_uses_observed_asin_and_ignores_uninstalled_or_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Changed Folder");
    let db = tmp.path().join("amazon.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE DbSet (ProductAsin TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER);").unwrap();
    conn.execute(
        "INSERT INTO DbSet VALUES (?1, ?2, ?3, 1)",
        ("B012345678", "Real Title", installed.to_str().unwrap()),
    )
    .unwrap();
    conn.execute(
        "INSERT INTO DbSet VALUES (?1, ?2, ?3, 0)",
        ("B999999999", "Not installed", installed.to_str().unwrap()),
    )
    .unwrap();
    conn.execute(
        "INSERT INTO DbSet VALUES (?1, ?2, ?3, 1)",
        (
            "B111111111",
            "Moved away",
            tmp.path().join("gone").to_str().unwrap(),
        ),
    )
    .unwrap();
    drop(conn);

    let result = scan_amazon_db(&db);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].platform, Platform::Amazon);
    assert_eq!(result.entries[0].name, "Real Title");
    assert_eq!(result.entries[0].store_id, "B012345678");
    assert_eq!(result.entries[0].path, installed);
    assert_eq!(result.entries[0].source, DiscoverySource::LauncherLibrary);
    assert_eq!(result.entries[0].title_source, TitleSource::Store);
}

#[test]
fn amazon_malformed_db_is_nonfatal_and_visible() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("broken.sqlite");
    std::fs::write(&db, b"not a database").unwrap();
    let result = scan_amazon_db(&db);
    assert!(result.entries.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn amazon_locked_db_is_nonfatal_and_visible() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("locked.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE DbSet (ProductAsin TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER); BEGIN EXCLUSIVE;").unwrap();
    let result = scan_amazon_db(&db);
    assert!(result.entries.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn ubisoft_requires_numeric_id_and_live_game_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "New Location");
    let launcher = game_dir(tmp.path(), "Ubisoft Launcher");
    let result = scan_ubisoft_records([
        RegistryRecord::new(
            "6100",
            Some(installed.to_string_lossy().into_owned()),
            None,
            None,
        ),
        RegistryRecord::new(
            "launcher",
            Some(launcher.to_string_lossy().into_owned()),
            None,
            None,
        ),
        RegistryRecord::new(
            "5210",
            Some(tmp.path().join("gone").to_string_lossy().into_owned()),
            None,
            None,
        ),
    ]);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].store_id, "6100");
    assert_eq!(result.entries[0].name, "New Location");
    assert_eq!(result.entries[0].platform, Platform::Ubisoft);
    assert_eq!(result.entries[0].source, DiscoverySource::Registry);
    assert_eq!(result.entries[0].title_source, TitleSource::Folder);
}

#[test]
fn ea_requires_product_guid_and_does_not_use_title_as_id() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Changed Folder");
    let path = installed.to_string_lossy().into_owned();
    let result = scan_ea_records([
        RegistryRecord::new(
            "My Game",
            Some(path.clone()),
            Some("Official Title".into()),
            Some("{A1B2-C3D4}".into()),
        ),
        RegistryRecord::new(
            "Another Game",
            Some(path.clone()),
            Some("Another Title".into()),
            None,
        ),
        RegistryRecord::new(
            "Moved Game",
            Some(tmp.path().join("gone").to_string_lossy().into_owned()),
            Some("Moved".into()),
            Some("{F00}".into()),
        ),
        RegistryRecord::new(
            "EA Desktop",
            Some(path),
            Some("EA Desktop".into()),
            Some("{LAUNCHER}".into()),
        ),
    ]);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].platform, Platform::Ea);
    assert_eq!(result.entries[0].name, "Official Title");
    assert_eq!(result.entries[0].store_id, "{A1B2-C3D4}");
    assert_eq!(result.entries[0].source, DiscoverySource::Registry);
    assert_eq!(result.entries[0].title_source, TitleSource::Store);
}

#[test]
fn ea_manifest_discovers_game_without_registry_and_uses_content_id() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Folder Moved Here");
    std::fs::create_dir(installed.join("__Installer")).unwrap();
    std::fs::write(
        installed.join("__Installer").join("installerdata.xml"),
        br#"<DiPManifest><contentIDs><contentID>OFB-EAST:12345</contentID></contentIDs><gameTitles><gameTitle locale="en_US">Official EA Title</gameTitle></gameTitles></DiPManifest>"#,
    ).unwrap();
    let launcher = game_dir(tmp.path(), "EA Desktop");
    std::fs::create_dir(launcher.join("__Installer")).unwrap();
    std::fs::write(
        launcher.join("__Installer").join("installerdata.xml"),
        br#"<DiPManifest><contentIDs><contentID>app</contentID></contentIDs></DiPManifest>"#,
    )
    .unwrap();
    let result = scan_ea_root(tmp.path());
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].store_id, "OFB-EAST:12345");
    assert_eq!(result.entries[0].name, "Official EA Title");
    assert_eq!(result.entries[0].path, installed);
    assert_eq!(result.entries[0].source, DiscoverySource::StoreManifest);
    assert_eq!(result.entries[0].title_source, TitleSource::Store);
}

#[test]
fn ea_malformed_manifest_is_nonfatal_and_visible() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Broken Game");
    std::fs::create_dir(installed.join("__Installer")).unwrap();
    std::fs::write(
        installed.join("__Installer").join("installerdata.xml"),
        b"<DiPManifest><contentIDs>",
    )
    .unwrap();
    let result = scan_ea_root(tmp.path());
    assert!(result.entries.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn ea_manifest_rejects_content_id_in_unrelated_element() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Wrong Metadata");
    std::fs::create_dir(installed.join("__Installer")).unwrap();
    std::fs::write(installed.join("__Installer").join("installerdata.xml"), br#"<DiPManifest><unrelated><contentIDs><contentID>FAKE</contentID></contentIDs></unrelated></DiPManifest>"#).unwrap();
    let result = scan_ea_root(tmp.path());
    assert!(result.entries.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn ea_manifest_decodes_entities_without_truncating_text() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Entity Game");
    std::fs::create_dir(installed.join("__Installer")).unwrap();
    std::fs::write(installed.join("__Installer").join("installerdata.xml"), br#"<DiPManifest><contentIDs><contentID>OFB-EAST&#58;12345</contentID></contentIDs><gameTitles><gameTitle locale="en_US">One &amp; Two</gameTitle></gameTitles></DiPManifest>"#).unwrap();
    let result = scan_ea_root(tmp.path());
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].store_id, "OFB-EAST:12345");
    assert_eq!(result.entries[0].name, "One & Two");
}

#[test]
#[ignore = "reads installed launcher metadata on this machine"]
fn local_store_smoke_reports_counts_only() {
    let result = opticore::scan::stores::scan_installed();
    for platform in [
        Platform::Amazon,
        Platform::Ubisoft,
        Platform::Ea,
        Platform::BattleNet,
    ] {
        let count = result
            .entries
            .iter()
            .filter(|entry| entry.platform == platform)
            .count();
        println!("{}: {count}", platform.label());
    }
    println!("warnings: {}", result.warnings.len());
}

#[test]
fn battlenet_uses_product_code_not_title_and_rejects_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let installed = game_dir(tmp.path(), "Renamed Game");
    let agent = game_dir(tmp.path(), "Agent");
    let mut db = product("agent", &agent);
    db.extend(product("d4", &installed));
    db.extend(product("wow", &tmp.path().join("gone")));
    let result = scan_battlenet_db(&db);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].platform, Platform::BattleNet);
    assert_eq!(result.entries[0].store_id, "d4");
    assert_eq!(result.entries[0].name, "Renamed Game");
    assert_eq!(result.entries[0].source, DiscoverySource::LauncherLibrary);
    assert_eq!(result.entries[0].title_source, TitleSource::Folder);
}

#[test]
fn battlenet_rejects_malformed_or_oversized_wire_data() {
    let truncated = scan_battlenet_db(&[0x0a, 0x7f, 0x01]);
    assert!(truncated.entries.is_empty());
    assert!(!truncated.warnings.is_empty());
    let oversized = scan_battlenet_db(&vec![0u8; 16 * 1024 * 1024 + 1]);
    assert!(oversized.entries.is_empty());
    assert!(!oversized.warnings.is_empty());
}
