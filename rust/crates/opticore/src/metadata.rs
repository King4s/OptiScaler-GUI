//! Game-page metadata: description, developers, genres, release date —
//! fetched from public store endpoints (Steam appdetails, GOG products,
//! no auth) and cached in `cache/metadata/` with a TTL so the store APIs
//! are hit at most once a week per game.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const TTL_SECS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameMetadata {
    pub description: String,
    #[serde(default)]
    pub developers: Vec<String>,
    #[serde(default)]
    pub publishers: Vec<String>,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub store_url: String,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    fetched_at: u64,
    meta: GameMetadata,
}

/// Everything the metadata pipeline can use for one game.
pub struct MetaRequest {
    pub name: String,
    pub appid: Option<u32>,
    pub platform_is_gog: bool,
}

pub struct MetadataCache {
    cache_dir: PathBuf,
}

impl MetadataCache {
    pub fn new(base_dir: &Path) -> Self {
        let cache_dir = base_dir.join("cache").join("metadata");
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    fn cache_file(&self, request: &MetaRequest) -> PathBuf {
        let stem = match request.appid {
            Some(id) => format!("appid_{id}"),
            None => crate::images::ImageCache::safe_name(&request.name),
        };
        self.cache_dir.join(format!("{stem}.json"))
    }

    /// Cached or freshly fetched metadata; None when no store knows the game.
    pub fn fetch(&self, request: &MetaRequest) -> Option<GameMetadata> {
        let file = self.cache_file(request);
        if let Some(entry) = read_cache(&file) {
            if now_unix().saturating_sub(entry.fetched_at) < TTL_SECS {
                return Some(entry.meta);
            }
        }
        let meta = fetch_fresh(request)?;
        let entry = CacheEntry {
            fetched_at: now_unix(),
            meta: meta.clone(),
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = std::fs::write(&file, json);
        }
        Some(meta)
    }
}

fn read_cache(path: &Path) -> Option<CacheEntry> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fetch_fresh(request: &MetaRequest) -> Option<GameMetadata> {
    if let Some(appid) = request.appid {
        if let Some(meta) = steam_appdetails(appid) {
            return Some(meta);
        }
    }
    if request.platform_is_gog || request.appid.is_none() {
        return gog_product(&request.name);
    }
    None
}

/// Steam's public appdetails endpoint (same one the artwork pipeline uses).
fn steam_appdetails(appid: u32) -> Option<GameMetadata> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&l=english");
    let body = crate::images::http_get_public(&url)?;
    let root: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let data = root.get(appid.to_string())?.get("data")?;
    parse_steam_data(data, appid)
}

