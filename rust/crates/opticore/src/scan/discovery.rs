//! Native library-root discovery: fixed-drive probing for well-known game
//! library folders. Replaces the Python app's PowerShell discovery pipeline
//! (registry + drive enumeration run in milliseconds natively, so no cache).

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    Steam,
    Epic,
    Gog,
    Xbox,
}

#[derive(Debug, Clone)]
pub struct LibraryRoot {
    pub kind: RootKind,
    pub path: PathBuf,
}

/// Well-known library folder names probed on every fixed drive.
const DRIVE_PATTERNS: &[(RootKind, &str)] = &[
    (RootKind::Steam, "SteamLibrary"),
    (RootKind::Steam, r"Games\SteamLibrary"),
    (RootKind::Xbox, "XboxGames"),
    (RootKind::Epic, "Epic Games"),
    (RootKind::Gog, "GOG Games"),
    (RootKind::Gog, r"GOG Galaxy\Games"),
];

/// Drive letters present on the system (fixed or otherwise reachable).
fn drive_roots() -> Vec<PathBuf> {
    ('A'..='Z')
        .map(|letter| PathBuf::from(format!("{letter}:\\")))
        .filter(|p| p.is_dir())
        .collect()
}

/// Probe all drives for well-known game library folders, skipping excluded
/// drive letters (uppercase, no colon — mirrors the Python config format).
pub fn discover_roots(excluded_drives: &[char]) -> Vec<LibraryRoot> {
    let mut found = Vec::new();
    for drive in drive_roots() {
        let letter = drive
            .to_string_lossy()
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase();
        if excluded_drives.contains(&letter) {
            continue;
        }
        for (kind, pattern) in DRIVE_PATTERNS {
            let candidate = drive.join(pattern);
            if candidate.is_dir() {
                found.push(LibraryRoot {
                    kind: *kind,
                    path: candidate,
                });
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_drives_are_skipped() {
        // Excluding every letter must yield nothing, regardless of machine state.
        let all: Vec<char> = ('A'..='Z').collect();
        assert!(discover_roots(&all).is_empty());
    }
}
