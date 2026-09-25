//! Shared target resolution. Unknown or ambiguous layouts require user selection.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CHOICE: &str = ".optiscaler-gui-target.json";

#[derive(Debug, Clone)]
pub struct InstallTarget {
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
struct Choice {
    executable: PathBuf,
}

fn game_exe(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    name.ends_with(".exe")
        && ![
            "crash",
            "launcher",
            "setup",
            "unins",
            "install",
            "report",
            "helper",
            "benchmark",
            "ue4prereq",
            "easyanticheat",
            "beservice",
            "beclient",
            "battleye",
            "vcredist",
            "dxsetup",
            "optiscaler",
            "dlssnr",
        ]
        .iter()
        .any(|part| name.contains(part))
}

pub fn choose(root: &Path, exe: &Path) -> Result<InstallTarget, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let exe = exe.canonicalize().map_err(|e| e.to_string())?;
    if !exe.is_file() || !game_exe(&exe) || !exe.starts_with(&root) {
        return Err(
            "Select a game executable inside the game directory (not a launcher or tool)".into(),
        );
    }
    let directory = if exe
        .parent()
        .is_some_and(|parent| parent == root.join("FactoryGame/Binaries/Win64"))
        && exe.file_name().is_some_and(|name| {
            let name = name.to_string_lossy().to_ascii_lowercase();
            name.starts_with("factorygame") && name.ends_with("-win64-shipping.exe")
        }) {
        let engine = root
            .join("Engine/Binaries/Win64")
            .canonicalize()
            .map_err(|_| {
                "Satisfactory wiki target is missing; select a safe installation target"
            })?;
        if !engine.is_dir() || !engine.starts_with(&root) {
            return Err("Satisfactory wiki target is outside the game directory".into());
        }
        engine
    } else {
        exe.parent().unwrap().to_path_buf()
    };
    Ok(InstallTarget {
        directory,
        executable: exe,
        reason: "Explicit executable selection".into(),
    })
}

/// Called only after explicit selection in the GUI, never on discovery/startup.
pub fn remember(root: &Path, exe: &Path) -> Result<InstallTarget, String> {
    let target = choose(root, exe)?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let choice = Choice {
        executable: target
            .executable
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_path_buf(),
    };
    use std::io::Write;
    let temp = root.join(format!("{CHOICE}.{}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(&serde_json::to_vec_pretty(&choice).map_err(std::io::Error::other)?)?;
        file.sync_all()?;
        drop(file);
        // Replacing the sidecar rather than writing through it cannot follow a link.
        std::fs::rename(&temp, root.join(CHOICE))
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(e.to_string());
    }
    Ok(target)
}

pub fn candidates(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
        if depth > 5 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_file() && game_exe(&path) && choose(root, &path).is_ok() {
                found.push(path);
                continue;
            }
            if kind.is_dir() {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                if ![
                    "engine",
                    "plugins",
                    "redist",
                    "_commonredist",
                    "extras",
                    "saved",
                    "content",
                    ".git",
                ]
                .contains(&name.as_str())
                {
                    visit(root, &path, depth + 1, found);
                }
            }
        }
    }
    let mut result = Vec::new();
    visit(root, root, 0, &mut result);
    // Some Game Pass layouts put the entire game below Content.
    visit(root, &root.join("Content"), 0, &mut result);
    result.sort();
    result.dedup();
    result
}

pub fn resolve(root: &Path) -> Result<InstallTarget, String> {
    let choice = root.join(CHOICE);
    if choice.exists() {
        let saved: Choice =
            serde_json::from_slice(&std::fs::read(choice).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid target selection: {e}"))?;
        if saved.executable.is_absolute()
            || saved
                .executable
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("Unsafe saved executable path".into());
        }
        return choose(root, &root.join(saved.executable));
    }
    // Documented exception: the actual EXE and proxy target are different directories.
    // https://github.com/optiscaler/OptiScaler/wiki/Compatibility-List
    {
        let dir = root.join("FactoryGame/Binaries/Win64");
        if dir.is_dir() {
            let mut exes: Vec<_> = std::fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    game_exe(p)
                        && p.file_name().is_some_and(|name| {
                            let name = name.to_string_lossy().to_ascii_lowercase();
                            name.starts_with("factorygame") && name.ends_with("-win64-shipping.exe")
                        })
                })
                .collect();
            if exes.len() == 1 {
                let mut t = choose(root, &exes.remove(0))?;
                t.reason = "Satisfactory wiki exception (verify installed game version)".into();
                return Ok(t);
            }
            if exes.len() > 1 {
                return Err(
                    "Multiple Satisfactory game executables; choose the actual game EXE".into(),
                );
            }
            return Err("FactoryGame Shipping executable is missing; choose the game EXE".into());
        }
    }
    let mut exes = candidates(root);
    let shipping: Vec<_> = exes
        .iter()
        .filter(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with("-shipping.exe")
        })
        .cloned()
        .collect();
    if shipping.len() == 1 {
        exes = shipping;
    }
    if exes.len() != 1 {
        return Err(format!(
            "{} possible game executables; choose the actual game EXE before continuing",
            exes.len()
        ));
    }
    let mut target = choose(root, &exes.remove(0))?;
    target.reason = "Unique game executable; verify the preview before installing".into();
    Ok(target)
}
