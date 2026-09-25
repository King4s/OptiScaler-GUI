//! Read-only, best-effort facts about a discovered game and its install target.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameObservation {
    pub name: String,
    pub platform: String,
    pub store_id: Option<String>,
    pub build_version: Option<String>,
    pub executable: Option<PathBuf>,
    pub target_directory: Option<PathBuf>,
    pub optiscaler_version: Option<String>,
    /// A recorded GUI install with its proxy present, not verified runtime loading.
    pub installed: bool,
    /// Named proxy/upscaler DLLs present; each is only an indication, not ownership proof.
    pub dll_hints: Vec<String>,
    pub loaded_from_log: Option<bool>,
    /// Modification time of OptiScaler.log, if available. The log may be stale.
    pub log_modified: Option<String>,
}

/// Observes local state without changing the game directory. This may block on disk I/O.
pub fn observe(game: &crate::model::Game) -> GameObservation {
    let mut observation = GameObservation {
        name: game.name.clone(),
        platform: game.platform.label().to_string(),
        store_id: if game.platform == crate::model::Platform::Steam {
            game.steam_appid.map(|id| id.to_string())
        } else {
            None
        },
        build_version: steam_build(game),
        ..GameObservation::default()
    };

    let Ok(target) = crate::resolver::resolve(&game.path) else {
        return observation;
    };
    let log_directory = target.executable.parent().map(Path::to_path_buf);
    observation.executable = Some(target.executable);
    observation.target_directory = Some(target.directory.clone());

    let valid_manifest = crate::install::manifest::read(&target.directory).filter(|record| {
        (record.schema_version == 1 || (record.schema_version == 2 && record.is_owned_v2()))
            && record.installed_by == "OptiScaler-GUI"
            && crate::install::payload::PROXY_FILENAMES
                .iter()
                .chain(crate::install::payload::LEGACY_PROXY_FILENAMES.iter())
                .any(|name| *name == record.target_filename.as_str())
            && target.directory.join(&record.target_filename).is_file()
    });
    observation.installed = valid_manifest.is_some();
    observation.optiscaler_version = valid_manifest
        .as_ref()
        .map(|record| record.optiscaler_version.clone())
        .filter(|version| !version.is_empty());

    for filename in crate::install::payload::PROXY_FILENAMES
        .iter()
        .chain(crate::install::payload::LEGACY_PROXY_FILENAMES.iter())
        .copied()
        .chain([
            "nvngx_dlss.dll",
            "libxess.dll",
            "amd_fidelityfx_dx12.dll",
            "ffx_fsr2_api_x64.dll",
            "sl.interposer.dll",
            "vulkan-1.dll",
        ])
        .filter(|name| name.ends_with(".dll"))
    {
        if target.directory.join(filename).is_file() {
            observation.dll_hints.push(filename.to_string());
        }
    }

    let log_path = log_directory.map(|directory| directory.join("OptiScaler.log"));
    observation.log_modified = log_path
        .as_ref()
        .and_then(|path| path.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| {
            time::OffsetDateTime::from(modified)
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        });

    if observation.installed && observation.optiscaler_version.as_deref() == Some("v0.9.4") {
        observation.loaded_from_log = log_path.as_deref().and_then(v094_log_loaded);
    }
    observation
}

fn v094_log_loaded(path: &Path) -> Option<bool> {
    const MAX_LOG_BYTES: u64 = 1024 * 1024;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_LOG_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_LOG_BYTES {
        return None;
    }
    let text = std::str::from_utf8(&bytes).ok()?;
    text.lines().any(v094_init_line).then_some(true)
}

fn v094_init_line(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    let Some((timestamp, message)) = line
        .strip_prefix('[')
        .and_then(|rest| rest.split_once("] [I] "))
        .or_else(|| line.split_once("\tI\t"))
    else {
        return false;
    };
    let bytes = timestamp.as_bytes();
    bytes.len() == 15
        && [2, 5].iter().all(|&index| bytes[index] == b':')
        && bytes[8] == b'.'
        && [0, 1, 3, 4, 6, 7]
            .iter()
            .all(|&index| bytes[index].is_ascii_digit())
        && bytes[9..].iter().all(u8::is_ascii_digit)
        && message == "Init done"
}

fn steam_build(game: &crate::model::Game) -> Option<String> {
    if game.platform != crate::model::Platform::Steam {
        return None;
    }
    let appid = game.steam_appid?;
    let common = game.path.parent()?;
    if !common.file_name()?.eq_ignore_ascii_case("common") {
        return None;
    }
    let steamapps = common.parent()?;
    if !steamapps.file_name()?.eq_ignore_ascii_case("steamapps") {
        return None;
    }
    let path = steamapps.join(format!("appmanifest_{appid}.acf"));
    parse_buildid(&path)
}

fn parse_buildid(path: &Path) -> Option<String> {
    const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return None;
    }
    let content = std::str::from_utf8(&bytes).ok()?;
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix("\"buildid\"") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(value) = rest.strip_prefix('"') else {
            continue;
        };
        let Some((digits, trailing)) = value.split_once('"') else {
            continue;
        };
        if !trailing.trim().is_empty()
            || digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        return digits.parse::<u64>().ok().map(|id| id.to_string());
    }
    None
}
