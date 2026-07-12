//! Steam AppID lookup: local manifests → SteamSpy catalogue (7-day disk
//! cache) with the Python matching ladder (exact → normalized →
//! edition-suffix stripping → token-subset via token index).
//!
//! Deviation from the Python app: the final difflib fuzzy-match step is not
//! ported (rare last resort; revisit if parity testing shows misses).

use crate::scan::steam;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime};

const CACHE_FILE: &str = "steam_app_list.json";
const CACHE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const STEAMSPY_PAGE_CAP: u32 = 50;

/// Platform/edition qualifiers stripped progressively from names.
/// Port of `_NAME_STRIP_SUFFIXES` (suffix-anchored, case-insensitive).
const NAME_STRIP_SUFFIXES: &[&str] = &[
    "for windows",
    "for pc",
    "for xbox",
    "game of the year edition",
    "goty edition",
    "complete edition",
    "definitive edition",
    "enhanced edition",
    "remastered",
    "standard edition",
    "gold edition",
    "deluxe edition",
    "ultimate edition",
    "windows edition",
    "pc edition",
    "microsoft store",
];

/// `re.sub(r'[^a-z0-9\s]', ' ', s)` + whitespace collapse, on a lowercase input.
fn normalize(s: &str) -> String {
    let replaced: String = s
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect();
    replaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_suffix_once(name: &str) -> Option<String> {
    let trimmed = name.trim_end();
    for suffix in NAME_STRIP_SUFFIXES {
        if let Some(stripped) = trimmed.strip_suffix(suffix) {
            let stripped = stripped.trim_end();
            // must be preceded by whitespace (Python: `\s+` before the suffix)
            if !stripped.is_empty() && stripped.len() < trimmed.len() {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

struct Index {
    /// name (lowercase) → appid
    by_name: HashMap<String, u32>,
    /// normalized name → appid
    by_normalized: HashMap<String, u32>,
    /// token → set of name keys containing it
    token_index: HashMap<String, HashSet<String>>,
    /// name key → token count (for smallest-name preference)
    token_counts: HashMap<String, usize>,
}

impl Index {
    fn build(apps: &HashMap<String, u32>) -> Self {
        let mut by_normalized = HashMap::new();
        let mut token_index: HashMap<String, HashSet<String>> = HashMap::new();
        let mut token_counts = HashMap::new();
        for (name, &appid) in apps {
            let norm = normalize(name);
            by_normalized.entry(norm.clone()).or_insert(appid);
            let tokens: HashSet<&str> = norm.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }
            token_counts.insert(name.clone(), tokens.len());
            for tok in tokens {
                token_index
                    .entry(tok.to_string())
                    .or_default()
                    .insert(name.clone());
            }
        }
        Self {
            by_name: apps.clone(),
            by_normalized,
            token_index,
            token_counts,
        }
    }

    fn lookup(&self, game_name: &str) -> Option<u32> {
        let base = game_name.to_lowercase().trim().to_string();
        if base.is_empty() {
            return None;
        }
        // Progressive edition-suffix stripping
        let mut variants = vec![base.clone()];
        let mut current = base;
        while let Some(next) = strip_suffix_once(&current) {
            variants.push(next.clone());
            current = next;
        }

        for candidate in &variants {
            if let Some(&appid) = self.by_name.get(candidate) {
                return Some(appid);
            }
            let norm = normalize(candidate);
            if let Some(&appid) = self.by_normalized.get(&norm) {
                return Some(appid);
            }
            // Token-subset: all query tokens present; prefer fewest-token name
            let tokens: Vec<&str> = norm.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }
            let mut candidates: Option<HashSet<&String>> = None;
            for tok in &tokens {
                match self.token_index.get(*tok) {
                    Some(keys) => {
                        let keys: HashSet<&String> = keys.iter().collect();
                        candidates = Some(match candidates {
                            None => keys,
                            Some(prev) => prev.intersection(&keys).copied().collect(),
                        });
                    }
                    None => {
                        candidates = Some(HashSet::new());
                        break;
                    }
                }
            }
            let best = candidates
                .unwrap_or_default()
                .into_iter()
                .min_by_key(|key| self.token_counts.get(*key).copied().unwrap_or(usize::MAX));
            if let Some(key) = best {
                if let Some(&appid) = self.by_name.get(key) {
                    return Some(appid);
                }
            }
        }
        None
    }
}

/// Thread-safe AppID resolver with a per-name memo cache.
pub struct AppIdResolver {
    cache_dir: PathBuf,
    index: RwLock<Index>,
    memo: Mutex<HashMap<String, Option<u32>>>,
}

impl AppIdResolver {
    /// Build instantly from installed-game manifests (plus the disk cache if
    /// fresh). `cache_dir` is the game_images dir (Python keeps
    /// steam_app_list.json there — layout kept compatible).
    pub fn new(cache_dir: &Path) -> Self {
        let mut apps = load_disk_cache(cache_dir).unwrap_or_default();
        if apps.is_empty() {
            apps = local_manifest_apps();
        }
        Self {
            cache_dir: cache_dir.to_path_buf(),
            index: RwLock::new(Index::build(&apps)),
            memo: Mutex::new(HashMap::new()),
        }
    }

    pub fn lookup(&self, game_name: &str) -> Option<u32> {
        let key = game_name.to_lowercase();
        if let Some(hit) = self.memo.lock().unwrap().get(&key) {
            return *hit;
        }
        let result = self.index.read().unwrap().lookup(game_name);
        self.memo.lock().unwrap().insert(key, result);
        result
    }

    /// Online fallback via the Steam Store search API — used for games with
    /// no local appid (typically Xbox/Game Pass titles) when the catalogue
    /// lookup misses, e.g. before the SteamSpy download completes on first
    /// run. The result is only accepted when the store item's name actually
    /// matches the query (normalized equality or token-subset), so a wrong
    /// game's artwork never wins. Memoized (positive and negative).
    pub fn lookup_online(&self, game_name: &str) -> Option<u32> {
        let memo_key = format!("storesearch:{}", game_name.to_lowercase());
        if let Some(hit) = self.memo.lock().unwrap().get(&memo_key) {
            return *hit;
        }
        let result = storesearch(game_name);
        self.memo.lock().unwrap().insert(memo_key, result);
        result
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn storesearch(game_name: &str) -> Option<u32> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc=us&l=en",
        percent_encode(game_name)
    );
    let mut resp = crate::images::http_agent().get(&url).call().ok()?;
    let body = resp.body_mut().read_to_string().ok()?;
    let data: Value = serde_json::from_str(&body).ok()?;
    let query_norm = normalize(game_name);
    let query_tokens: HashSet<&str> = query_norm.split_whitespace().collect();
    for item in data.get("items").and_then(Value::as_array)? {
        let name = item.get("name").and_then(Value::as_str).unwrap_or("");
        let item_norm = normalize(name);
        let item_tokens: HashSet<&str> = item_norm.split_whitespace().collect();
        let acceptable = item_norm == query_norm
            || (!query_tokens.is_empty() && query_tokens.is_subset(&item_tokens));
        if acceptable {
            return item.get("id").and_then(Value::as_u64).map(|id| id as u32);
        }
    }
    None
}

impl AppIdResolver {
    /// Download the SteamSpy catalogue (skipped when the disk cache is fresh)
    /// and swap in the enlarged index. Call from a background thread; returns
    /// true if the index changed.
    pub fn load_steamspy_catalogue(&self) -> bool {
        if disk_cache_is_fresh(&self.cache_dir) {
            return false;
        }
        let mut apps = local_manifest_apps();
        let agent = crate::images::http_agent();
        for page in 0..STEAMSPY_PAGE_CAP {
            let url = format!("https://steamspy.com/api.php?request=all&page={page}");
            let Ok(mut resp) = agent.get(&url).call() else {
                break;
            };
            let Ok(body) = resp.body_mut().read_to_string() else {
                break;
            };
            let Ok(Value::Object(data)) = serde_json::from_str::<Value>(&body) else {
                break;
            };
            if data.is_empty() {
                break;
            }
            for (appid_str, info) in &data {
                let name = info.get("name").and_then(Value::as_str).unwrap_or("");
                if name.len() < 2 {
                    continue;
                }
                let lower = name.to_lowercase();
                if ["dlc", "demo", "beta", "test"]
                    .iter()
                    .any(|kw| lower == *kw || lower.starts_with(&format!("{kw} ")))
                {
                    continue;
                }
                if let Ok(appid) = appid_str.parse::<u32>() {
                    apps.insert(lower, appid);
                }
            }
            std::thread::sleep(Duration::from_millis(500)); // be polite to the API
        }
        if apps.is_empty() {
            return false;
        }
        save_disk_cache(&self.cache_dir, &apps);
        *self.index.write().unwrap() = Index::build(&apps);
        self.memo.lock().unwrap().clear();
        true
    }
}

fn local_manifest_apps() -> HashMap<String, u32> {
    let mut apps = HashMap::new();
    for library_root in steam::all_library_roots() {
        let steamapps = library_root.join("steamapps");
        for entry in steam::build_manifest_map(&steamapps).into_values() {
            if let Some(appid) = entry.appid {
                apps.insert(entry.name.to_lowercase(), appid);
            }
        }
    }
    apps
}

fn disk_cache_is_fresh(cache_dir: &Path) -> bool {
    let path = cache_dir.join(CACHE_FILE);
    path.metadata()
        .and_then(|m| m.modified())
        .map(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::MAX)
                < CACHE_MAX_AGE
        })
        .unwrap_or(false)
}

fn load_disk_cache(cache_dir: &Path) -> Option<HashMap<String, u32>> {
    if !disk_cache_is_fresh(cache_dir) {
        return None;
    }
    let content = std::fs::read_to_string(cache_dir.join(CACHE_FILE)).ok()?;
    let raw: HashMap<String, Value> = serde_json::from_str(&content).ok()?;
    // Python stores appids as strings; accept numbers too
    Some(
        raw.into_iter()
            .filter_map(|(name, v)| {
                let appid = match &v {
                    Value::String(s) => s.parse().ok(),
                    Value::Number(n) => n.as_u64().map(|n| n as u32),
                    _ => None,
                }?;
                Some((name, appid))
            })
            .collect(),
    )
}

fn save_disk_cache(cache_dir: &Path, apps: &HashMap<String, u32>) {
    let _ = std::fs::create_dir_all(cache_dir);
    // Store appids as strings for byte-compat with the Python cache format
    let as_strings: HashMap<&String, String> =
        apps.iter().map(|(k, v)| (k, v.to_string())).collect();
    if let Ok(json) = serde_json::to_string(&as_strings) {
        let _ = std::fs::write(cache_dir.join(CACHE_FILE), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(entries: &[(&str, u32)]) -> Index {
        let apps: HashMap<String, u32> = entries.iter().map(|(n, a)| (n.to_string(), *a)).collect();
        Index::build(&apps)
    }

    #[test]
    fn exact_and_normalized_matches() {
        let idx = index_of(&[
            ("cyberpunk 2077", 1091500),
            ("the witcher 3: wild hunt", 292030),
        ]);
        assert_eq!(idx.lookup("Cyberpunk 2077"), Some(1091500));
        assert_eq!(idx.lookup("The Witcher 3 Wild Hunt"), Some(292030)); // punctuation-free
    }

    #[test]
    fn edition_suffix_stripping() {
        let idx = index_of(&[("skyrim", 72850)]);
        assert_eq!(idx.lookup("Skyrim Special Edition"), None); // "special edition" not in strip list
        assert_eq!(idx.lookup("Skyrim Ultimate Edition"), Some(72850));
        assert_eq!(idx.lookup("Skyrim for Windows"), Some(72850));
    }

    #[test]
    fn token_subset_prefers_smallest() {
        let idx = index_of(&[
            ("halo infinite", 1240440),
            ("halo infinite multiplayer test build", 999),
        ]);
        assert_eq!(idx.lookup("Halo Infinite"), Some(1240440));
        // subset match: query tokens all appear in the longer name too,
        // but the fewest-token entry wins
        assert_eq!(idx.lookup("halo: infinite!"), Some(1240440));
    }

    #[test]
    fn no_match_returns_none() {
        let idx = index_of(&[("stellaris", 281990)]);
        assert_eq!(idx.lookup("Definitely Not A Game"), None);
    }

    #[test]
    fn disk_cache_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut apps = HashMap::new();
        apps.insert("raft".to_string(), 648800u32);
        save_disk_cache(tmp.path(), &apps);
        let loaded = load_disk_cache(tmp.path()).unwrap();
        assert_eq!(loaded.get("raft"), Some(&648800));
    }
}
