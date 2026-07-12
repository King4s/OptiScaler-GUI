//! Steam library scanning: appmanifest ACF parsing, libraryfolders.vdf,
//! and the once-per-library common/ folder walk.
//!
//! ACF/VDF values are extracted with a small line-based parser (the Python
//! app does the same with regexes) — it tolerates both the classic VDF
//! format and the newer JSON-styled libraryfolders.vdf.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Extract the first quoted value following a quoted key, e.g. `"name" "X"`.
/// Works line-by-line; tolerates `"key": "value"` (JSON-ish) too.
fn quoted_field(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    for line in content.lines() {
        if let Some(pos) = line.find(&needle) {
            let rest = &line[pos + needle.len()..];
            if let Some(value) = next_quoted(rest) {
                return Some(value);
            }
        }
    }
    None
}

/// All quoted values following any occurrence of the quoted key.
fn quoted_fields(content: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut out = Vec::new();
    for line in content.lines() {
        if let Some(pos) = line.find(&needle) {
            if let Some(value) = next_quoted(&line[pos + needle.len()..]) {
                out.push(value);
            }
        }
    }
    out
}

/// First quoted string in `s`, with VDF backslash-escapes collapsed.
fn next_quoted(s: &str) -> Option<String> {
    let start = s.find('"')? + 1;
    let rest = &s[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            }
            _ => out.push(c),
        }
    }
    None
}

/// One parsed appmanifest entry.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub name: String,
    pub appid: Option<u32>,
}

/// Parse every `appmanifest_*.acf` once: installdir (lowercased) → entry.
/// Port of `_build_steam_manifest_map`.
pub fn build_manifest_map(steamapps: &Path) -> HashMap<String, ManifestEntry> {
    let mut map = HashMap::new();
    let Ok(entries) = fs::read_dir(steamapps) else {
        return map;
    };
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
            continue;
        }
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let (Some(installdir), Some(name)) = (
            quoted_field(&content, "installdir"),
            quoted_field(&content, "name"),
        ) else {
            continue;
        };
        let appid = quoted_field(&content, "appid").and_then(|a| a.parse().ok());
        map.insert(installdir.to_lowercase(), ManifestEntry { name, appid });
    }
    map
}

/// Steam library roots listed in a `libraryfolders.vdf` (both formats).
pub fn library_roots_from_vdf(vdf_path: &Path) -> Vec<PathBuf> {
    let Ok(content) = fs::read_to_string(vdf_path) else {
        return Vec::new();
    };
    quoted_fields(&content, "path")
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

/// Default + registry Steam install roots. Port of `_find_steam_paths`.
pub fn steam_install_roots() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        for (hive, key) in [
            (
                windows_registry::LOCAL_MACHINE,
                r"SOFTWARE\Wow6432Node\Valve\Steam",
            ),
            (windows_registry::LOCAL_MACHINE, r"SOFTWARE\Valve\Steam"),
            (windows_registry::CURRENT_USER, r"SOFTWARE\Valve\Steam"),
        ] {
            if let Ok(k) = hive.open(key) {
                for value_name in ["InstallPath", "SteamPath"] {
                    if let Ok(path) = k.get_string(value_name) {
                        candidates.push(PathBuf::from(path));
                    }
                }
            }
        }
    }
    candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    candidates.push(PathBuf::from(r"C:\Program Files\Steam"));
    if let Some(home) = std::env::var_os("USERPROFILE") {
        candidates.push(PathBuf::from(home).join(r"AppData\Local\Steam"));
    }

    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|p| p.is_dir())
        .filter(|p| seen.insert(p.to_string_lossy().to_lowercase()))
        .collect()
}

/// All Steam library roots: install roots plus everything their
/// libraryfolders.vdf files reference.
pub fn all_library_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for install_root in steam_install_roots() {
        if seen.insert(install_root.to_string_lossy().to_lowercase()) {
            roots.push(install_root.clone());
        }
        let vdf = install_root.join("steamapps").join("libraryfolders.vdf");
        if vdf.exists() {
            for lib in library_roots_from_vdf(&vdf) {
                if seen.insert(lib.to_string_lossy().to_lowercase()) {
                    roots.push(lib);
                }
            }
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    const ACF: &str = r#""AppState"
{
	"appid"		"1091500"
	"name"		"Cyberpunk 2077"
	"installdir"		"Cyberpunk 2077"
}
"#;

    #[test]
    fn parses_acf_fields() {
        assert_eq!(quoted_field(ACF, "name").as_deref(), Some("Cyberpunk 2077"));
        assert_eq!(quoted_field(ACF, "appid").as_deref(), Some("1091500"));
        assert_eq!(
            quoted_field(ACF, "installdir").as_deref(),
            Some("Cyberpunk 2077")
        );
    }

    #[test]
    fn builds_manifest_map() {
        let tmp = tempfile::tempdir().unwrap();
        let steamapps = tmp.path().join("steamapps");
        std::fs::create_dir_all(&steamapps).unwrap();
        let mut f = File::create(steamapps.join("appmanifest_1091500.acf")).unwrap();
        f.write_all(ACF.as_bytes()).unwrap();

        let map = build_manifest_map(&steamapps);
        let entry = map.get("cyberpunk 2077").expect("entry present");
        assert_eq!(entry.name, "Cyberpunk 2077");
        assert_eq!(entry.appid, Some(1091500));
    }

    #[test]
    fn parses_libraryfolders_both_formats() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("OtherLib");
        std::fs::create_dir_all(&lib).unwrap();
        let escaped = lib.to_string_lossy().replace('\\', "\\\\");

        let classic = format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{escaped}\"\n\t}}\n}}\n"
        );
        let vdf_path = tmp.path().join("libraryfolders.vdf");
        std::fs::write(&vdf_path, classic).unwrap();
        assert_eq!(library_roots_from_vdf(&vdf_path), vec![lib.clone()]);

        let json_style = format!("{{\"libraryfolders\": {{\"0\": {{\"path\": \"{escaped}\"}}}}}}");
        std::fs::write(&vdf_path, json_style).unwrap();
        assert_eq!(library_roots_from_vdf(&vdf_path), vec![lib]);
    }
}
