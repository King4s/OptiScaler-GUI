//! Launch installed games, with or without the OptiScaler proxy.
//!
//! "Without" renames our proxy DLL to `<name>.optiscaler-disabled` so the
//! game boots clean; a later "with" launch renames it back. Only files named
//! in our own install manifest are ever touched — a game's original DLLs
//! are never renamed.

use crate::install::{manifest, payload};
use crate::model::{Game, Platform};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const DISABLED_SUFFIX: &str = ".optiscaler-disabled";

fn entry_metadata(path: &Path) -> io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn reject_reparse(metadata: &std::fs::Metadata) -> io::Result<()> {
    if metadata.file_type().is_symlink() {
        return Err(io::Error::other("Proxy path is a link; preserved"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::other("Proxy path is a reparse point; preserved"));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn rename_without_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileW(existing: *const u16, new: *const u16) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // MoveFileW fails if the destination exists; std::fs::rename may replace it.
    if unsafe { MoveFileW(source.as_ptr(), destination.as_ptr()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn rename_without_replace(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::hard_link(source, destination)?;
    std::fs::remove_file(source)
}

/// How a game will be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchMethod {
    /// Through the Steam client (overlay, cloud saves, playtime).
    SteamUrl(String),
    /// Direct executable start.
    Exe(PathBuf),
}

/// Pick the best launch method for a game.
pub fn resolve_method(game: &Game) -> Option<LaunchMethod> {
    if game.platform == Platform::Steam {
        if let Some(appid) = game.steam_appid {
            return Some(LaunchMethod::SteamUrl(format!("steam://rungameid/{appid}")));
        }
    }
    if game.platform == Platform::Xbox {
        // Game Pass ships its own launcher next to the game exe; starting
        // the exe directly is blocked by licensing/ACLs.
        for candidate in [
            game.path.join("Content").join("gamelaunchhelper.exe"),
            game.path.join("gamelaunchhelper.exe"),
        ] {
            if candidate.exists() {
                return Some(LaunchMethod::Exe(candidate));
            }
        }
    }
    crate::resolver::resolve(&game.path)
        .ok()
        .map(|t| LaunchMethod::Exe(t.executable))
}

/// The manifest-recorded proxy target for a game, if we installed one.
fn manifest_target(game_path: &Path) -> Option<(PathBuf, String)> {
    let install_dir = crate::resolver::resolve(game_path).ok()?.directory;
    let m = manifest::read(&install_dir)?;
    if !payload::PROXY_FILENAMES.contains(&m.target_filename.as_str()) {
        return None;
    }
    Some((install_dir, m.target_filename))
}

/// True when the proxy is currently renamed away ("play without" state).
pub fn optiscaler_bypassed(game_path: &Path) -> bool {
    match manifest_target(game_path) {
        Some((dir, target)) => dir.join(format!("{target}{DISABLED_SUFFIX}")).exists(),
        None => false,
    }
}

/// Enable or bypass the installed proxy by renaming it. No-op (Ok(false))
/// when there is no manifest or the state is already as requested.
pub fn set_optiscaler_enabled(game_path: &Path, enabled: bool) -> std::io::Result<bool> {
    let Some((install_dir, target)) = manifest_target(game_path) else {
        return Ok(false);
    };
    let active = install_dir.join(&target);
    let disabled = install_dir.join(format!("{target}{DISABLED_SUFFIX}"));
    let active_meta = entry_metadata(&active)?;
    let disabled_meta = entry_metadata(&disabled)?;
    if active_meta.is_some() && disabled_meta.is_some() {
        return Err(io::Error::other("Both proxy states exist; preserved"));
    }
    if (enabled && active_meta.is_some()) || (!enabled && disabled_meta.is_some()) {
        return Ok(false);
    }
    let (source, destination, source_meta, destination_meta) = if enabled {
        (&disabled, &active, disabled_meta, active_meta)
    } else {
        (&active, &disabled, active_meta, disabled_meta)
    };
    let Some(source_meta) = source_meta else {
        return Ok(false);
    };
    reject_reparse(&source_meta)?;
    if let Some(metadata) = &destination_meta {
        reject_reparse(metadata)?;
        return Err(io::Error::other(
            "Proxy destination already exists; preserved",
        ));
    }
    if !source_meta.is_file()
        || !payload::path_within(source, &install_dir)
        || !payload::path_within(destination, &install_dir)
    {
        return Err(io::Error::other(
            "Proxy path is not a regular file in the game directory",
        ));
    }
    let m =
        manifest::read(&install_dir).ok_or_else(|| std::io::Error::other("Missing manifest"))?;
    let owned = m
        .owned_files
        .get(&target)
        .filter(|_| m.is_owned_v2())
        .ok_or_else(|| std::io::Error::other("Legacy installation: proxy ownership is unknown"))?;
    if owned.sha256.len() != 64
        || !owned.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || crate::install::github::verify_digest(source, Some(&format!("sha256:{}", owned.sha256)))
            .is_err()
    {
        return Err(std::io::Error::other(
            "Proxy changed or outside game directory; preserved",
        ));
    }
    // Recheck the destination immediately before rename; never intentionally replace it.
    if entry_metadata(destination)?.is_some() {
        return Err(io::Error::other("Proxy destination appeared; preserved"));
    }
    rename_without_replace(source, destination)?;
    Ok(true)
}

/// Toggle the proxy as requested, then start the game. Returns a log line.
pub fn launch(game: &Game, with_optiscaler: bool) -> Result<String, String> {
    set_optiscaler_enabled(&game.path, with_optiscaler)
        .map_err(|e| format!("Could not toggle OptiScaler proxy: {e}"))?;
    let method = resolve_method(game).ok_or("no executable found")?;
    let suffix = if with_optiscaler {
        "with OptiScaler"
    } else {
        "without OptiScaler"
    };
    match method {
        LaunchMethod::SteamUrl(url) => {
            // explorer hands the steam:// URL to the protocol handler
            Command::new("explorer.exe")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(format!("Launching {} via Steam {suffix}", game.name))
        }
        LaunchMethod::Exe(exe) => {
            let workdir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
            Command::new(&exe)
                .current_dir(workdir)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "Launching {} ({}) {suffix}",
                game.name,
                exe.display()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::manifest::{InstallManifest, OwnedFile};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;

    fn game_with_manifest(dir: &Path, target: &str) {
        std::fs::write(dir.join("Game.exe"), b"game").unwrap();
        let m = InstallManifest::new(
            target,
            &[target.to_string()],
            &[],
            "v0.9.3",
            None,
            "2026-07-13T00:00:00".to_string(),
        );
        let mut m = m;
        m.schema_version = 2;
        m.owned_files = BTreeMap::from([(
            target.to_string(),
            OwnedFile {
                sha256: hex::encode(Sha256::digest(b"proxy")),
                original_backup: None,
                original_sha256: None,
                extra: BTreeMap::new(),
            },
        )]);
        manifest::write(dir, &m).unwrap();
        std::fs::write(dir.join(target), b"proxy").unwrap();
    }

    #[test]
    fn bypass_renames_and_restore_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        game_with_manifest(tmp.path(), "dxgi.dll");

        assert!(!optiscaler_bypassed(tmp.path()));
        assert!(set_optiscaler_enabled(tmp.path(), false).unwrap());
        assert!(optiscaler_bypassed(tmp.path()));
        assert!(!tmp.path().join("dxgi.dll").exists());
        assert!(tmp.path().join("dxgi.dll.optiscaler-disabled").exists());

        // idempotent
        assert!(!set_optiscaler_enabled(tmp.path(), false).unwrap());

        assert!(set_optiscaler_enabled(tmp.path(), true).unwrap());
        assert!(!optiscaler_bypassed(tmp.path()));
        assert!(tmp.path().join("dxgi.dll").exists());
    }

    #[test]
    fn no_manifest_means_no_touching_game_files() {
        let tmp = tempfile::tempdir().unwrap();
        // A game that ships its OWN dxgi.dll but has no OptiScaler manifest
        std::fs::write(tmp.path().join("Game.exe"), b"game").unwrap();
        std::fs::write(tmp.path().join("dxgi.dll"), b"the game's own dll").unwrap();
        assert!(!set_optiscaler_enabled(tmp.path(), false).unwrap());
        assert!(tmp.path().join("dxgi.dll").exists());
        assert!(!optiscaler_bypassed(tmp.path()));
    }

    #[test]
    fn steam_games_resolve_to_steam_url() {
        let mut game = Game::new("Test", std::env::temp_dir(), Platform::Steam);
        game.steam_appid = Some(123);
        assert_eq!(
            resolve_method(&game),
            Some(LaunchMethod::SteamUrl("steam://rungameid/123".into()))
        );
    }

    #[test]
    fn xbox_prefers_gamelaunchhelper() {
        let tmp = tempfile::tempdir().unwrap();
        let content = tmp.path().join("Content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("gamelaunchhelper.exe"), b"x").unwrap();
        std::fs::write(content.join("Game.exe"), vec![0u8; 4096]).unwrap();

        let game = Game::new("XG", tmp.path().to_path_buf(), Platform::Xbox);
        assert_eq!(
            resolve_method(&game),
            Some(LaunchMethod::Exe(content.join("gamelaunchhelper.exe")))
        );
    }

    #[test]
    fn ambiguous_exes_require_explicit_selection() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("small.exe"), vec![0u8; 100]).unwrap();
        std::fs::write(tmp.path().join("big.exe"), vec![0u8; 10_000]).unwrap();
        let game = Game::new("G", tmp.path().to_path_buf(), Platform::Gog);
        assert_eq!(resolve_method(&game), None);

        crate::resolver::remember(tmp.path(), &tmp.path().join("big.exe")).unwrap();
        assert_eq!(
            resolve_method(&game),
            Some(LaunchMethod::Exe(
                tmp.path().join("big.exe").canonicalize().unwrap()
            ))
        );
    }
}
