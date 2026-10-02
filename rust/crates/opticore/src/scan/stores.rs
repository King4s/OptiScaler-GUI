//! Read-only installed-game discovery for stores without library-root scans.
//! See docs/store-formats.md for the observed formats and confidence limits.

use super::{folder_facts, names};
use crate::model::{DiscoverySource, Platform, TitleSource};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_DB_BYTES: u64 = 16 * 1024 * 1024;
const MAX_XML_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RECORDS: usize = 4096;
const MAX_WARNINGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEntry {
    pub name: String,
    pub path: PathBuf,
    pub platform: Platform,
    pub store_id: String,
    pub art_url: Option<String>,
    pub source: DiscoverySource,
    pub title_source: TitleSource,
}

#[derive(Debug, Default)]
pub struct StoreScan {
    pub entries: Vec<StoreEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegistryRecord {
    pub key: String,
    pub install_dir: Option<String>,
    pub display_name: Option<String>,
    pub product_guid: Option<String>,
}

impl RegistryRecord {
    pub fn new(
        key: impl Into<String>,
        install_dir: Option<String>,
        display_name: Option<String>,
        product_guid: Option<String>,
    ) -> Self {
        Self {
            key: key.into(),
            install_dir,
            display_name,
            product_guid,
        }
    }
}

impl StoreScan {
    fn warn(&mut self, message: impl Into<String>) {
        if self.warnings.len() < MAX_WARNINGS {
            self.warnings.push(message.into());
        }
    }

    fn add(&mut self, platform: Platform, name: &str, id: &str, path: &str) {
        let (source, title_source) = match platform {
            Platform::Ubisoft | Platform::Ea => (
                DiscoverySource::Registry,
                if platform == Platform::Ea {
                    TitleSource::Store
                } else {
                    TitleSource::Folder
                },
            ),
            Platform::Amazon => (DiscoverySource::LauncherLibrary, TitleSource::Store),
            Platform::BattleNet => (DiscoverySource::LauncherLibrary, TitleSource::Folder),
            _ => unreachable!("only store adapters call StoreScan::add"),
        };
        self.add_with_provenance(platform, name, id, path, source, title_source);
    }

    fn add_with_provenance(
        &mut self,
        platform: Platform,
        name: &str,
        id: &str,
        path: &str,
        source: DiscoverySource,
        title_source: TitleSource,
    ) {
        let name = name.trim();
        let id = id.trim();
        let path = path.trim();
        if name.is_empty() || id.is_empty() || path.is_empty() || path.contains('\0') {
            self.warn(format!("{platform:?}: incomplete installed-game metadata"));
            return;
        }
        if is_launcher_name(name) {
            return;
        }
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            self.warn(format!("{platform:?}: relative install path for {id}"));
            return;
        }
        if path
            .file_name()
            .is_some_and(|name| is_launcher_name(&name.to_string_lossy()))
        {
            return;
        }
        if !path.is_dir() {
            self.warn(format!("{platform:?}: stale install path for {id}"));
            return;
        }
        let Some(facts) = folder_facts::collect(&path) else {
            self.warn(format!("{platform:?}: unreadable install path for {id}"));
            return;
        };
        if !folder_facts::is_game_folder(&path, &facts) {
            return;
        }
        self.entries.push(StoreEntry {
            name: name.to_owned(),
            path,
            platform,
            store_id: id.to_owned(),
            art_url: None,
            source,
            title_source,
        });
    }

    fn extend(&mut self, other: StoreScan) {
        self.entries.extend(other.entries);
        for warning in other.warnings {
            self.warn(warning);
        }
    }

    fn dedup(&mut self) {
        let mut seen = HashSet::new();
        self.entries.retain(|entry| {
            seen.insert((
                entry.platform,
                entry.store_id.to_lowercase(),
                crate::model::GameKey::path_key(&entry.path),
            ))
        });
    }
}

fn folder_title(path: &str) -> Option<String> {
    let title = Path::new(path)
        .file_name()?
        .to_string_lossy()
        .trim()
        .to_owned();
    (!title.is_empty()).then_some(title)
}

