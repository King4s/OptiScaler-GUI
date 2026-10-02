# Specification: Steam's store portrait for an app id (T4)

Scope: ask Steam's store API for an app id's portrait and use it before the
existing header/appdetails fallback, treating every response as untrusted input.
Not in scope: the local library cache (T3), per-store provenance and cache keys
(T10), the other stores.

## Evidence this is built on

Captured live on 2026-09-26 for app id 620:

```
GET https://api.steampowered.com/IStoreBrowseService/GetItems/v1/
      ?input_json={"ids":[{"appid":620}],
                   "context":{"language":"english","country_code":"US"},
                   "data_request":{"include_assets":true}}
      &format=json
```

`response.store_items[0]` = `{ appid: 620, success: 1, name: "Portal 2", assets: {...} }`, and

| asset key | value |
|---|---|
| `asset_url_format` | `steam/apps/620/${FILENAME}?t=1790187113` |
| `library_capsule` | `library_600x900.jpg` |
| `library_capsule_2x` | `library_600x900_2x.jpg` |
| `header` | `faffc0f560786e2f05104a8d2fac837c6969bf13/header.jpg` |
| `main_capsule` | `02b755eb0cf5f41c1838a99c9a353ebce32a88cd/capsule_616x353.jpg` |

So the portrait URL is `asset_url_format` with `${FILENAME}` replaced, i.e.
`https://cdn.akamai.steamstatic.com/steam/apps/620/library_600x900.jpg?t=1790187113`
— the host and shape the existing header fetch already uses.

Note the 2x variant exists remotely while the local cache stores only 1x. 600x900
is already larger than the 300x450 the pipeline stores, so 1x is requested and
2x is not.

## Rules

1. The portrait URL comes from `GetItems`, parsed as untrusted JSON: the item must
   report success — the service sends the integer `1` here, and the parser takes a
   non-zero integer that fits in an `i64`, or a boolean `true` — its `appid` must
   equal the requested app
   id, and both
   `assets.asset_url_format` and `assets.library_capsule` must be present and
   non-empty. Anything else yields nothing — no guessing from a sibling field, no
   use of a neighbouring item.
2. Substitution is textual on `${FILENAME}` only; a format string without the
   placeholder contributes nothing.
3. Everything from the network is bounded: a response body over 4 MiB is not
   parsed, downloaded image bytes over 4 MiB are not decoded, and decoding is
   capped at 4096x4096 pixels / 64 MiB. The fetch itself refuses to read more than
   20 MiB. A refusal is a miss, never an error.
4. Order for a Steam app id: `GetItems` portrait → CDN `header.jpg` → `appdetails`
   (`header_image`, `capsule_image`) → the rest of the chain unchanged.
5. HTTP failure, an empty body, non-JSON, a missing item for this app id: all
   misses that fall through to the next source. Nothing surfaces as an error.
6. The HTTP fetch is injectable, so failures, mock responses and image bounds are
   testable without a network, and the local-portrait order test stops depending
   on a live CDN. The response bound is pinned in-tree for the store parser and for
   the appdetails rung — without the guard, an oversized body from that rung's own
   request would be parsed and its `header_image` fetched; the GOG search body keeps
   the same guard without a
   test of its own — the guard is one shared helper, not a per-caller rule.

## Known defect on the rung below this one

The `appdetails` rung does not work today: it looks the response up by
`appid.to_string()`, but a live response is normally keyed by a different id than
the one asked for — 620 comes back keyed `323180`, 730 as `2678630`, 220 as
`323140`, 440 as `629330`, 570 as `2120612`, 400 as `622640`, 70 as `632440` —
while `data.steam_appid` holds the requested app id. The rung is therefore
unreliable in both directions, and the second one is the worse: when the key does
match, the payload can belong to another app, so `appids=100` answers keyed `100`
with `data.steam_appid` 80 and the rung would return app 80's artwork for app 100.
This is pre-existing and unchanged here; it is written down rather than fixed so it
gets its own change and its own live verification. That change is
`tasks/spec-steam-appdetails-art.md`, which replaces the key lookup with a rule that
requires the payload's own `steam_appid` and the artwork URL's path to name the
requested app id.

## Why the seam is part of this task

Before this change `fetch_steam` read the CDN first and `download_and_cache`
decoded whatever the network returned with no bound of its own. Both the plan's verify list
(HTTP failure, oversized and malformed images) and the first review's residual
finding (an order test that cannot fail offline) need a fitted fetcher; adding it
here is what makes those checks real instead of asserted.
