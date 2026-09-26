//! Fixture tests for the Steam store portrait source (`opticore::images`).
//!
//! The GetItems body is the live response captured for app 620 on 2026-09-26; the
//! image bytes are synthetic. Every HTTP call goes through an injected fetcher, so
//! no test reads the network, the registry or this machine's Steam installs.

use std::sync::{Arc, Mutex};

use opticore::images::{store_item_portrait_url, store_items_url, Fetcher, ImageCache};

/// The live `IStoreBrowseService/GetItems/v1` response for app id 620, verbatim.
const GET_ITEMS_620: &str = r#"{"response":{"store_items":[{"item_type":0,"id":620,"success":1,"visible":true,"name":"Portal 2","store_url_path":"app/620/Portal_2","store_url_slug":"Portal_2","appid":620,"type":0,"categories":{"supported_player_categoryids":[2,1,9,38,39,24],"feature_categoryids":[22,29,13,30,51,67,68,69,70,74,79,23,15,17,14,41,42,43,44,62],"controller_categoryids":[28,55,56,57,58,59]},"assets":{"asset_url_format":"steam/apps/620/${FILENAME}?t=1790187113","main_capsule":"02b755eb0cf5f41c1838a99c9a353ebce32a88cd/capsule_616x353.jpg","main_capsule_2x":"02b755eb0cf5f41c1838a99c9a353ebce32a88cd/capsule_616x353_2x.jpg","small_capsule":"bd23943dcd0280aa2cdc066e2f86d2185cfd4cce/capsule_231x87.jpg","small_capsule_2x":"bd23943dcd0280aa2cdc066e2f86d2185cfd4cce/capsule_231x87_2x.jpg","header":"faffc0f560786e2f05104a8d2fac837c6969bf13/header.jpg","header_2x":"faffc0f560786e2f05104a8d2fac837c6969bf13/header_2x.jpg","hero_capsule":"9a8ae9bb0b85f125bd956024f86f30a3aeb799e8/hero_capsule.jpg","hero_capsule_2x":"9a8ae9bb0b85f125bd956024f86f30a3aeb799e8/hero_capsule_2x.jpg","library_capsule":"library_600x900.jpg","library_capsule_2x":"library_600x900_2x.jpg","library_hero":"a58588d857b8683f8065e55f97ab82ad8a945c51/library_hero.jpg","library_hero_2x":"a58588d857b8683f8065e55f97ab82ad8a945c51/library_hero_2x.jpg","community_icon":"25a5a16b2423bf7487ac5340b5b0948cef48c5f8","page_background_path":"app/620?t=1790187113","last_modified":1790187113},"best_purchase_option":{"packageid":7877,"purchase_option_name":"Portal 2","final_price_in_cents":"999","formatted_final_price":"$9.99","user_can_purchase_as_gift":true,"hide_discount_pct_for_compliance":false,"included_game_count":1,"must_purchase_as_set":false,"package_group":"default","price_cannot_be_displayed_as_discount":false}}]}}"#;

const APP_ID: u32 = 620;
const PORTRAIT_URL: &str =
    "https://cdn.akamai.steamstatic.com/steam/apps/620/library_600x900.jpg?t=1790187113";

/// A real 6x9 magenta PNG: a valid download to cache.
const PORTRAIT_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x09, 0x08, 0x02, 0x00, 0x00, 0x00, 0x9e, 0xf8, 0xca,
    0xca, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xf0, 0x1f,
    0x0d, 0x31, 0x0c, 0x52, 0x21, 0x00, 0xf0, 0xa6, 0x6b, 0x95, 0x1a, 0xa4, 0x0b, 0xb6, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// A real, valid 5000x1 PNG: wider than the decode bound.
const OVER_BOUND_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x13, 0x88, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0xbd, 0x73, 0xd3,
    0xdf, 0x00, 0x00, 0x00, 0x27, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xc2, 0x01, 0x09, 0x00,
    0x00, 0x00, 0x02, 0xa0, 0xfe, 0x9f, 0xae, 0x09, 0x1d, 0x50, 0x4c, 0x53, 0x55, 0x55, 0x55, 0x55,
    0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0xbd, 0x07, 0xd6, 0x6b, 0xeb, 0x2b,
    0xce, 0x2b, 0xea, 0x92, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

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