fn is_launcher_name(name: &str) -> bool {
    names::is_launcher_entry(name)
        || matches!(
            name.trim().to_ascii_lowercase().as_str(),
            "ea desktop"
                | "ea app"
                | "origin"
                | "ubisoft connect"
                | "ubisoft game launcher"
                | "amazon games"
                | "battle.net"
        )
}

fn read_bounded(path: &Path, limit: u64) -> std::io::Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "metadata exceeds size limit",
        ));
    }
    Ok(Some(bytes))
}

/// Parse records obtained from either Ubisoft registry view. The key is the ID.
pub fn scan_ubisoft_records(records: impl IntoIterator<Item = RegistryRecord>) -> StoreScan {
    let mut result = StoreScan::default();
    for (index, record) in records.into_iter().enumerate() {
        if index >= MAX_RECORDS {
            result.warn("Ubisoft: record limit reached");
            break;
        }
        if record.key.is_empty() || !record.key.bytes().all(|byte| byte.is_ascii_digit()) {
            result.warn("Ubisoft: ignored nonnumeric install key");
            continue;
        }
        let Some(path) = record.install_dir.as_deref() else {
            result.warn(format!("Ubisoft: missing InstallDir for {}", record.key));
            continue;
        };
        let Some(title) = folder_title(path.trim_end_matches(['/', '\\'])) else {
            result.warn(format!("Ubisoft: missing title for {}", record.key));
            continue;
        };
        result.add(Platform::Ubisoft, &title, &record.key, path);
    }
    result.dedup();
    result
}

/// Parse EA Games / Origin Games registry records. A title-shaped key is not an ID.
pub fn scan_ea_records(records: impl IntoIterator<Item = RegistryRecord>) -> StoreScan {
    let mut result = StoreScan::default();
    for (index, record) in records.into_iter().enumerate() {
        if index >= MAX_RECORDS {
            result.warn("EA: record limit reached");
            break;
        }
        let (Some(path), Some(title), Some(guid)) = (
            record.install_dir.as_deref(),
            record.display_name.as_deref(),
            record.product_guid.as_deref(),
        ) else {
            result.warn(format!(
                "EA: missing path, title or Product GUID for {}",
                record.key
            ));
            continue;
        };
        result.add(Platform::Ea, title, guid, path);
    }
    result.dedup();
    result
}

fn ea_manifest_fields(xml: &[u8]) -> Result<(String, Option<String>), String> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut id = None;
    let mut title = None;
    let mut preferred_title = None;
    let mut locale = None;
    let mut text = String::new();
    let mut collecting = false;
    let mut root_seen = false;
    let mut event_count = 0usize;
    loop {
        event_count += 1;
        if event_count > 50_000 {
            return Err("event limit reached".into());
        }
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| error.to_string())?
        {
            Event::Start(start) => {
                if collecting || stack.len() >= 64 {
                    return Err("nested metadata text or excessive XML depth".into());
                }
                let tag = start.name().as_ref().to_owned();
                if stack.is_empty() {
                    if root_seen || tag != "DiPManifest" {
                        return Err("expected one DiPManifest root".into());
                    }
                    root_seen = true;
                }
                if tag == "gameTitle" {
                    locale = start
                        .attributes()
                        .flatten()
                        .find(|attribute| attribute.key.as_ref() == "locale")
                        .map(|attribute| attribute.value.as_ref().to_owned());
                }
                stack.push(tag);
                collecting = matches!(stack.as_slice(), [root, parent, child]
                    if root == "DiPManifest" && ((parent == "contentIDs" && child == "contentID")
                        || (parent == "gameTitles" && child == "gameTitle")));
                if collecting {
                    text.clear();
                }
            }
            Event::Text(value) if collecting => {
                let decoded = quick_xml::escape::unescape(value.as_ref())
                    .map_err(|error| error.to_string())?;
                text.push_str(&decoded);
            }
            Event::GeneralRef(value) if collecting => {
                let name = value.as_ref();
                let reference = format!("&{name};");
                text.push_str(
                    &quick_xml::escape::unescape(&reference).map_err(|error| error.to_string())?,
                );
            }
            Event::CData(value) if collecting => {
                text.push_str(value.as_ref());
            }
            Event::End(_) => {
                if collecting {
                    let content = text.trim();
                    if !content.is_empty() {
                        if stack.last().is_some_and(|tag| tag == "contentID") {
                            id.get_or_insert_with(|| content.to_owned());
                        } else if locale.as_deref() == Some("en_US") {
                            preferred_title = Some(content.to_owned());
                        } else {
                            title.get_or_insert_with(|| content.to_owned());
                        }
                    }
                    collecting = false;
                }
                if stack.pop().is_none() {
                    return Err("unmatched end tag".into());
                }
                locale = None;
            }
            Event::DocType(_) => return Err("DTDs are unsupported".into()),
            Event::Empty(_) if collecting => return Err("nested metadata element".into()),
            Event::Eof => break,
            _ => {}
        }
        if text.len() > 16_384 {
            return Err("metadata text exceeds size limit".into());
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err("truncated manifest".into());
    }
    let id = id.ok_or("missing contentID")?;
    Ok((id, preferred_title.or(title)))
}

