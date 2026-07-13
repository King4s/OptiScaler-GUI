//! Epic Games Store integration: authorization-code login, owned-games
//! listing, and launch/install handoff through the `com.epicgames.launcher`
//! protocol. Protocol implemented from its public behavior; no code from
//! other launchers.
//!
//! Downloading game content directly (Epic's chunked manifest CDN format)
//! is deliberately not implemented yet — installs are handed to the Epic
//! launcher protocol, which every Epic install already has.

use serde::{Deserialize, Serialize};

// Epic's own launcher client credentials — public constants required by
// the OAuth endpoint (the same pair every open Epic client presents).
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";
const TOKEN_URL: &str =
    "https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token";
const ASSETS_URL: &str =
    "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows?label=Live";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix time the access token expires.
    pub expires_at: u64,
    pub account_id: String,
    #[serde(default)]
    pub display_name: String,
}

impl EpicTokens {
    pub fn expired(&self) -> bool {
        super::now_unix() + 60 >= self.expires_at
    }
}

/// One owned game (base game assets only, no DLC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicOwned {
    pub app_name: String,
    pub title: String,
    pub namespace: String,
    pub catalog_item_id: String,
    pub image_url: String,
}

/// Browser URL producing the authorization code the user pastes back.
/// The redirect target returns a small JSON page containing the code.
pub fn login_url() -> String {
    format!(
        "https://www.epicgames.com/id/login?redirectUrl=\
         https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D{CLIENT_ID}%26responseType%3Dcode"
    )
}

