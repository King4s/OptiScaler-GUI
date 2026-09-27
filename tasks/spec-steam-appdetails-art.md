# Specification: the appdetails rung must serve the app id's own artwork (T4b)

Scope: fix the `appdetails` rung of the Steam artwork chain so a game can only be
given the artwork of the app id it was looked up for. Not in scope: per-source cache
keys and per-store provenance (T10), the local library cache (T3), the store portrait
source (T4), the other stores.

This closes the defect that `tasks/spec-steam-getitems-art.md` recorded under
"Known defect on the rung below this one" rather than fixed.

## Evidence this is built on

Captured live on 2026-09-27 with the request the rung itself makes
(`filters=basic`), from this machine:

```
curl -s "https://store.steampowered.com/api/appdetails?appids=620&filters=basic" -o appdetails-620.json
curl -s "https://store.steampowered.com/api/appdetails?appids=100&filters=basic" -o appdetails-100.json
```

| asked for | response key | `success` | `data.steam_appid` | `data.name` | `data.header_image` path |
|---|---|---|---|---|---|
| `appids=620` | `323180` | `true` | `620` | Portal 2 | `/steam/apps/620/…/header.jpg` |
| `appids=100` | `100` | `true` | `80` | Counter-Strike: Condition Zero | `/steam/apps/80/header.jpg` |

Both facts matter, and they are the two ways the rung was wrong:

1. **The key is usually not the app id asked for.** `620` answers under `323180`. The
   old code did `data.get(appid.to_string())`, so the rung returned `None` for most
   games — the whole rung was dead in production, which is why the chain below it
   looked like it "sometimes" worked.
2. **A matching key does not make the payload ours.** `100` answers under key `100`
   with `data.steam_appid` `80`, the name of app 80 and app 80's artwork. Trusting the
   key — or the name — would hand app 100 the bytes of app 80's cover and cache them
   under `appid_100`.

The payload's own label (`data.steam_appid`) and the artwork path therefore both
disagree with the requested id in the second shape, and both agree in the first. That
is what the rules below are built on.

Both artwork URLs behind these two shapes answer `200 image/jpeg` (measured with
`curl -I`: app 620's own header 41191 bytes, app 80's header 27592 bytes), so the wrong
cover was downloadable, not hypothetical. How visible each half of the defect was
differs: the key mismatch made the rung return nothing for most games, which is the
symptom that was noticed, while the wrong-art case needed a matching key *and* the CDN
rung above it to miss as well — for `appids=100` the CDN header for app 100 does exist
(29372 bytes), so that particular game was covered by the rung above. The guard is
what makes the wrong-art half impossible rather than merely unobserved.

## Rules

1. `appdetails_image_url(body, appid)` parses the response as untrusted JSON: the
   top-level value must be an object, and each entry's `success` must be reported
   (the same `reports_success` rule the store rung uses: a boolean `true`, or a
   non-zero integer that fits in an `i64` — `false`, `0`, a string and a missing field
   are all "not reported"). The entry's key is ignored on purpose: the store chooses
   it, and it is not the app id.
2. Identity is required twice, from two independent places, before either candidate
   field is used:
   - `data.steam_appid` must be an integer equal to the requested app id (the store's
     own label), and
   - the candidate URL's **path** must hold the requested app id as a whole segment
     directly after an `apps` segment (`…/apps/620/…`) — the path the CDN actually
     serves the bytes from, and the path that ends up as the cache stem `appid_<id>`.
   Requiring both is the point: the label says what the payload claims to be, the path
   says what will be downloaded, and the file is cached under the requested app id.
   Two sources that can disagree must agree.
3. A candidate URL must be an absolute URL whose scheme is `https`, compared
   case-insensitively (`HTTPS://…` from the store is still the store). Relative and
   protocol-relative (`//…`) URLs contribute nothing. (The shared downloader still
   repairs protocol-relative URLs for the Heroic and GOG sources; this rung does not
   accept them, because nothing in a live appdetails response is protocol-relative.)
