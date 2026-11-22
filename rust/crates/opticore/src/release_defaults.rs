//! Small offline INI snapshots, written only from a verified release payload.
use crate::{ini, install::github};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

const MAX_INI: u64 = 1024 * 1024;

#[derive(Deserialize, Serialize)]
struct Snapshot {
    tag: String,
    sha256: String,
    text: String,
}

pub fn valid_tag(tag: &str) -> bool {
    tag.len() <= 80
        && tag
            .strip_prefix('v')
            .unwrap_or(tag)
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_digit)
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
        && !tag.contains("..")
        && !tag.ends_with('.')
}

fn bounded_read(path: &Path, limit: u64) -> io::Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "INI exceeds size limit",
        ));
    }
    Ok(text)
}

fn checksum(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn remember(cache: &Path, tag: &str, payload: &Path) -> io::Result<()> {
    if !valid_tag(tag) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown release tag",
        ));
    }
    let dir = cache.join("ini-defaults");
    let text = bounded_read(&payload.join("OptiScaler.ini"), MAX_INI)?;
    let snapshot = Snapshot {
        tag: tag.into(),
        sha256: checksum(&text),
        text,
    };
    fs::create_dir_all(&dir)?;
    crate::install::reject_reparse(&dir).map_err(io::Error::other)?;
    let mut tmp = tempfile::NamedTempFile::new_in(&dir)?;
    serde_json::to_writer(&mut tmp, &snapshot)?;
    tmp.flush()?;
    tmp.persist(dir.join(format!("{tag}.json")))?;
    Ok(())
}

pub fn cached(cache: &Path, tag: &str) -> Option<ini::IniDocument> {
    if !valid_tag(tag) {
        return None;
    }
    let text = bounded_read(
        &cache.join("ini-defaults").join(format!("{tag}.json")),
        6 * MAX_INI + 512,
    )
    .ok()?;
    let snapshot: Snapshot = serde_json::from_str(&text).ok()?;
    (snapshot.tag == tag
        && snapshot.text.len() as u64 <= MAX_INI
        && snapshot.sha256 == checksum(&snapshot.text))
    .then(|| ini::parse(&snapshot.text))
}

pub fn fetch_exact(tag: &str) -> Result<github::ReleaseInfo, String> {
    exact_with(tag, |url| {
        github::fetch_release_from(url).map_err(|e| e.to_string())
    })
}

fn exact_with(
    tag: &str,
    fetch: impl FnOnce(&str) -> Result<github::ReleaseInfo, String>,
) -> Result<github::ReleaseInfo, String> {
    if !valid_tag(tag) {
        return Err("The installed release tag is unknown".into());
    }
    let release = fetch(&format!(
        "https://api.github.com/repos/optiscaler/OptiScaler/releases/tags/{tag}"
    ))?;
    if release.tag_name.as_deref() != Some(tag) {
        return Err(format!("Upstream did not return {tag}"));
    }
    Ok(release)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_fetches_latest_or_accepts_a_different_tag() {
        assert!(exact_with("Unknown", |_| panic!("network lookup for unknown tag")).is_err());
        assert!(exact_with("v0.9.4", |url| {
            assert!(url.ends_with("/tags/v0.9.4"));
            Ok(serde_json::from_str(r#"{"tag_name":"v0.9.5"}"#).unwrap())
        })
        .is_err());
    }
}