/// Users paste the whole JSON, the whole URL, or the bare code — accept all.
pub fn extract_code(input: &str) -> String {
    let trimmed = input.trim();
    if let Some((_, rest)) = trimmed.split_once("authorizationCode\"") {
        // {"redirectUrl":"…","authorizationCode":"abc","sid":null}
        if let Some(start) = rest.find('"') {
            let rest = &rest[start + 1..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    if let Some((_, rest)) = trimmed.split_once("code=") {
        return rest
            .split(['&', '#', ' '])
            .next()
            .unwrap_or_default()
            .to_string();
    }
    trimmed.to_string()
}

pub fn exchange_code(code: &str) -> Result<EpicTokens, String> {
    token_request(&[("grant_type", "authorization_code"), ("code", code.trim())])
}

pub fn refresh(refresh_token: &str) -> Result<EpicTokens, String> {
    token_request(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ])
}

pub fn ensure_fresh(tokens: &EpicTokens) -> Result<EpicTokens, String> {
    if tokens.expired() {
        refresh(&tokens.refresh_token)
    } else {
        Ok(tokens.clone())
    }
}

fn token_request(form: &[(&str, &str)]) -> Result<EpicTokens, String> {
    let basic = base64(format!("{CLIENT_ID}:{CLIENT_SECRET}").as_bytes());
    let body: String = form
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let mut resp = crate::images::http_agent()
        .post(TOKEN_URL)
        .header("Authorization", &format!("basic {basic}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(&body)
        .map_err(|e| e.to_string())?;
    if resp.status() != 200 {
        return Err(format!("Epic auth failed: HTTP {}", resp.status()));
    }
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    parse_token_response(&json, super::now_unix())
}

fn parse_token_response(json: &serde_json::Value, now: u64) -> Result<EpicTokens, String> {
    let field = |k: &str| {
        json.get(k)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("Epic token response missing {k}"))
    };
    Ok(EpicTokens {
        access_token: field("access_token")?,
        refresh_token: field("refresh_token")?,
        expires_at: now
            + json
                .get("expires_in")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(3600),
        account_id: field("account_id")?,
        display_name: json
            .get("displayName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

/// Owned games: the Windows assets list gives appName/namespace/catalog id;
/// titles and art come from the catalog bulk endpoint per namespace.
pub fn owned_games(access_token: &str) -> Result<Vec<EpicOwned>, String> {
    let assets = super::get_json_auth(ASSETS_URL, access_token)?;
    let mut games = parse_assets(&assets);
    // Fill titles from the catalog (one request per namespace batch)
    for game in &mut games {
        if let Some((title, image)) =
            catalog_title(access_token, &game.namespace, &game.catalog_item_id)
        {
            game.title = title;
            game.image_url = image;
        }
    }
    games.retain(|g| !g.title.is_empty());
    games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(games)
}

fn parse_assets(json: &serde_json::Value) -> Vec<EpicOwned> {
    json.as_array()
        .map(|assets| {
            assets
                .iter()
                .filter_map(|a| {
                    let app_name = a.get("appName")?.as_str()?.to_string();
                    // DLC and non-game assets share the list; namespace "ue"
                    // is Unreal marketplace content.
                    let namespace = a.get("namespace")?.as_str()?.to_string();
                    if namespace == "ue" {
                        return None;
                    }
                    Some(EpicOwned {
                        app_name,
                        title: String::new(),
                        namespace,
                        catalog_item_id: a.get("catalogItemId")?.as_str()?.to_string(),
                        image_url: String::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn catalog_title(
    access_token: &str,
    namespace: &str,
    catalog_item_id: &str,
) -> Option<(String, String)> {
    let url = format!(
        "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/{namespace}/bulk/items?id={catalog_item_id}&includeDLCDetails=false&includeMainGameDetails=false&country=US&locale=en-US"
    );
    let json = super::get_json_auth(&url, access_token).ok()?;
    parse_catalog_item(&json, catalog_item_id)
}

fn parse_catalog_item(json: &serde_json::Value, id: &str) -> Option<(String, String)> {
    let item = json.get(id)?;
    // Only actual games (categories contain "games") get a title
    let is_game = item
        .get("categories")
        .and_then(serde_json::Value::as_array)
        .map(|c| {
            c.iter()
                .any(|cat| cat.get("path").and_then(serde_json::Value::as_str) == Some("games"))
        })
        .unwrap_or(true);
    if !is_game {
        return None;
    }
    let title = item.get("title")?.as_str()?.to_string();
    let image = item
        .get("keyImages")
        .and_then(serde_json::Value::as_array)
        .and_then(|images| {
            for wanted in ["DieselGameBoxTall", "DieselGameBox", "Thumbnail"] {
                for img in images {
                    if img.get("type").and_then(serde_json::Value::as_str) == Some(wanted) {
                        return img
                            .get("url")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string);
                    }
                }
            }
            None
        })
        .unwrap_or_default();
    Some((title, image))
}

/// Launch an installed Epic game through the launcher protocol (handles
/// EOS auth, cloud saves, and DRM the way the store expects).
pub fn launch_url(app_name: &str) -> String {
    format!("com.epicgames.launcher://apps/{app_name}?action=launch&silent=true")
}

/// Hand an install off to the Epic launcher (until our own chunked
/// downloader lands).
pub fn install_url(app_name: &str) -> String {
    format!("com.epicgames.launcher://apps/{app_name}?action=install")
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

/// Standard base64 (encode only — enough for HTTP basic auth).
fn base64(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn code_extraction_accepts_json_url_or_bare() {
        assert_eq!(extract_code("rawcode"), "rawcode");
        assert_eq!(
            extract_code(r#"{"redirectUrl":"x","authorizationCode":"abc123","sid":null}"#),
            "abc123"
        );
        assert_eq!(extract_code("https://x/?code=zzz&y=1"), "zzz");
    }

    #[test]
    fn parses_token_response() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"access_token": "at", "refresh_token": "rt", "expires_in": 7200,
                "account_id": "acc1", "displayName": "King"}"#,
        )
        .unwrap();
        let tokens = parse_token_response(&json, 100).unwrap();
        assert_eq!(tokens.expires_at, 7300);
        assert_eq!(tokens.display_name, "King");
    }

    #[test]
    fn parses_assets_and_skips_unreal_content() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[
                {"appName": "Fortnite", "namespace": "fn", "catalogItemId": "c1", "buildVersion": "1"},
                {"appName": "UEPlugin", "namespace": "ue", "catalogItemId": "c2", "buildVersion": "1"}
            ]"#,
        )
        .unwrap();
        let games = parse_assets(&json);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "Fortnite");
    }

    #[test]
    fn parses_catalog_item_with_tall_art_preferred() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"c1": {"title": "Game One", "categories": [{"path": "games"}],
                "keyImages": [
                    {"type": "Thumbnail", "url": "https://t"},
                    {"type": "DieselGameBoxTall", "url": "https://tall"}
                ]}}"#,
        )
        .unwrap();
        let (title, image) = parse_catalog_item(&json, "c1").unwrap();
        assert_eq!(title, "Game One");
        assert_eq!(image, "https://tall");
    }

    #[test]
    fn launcher_urls() {
        assert_eq!(
            launch_url("Foo"),
            "com.epicgames.launcher://apps/Foo?action=launch&silent=true"
        );
        assert!(install_url("Foo").ends_with("action=install"));
    }
}