4. The URL is read literally and never normalised. Its authority must be a plain host:
   non-empty, and only host, port or IPv6-literal characters — `https://bad host/…` and
   a userinfo part are not URLs this rung follows. Its path (what follows the authority,
   before any query or fragment) must consist of non-empty plain segments: ASCII
   alphanumerics plus `-`, `_` and `.`, with no `.` or `..` segment and no
   percent-encoding. The requested app id is then looked for in that path only, as a
   whole segment directly after an `apps` segment, so a host spelled `apps` cannot stand
   in for one.
   This is narrower than "looks like a URL" on purpose. Review round 1 falsified the
   earlier, purely textual rule with three inputs the tests had not covered:
   `https://apps/620/header.jpg`, where the authority was mistaken for a path segment;
   `https://bad host/apps/620/header.jpg`, which is not a URL yet was accepted while a
   valid `HTTPS://…` was refused; and `…/steam/apps/620/../80/header.jpg`, where an HTTP
   client normalises the path to app 80's file — app 80's bytes would have been cached
   as `appid_620.jpg`, which is the one outcome this whole change exists to prevent. A
   path that cannot be read literally is now a miss: it costs a cover, never
   correctness.
5. `header_image` is preferred, then `capsule_image`. A field that is missing, not a
   string, or empty contributes nothing, and the second field is tried.
6. A body larger than the shared store response bound (4 MiB) is not parsed. Every
   refusal is a miss: the rung returns nothing, `fetch_steam` falls through to the
   later sources, and nothing surfaces as an error.
7. The request URL stays what it is today
   (`https://store.steampowered.com/api/appdetails?appids=<id>&filters=basic`), now
   built by `appdetails_url(appid)` so the test suite can assert it.
8. The rung keeps its place in the source order: local Steam `librarycache` portrait →
   store `GetItems` portrait → CDN `header.jpg` → `appdetails` → Heroic/GOG/Xbox →
   EXE icon.

## What changes in the pipeline

`ImageCache::fetch_steam` no longer parses the response inline. It asks the injected
fetcher for `appdetails_url(appid)`, passes the body to `appdetails_image_url`, and
caches the accepted URL under `appid_<id>` through the existing `download_and_cache`
(bounded decode). The explicit `within_response_bound` check that used to sit in
`fetch_steam` is now inside the parser, where it also covers a caller that is not this
rung.

## Live verification (manual, for a reviewer)

There is deliberately no live test in the suite: the crate's HTTP client is not a dev
dependency, and adding one to fetch two small JSON bodies is not worth a new
dependency. The live check is two commands and two readings:

```
curl -s "https://store.steampowered.com/api/appdetails?appids=620&filters=basic" \
  | python -c "import json,sys;d=json.load(sys.stdin);k=list(d)[0];print('key',k,'steam_appid',d[k]['data']['steam_appid'],'header',d[k]['data']['header_image'])"
# expected: key 323180, steam_appid 620, header path /steam/apps/620/…  -> the parser must accept it

curl -s "https://store.steampowered.com/api/appdetails?appids=100&filters=basic" \
  | python -c "import json,sys;d=json.load(sys.stdin);k=list(d)[0];print('key',k,'steam_appid',d[k]['data']['steam_appid'],'header',d[k]['data']['header_image'])"
# expected: key 100, steam_appid 80, header path /steam/apps/80/…  -> the parser must refuse it
```

The second command is the one that used to produce a wrong cover. What the suite pins
offline is exactly these two bodies, captured above and committed as fixtures, so the
rules cannot drift away from the shapes that were measured.

## Known limits, recorded rather than hidden

- Identity now depends on `data.steam_appid` being present and an integer. Both
  captured shapes have it. A response that omits it is a miss and falls through to the
  next source; that is deliberate — an id we cannot prove is not an id we may cache
  under.
- The cache stem is still `appid_<id>` for every Steam source, so this rung's failure
  mode is a missing cover, never a wrong one. Per-source cache keys remain T10's job.
- Only the two shapes above were measured. `appids=730`, `220`, `440`, `570`, `400` and
  `70` are recorded in the store-portrait spec as keyed by other ids; they were not
  re-captured for this change.