fn cache_in(
    dir: &std::path::Path,
    log: &Log,
    responses: Vec<(&'static str, Vec<u8>)>,
) -> ImageCache {
    ImageCache::with_sources(dir, Vec::new(), fetcher(log, responses))
}

/// A store response whose single item carries the given assets object.
fn body_with(assets: &str) -> String {
    format!(
        r#"{{"response":{{"store_items":[{{"appid":620,"success":true,"assets":{assets}}}]}}}}"#
    )
}

fn request() -> opticore::images::ArtRequest {
    opticore::images::ArtRequest {
        name: "Portal 2".to_string(),
        appid: Some(APP_ID),
        art_url: None,
        platform_is_gog: false,
        game_path: None,
    }
}

// ---------- the parser ----------

#[test]
fn a_successful_item_yields_the_portrait_url() {
    assert_eq!(
        store_item_portrait_url(GET_ITEMS_620.as_bytes(), APP_ID).as_deref(),
        Some(PORTRAIT_URL)
    );
}

#[test]
fn an_item_for_another_app_id_yields_nothing() {
    assert!(store_item_portrait_url(GET_ITEMS_620.as_bytes(), 730).is_none());
}

#[test]
fn a_body_without_that_app_id_yields_nothing() {
    let body = body_with(r#"{"asset_url_format":"steam/apps/730/${FILENAME}","library_capsule":"library_600x900.jpg"}"#)
        .replace(r#""appid":620"#, r#""appid":730"#);
    assert!(store_item_portrait_url(body.as_bytes(), APP_ID).is_none());
}

#[test]
fn success_false_or_missing_yields_nothing() {
    let no_success = r#"{"response":{"store_items":[{"appid":620,"assets":{"asset_url_format":"a/${FILENAME}","library_capsule":"p.jpg"}}]}}"#;
    let false_success = r#"{"response":{"store_items":[{"appid":620,"success":false,"assets":{"asset_url_format":"a/${FILENAME}","library_capsule":"p.jpg"}}]}}"#;
    assert!(store_item_portrait_url(no_success.as_bytes(), APP_ID).is_none());
    assert!(store_item_portrait_url(false_success.as_bytes(), APP_ID).is_none());
}

#[test]
fn a_missing_items_array_yields_nothing() {
    assert!(store_item_portrait_url(br#"{"response":{}}"#, APP_ID).is_none());
    assert!(store_item_portrait_url(br#"{"response":{"store_items":[]}}"#, APP_ID).is_none());
}

#[test]
fn missing_or_empty_asset_fields_yield_nothing() {
    for assets in [
        r#"{}"#,
        r#"{"asset_url_format":"steam/apps/620/${FILENAME}?t=1"}"#,
        r#"{"library_capsule":"library_600x900.jpg"}"#,
        r#"{"asset_url_format":"","library_capsule":"library_600x900.jpg"}"#,
        r#"{"asset_url_format":"steam/apps/620/${FILENAME}","library_capsule":""}"#,
    ] {
        let body = body_with(assets);
        assert!(
            store_item_portrait_url(body.as_bytes(), APP_ID).is_none(),
            "assets {assets} must not yield a portrait"
        );
    }
}

#[test]
fn a_format_without_the_filename_placeholder_yields_nothing() {
    let body = body_with(
        r#"{"asset_url_format":"steam/apps/620/library_600x900.jpg","library_capsule":"library_600x900.jpg"}"#,
    );
    assert!(store_item_portrait_url(body.as_bytes(), APP_ID).is_none());
}

#[test]
fn malformed_and_empty_bodies_yield_nothing() {
    for body in ["{not json", "", "[]", "null", "<html>"] {
        assert!(store_item_portrait_url(body.as_bytes(), APP_ID).is_none());
    }
}

#[test]
fn the_request_url_is_a_valid_query_string() {
    let url = store_items_url(APP_ID);
    assert!(url
        .starts_with("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json="));
    assert!(url.ends_with("&format=json"));
    assert!(url.contains("%7B%22ids%22%3A%5B%7B%22appid%22%3A620%7D%5D"));
    for raw in ['"', '{', '}', ' '] {
        assert!(
            !url.contains(raw),
            "raw {raw} left in the query string: {url}"
        );
    }
}

// ---------- the injected fetcher ----------

#[test]
fn the_store_portrait_is_used_before_the_cdn() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", GET_ITEMS_620.as_bytes().to_vec()),
            ("library_600x900.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let found = images
        .fetch(&request())
        .expect("the store portrait must be used");

    assert_eq!(found, dir.path().join(format!("appid_{APP_ID}.jpg")));
    let asked = log.lock().unwrap().clone();
    assert_eq!(asked.len(), 2, "only GetItems and its portrait: {asked:?}");
    assert!(asked[0].contains("GetItems"), "{asked:?}");
    assert_eq!(asked[1], PORTRAIT_URL);
    assert!(
        !asked
            .iter()
            .any(|u| u.contains("header.jpg") || u.contains("appdetails")),
        "no fallback may be asked after a hit: {asked:?}"
    );
}

#[test]
fn the_cdn_header_is_the_fallback_when_the_store_has_no_portrait() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", br#"{"response":{"store_items":[]}}"#.to_vec()),
            ("header.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let found = images
        .fetch(&request())
        .expect("the CDN header must be used");

    assert_eq!(found, dir.path().join(format!("appid_{APP_ID}.jpg")));
    let asked = log.lock().unwrap().clone();
    assert!(asked[0].contains("GetItems"), "{asked:?}");
    assert!(asked[1].contains("header.jpg"), "{asked:?}");
}

#[test]
fn an_http_failure_is_a_miss_that_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let images = cache_in(dir.path(), &log, Vec::new());

    assert!(images.fetch(&request()).is_none());
    let asked = log.lock().unwrap().clone();
    assert!(asked[0].contains("GetItems"), "{asked:?}");
    assert!(
        asked.len() >= 3,
        "the fallbacks were still tried: {asked:?}"
    );
}

#[test]
fn an_oversized_download_is_a_miss_that_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let mut oversized = PORTRAIT_PNG.to_vec();
    oversized.resize(5 * 1024 * 1024, 0);
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", GET_ITEMS_620.as_bytes().to_vec()),
            ("library_600x900.jpg", oversized),
            ("header.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let found = images
        .fetch(&request())
        .expect("the CDN header must be used");
    assert_eq!(found, dir.path().join(format!("appid_{APP_ID}.jpg")));
}

#[test]
fn a_malformed_download_is_a_miss_that_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", GET_ITEMS_620.as_bytes().to_vec()),
            ("library_600x900.jpg", b"not an image at all".to_vec()),
            ("header.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let found = images
        .fetch(&request())
        .expect("the CDN header must be used");
    assert_eq!(found, dir.path().join(format!("appid_{APP_ID}.jpg")));
}

#[test]
fn a_download_declaring_too_many_pixels_is_a_miss_that_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", GET_ITEMS_620.as_bytes().to_vec()),
            ("library_600x900.jpg", OVER_BOUND_PNG.to_vec()),
            ("header.jpg", PORTRAIT_PNG.to_vec()),
        ],
    );

    let found = images
        .fetch(&request())
        .expect("the CDN header must be used");
    assert_eq!(found, dir.path().join(format!("appid_{APP_ID}.jpg")));
}

// The response reports `success` as an integer bitfield, not a boolean: the live
// value is 1 for app 620 and 15 for an app id that does not exist. These two pin
// both ends of the rule the parser implements, on a body that is otherwise
// perfect, so a change back to a boolean-only check fails here rather than in
// production.

#[test]
fn a_non_boolean_non_zero_success_is_accepted() {
    let body = GET_ITEMS_620.replace("\"success\":1", "\"success\":15");
    assert_ne!(
        body, GET_ITEMS_620,
        "the fixture must carry the measured value"
    );
    assert_eq!(
        store_item_portrait_url(body.as_bytes(), APP_ID),
        Some(PORTRAIT_URL.to_string()),
        "a reported item is an item, whatever integer the bitfield holds"
    );
}

#[test]
fn success_zero_yields_nothing() {
    let body = GET_ITEMS_620.replace("\"success\":1", "\"success\":0");
    assert_eq!(store_item_portrait_url(body.as_bytes(), APP_ID), None);
}

/// The bound on a response body, told apart from the JSON parser by using the
/// captured body itself: the padded copy is the same valid JSON plus one long
/// string field, and the unpadded copy does yield the portrait.
#[test]
fn an_oversized_response_is_not_parsed() {
    let at = GET_ITEMS_620
        .find("\"store_items\"")
        .expect("the fixture lists items");
    let mut padded = String::with_capacity(GET_ITEMS_620.len() + 5 * 1024 * 1024);
    padded.push_str(&GET_ITEMS_620[..at]);
    padded.push_str(&format!("\"padding\":\"{}\",", "a".repeat(5 * 1024 * 1024)));
    padded.push_str(&GET_ITEMS_620[at..]);

    assert!(padded.len() > 5 * 1024 * 1024);
    assert_eq!(
        store_item_portrait_url(padded.as_bytes(), APP_ID),
        None,
        "a response over the byte bound must not be parsed"
    );
    assert_eq!(
        store_item_portrait_url(GET_ITEMS_620.as_bytes(), APP_ID),
        Some(PORTRAIT_URL.to_string()),
        "the same body within the bound does yield the portrait"
    );
}

/// The appdetails rung takes a response body from the network too, and only its
/// payload is worth anything: with the byte bound removed this body is valid JSON,
/// keyed the way the code looks it up, and its `header_image` gets fetched. The
/// live service does not key it that way (see the task spec), which is exactly why
/// the guard is worth pinning here rather than trusting the rung to be harmless.
#[test]
fn an_oversized_appdetails_body_is_not_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let no_portrait = GET_ITEMS_620.replace(
        "\"library_capsule\":\"library_600x900.jpg\"",
        "\"library_capsule\":\"\"",
    );
    let appdetails = format!(
        "{{\"620\":{{\"success\":true,\"data\":{{\"header_image\":\"https://cdn.example/appdetails-header.jpg\",\"padding\":\"{}\"}}}}}}",
        "a".repeat(5 * 1024 * 1024)
    );
    let images = cache_in(
        dir.path(),
        &log,
        vec![
            ("GetItems", no_portrait.into_bytes()),
            ("appdetails", appdetails.into_bytes()),
        ],
    );

    assert!(images.fetch(&request()).is_none());

    let asked = log.lock().unwrap().clone();
    assert!(
        asked.iter().any(|u| u.contains("appdetails")),
        "the appdetails rung must be reached: {asked:?}"
    );
    assert!(
        !asked.iter().any(|u| u.contains("appdetails-header.jpg")),
        "an oversized appdetails body must not be parsed: {asked:?}"
    );
}

// The success rule at its edges, against the captured body with only that field
// changed: the service sends integers, `true` is also accepted, and a float, a
// string, `0` and a missing field are not.
#[test]
fn the_success_boundary_is_integers_and_booleans() {
    for (value, accepted) in [
        ("true", true),
        ("-1", true),
        ("0", false),
        ("1.0", false),
        ("\"1\"", false),
    ] {
        let body = GET_ITEMS_620.replace("\"success\":1", &format!("\"success\":{value}"));
        assert_ne!(body, GET_ITEMS_620, "the fixture must carry the value");
        assert_eq!(
            store_item_portrait_url(body.as_bytes(), APP_ID).is_some(),
            accepted,
            "success:{value} must {} report the item",
            if accepted { "" } else { "not" }
        );
    }
}
