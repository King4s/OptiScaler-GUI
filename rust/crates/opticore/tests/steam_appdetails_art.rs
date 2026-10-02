//! Fixture tests for the appdetails rung of the Steam artwork source (`opticore::images`).
//!
//! The two response bodies are the live `appdetails?filters=basic` answers captured on
//! 2026-09-27 for app ids 620 and 100 — see `tasks/spec-steam-appdetails-art.md` for the
//! capture commands and the readings. The image bytes are synthetic. Every HTTP call goes
//! through an injected fetcher, so no test reads the network, the registry or this
//! machine's Steam installs.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use opticore::images::{
    appdetails_image_url, appdetails_image_urls, appdetails_url, ArtRequest, Fetcher, ImageCache,
};

/// The live response for `appids=620`, verbatim: keyed `323180`, payload Portal 2.
const APP_DETAILS_620: &str = include_str!("fixtures/appdetails-620.json");

/// The live response for `appids=100`, verbatim: keyed `100`, payload app 80.
const APP_DETAILS_100: &str = include_str!("fixtures/appdetails-100.json");

const APP_ID: u32 = 620;

/// The `header_image` the live 620 response carries, and the `capsule_image` beside it.
const HEADER_620: &str = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/header.jpg?t=1790187113";
const CAPSULE_620: &str = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/bd23943dcd0280aa2cdc066e2f86d2185cfd4cce/capsule_231x87.jpg?t=1790187113";

/// A real 6x9 magenta PNG: a valid download to cache.
const PORTRAIT_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x09, 0x08, 0x02, 0x00, 0x00, 0x00, 0x9e, 0xf8, 0xca,
    0xca, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xf0, 0x1f,
    0x0d, 0x31, 0x0c, 0x52, 0x21, 0x00, 0xf0, 0xa6, 0x6b, 0x95, 0x1a, 0xa4, 0x0b, 0xb6, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// A synthetic response whose entry is always keyed `999`, so every case below also
/// exercises the fact that the key is not the app id.
fn body_for(
    steam_appid: &str,
    success: &str,
    header: Option<&str>,
    capsule: Option<&str>,
) -> String {
    let mut data = String::from("{\"name\":\"Synthetic\"");
    data.push_str(&format!(",\"steam_appid\":{steam_appid}"));
    if let Some(url) = header {
        data.push_str(&format!(",\"header_image\":\"{url}\""));
    }
    if let Some(url) = capsule {
        data.push_str(&format!(",\"capsule_image\":\"{url}\""));
    }
    data.push('}');
    format!("{{\"999\":{{\"success\":{success},\"data\":{data}}}}}")
}