/// Read one EA game's own installer manifest. A missing manifest is not an error.
pub fn scan_ea_manifest(folder: &Path) -> StoreScan {
    let mut result = StoreScan::default();
    let file = folder.join("__Installer").join("installerdata.xml");
    let xml = match read_bounded(&file, MAX_XML_BYTES) {
        Ok(Some(xml)) => xml,
        Ok(None) => return result,
        Err(error) => {
            result.warn(format!("EA: cannot read installerdata.xml: {error}"));
            return result;
        }
    };
    let (id, manifest_title) = match ea_manifest_fields(&xml) {
        Ok(fields) => fields,
        Err(error) => {
            result.warn(format!("EA: malformed installerdata.xml: {error}"));
            return result;
        }
    };
    let path = folder.to_string_lossy();
    let title_source = if manifest_title.is_some() {
        TitleSource::Store
    } else {
        TitleSource::Folder
    };
    let title = manifest_title.or_else(|| folder_title(&path));
    if let Some(title) = title {
        result.add_with_provenance(
            Platform::Ea,
            &title,
            &id,
            &path,
            DiscoverySource::StoreManifest,
            title_source,
        );
    }
    result
}

/// Scan immediate children of an EA library root, without recursive root discovery.
pub fn scan_ea_root(root: &Path) -> StoreScan {
    let mut result = StoreScan::default();
    let children = match std::fs::read_dir(root) {
        Ok(children) => children,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return result,
        Err(error) => {
            result.warn(format!("EA: cannot list game root: {error}"));
            return result;
        }
    };
    for (index, child) in children.enumerate() {
        if index >= MAX_RECORDS {
            result.warn("EA: root entry limit reached");
            break;
        }
        match child {
            Ok(child) if child.path().is_dir() => result.extend(scan_ea_manifest(&child.path())),
            Ok(_) => {}
            Err(error) => result.warn(format!("EA: cannot read root entry: {error}")),
        }
    }
    result.dedup();
    result
}

/// Read Amazon's installed-game table without creating or changing the DB.
pub fn scan_amazon_db(db: &Path) -> StoreScan {
    let mut result = StoreScan::default();
    let size = match std::fs::metadata(db) {
        Ok(meta) => meta.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return result,
        Err(error) => {
            result.warn(format!("Amazon: cannot stat database: {error}"));
            return result;
        }
    };
    if size > MAX_DB_BYTES {
        result.warn("Amazon: database exceeds size limit");
        return result;
    }
    let conn = match rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => conn,
        Err(error) => {
            result.warn(format!("Amazon: cannot open database: {error}"));
            return result;
        }
    };
    if let Err(error) = conn.busy_timeout(std::time::Duration::ZERO) {
        result.warn(format!("Amazon: cannot set database timeout: {error}"));
        return result;
    }
    let mut stmt = match conn.prepare(
        "SELECT ProductAsin, ProductTitle, InstallDirectory FROM DbSet WHERE Installed = 1 LIMIT 4097",
    ) {
        Ok(stmt) => stmt,
        Err(error) => {
            result.warn(format!("Amazon: unsupported database schema: {error}"));
            return result;
        }
    };
    let mut rows = match stmt.query([]) {
        Ok(rows) => rows,
        Err(error) => {
            result.warn(format!("Amazon: query failed: {error}"));
            return result;
        }
    };
    for index in 0..=MAX_RECORDS {
        match rows.next() {
            Ok(Some(row)) if index < MAX_RECORDS => {
                let fields = (
                    row.get::<_, String>(0),
                    row.get::<_, String>(1),
                    row.get::<_, String>(2),
                );
                match fields {
                    (Ok(id), Ok(title), Ok(path)) => {
                        result.add(Platform::Amazon, &title, &id, &path)
                    }
                    _ => result.warn("Amazon: malformed installed-game row"),
                }
            }
            Ok(Some(_)) => {
                result.warn("Amazon: record limit reached");
                break;
            }
            Ok(None) => break,
            Err(error) => {
                result.warn(format!("Amazon: row read failed: {error}"));
                break;
            }
        }
    }
    result.dedup();
    result
}

