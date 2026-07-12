//! Single-pass bounded folder walk feeding all per-game detectors.
//! Port of the Python `FolderFacts` / `_collect_folder_facts` design
//! (one traversal per game folder instead of 3–4).

use crate::model::{AntiCheat, Engine};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_SCAN_DEPTH: usize = 3;
pub const MAX_FILES_TO_CHECK: usize = 1000;

/// File suffixes that indicate significant game content.
const GAME_CONTENT_SUFFIXES: &[&str] = &[".pak", ".uasset", ".dll", ".bin", ".unity3d"];

/// Common non-game folder names excluded from game detection
/// (matched as substrings of the folder name, case-insensitive).
const EXCLUDE_FOLDERS: &[&str] = &[
    "_commonredist",
    "redist",
    "sdk",
    "tools",
    "server",
    "addons",
    "dlc",
    "soundtrack",
    "artbook",
    "manuals",
    "bonuscontent",
    "support",
    "installer",
];

/// OptiScaler indicator files (port of `OPTISCALER_INDICATOR_FILES`).
const OPTISCALER_FILES: &[&str] = &[
    "nvngx_dlss.dll",
    "nvngx_dlssg.dll",
    "optiscaler.dll",
    "nvngx.dll",
    "dxgi.dll",
    "winmm.dll",
];
const OPTISCALER_SUBDIRS: &[&str] = &["D3D12_Optiscaler", "OptiScaler", "mods", "plugins"];

#[derive(Debug, Default)]
pub struct FolderFacts {
    pub root: PathBuf,
    /// Lowercase file names directly in the folder.
    pub top_files: Vec<String>,
    /// Lowercase directory names directly in the folder.
    pub top_dirs: Vec<String>,
    /// Lowercase file names in direct child directories.
    pub depth1_files: Vec<String>,
    pub file_count: usize,
    pub found_exe: bool,
    pub found_game_content: bool,
}

/// Walk `root` up to MAX_SCAN_DEPTH / MAX_FILES_TO_CHECK, collecting facts.
/// Returns None if the folder can't be read at all.
pub fn collect(root: &Path) -> Option<FolderFacts> {
    let mut facts = FolderFacts {
        root: root.to_path_buf(),
        ..Default::default()
    };
    if walk(root, 0, &mut facts).is_err() && facts.file_count == 0 && facts.top_dirs.is_empty() {
        return None;
    }
    Some(facts)
}

fn walk(dir: &Path, depth: usize, facts: &mut FolderFacts) -> std::io::Result<()> {
    let entries = fs::read_dir(dir)?;
    let in_unreal_dir = dir
        .to_string_lossy()
        .to_lowercase()
        .contains("unrealengine");
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let name_lower = entry.file_name().to_string_lossy().to_lowercase();
        if ft.is_dir() {
            if depth == 0 {
                facts.top_dirs.push(name_lower);
            }
            subdirs.push(entry.path());
        } else {
            facts.file_count += 1;
            if facts.file_count > MAX_FILES_TO_CHECK {
                return Ok(());
            }
            if depth == 0 {
                facts.top_files.push(name_lower.clone());
            } else if depth == 1 {
                facts.depth1_files.push(name_lower.clone());
            }
            if name_lower.ends_with(".exe") {
                facts.found_exe = true;
            }
            if GAME_CONTENT_SUFFIXES
                .iter()
                .any(|s| name_lower.ends_with(s))
                || in_unreal_dir
            {
                facts.found_game_content = true;
            }
        }
    }
    // Python's walker visits directories while relpath.count(sep) < max_scan_depth,
    // i.e. dirs up to MAX_SCAN_DEPTH levels below root (files one level deeper
    // still get read). depth here is 1-based for children of root.
    if depth < MAX_SCAN_DEPTH {
        for sub in subdirs {
            if facts.file_count > MAX_FILES_TO_CHECK {
                break;
            }
            // Ignore unreadable subdirectories, matching the Python walker
            let _ = walk(&sub, depth + 1, facts);
        }
    }
    Ok(())
}

/// Game-folder heuristic: exe or game content present, and more than 5 files.
/// The folder's own name must not match the exclusion list.
pub fn is_game_folder(root: &Path, facts: &FolderFacts) -> bool {
    let folder_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if EXCLUDE_FOLDERS.iter().any(|ex| folder_name.contains(ex)) {
        return false;
    }
    (facts.found_exe || facts.found_game_content) && facts.file_count > 5
}

/// Engine detection. Port of `_detect_engine_type`.
pub fn detect_engine(root: &Path, facts: &FolderFacts) -> Engine {
    if root.join("Engine").join("Binaries").join("Win64").is_dir() {
        return Engine::Unreal;
    }
    if facts.top_files.iter().any(|f| f == "unityplayer.dll")
        || facts.top_dirs.iter().any(|d| d == "assets")
    {
        return Engine::Unity;
    }
    if facts.top_files.iter().any(|f| f.ends_with(".godot"))
        || facts.top_dirs.iter().any(|d| d == ".import")
    {
        return Engine::Godot;
    }
    for name in &facts.top_files {
        if name.ends_with(".exe") {
            let squeezed: String = name.chars().filter(|c| *c != ' ').collect();
            if name.contains("prism")
                || (name.contains("euro") && name.contains("truck"))
                || name.contains("ats")
                || squeezed.contains("eurotruck")
            {
                return Engine::Prism3D;
            }
        } else if name.ends_with(".dll")
            && (name.contains("prism") || name.contains("prism3d") || name.contains("scs"))
        {
            return Engine::Prism3D;
        }
    }
    const PRISM_CONFIGS: &[&str] = &[
        "prismengine.ini",
        "engine.ini",
        "scs_game.ini",
        "scs_game.txt",
    ];
    if facts
        .top_files
        .iter()
        .any(|f| PRISM_CONFIGS.contains(&f.as_str()))
    {
        return Engine::Prism3D;
    }
    Engine::Unknown
}