#[test]
fn the_captured_shapes_are_the_defect_this_change_fixes() {
    // If either reading stops holding, the rules below are built on sand and the
    // fixtures must be re-captured rather than adjusted.
    assert!(
        APP_DETAILS_620.starts_with(r#"{"323180":"#),
        "the 620 response must answer under another id"
    );
    assert!(APP_DETAILS_620.contains(r#""steam_appid":620"#));
    assert!(
        APP_DETAILS_100.starts_with(r#"{"100":"#),
        "the 100 response must answer under its own key"
    );
    assert!(
        APP_DETAILS_100.contains(r#""steam_appid":80"#),
        "and must carry app 80's payload, which is why the key cannot be trusted"
    );
}

#[test]
fn the_live_answer_keyed_by_another_id_is_used() {
    assert_eq!(
        appdetails_image_url(APP_DETAILS_620.as_bytes(), APP_ID),
        Some(HEADER_620.to_string()),
        "the payload of the single returned entry is the app we asked about"
    );
}

#[test]
fn the_live_answer_carrying_another_apps_artwork_is_refused() {
    assert_eq!(
        appdetails_image_url(APP_DETAILS_100.as_bytes(), 100),
        None,
        "app 80's artwork must not be handed to app 100"
    );
    assert_eq!(
        appdetails_image_url(APP_DETAILS_100.as_bytes(), APP_ID),
        None,
        "and not to any other app either"
    );
}

#[test]
fn header_image_is_preferred_and_capsule_image_is_the_fallback() {
    let both = body_for("620", "true", Some(HEADER_620), Some(CAPSULE_620));
    assert_eq!(
        appdetails_image_url(both.as_bytes(), APP_ID),
        Some(HEADER_620.to_string())
    );

    let capsule_only = body_for("620", "true", None, Some(CAPSULE_620));
    assert_eq!(
        appdetails_image_url(capsule_only.as_bytes(), APP_ID),
        Some(CAPSULE_620.to_string())
    );

    // An unusable header_image does not stop the capsule beside it from being used.
    let unusable_header = body_for(
        "620",
        "true",
        Some("http://shared.akamai.steamstatic.com/steam/apps/620/header.jpg"),
        Some(CAPSULE_620),
    );
    assert_eq!(
        appdetails_image_url(unusable_header.as_bytes(), APP_ID),
        Some(CAPSULE_620.to_string())
    );

    // A field that is present but not a string is not a URL.
    let numeric_header = format!(
        "{{\"999\":{{\"success\":true,\"data\":{{\"steam_appid\":620,\"header_image\":620,\"capsule_image\":\"{CAPSULE_620}\"}}}}}}"
    );
    assert_eq!(
        appdetails_image_url(numeric_header.as_bytes(), APP_ID),
        Some(CAPSULE_620.to_string())
    );

    let empty_both = body_for("620", "true", Some(""), Some(""));
    assert_eq!(appdetails_image_url(empty_both.as_bytes(), APP_ID), None);
}

#[test]
fn success_must_be_reported() {
    for not_reported in ["false", "0", "\"true\"", "null"] {
        let body = body_for("620", not_reported, Some(HEADER_620), None);
        assert_eq!(
            appdetails_image_url(body.as_bytes(), APP_ID),
            None,
            "success {not_reported} is not a report"
        );
    }
    let missing = format!(
        "{{\"999\":{{\"data\":{{\"steam_appid\":620,\"header_image\":\"{HEADER_620}\"}}}}}}"
    );
    assert_eq!(appdetails_image_url(missing.as_bytes(), APP_ID), None);

    let missing_data = r#"{"999":{"success":true}}"#;
    assert_eq!(appdetails_image_url(missing_data.as_bytes(), APP_ID), None);

    // A non-zero integer is a report, as it is on the store rung.
    let integer = body_for("620", "1", Some(HEADER_620), None);
    assert_eq!(
        appdetails_image_url(integer.as_bytes(), APP_ID),
        Some(HEADER_620.to_string())
    );
}

#[test]
fn the_requested_id_must_be_the_payloads_own_label() {
    // The label disagrees with the path: refuse rather than let two sources contradict.
    let other_label = body_for("80", "true", Some(HEADER_620), None);
    assert_eq!(appdetails_image_url(other_label.as_bytes(), APP_ID), None);

    let string_label = body_for("\"620\"", "true", Some(HEADER_620), None);
    assert_eq!(appdetails_image_url(string_label.as_bytes(), APP_ID), None);

    // The store labelled nothing, so there is no id to agree with the path.
    let absent_label = format!(
        "{{\"999\":{{\"success\":true,\"data\":{{\"name\":\"Synthetic\",\"header_image\":\"{HEADER_620}\"}}}}}}"
    );
    assert_eq!(appdetails_image_url(absent_label.as_bytes(), APP_ID), None);

    // Asked for through the wrong app id, the same body is not evidence for 730.
    let agreed = body_for("620", "true", Some(HEADER_620), None);
    assert_eq!(appdetails_image_url(agreed.as_bytes(), 730), None);
}

#[test]
fn only_an_https_path_whose_apps_segment_names_the_app_id_is_accepted() {
    for refused in [
        "http://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "//shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "steam/apps/620/header.jpg",
        "/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/6200/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/foo/620/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/80/header.jpg?t=620",
        "https://cdn620.example/store_item_assets/steam/apps/80/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620x/header.jpg",
        // A host spelled `apps` is not a path segment.
        "https://apps/620/header.jpg",
        // A URL whose authority is not a host cannot be the store's.
        "https://bad host/store_item_assets/steam/apps/620/header.jpg",
        "https://user@shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        // A host with nothing before the port, a port that is not a number, a port out of
        // range, a hyphen-edged or underscored label, brackets, and an IPv6 literal.
        "https://:/store_item_assets/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:abc/store_item_assets/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:123456/store_item_assets/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:65536/store_item_assets/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:99999/store_item_assets/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:+443/store_item_assets/steam/apps/620/header.jpg",
        "https://bad-.host/store_item_assets/steam/apps/620/header.jpg",
        "https://foo_bar.com/store_item_assets/steam/apps/620/header.jpg",
        "https://[invalid]/store_item_assets/steam/apps/620/header.jpg",
        "https://[2001:db8::1]/store_item_assets/steam/apps/620/header.jpg",
        // An address literal is not a name the store serves artwork from.
        "https://999.999.999.999/store_item_assets/steam/apps/620/header.jpg",
        "https://127.0.0.1/store_item_assets/steam/apps/620/header.jpg",
        // A host the store does not own is not the store, however well its path is shaped.
        "https://evil.example/store_item_assets/steam/apps/620/header.jpg",
        "https://steamstatic.com.evil.example/store_item_assets/steam/apps/620/header.jpg",
        "https://notsteamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "https://akamai.steamstatic.com./store_item_assets/steam/apps/620/header.jpg",
        // Case does not help a lookalike either.
        "https://Steamstatic.com.Evil.Example/store_item_assets/steam/apps/620/header.jpg",
        // An open image proxy: its path says `apps/620` while the URL in its query hands back
        // another app's bytes (measured: 200 image/jpeg, 20,694 bytes of app 80's artwork).
        // The host pin is the rule that refuses it.
        "https://wsrv.nl/store_item_assets/steam/apps/620/header.jpg?url=https%3A%2F%2Fshared.akamai.steamstatic.com%2Fstore_item_assets%2Fsteam%2Fapps%2F80%2Fheader.jpg",
        // A query the downloader's URI parser would refuse is not a candidate either.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=y z",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=é",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg#a b",
        // "Printable ASCII" is not the same as "a URI character": these are all printable and
        // all outside RFC 3986's set, which is the set the downloader enforces.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=<",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=>",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=\"",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=\\",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x={",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=|",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=}",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=`",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=^",
        // An HTTP client normalises these away, so the file behind them may belong to
        // another app id even though the app id appears in the path.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/../80/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/%2e%2e/80/header.jpg",
        // An escape that *forms* a dot segment is refused for the same reason as the literal
        // `../`: a client normalises the path away and lands on another app's file.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/%2e/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/.%2e/80/header.jpg",
        // An escape is judged by what it stands for: these four can be decoded into a path
        // character (`.` `/` `\` `%`), the rest of the escapes are just file-name bytes.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/%2E%2E/80/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620%2f..%2f80/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/%5Chello.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/he%25ader.jpg",
        // A truncated or non-hex escape is not a URL a client can send.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/he%6.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/he%zzader.jpg",
        // An escape has to be well formed anywhere in the URI, not only in the path: the
        // downloader refuses these requests, so accepting them here would make the answer
        // depend on which layer was asked.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=%zz",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=%",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?x=%2",
        // An escape may only spell a byte a file name can hold: `%c0%ae` is half of an overlong
        // `.`, and a lenient decoder would resolve `%c0%ae%c0%ae` to `..` and leave this app's
        // path for another one.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/%c0%ae%c0%ae/80/header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header%ff.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header%0Atab.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/./header.jpg",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam//apps/620/header.jpg",
    ] {
        let body = body_for("620", "true", Some(refused), None);
        assert_eq!(
            appdetails_image_url(body.as_bytes(), APP_ID),
            None,
            "{refused} must be refused"
        );
    }

    for accepted in [
        HEADER_620,
        // Schemes are case-insensitive, and the port is part of the authority.
        "HTTPS://shared.akamai.steamstatic.com/steam/apps/620/header.jpg",
        "https://shared.akamai.steamstatic.com:443/store_item_assets/steam/apps/620/header.jpg",
        // The reserved characters a real query uses are URI characters, not invalid ones.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?t=1790187113&x=~a-_.b",
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg?t=1%202",
        // An escape in the path that stands for an ordinary file-name byte is fine: this is
        // the live 620 artwork with its file name written `%68eader.jpg`, and the downloader
        // fetches the very same 41,191 bytes.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/%68eader.jpg?t=1790187113",
        // Likewise an escaped space: a space is a byte of a file name, not a separator.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/header%20x.jpg",
        // And an escaped dot inside a file name: it decodes to `header.jpg`, an ordinary name
        // a reviewer fetched 41,191 bytes for. The dot only matters when the whole decoded
        // segment is `.` or `..`.
        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/header%2Ejpg?t=1790187113",
        // Every host the store serves artwork from is under its own domain, and host names
        // are case-insensitive, exactly as the scheme is.
        "https://cdn.akamai.steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "https://steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "https://Shared.Akamai.Steamstatic.com/store_item_assets/steam/apps/620/header.jpg",
        "https://SHARED.AKAMAI.STEAMSTATIC.COM/store_item_assets/steam/apps/620/header.jpg",
    ] {
        let body = body_for("620", "true", Some(accepted), None);
        assert_eq!(
            appdetails_image_url(body.as_bytes(), APP_ID),
            Some(accepted.to_string()),
            "{accepted} must be accepted"
        );
    }
}

#[test]
fn malformed_and_oversized_bodies_yield_nothing() {
    for body in [
        &b""[..],
        &b"{"[..],
        &b"[]"[..],
        &b"null"[..],
        &b"\"620\""[..],
        &b"{}"[..],
        &b"not json"[..],
    ] {
        assert_eq!(appdetails_image_url(body, APP_ID), None);
    }

    // The bound is on the body, not on the JSON: a valid response of exactly the bound is
    // parsed, and the same response padded one byte past it is refused. (Filling the body
    // with spaces, as this test used to, proves nothing — that is not JSON at all, so serde
    // refuses it with or without a bound.)
    let body_of_len = |target: usize| -> String {
        let body = format!(
            "{{\"999\":{{\"success\":true,\"data\":{{\"steam_appid\":620,\"header_image\":\"{HEADER_620}\",\"pad\":\"\"}}}}}}"
        );
        assert!(body.len() < target);
        body.replace(
            "\"pad\":\"\"",
            &format!("\"pad\":\"{}\"", "x".repeat(target - body.len())),
        )
    };

    let at_bound = body_of_len(4 * 1024 * 1024);
    assert_eq!(at_bound.len(), 4 * 1024 * 1024);
    assert_eq!(
        appdetails_image_url(at_bound.as_bytes(), APP_ID),
        Some(HEADER_620.to_string()),
        "a valid body of exactly the bound is parsed"
    );

    let over_bound = body_of_len(4 * 1024 * 1024 + 1);
    assert_eq!(
        appdetails_image_url(over_bound.as_bytes(), APP_ID),
        None,
        "one byte over the bound is refused, however valid the JSON is"
    );
}

#[test]
fn the_request_asks_for_the_app_id_with_basic_filters() {
    let url = appdetails_url(APP_ID);
    assert!(url.starts_with("https://store.steampowered.com/api/appdetails?"));
    assert!(url.contains("appids=620"));
    assert!(url.contains("filters=basic"));
}

#[test]
fn the_candidates_come_in_order_and_only_from_this_app() {
    let both = body_for("620", "true", Some(HEADER_620), Some(CAPSULE_620));
    assert_eq!(
        appdetails_image_urls(both.as_bytes(), APP_ID),
        vec![HEADER_620.to_string(), CAPSULE_620.to_string()]
    );

    let capsule_only = body_for("620", "true", None, Some(CAPSULE_620));
    assert_eq!(
        appdetails_image_urls(capsule_only.as_bytes(), APP_ID),
        vec![CAPSULE_620.to_string()]
    );

    // A header that belongs to another app is not a candidate; the capsule beside it is.
    let other_apps_header = body_for(
        "620",
        "true",
        Some("https://shared.akamai.steamstatic.com/steam/apps/80/header.jpg"),
        Some(CAPSULE_620),
    );
    assert_eq!(
        appdetails_image_urls(other_apps_header.as_bytes(), APP_ID),
        vec![CAPSULE_620.to_string()]
    );

    let neither = body_for("620", "true", None, None);
    assert!(appdetails_image_urls(neither.as_bytes(), APP_ID).is_empty());

    // The live 100 body offers no candidate at all for the app it was asked about.
    assert!(appdetails_image_urls(APP_DETAILS_100.as_bytes(), 100).is_empty());
}

#[test]
fn a_foreign_host_never_supplies_the_cover() {
    // Review round 6's counterexample, end to end: a body that claims app 620 and names a
    // host the store does not own, whose path does name the app id. The rung must neither
    // fetch it nor write anything for it, even though that host is ready to serve artwork.
    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let foreign_body = r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://evil.example/steam/apps/620/header.jpg"}}}"#;
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![
            (
                "store.steampowered.com/api/appdetails",
                foreign_body.as_bytes().to_vec(),
            ),
            ("evil.example", PORTRAIT_PNG.to_vec()),
        ],
    );

    assert!(
        cache.fetch(&request("Portal 2", Some(APP_ID))).is_none(),
        "a URL on a host the store does not own is a miss"
    );
    let urls = log.lock().unwrap().clone();
    assert!(
        !urls.iter().any(|url| url.contains("evil.example")),
        "the foreign host must never be asked for artwork: {urls:?}"
    );
    assert!(
        !tmp.path().join("appid_620.jpg").exists(),
        "nothing may be written for the foreign host"
    );
}

type Log = Arc<Mutex<Vec<String>>>;

/// A fetcher that records every URL and answers from `responses`, matched by
/// substring. An unmatched URL is a failed request, like a 404.
fn fetcher(log: &Log, responses: Vec<(&'static str, Vec<u8>)>) -> Fetcher {
    let log = Arc::clone(log);
    Arc::new(move |url: &str| {
        log.lock().unwrap().push(url.to_string());
        responses
            .iter()
            .find(|(needle, _)| url.contains(needle))
            .map(|(_, bytes)| bytes.clone())
    })
}

fn request(name: &str, appid: Option<u32>) -> ArtRequest {
    ArtRequest {
        name: name.to_string(),
        appid,
        art_url: None,
        platform_is_gog: false,
        game_path: None,
    }
}

fn cache_in(
    dir: &std::path::Path,
    log: &Log,
    responses: Vec<(&'static str, Vec<u8>)>,
) -> ImageCache {
    ImageCache::with_sources(dir, Vec::<PathBuf>::new(), fetcher(log, responses))
}

#[test]
fn the_rung_caches_the_live_620_answer_under_the_requested_app_id() {
    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![
            (
                "store.steampowered.com/api/appdetails",
                APP_DETAILS_620.as_bytes().to_vec(),
            ),
            ("store_item_assets/steam/apps/620", PORTRAIT_PNG.to_vec()),
        ],
    );

    let path = cache
        .fetch(&request("Portal 2", Some(APP_ID)))
        .expect("the live 620 answer must produce a cached image");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("appid_620.jpg")
    );
    let urls = log.lock().unwrap().clone();
    assert!(
        urls.iter()
            .any(|url| url.contains("store.steampowered.com/api/appdetails")),
        "the rung must be the one that produced it: {urls:?}"
    );
}

#[test]
fn the_rung_never_downloads_another_apps_artwork() {
    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![
            (
                "store.steampowered.com/api/appdetails",
                APP_DETAILS_100.as_bytes().to_vec(),
            ),
            ("store_item_assets/steam/apps/80", PORTRAIT_PNG.to_vec()),
        ],
    );

    assert_eq!(
        cache.fetch(&request("Counter-Strike: Condition Zero", Some(100))),
        None,
        "a refused rung yields no cover"
    );

    let urls = log.lock().unwrap().clone();
    assert!(
        urls.iter().any(|url| url.contains("api/appdetails")),
        "the rung must have been consulted, or this test passes for the wrong reason: {urls:?}"
    );
    assert!(
        !urls.iter().any(|url| url.contains("apps/80")),
        "app 80's artwork must never be requested: {urls:?}"
    );
    assert!(
        tmp.path().read_dir().unwrap().next().is_none(),
        "a refusal must write nothing into the cache"
    );
}

#[test]
fn a_refusal_leaves_the_source_order_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![(
            "store.steampowered.com/api/appdetails",
            APP_DETAILS_100.as_bytes().to_vec(),
        )],
    );

    assert_eq!(cache.fetch(&request("Synthetic", Some(100))), None);

    let kinds: Vec<&str> = log
        .lock()
        .unwrap()
        .iter()
        .map(|url| {
            if url.contains("GetItems") {
                "store portrait"
            } else if url.contains("cdn.akamai.steamstatic.com") {
                "cdn header"
            } else if url.contains("api/appdetails") {
                "appdetails"
            } else {
                "other"
            }
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["store portrait", "cdn header", "appdetails"],
        "the rung keeps its place and a refusal does not abort the chain"
    );
}

#[test]
fn the_review_counterexamples_are_closed() {
    // The exact bodies the two review rounds used, verbatim, so a later reviewer can see
    // the findings are pinned rather than quietly patched over.
    for refused in [
        // Round 1: the authority mistaken for a path segment.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://apps/620/header.jpg"}}}"#,
        // Round 1: an authority that is not a host.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://bad host/apps/620/header.jpg"}}}"#,
        // Round 1: dot segments an HTTP client normalises to app 80's file.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://shared.akamai.steamstatic.com/steam/apps/620/../80/header.jpg"}}}"#,
        // Round 2: an authority that passes a character check but has no host.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://:/apps/620/header.jpg"}}}"#,
        // Round 2: brackets that are not a valid IPv6 literal.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://[invalid]/apps/620/header.jpg"}}}"#,
        // Round 2: a port that is not a number.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://shared.akamai.steamstatic.com:abc/apps/620/header.jpg"}}}"#,
        // Round 4: a NUL in the query, which the downloader's URI parser refuses while the
        // path rule alone would have accepted the URL.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://shared.akamai.steamstatic.com/steam/apps/620/header.jpg?x=\u0000"}}}"#,
        // Round 4: an address literal as the host.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://999.999.999.999/steam/apps/620/header.jpg"}}}"#,
        // Round 5: a `<` in the query. Printable, so a "printable ASCII" rule let it through,
        // but not a URI character, so the downloader refused the request.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://shared.akamai.steamstatic.com/steam/apps/620/header.jpg?x=<"}}}"#,
        // Round 6: a foreign host whose path names this app id. Shape alone cannot tell it
        // from the store's own CDN, so the host itself has to be the store's.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://evil.example/steam/apps/620/header.jpg"}}}"#,
        // Round 6/7 (codex): an open image proxy, whose path says the app id while the URL in
        // its query serves another app's artwork. Refused by the host pin, not by the path.
        r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://wsrv.nl/store_item_assets/steam/apps/620/header.jpg?url=https%3A%2F%2Fshared.akamai.steamstatic.com%2Fstore_item_assets%2Fsteam%2Fapps%2F80%2Fheader.jpg"}}}"#,
    ] {
        assert_eq!(
            appdetails_image_url(refused.as_bytes(), APP_ID),
            None,
            "{refused} must be refused"
        );
    }

    let uppercase_scheme = r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"HTTPS://shared.akamai.steamstatic.com/steam/apps/620/header.jpg"}}}"#;
    assert_eq!(
        appdetails_image_url(uppercase_scheme.as_bytes(), APP_ID),
        Some("HTTPS://shared.akamai.steamstatic.com/steam/apps/620/header.jpg".to_string()),
        "a valid HTTPS URL must not be refused for the case of its scheme"
    );

    // The other half of a falsification: a body an earlier round showed the code must
    // *accept*, kept verbatim so a later narrowing of the rules cannot quietly refuse it.
    for (body, expected) in [
        // Round 7: the store's own host written in capitals — DNS is case-insensitive.
        (
            r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://SHARED.AKAMAI.STEAMSTATIC.COM/steam/apps/620/header.jpg"}}}"#,
            "https://SHARED.AKAMAI.STEAMSTATIC.COM/steam/apps/620/header.jpg",
        ),
        // Round 8: the live 620 file name written `%68eader.jpg` — the same 41,191 bytes.
        (
            r#"{"999":{"success":true,"data":{"steam_appid":620,"header_image":"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/%68eader.jpg?t=1790187113"}}}"#,
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/%68eader.jpg?t=1790187113",
        ),
    ] {
        assert_eq!(
            appdetails_image_url(body.as_bytes(), APP_ID),
            Some(expected.to_string()),
            "a body an earlier round required must still be accepted"
        );
    }
}