fn varint(bytes: &[u8], cursor: &mut usize) -> Result<u64, &'static str> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*cursor).ok_or("truncated varint")?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err("varint overflow");
        }
        value |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("varint too long")
}

fn fields<'a>(
    mut bytes: &'a [u8],
    mut visit: impl FnMut(u64, u8, &'a [u8]) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let mut count = 0usize;
    while !bytes.is_empty() {
        count += 1;
        if count > 50_000 {
            return Err("too many protobuf fields");
        }
        let mut cursor = 0;
        let tag = varint(bytes, &mut cursor)?;
        let number = tag >> 3;
        let wire = (tag & 7) as u8;
        if number == 0 {
            return Err("zero protobuf field number");
        }
        let value = match wire {
            0 => {
                let start = cursor;
                varint(bytes, &mut cursor)?;
                &bytes[start..cursor]
            }
            1 => {
                let end = cursor.checked_add(8).ok_or("field overflow")?;
                let value = bytes.get(cursor..end).ok_or("truncated fixed64")?;
                cursor = end;
                value
            }
            2 => {
                let len = usize::try_from(varint(bytes, &mut cursor)?)
                    .map_err(|_| "field length overflow")?;
                let end = cursor.checked_add(len).ok_or("field overflow")?;
                let value = bytes
                    .get(cursor..end)
                    .ok_or("truncated length-delimited field")?;
                cursor = end;
                value
            }
            5 => {
                let end = cursor.checked_add(4).ok_or("field overflow")?;
                let value = bytes.get(cursor..end).ok_or("truncated fixed32")?;
                cursor = end;
                value
            }
            _ => return Err("unsupported protobuf wire type"),
        };
        visit(number, wire, value)?;
        bytes = &bytes[cursor..];
    }
    Ok(())
}

fn product_fields(bytes: &[u8]) -> Result<(Option<&str>, Option<&str>), &'static str> {
    let mut code = None;
    let mut path = None;
    fields(bytes, |number, wire, value| {
        if wire == 2 && number == 2 {
            code = Some(std::str::from_utf8(value).map_err(|_| "invalid product code")?);
        } else if wire == 2 && number == 3 {
            fields(value, |number, wire, value| {
                if number == 1 && wire == 2 {
                    path = Some(std::str::from_utf8(value).map_err(|_| "invalid install path")?);
                }
                Ok(())
            })?;
        }
        Ok(())
    })?;
    Ok((code, path))
}

/// Decode only documented ProductDb fields, with no generated protobuf dependency.
pub fn scan_battlenet_db(bytes: &[u8]) -> StoreScan {
    let mut result = StoreScan::default();
    if bytes.len() > MAX_DB_BYTES as usize {
        result.warn("Battle.net: database exceeds size limit");
        return result;
    }
    let mut products = 0usize;
    let parsed = fields(bytes, |number, wire, value| {
        if number != 1 || wire != 2 {
            return Ok(());
        }
        products += 1;
        if products > MAX_RECORDS {
            return Err("product limit reached");
        }
        let (code, path) = product_fields(value)?;
        let (Some(code), Some(path)) = (code, path) else {
            result.warn("Battle.net: incomplete product record");
            return Ok(());
        };
        if matches!(code.to_ascii_lowercase().as_str(), "agent" | "bna") {
            return Ok(());
        }
        let Some(title) = folder_title(path.trim_end_matches(['/', '\\'])) else {
            result.warn(format!("Battle.net: missing folder title for {code}"));
            return Ok(());
        };
        result.add(Platform::BattleNet, &title, code, path);
        Ok(())
    });
    if let Err(error) = parsed {
        result.warn(format!("Battle.net: malformed product.db: {error}"));
    }
    result.dedup();
    result
}

