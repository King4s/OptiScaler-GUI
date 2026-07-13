//! Launch installed games, with or without the OptiScaler proxy.
//!
//! "Without" renames our proxy DLL to `<name>.optiscaler-disabled` so the
//! game boots clean; a later "with" launch renames it back. Only files named
//! in our own install manifest are ever touched — a game's original DLLs
//! are never renamed.

use crate::install::{manifest, payload};
use crate::library::GameEntry;
use crate::model::{Game, Platform};
use std::path::{Path, PathBuf};
use std::process::Command;

const DISABLED_SUFFIX: &str = ".optiscaler-disabled";

/// Per-game launch options from the user's library entry.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Extra command-line arguments (whitespace-split; "quoted parts" kept).
    pub args: Vec<String>,
    /// Absolute path replacing the auto-detected exe (direct launches only).
    pub exe_override: Option<PathBuf>,
    /// Extra environment variables (direct launches only).
    pub env: Vec<(String, String)>,
}

impl LaunchOptions {
    pub fn from_entry(entry: &GameEntry) -> Self {
        Self {
            args: split_args(&entry.launch_args),
            exe_override: (!entry.exe_override.is_empty())
                .then(|| PathBuf::from(&entry.exe_override)),
            env: entry
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

/// Split a command-line string into arguments, honoring "double quotes".
pub fn split_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in raw.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// How a game will be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchMethod {
    /// Through the Steam client (overlay, cloud saves, playtime).
    SteamUrl(String),
    /// Through the Epic launcher protocol (EOS auth, cloud saves).
    EpicUrl(String),
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
    if matches!(game.platform, Platform::Epic | Platform::Heroic) {
        if let Some(app_name) = crate::scan::epic::read_app_name(&game.path) {
            return Some(LaunchMethod::EpicUrl(crate::stores::epic::launch_url(
                &app_name,
            )));
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
    crate::images::largest_exe(&game.path).map(LaunchMethod::Exe)
}

/// The manifest-recorded proxy target for a game, if we installed one.
fn manifest_target(game_path: &Path) -> Option<(PathBuf, String)> {
    let install_dir = payload::determine_install_directory(game_path);
    let m = manifest::read(&install_dir)?;
    if m.target_filename.is_empty() {
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
    if enabled {
        if disabled.exists() && !active.exists() {
            std::fs::rename(&disabled, &active)?;
            return Ok(true);
        }
    } else if active.exists() {
        if disabled.exists() {
            std::fs::remove_file(&disabled)?;
        }
        std::fs::rename(&active, &disabled)?;
        return Ok(true);
    }
    Ok(false)
}

/// Toggle the proxy as requested, then start the game. Returns a log line.
pub fn launch(game: &Game, with_optiscaler: bool) -> Result<String, String> {
    launch_with_options(game, with_optiscaler, &LaunchOptions::default())
}

/// Like [`launch`], applying the user's per-game launch options: an exe
/// override wins over auto-detection, extra args are appended (for Steam
/// launches via `steam://run/<id>//<args>/`), env vars apply to direct
/// exe starts.
pub fn launch_with_options(
    game: &Game,
    with_optiscaler: bool,
    options: &LaunchOptions,
) -> Result<String, String> {
    set_optiscaler_enabled(&game.path, with_optiscaler)
        .map_err(|e| format!("Could not toggle OptiScaler proxy: {e}"))?;
    let method = match &options.exe_override {
        Some(exe) if exe.exists() => LaunchMethod::Exe(exe.clone()),
        Some(exe) => return Err(format!("launch exe not found: {}", exe.display())),
        None => resolve_method(game).ok_or("no executable found")?,
    };
    let suffix = if with_optiscaler {
        "with OptiScaler"
    } else {
        "without OptiScaler"
    };
    match method {
        LaunchMethod::SteamUrl(url) => {
            // steam://run/<appid>//<args>/ passes arguments through Steam
            let url = if options.args.is_empty() {
                url
            } else if let Some(appid) = game.steam_appid {
                format!("steam://run/{appid}//{}/", options.args.join(" "))
            } else {
                url
            };
            // explorer hands the steam:// URL to the protocol handler
            Command::new("explorer.exe")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(format!("Launching {} via Steam {suffix}", game.name))
        }
        LaunchMethod::EpicUrl(url) => {
            Command::new("explorer.exe")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(format!("Launching {} via Epic {suffix}", game.name))
        }
        LaunchMethod::Exe(exe) => {
            let workdir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
            let mut command = Command::new(&exe);
            command.current_dir(workdir).args(&options.args);
            for (key, value) in &options.env {
                command.env(key, value);
            }
            command.spawn().map_err(|e| e.to_string())?;
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
    use crate::install::manifest::InstallManifest;

    fn game_with_manifest(dir: &Path, target: &str) {
        let m = InstallManifest::new(
            target,
            &[target.to_string()],
            &[],
            "v0.9.3",
            None,
            "2026-07-13T00:00:00".to_string(),
        );
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
    fn arg_splitting_honors_quotes() {
        assert_eq!(split_args(""), Vec::<String>::new());
        assert_eq!(
            split_args("-windowed -w 1920"),
            vec!["-windowed", "-w", "1920"]
        );
        assert_eq!(
            split_args(r#"--path "C:\My Games\save" -x"#),
            vec!["--path", r"C:\My Games\save", "-x"]
        );
    }

    #[test]
    fn options_from_library_entry() {
        let mut entry = GameEntry {
            launch_args: "-dx12".into(),
            exe_override: r"C:\g\game.exe".into(),
            ..Default::default()
        };
        entry.env.insert("MANGOHUD".into(), "1".into());
        let options = LaunchOptions::from_entry(&entry);
        assert_eq!(options.args, vec!["-dx12"]);
        assert_eq!(options.exe_override, Some(PathBuf::from(r"C:\g\game.exe")));
        assert_eq!(options.env, vec![("MANGOHUD".to_string(), "1".to_string())]);
    }

    #[test]
    fn missing_exe_override_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let game = Game::new("G", tmp.path().to_path_buf(), Platform::Manual);
        let options = LaunchOptions {
            exe_override: Some(tmp.path().join("nope.exe")),
            ..Default::default()
        };
        assert!(launch_with_options(&game, true, &options)
            .unwrap_err()
            .contains("not found"));
    }

    #[test]
    fn fallback_is_largest_exe() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("small.exe"), vec![0u8; 100]).unwrap();
        std::fs::write(tmp.path().join("big.exe"), vec![0u8; 10_000]).unwrap();
        let game = Game::new("G", tmp.path().to_path_buf(), Platform::Gog);
        assert_eq!(
            resolve_method(&game),
            Some(LaunchMethod::Exe(tmp.path().join("big.exe")))
        );
    }
}