#[test]
fn a_header_whose_bytes_fail_still_leaves_the_capsule() {
    // The rung tried header_image, then capsule_image, when the first download missed.
    // Moving the parsing out of `fetch_steam` must not cost that fallback: this pins the
    // live 620 body, whose header URL is left unmatched (a 404) while its capsule answers.
    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![
            (
                "store.steampowered.com/api/appdetails",
                APP_DETAILS_620.as_bytes().to_vec(),
            ),
            ("capsule_231x87.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let path = cache
        .fetch(&request("Portal 2", Some(APP_ID)))
        .expect("the capsule beside a dead header must still be tried");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("appid_620.jpg")
    );

    let urls = log.lock().unwrap().clone();
    let header_at = urls
        .iter()
        .position(|url| url.contains("faffc0f560786e2f05104a8d2fac837c6969bf13"));
    let capsule_at = urls
        .iter()
        .position(|url| url.contains("capsule_231x87.jpg"));
    assert!(
        matches!((header_at, capsule_at), (Some(header), Some(capsule)) if header < capsule),
        "the header is tried first and the capsule after it fails: {urls:?}"
    );
}

#[test]
fn a_dot_segment_cannot_smuggle_another_apps_artwork_into_the_cache() {
    // An HTTP client normalises /apps/620/../80/header.jpg to app 80's file, so a path
    // that only appears to name app 620 would still cache app 80's bytes under
    // appid_620.jpg. The rung must refuse the URL outright.
    let body = body_for(
        "620",
        "true",
        Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/620/../80/header.jpg"),
        None,
    );

    let tmp = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let cache = cache_in(
        tmp.path(),
        &log,
        vec![
            (
                "store.steampowered.com/api/appdetails",
                body.as_bytes().to_vec(),
            ),
            ("store_item_assets/steam/apps/80", PORTRAIT_PNG.to_vec()),
        ],
    );

    assert_eq!(cache.fetch(&request("Portal 2", Some(APP_ID))), None);

    let urls = log.lock().unwrap().clone();
    assert!(
        urls.iter().any(|url| url.contains("api/appdetails")),
        "the rung must have been consulted: {urls:?}"
    );
    assert!(
        !urls.iter().any(|url| url.contains("apps/80")),
        "app 80's artwork must never be requested through a dot segment: {urls:?}"
    );
    assert!(
        tmp.path().read_dir().unwrap().next().is_none(),
        "nothing may be written for this app id"
    );
}
