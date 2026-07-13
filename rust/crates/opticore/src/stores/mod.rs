//! Store integrations (GOG, Epic): login with the user's own account, list
//! owned games, download and install what the account owns. Own protocol
//! implementations — no code from other launchers.
//!
//! Tokens live in `cache/store_auth.json` next to the other caches. They
//! grant access to the user's own store account only and are never logged.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod epic;
pub mod gog;

/// Persisted store credentials (absent = not connected).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreAuth {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gog: Option<gog::GogTokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<epic::EpicTokens>,
}

impl StoreAuth {
    pub fn auth_path(base_dir: &Path) -> PathBuf {
        base_dir.join("cache").join("store_auth.json")
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// GET with a Bearer token, JSON response.
pub(crate) fn get_json_auth(url: &str, access_token: &str) -> Result<serde_json::Value, String> {
    let mut resp = crate::images::http_agent()
        .get(url)
        .header("Authorization", &format!("Bearer {access_token}"))
        .call()
        .map_err(|e| e.to_string())?;
    if resp.status() != 200 {
        return Err(format!("HTTP {} from {url}", resp.status()));
    }
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// Streaming download to disk with resume, an MD5 running hash, and a
/// progress callback. Returns the hex MD5 of the complete file.
pub(crate) fn download_with_md5(
    url: &str,
    dest: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<String, String> {
    use md5::{Digest, Md5};
    use std::io::{Read, Write};

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Hash any partial file first so a resumed download still verifies.
    let mut hasher = Md5::new();
    let mut have: u64 = 0;
    if dest.exists() {
        let mut existing = std::fs::File::open(dest).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 512 * 1024];
        loop {
            let n = existing.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            have += n as u64;
        }
    }

    let agent = crate::images::http_agent();
    let request = if have > 0 {
        agent.get(url).header("Range", &format!("bytes={have}-"))
    } else {
        agent.get(url)
    };
    let mut resp = request.call().map_err(|e| e.to_string())?;
    match (resp.status().as_u16(), have) {
        (200, _) => {
            // Server ignored the range — start over
            hasher = Md5::new();
            have = 0;
        }
        (206, _) => {}
        (416, h) if h > 0 => {
            // Already fully downloaded
            return Ok(hex(&hasher.finalize()));
        }
        (status, _) => return Err(format!("HTTP {status} from download")),
    }
    let total = have
        + resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(have > 0)
        .write(true)
        .truncate(have == 0)
        .open(dest)
        .map_err(|e| e.to_string())?;
    let mut reader = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 512 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        have += n as u64;
        progress(have, total.max(have));
    }
    Ok(hex(&hasher.finalize()))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_roundtrip_and_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = StoreAuth::auth_path(tmp.path());
        assert!(StoreAuth::load(&path).gog.is_none());

        let auth = StoreAuth {
            gog: Some(gog::GogTokens {
                access_token: "a".into(),
                refresh_token: "r".into(),
                expires_at: 123,
                user_id: "42".into(),
            }),
            epic: None,
        };
        auth.save(&path).unwrap();
        let loaded = StoreAuth::load(&path);
        assert_eq!(loaded.gog.unwrap().user_id, "42");
        assert!(loaded.epic.is_none());
    }

    #[test]
    fn hex_encoding() {
        assert_eq!(hex(&[0xde, 0xad, 0x01]), "dead01");
    }
}
