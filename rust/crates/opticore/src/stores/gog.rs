//! GOG integration: OAuth login (the public "embed" client every GOG
//! client uses), owned-games listing, and DRM-free installs via GOG's
//! offline installers (downloaded with MD5 verification, run silently —
//! they are InnoSetup packages). Protocol implemented from its public
//! behavior; no code from other launchers.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// GOG's official client credentials — public constants baked into GOG
// Galaxy and required by the auth endpoint; they are not user secrets.
const CLIENT_ID: &str = "46899977096215655";
const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
const REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GogTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix time the access token expires.
    pub expires_at: u64,
    pub user_id: String,
}

impl GogTokens {
    pub fn expired(&self) -> bool {
        super::now_unix() + 60 >= self.expires_at
    }
}

/// One owned game from the account library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GogOwned {
    pub id: u64,
    pub title: String,
    pub image_url: String,
    pub works_on_windows: bool,
}

/// One downloadable offline-installer file.
#[derive(Debug, Clone)]
pub struct InstallerFile {
    pub file_name_hint: String,
    pub version: String,
    pub size_bytes: u64,
    /// API path resolved via [`resolve_downlink`] just before download.
    pub downlink: String,
}

/// Browser URL for the login page; the user pastes the code from the
/// success page's address bar back into the app.
pub fn login_url() -> String {
    format!(
        "https://auth.gog.com/auth?client_id={CLIENT_ID}&redirect_uri={}&response_type=code&layout=client2",
        urlencode(REDIRECT_URI)
    )
}

/// Users paste either the bare code or the whole success URL — accept both.
pub fn extract_code(input: &str) -> String {
    let trimmed = input.trim();
    match trimmed.split_once("code=") {
        Some((_, rest)) => rest
            .split(['&', '#', ' '])
            .next()
            .unwrap_or_default()
            .to_string(),
        None => trimmed.to_string(),
    }
}

pub fn exchange_code(code: &str) -> Result<GogTokens, String> {
    let url = format!(
        "https://auth.gog.com/token?client_id={CLIENT_ID}&client_secret={CLIENT_SECRET}\
         &grant_type=authorization_code&code={}&redirect_uri={}",
        urlencode(code.trim()),
        urlencode(REDIRECT_URI)
    );
    token_request(&url)
}

pub fn refresh(refresh_token: &str) -> Result<GogTokens, String> {
    let url = format!(
        "https://auth.gog.com/token?client_id={CLIENT_ID}&client_secret={CLIENT_SECRET}\
         &grant_type=refresh_token&refresh_token={}",
        urlencode(refresh_token)
    );
    token_request(&url)
}

/// Valid tokens, refreshing (and persisting through the callback) if needed.
pub fn ensure_fresh(tokens: &GogTokens) -> Result<GogTokens, String> {
    if tokens.expired() {
        refresh(&tokens.refresh_token)
    } else {
        Ok(tokens.clone())
    }
}

fn token_request(url: &str) -> Result<GogTokens, String> {
    let mut resp = crate::images::http_agent()
        .get(url)
        .call()
        .map_err(|e| e.to_string())?;
    if resp.status() != 200 {
        return Err(format!("GOG auth failed: HTTP {}", resp.status()));
    }
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parse_token_response(&json, super::now_unix())
}

fn parse_token_response(json: &serde_json::Value, now: u64) -> Result<GogTokens, String> {
    let field = |k: &str| {
        json.get(k)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("GOG token response missing {k}"))
    };
    Ok(GogTokens {
        access_token: field("access_token")?,
        refresh_token: field("refresh_token")?,
        expires_at: now
            + json
                .get("expires_in")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(3600),
        user_id: json
            .get("user_id")
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_default(),
    })
}