/// Probe known launcher metadata locations. Missing launchers are not errors.
pub fn scan_installed() -> StoreScan {
    let mut result = StoreScan::default();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        let root = PathBuf::from(program_files);
        result.extend(scan_ea_root(&root.join("EA Games")));
        result.extend(scan_ea_root(&root.join("Electronic Arts").join("EA Games")));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        let root = PathBuf::from(program_files_x86);
        result.extend(scan_ea_root(&root.join("EA Games")));
        result.extend(scan_ea_root(&root.join("Origin Games")));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        result.extend(scan_amazon_db(
            &PathBuf::from(local).join(r"Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite"),
        ));
    }
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        let db = PathBuf::from(program_data).join(r"Battle.net\Agent\product.db");
        match read_bounded(&db, MAX_DB_BYTES) {
            Ok(Some(bytes)) => result.extend(scan_battlenet_db(&bytes)),
            Ok(None) => {}
            Err(error) => result.warn(format!("Battle.net: cannot read product.db: {error}")),
        }
    }
    #[cfg(windows)]
    scan_windows_registry(&mut result);
    result.dedup();
    result
}

#[cfg(windows)]
fn registry_records(
    root: &str,
    label: &str,
    install_value: &str,
    result: &mut StoreScan,
) -> Vec<RegistryRecord> {
    let key = match windows_registry::LOCAL_MACHINE.open(root) {
        Ok(key) => key,
        Err(error) => {
            if !matches!(error.code().0 as u32, 0x80070002 | 0x80070003) {
                result.warn(format!("{label}: cannot open install registry: {error}"));
            }
            return Vec::new();
        }
    };
    let children = match key.keys() {
        Ok(children) => children,
        Err(error) => {
            result.warn(format!("{label}: cannot list install registry: {error}"));
            return Vec::new();
        }
    };
    let mut records = Vec::new();
    for (index, name) in children.enumerate() {
        if index >= MAX_RECORDS {
            result.warn(format!("{label}: registry entry limit reached"));
            break;
        }
        match key.open(&name) {
            Ok(child) => records.push(RegistryRecord::new(
                name,
                child.get_string(install_value).ok(),
                child.get_string("DisplayName").ok(),
                child.get_string("Product GUID").ok(),
            )),
            Err(error) => result.warn(format!(
                "{label}: cannot read install registry entry: {error}"
            )),
        }
    }
    records
}

#[cfg(windows)]
fn scan_windows_registry(result: &mut StoreScan) {
    for root in [
        r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs",
        r"SOFTWARE\Ubisoft\Launcher\Installs",
    ] {
        let records = registry_records(root, "Ubisoft", "InstallDir", result);
        result.extend(scan_ubisoft_records(records));
    }
    for root in [
        r"SOFTWARE\WOW6432Node\EA Games",
        r"SOFTWARE\EA Games",
        r"SOFTWARE\WOW6432Node\Origin Games",
        r"SOFTWARE\Origin Games",
        r"SOFTWARE\Respawn",
        r"SOFTWARE\WOW6432Node\Respawn",
        r"SOFTWARE\Maxis",
        r"SOFTWARE\WOW6432Node\Maxis",
        r"SOFTWARE\BioWare",
        r"SOFTWARE\WOW6432Node\BioWare",
    ] {
        let records = registry_records(root, "EA", "Install Dir", result);
        let mut fallback = Vec::new();
        for record in records {
            let manifest = record
                .install_dir
                .as_deref()
                .map(|path| scan_ea_manifest(Path::new(path)))
                .unwrap_or_default();
            if manifest.entries.is_empty() {
                fallback.push(record);
            }
            result.extend(manifest);
        }
        result.extend(scan_ea_records(fallback));
    }
}
