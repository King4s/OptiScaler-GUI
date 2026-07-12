//! Xbox / Game Pass helpers: XboxGames packaging heuristics and roots.

use std::fs;
use std::path::{Path, PathBuf};

/// Known Xbox roots (XboxGames on any fixed drive is added by discovery).
pub fn default_roots() -> Vec<PathBuf> {
    [r"C:\XboxGames", r"C:\Program Files\WindowsApps"]
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect()
}

/// Xbox streaming/packaging file extensions identifying a Game Pass title.
const XBOX_GAME_EXTENSIONS: &[&str] = &["xsp", "smd", "xct", "xvi"];

/// True if the folder looks like an Xbox Game Pass title (packaging files,
/// gamelaunchhelper.exe, or a Content subfolder). Port of `_is_xbox_game_folder`.
pub fn is_xbox_game_folder(game_folder: &Path) -> bool {
    let Ok(entries) = fs::read_dir(game_folder) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if let Some(ext) = Path::new(&name).extension().and_then(|e| e.to_str()) {
            if XBOX_GAME_EXTENSIONS.contains(&ext) {
                return true;
            }
        }
        if name == "gamelaunchhelper.exe" {
            return true;
        }
        if name == "content" && entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn detects_packaging_files() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("XGame");
        fs::create_dir_all(&game).unwrap();
        File::create(game.join("data.xsp")).unwrap();
        assert!(is_xbox_game_folder(&game));
    }

    #[test]
    fn detects_gamelaunchhelper() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("XGame2");
        fs::create_dir_all(&game).unwrap();
        File::create(game.join("GameLaunchHelper.exe")).unwrap();
        assert!(is_xbox_game_folder(&game));
    }

    #[test]
    fn plain_folder_is_not_xbox() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Plain");
        fs::create_dir_all(&game).unwrap();
        File::create(game.join("app.exe")).unwrap();
        assert!(!is_xbox_game_folder(&game));
    }
}