/// Engines OptiScaler installs are known to work with.
/// Port of `compatibility_checker.supported_engines`.
pub fn is_engine_supported(engine: Engine) -> bool {
    matches!(engine, Engine::Unreal | Engine::Unity | Engine::Godot)
}

/// Anti-cheat detection over top-level names + direct-child file names.
/// Port of `_detect_anti_cheat` (post-v0.5.2 semantics).
pub fn detect_anti_cheat(facts: &FolderFacts) -> Vec<AntiCheat> {
    let indicators: &[(AntiCheat, &[&str])] = &[
        (
            AntiCheat::EasyAntiCheat,
            &["easyanticheat.sys", "easyanticheat.exe", "easyanticheat"],
        ),
        (
            AntiCheat::BattlEye,
            &["beclient.dll", "beservice.exe", "battleye"],
        ),
        (AntiCheat::Vanguard, &["vgc.sys", "vgtray.exe", "vanguard"]),
    ];
    let candidates = facts
        .top_files
        .iter()
        .chain(facts.top_dirs.iter())
        .chain(facts.depth1_files.iter());
    let mut found = Vec::new();
    for name in candidates {
        for (ac, patterns) in indicators {
            if !found.contains(ac) && patterns.iter().any(|p| name.contains(p)) {
                found.push(*ac);
            }
        }
    }
    found
}

/// Detect an existing OptiScaler install. Port of `_detect_optiscaler`.
pub fn detect_optiscaler(root: &Path, facts: &FolderFacts) -> bool {
    if facts
        .top_files
        .iter()
        .any(|f| OPTISCALER_FILES.contains(&f.as_str()))
    {
        return true;
    }
    let subdirs_present: Vec<&&str> = OPTISCALER_SUBDIRS
        .iter()
        .filter(|s| facts.top_dirs.iter().any(|d| d == &s.to_lowercase()))
        .collect();
    for subdir in subdirs_present {
        let sub = root.join(subdir);
        if OPTISCALER_FILES.iter().any(|f| sub.join(f).exists()) {
            return true;
        }
    }
    let ue_dir = root.join("Engine").join("Binaries").join("Win64");
    if ue_dir.is_dir() {
        if OPTISCALER_FILES.iter().any(|f| ue_dir.join(f).exists()) {
            return true;
        }
        for subdir in OPTISCALER_SUBDIRS {
            let sub = ue_dir.join(subdir);
            if sub.is_dir() && OPTISCALER_FILES.iter().any(|f| sub.join(f).exists()) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn make_unity_game(base: &Path, name: &str) -> PathBuf {
        let d = base.join(name);
        fs::create_dir_all(d.join("Data")).unwrap();
        File::create(d.join(format!("{name}.exe"))).unwrap();
        File::create(d.join("UnityPlayer.dll")).unwrap();
        for i in 0..6 {
            File::create(d.join("Data").join(format!("data{i}.bin"))).unwrap();
        }
        fs::create_dir_all(d.join("EasyAntiCheat")).unwrap();
        File::create(d.join("EasyAntiCheat").join("EasyAntiCheat.exe")).unwrap();
        d
    }

    #[test]
    fn facts_feed_all_detectors() {
        let tmp = tempfile::tempdir().unwrap();
        let game = make_unity_game(tmp.path(), "FactsGame");
        let facts = collect(&game).unwrap();
        assert!(facts.found_exe);
        assert!(facts.found_game_content);
        assert!(facts.file_count > 5);
        assert!(is_game_folder(&game, &facts));
        assert_eq!(detect_engine(&game, &facts), Engine::Unity);
        assert!(detect_anti_cheat(&facts).contains(&AntiCheat::EasyAntiCheat));
        assert!(!detect_optiscaler(&game, &facts));

        File::create(game.join("dxgi.dll")).unwrap();
        let facts2 = collect(&game).unwrap();
        assert!(detect_optiscaler(&game, &facts2));
    }

    #[test]
    fn excluded_folder_names_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let d = make_unity_game(tmp.path(), "Redist");
        let facts = collect(&d).unwrap();
        assert!(!is_game_folder(&d, &facts));
    }

    #[test]
    fn unreal_detection_via_engine_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path().join("UeGame");
        fs::create_dir_all(d.join("Engine").join("Binaries").join("Win64")).unwrap();
        File::create(d.join("UeGame.exe")).unwrap();
        let facts = collect(&d).unwrap();
        assert_eq!(detect_engine(&d, &facts), Engine::Unreal);
        assert!(is_engine_supported(Engine::Unreal));
        assert!(!is_engine_supported(Engine::Unknown));
    }

    #[test]
    fn small_folders_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path().join("Tiny");
        fs::create_dir_all(&d).unwrap();
        File::create(d.join("app.exe")).unwrap();
        let facts = collect(&d).unwrap();
        assert!(!is_game_folder(&d, &facts)); // ≤5 files
    }
}