/// All owned games (paged endpoint, one page per request).
pub fn owned_games(access_token: &str) -> Result<Vec<GogOwned>, String> {
    let mut games = Vec::new();
    let mut page = 1u32;
    loop {
        let url =
            format!("https://embed.gog.com/account/getFilteredProducts?mediaType=1&page={page}");
        let json = super::get_json_auth(&url, access_token)?;
        let total_pages = json
            .get("totalPages")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        games.extend(parse_products_page(&json));
        if u64::from(page) >= total_pages {
            break;
        }
        page += 1;
    }
    games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(games)
}

fn parse_products_page(json: &serde_json::Value) -> Vec<GogOwned> {
    json.get("products")
        .and_then(serde_json::Value::as_array)
        .map(|products| {
            products
                .iter()
                .filter_map(|p| {
                    Some(GogOwned {
                        id: p.get("id").and_then(serde_json::Value::as_u64)?,
                        title: p.get("title")?.as_str()?.to_string(),
                        image_url: p
                            .get("image")
                            .and_then(serde_json::Value::as_str)
                            .map(|i| format!("https:{i}.jpg"))
                            .unwrap_or_default(),
                        works_on_windows: p
                            .get("worksOn")
                            .and_then(|w| w.get("Windows"))
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Windows offline-installer files for a product (English preferred).
pub fn windows_installers(
    access_token: &str,
    product_id: u64,
) -> Result<Vec<InstallerFile>, String> {
    let url = format!("https://api.gog.com/products/{product_id}?expand=downloads");
    let json = super::get_json_auth(&url, access_token)?;
    let files = parse_windows_installers(&json);
    if files.is_empty() {
        return Err("no Windows installer available for this game".into());
    }
    Ok(files)
}

fn parse_windows_installers(json: &serde_json::Value) -> Vec<InstallerFile> {
    let installers = json
        .pointer("/downloads/installers")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let windows: Vec<&serde_json::Value> = installers
        .iter()
        .filter(|i| i.get("os").and_then(serde_json::Value::as_str) == Some("windows"))
        .collect();
    // English preferred, else the first language offered
    let chosen: Vec<&serde_json::Value> = {
        let english: Vec<&serde_json::Value> = windows
            .iter()
            .copied()
            .filter(|i| i.get("language").and_then(serde_json::Value::as_str) == Some("en"))
            .collect();
        if english.is_empty() {
            windows.into_iter().take(1).collect()
        } else {
            english.into_iter().take(1).collect()
        }
    };
    chosen
        .iter()
        .flat_map(|installer| {
            let version = installer
                .get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            installer
                .get("files")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(move |f| {
                    Some(InstallerFile {
                        file_name_hint: f
                            .get("id")
                            .map(|v| {
                                v.as_str()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| v.to_string())
                            })
                            .unwrap_or_default(),
                        version: version.clone(),
                        size_bytes: f
                            .get("size")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                        downlink: f.get("downlink")?.as_str()?.to_string(),
                    })
                })
        })
        .collect()
}

/// A downlink resolves (with auth) to the real CDN URL + a checksum URL.
pub struct DownlinkTarget {
    pub download_url: String,
    pub checksum_url: String,
}

pub fn resolve_downlink(access_token: &str, downlink: &str) -> Result<DownlinkTarget, String> {
    let json = super::get_json_auth(downlink, access_token)?;
    Ok(DownlinkTarget {
        download_url: json
            .get("downlink")
            .and_then(serde_json::Value::as_str)
            .ok_or("downlink response missing download url")?
            .to_string(),
        checksum_url: json
            .get("checksum")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

/// GOG checksum documents are tiny XML files: `<file ... md5="..." .../>`.
pub fn fetch_md5(checksum_url: &str) -> Option<String> {
    if checksum_url.is_empty() {
        return None;
    }
    let body = crate::images::http_get_public(checksum_url)?;
    parse_md5_xml(&String::from_utf8_lossy(&body))
}

fn parse_md5_xml(xml: &str) -> Option<String> {
    let start = xml.find("md5=\"")? + 5;
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_lowercase())
}

/// Download one installer file (resumable, MD5-verified when GOG provides
/// a checksum). Returns the local file path.
pub fn download_installer_file(
    access_token: &str,
    file: &InstallerFile,
    dest_dir: &Path,
    progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let target = resolve_downlink(access_token, &file.downlink)?;
    // Real filename comes from the CDN URL path
    let name = target
        .download_url
        .split('?')
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("gog_{}.exe", file.file_name_hint));
    let dest = dest_dir.join(name);
    let md5 = super::download_with_md5(&target.download_url, &dest, progress)?;
    if let Some(expected) = fetch_md5(&target.checksum_url) {
        if md5 != expected {
            let _ = std::fs::remove_file(&dest);
            return Err(format!("checksum mismatch for {}", dest.display()));
        }
    }
    Ok(dest)
}

/// Run a downloaded offline installer silently into `target_dir`.
/// GOG offline installers are InnoSetup packages and honor its silent
/// flags; exit code 0 = success.
pub fn run_installer_silent(installer: &Path, target_dir: &Path) -> Result<(), String> {
    let status = std::process::Command::new(installer)
        .arg("/VERYSILENT")
        .arg("/SUPPRESSMSGBOXES")
        .arg("/NORESTART")
        .arg("/NOICONS")
        .arg(format!("/DIR={}", target_dir.display()))
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("installer exited with {status}"))
    }
}

fn urlencode(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_url_contains_client_and_redirect() {
        let url = login_url();
        assert!(url.contains(CLIENT_ID));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("embed.gog.com%2Fon_login_success"));
    }

    #[test]
    fn code_extraction_accepts_url_or_bare_code() {
        assert_eq!(extract_code("abc123"), "abc123");
        assert_eq!(
            extract_code("https://embed.gog.com/on_login_success?origin=client&code=xyz789#top"),
            "xyz789"
        );
    }

    #[test]
    fn parses_token_response() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"access_token": "at", "refresh_token": "rt", "expires_in": 3600, "user_id": "12345"}"#,
        )
        .unwrap();
        let tokens = parse_token_response(&json, 1000).unwrap();
        assert_eq!(tokens.access_token, "at");
        assert_eq!(tokens.expires_at, 4600);
        assert_eq!(tokens.user_id, "12345");
        assert!(tokens.expired()); // 1000+3600 is long past
    }

    #[test]
    fn parses_products_page() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"totalPages": 1, "products": [
                {"id": 1207658924, "title": "Some Game", "image": "//images.gog.com/x", "worksOn": {"Windows": true, "Mac": false, "Linux": false}},
                {"id": 2, "title": "No Image Game", "worksOn": {"Windows": false}}
            ]}"#,
        )
        .unwrap();
        let games = parse_products_page(&json);
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].id, 1207658924);
        assert_eq!(games[0].image_url, "https://images.gog.com/x.jpg");
        assert!(games[0].works_on_windows);
        assert!(!games[1].works_on_windows);
    }

    #[test]
    fn parses_windows_installers_english_preferred() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"downloads": {"installers": [
                {"os": "mac", "language": "en", "version": "1.0", "files": [{"id": "m", "size": 5, "downlink": "https://api.gog.com/x/mac"}]},
                {"os": "windows", "language": "de", "version": "1.0", "files": [{"id": "de1", "size": 10, "downlink": "https://api.gog.com/x/de"}]},
                {"os": "windows", "language": "en", "version": "1.0", "files": [
                    {"id": "en1", "size": 100, "downlink": "https://api.gog.com/x/en1"},
                    {"id": "en2", "size": 200, "downlink": "https://api.gog.com/x/en2"}
                ]}
            ]}}"#,
        )
        .unwrap();
        let files = parse_windows_installers(&json);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].downlink, "https://api.gog.com/x/en1");
        assert_eq!(files[1].size_bytes, 200);
    }

    #[test]
    fn parses_md5_from_checksum_xml() {
        let xml = r#"<file name="setup.exe" available="1" md5="ABCDEF0123456789" total_size="1"/>"#;
        assert_eq!(parse_md5_xml(xml).unwrap(), "abcdef0123456789");
        assert!(parse_md5_xml("<file/>").is_none());
    }
}