fn parse_steam_data(data: &serde_json::Value, appid: u32) -> Option<GameMetadata> {
    let str_list = |key: &str| -> Vec<String> {
        data.get(key)
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(GameMetadata {
        description: strip_html(
            data.get("short_description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        ),
        developers: str_list("developers"),
        publishers: str_list("publishers"),
        release_date: data
            .get("release_date")
            .and_then(|r| r.get("date"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        genres: data
            .get("genres")
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|g| g.get("description").and_then(serde_json::Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        store_url: format!("https://store.steampowered.com/app/{appid}"),
    })
}

/// GOG: catalogue search (name-verified, same rule as the artwork chain)
/// → products endpoint with the description expanded.
fn gog_product(name: &str) -> Option<GameMetadata> {
    let id = crate::images::gog_search_id(name)?;
    let url = format!("https://api.gog.com/products/{id}?expand=description");
    let body = crate::images::http_get_public(&url)?;
    let data: serde_json::Value = serde_json::from_slice(&body).ok()?;
    parse_gog_product(&data)
}

fn parse_gog_product(data: &serde_json::Value) -> Option<GameMetadata> {
    let description = data
        .get("description")
        .and_then(|d| d.get("lead").or_else(|| d.get("full")))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    Some(GameMetadata {
        description: strip_html(description),
        developers: Vec::new(),
        publishers: Vec::new(),
        release_date: data
            .get("release_date")
            .and_then(serde_json::Value::as_str)
            .map(|d| d.chars().take(10).collect())
            .unwrap_or_default(),
        genres: Vec::new(),
        store_url: data
            .get("links")
            .and_then(|l| l.get("product_card"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

/// Minimal HTML → plain text: tags out, common entities decoded, whitespace
/// collapsed. Store descriptions only need this much.
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    // The tag→space replacement leaves " ." after inline closers like </b>.
    let mut result = collapsed;
    for punct in [" .", " ,", " !", " ?", " ;", " :"] {
        result = result.replace(punct, &punct[1..]);
    }
    result
}

/// Total size on disk of a game folder (one background walk per session).
pub fn dir_size_bytes(dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// "24.6 GB"-style size label.
pub fn format_size(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_and_entities() {
        assert_eq!(
            strip_html("<b>Bold</b> &amp; <i>fancy</i>&nbsp;text"),
            "Bold & fancy text"
        );
        assert_eq!(strip_html("plain"), "plain");
    }

    #[test]
    fn parses_steam_appdetails_shape() {
        let data: serde_json::Value = serde_json::from_str(
            r#"{
                "short_description": "A great <b>game</b>.",
                "developers": ["Dev A"],
                "publishers": ["Pub B"],
                "release_date": {"coming_soon": false, "date": "12 Jul, 2026"},
                "genres": [{"id": "1", "description": "Action"}, {"id": "23", "description": "Indie"}]
            }"#,
        )
        .unwrap();
        let meta = parse_steam_data(&data, 123).unwrap();
        assert_eq!(meta.description, "A great game.");
        assert_eq!(meta.developers, vec!["Dev A"]);
        assert_eq!(meta.publishers, vec!["Pub B"]);
        assert_eq!(meta.release_date, "12 Jul, 2026");
        assert_eq!(meta.genres, vec!["Action", "Indie"]);
        assert_eq!(meta.store_url, "https://store.steampowered.com/app/123");
    }

    #[test]
    fn parses_gog_product_shape() {
        let data: serde_json::Value = serde_json::from_str(
            r#"{
                "title": "Some Game",
                "release_date": "2020-05-28T00:00:00+0300",
                "links": {"product_card": "https://www.gog.com/game/some_game"},
                "description": {"lead": "Lead <i>text</i> here."}
            }"#,
        )
        .unwrap();
        let meta = parse_gog_product(&data).unwrap();
        assert_eq!(meta.description, "Lead text here.");
        assert_eq!(meta.release_date, "2020-05-28");
        assert_eq!(meta.store_url, "https://www.gog.com/game/some_game");
    }

    #[test]
    fn cache_roundtrip_with_ttl() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = MetadataCache::new(tmp.path());
        let request = MetaRequest {
            name: "X".into(),
            appid: Some(7),
            platform_is_gog: false,
        };
        // Seed the cache file directly (no network in tests)
        let entry = CacheEntry {
            fetched_at: now_unix(),
            meta: GameMetadata {
                description: "cached".into(),
                ..Default::default()
            },
        };
        std::fs::write(
            cache.cache_file(&request),
            serde_json::to_string(&entry).unwrap(),
        )
        .unwrap();
        assert_eq!(cache.fetch(&request).unwrap().description, "cached");
    }

    #[test]
    fn size_formatting() {
        assert_eq!(format_size(500), "0 KB");
        assert_eq!(format_size(3 * 1024 * 1024), "3 MB");
        assert_eq!(format_size(26_522_222_222), "24.7 GB");
    }

    #[test]
    fn dir_size_sums_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.bin"), vec![0u8; 1000]).unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub").join("b.bin"), vec![0u8; 500]).unwrap();
        assert_eq!(dir_size_bytes(tmp.path()), 1500);
    }
}
