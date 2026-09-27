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

| asked for | response key | `success` | `data.steam_appid` | `data.name` | `data.header_image` (full path, as observed) |
|---|---|---|---|---|---|
| `appids=620` | `323180` | `true` | `620` | Portal 2 | `/store_item_assets/steam/apps/620/faffc0f560786e2f05104a8d2fac837c6969bf13/header.jpg?t=1790187113` |
| `appids=100` | `100` | `true` | `80` | Counter-Strike: Condition Zero | `/store_item_assets/steam/apps/80/header.jpg?t=1745368574` |

(`/store_item_assets` is the prefix the store serves today; an earlier draft of this table
showed the path from `/steam/apps/…` on and was corrected after a reviewer compared the
column against the committed fixtures.)

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
4. The URL is read literally and never normalised. Its authority must be a host name —
   dotted labels of letters, digits and hyphens, none empty and none hyphen-edged — with
   an optional port that is a number in 0–65535 written as digits, and nothing else:
   `https://bad host/…`, a userinfo part, an underscore, brackets, an IPv6 literal,
   `https://:/…` with no host, `…:abc` with a port that is not a number, `…:+443` and
   `…:65536` with a port no URL carries are all refused. A host of nothing but digits and
   dots is an address literal — the store serves its artwork from names — and is refused
   rather than half-validated as an IPv4 address. Every byte of the whole URI, query and
   fragment included, must be one a URI may contain: RFC 3986's unreserved and reserved
   characters plus `%`. A space, a control byte, a non-ASCII byte, and equally `<`, `>`,
   `"`, a backslash, `^`, a backtick, `{`, `|` or `}` is refused. For a space or a `<` the
   downloader's own parser refuses such a request as well — measured, not assumed — while for
   `|`, a backtick and braces this rule is *narrower* than that parser, which accepts them:
   those refusals carry a cost, and they are kept only because a URL this code cannot read
   literally is a miss rather than a guess. Its path (what
   follows the authority, before any query or fragment) must consist of non-empty plain
   segments: ASCII alphanumerics plus `-`, `_` and `.`, with no `.` or `..` segment. A
   segment may carry `%` escapes, and each is judged by what it decodes to. Two are refused
   outright, because they change how the path is cut up or how often it is decoded: `%2f`
   and `%5c` are separators — a server that decodes before it splits would see different
   segments than the ones checked — and `%25` is a percent sign, which lets the path be
   decoded a second time by someone else. A dot is judged by the segment it forms: `%2e`,
   `%2e%2e` and `.%2e` are refused when the decoded segment *is* `.` or `..`, the shape a
   client normalises away, while `header%2Ejpg` and `%68eader.jpg` decode to ordinary file
   names and are accepted — a reviewer fetched the same 41,191 bytes for both.
   Two further bounds apply to escapes, and they apply in different places on purpose. Every
   `%` in the URI, query and fragment included, must introduce two hex digits, because RFC
   3986 says a percent sign does (`pct-encoded = "%" HEXDIG HEXDIG`) and a URL that is not a
   URI is one whose meaning depends on who reads it. That bound is **strict-only**: measured
   2026-09-27, `http::Uri::try_from` returns `Ok` for `?x=%zz`, `?x=%` and `?x=%2`, and a
   request through the downloader's own agent config to `…?t=1790187113%zz` answered `200`
   with the same 41,191 bytes as the clean URL. So it refuses URLs that would work; the price
   is a cover in the hypothetical where the store emits a malformed escape, and the measured
   cost is zero, because no artwork candidate in any sample here (52, 38, 188, 192, 196, 198
   and 358 candidates, in different samples) carries an escape at all. The second bound — an
   escape may only spell a byte a file name can hold, printable ASCII or a space — applies to
   **path segments only**. The query is not held to it (`?x=%0a`, `?x=%c0%ae%c0%ae`, `?x=%2f`
   are accepted), because the query cannot change which file the path resolves to: the app id
   is read from the path and the host is pinned. In a path, `%c0%ae%c0%ae` is `..` to a
   decoder that accepts overlong UTF-8, which would resolve the path away from this app's
   file; the live CDN answers `404` for such a path instead of decoding it, so the hazard is
   not one today's server shows — and the path is refused anyway, because a path this code
   cannot read literally is a miss, never a guess. A truncated or non-hex escape is refused
   wherever it appears, for the grammar's reason above. The requested app id is then looked
   for in that path only, as a whole segment directly after an `apps` segment, compared
   literally — path segments are case-sensitive, unlike the host and the scheme — so neither a
   host spelled `apps` nor a segment spelled `APPS` can stand in for one.
   This is narrower than "looks like a URL" on purpose, and ten review rounds are why.
   Round 1 falsified the first, purely textual rule with three inputs the tests had not
   covered: `https://apps/620/header.jpg`, where the authority was mistaken for a path
   segment; `https://bad host/apps/620/header.jpg`, which is not a URL yet was accepted
   while a valid `HTTPS://…` was refused; and `…/steam/apps/620/../80/header.jpg`, where an
   HTTP client normalises the path to app 80's file — app 80's bytes would have been cached
   as `appid_620.jpg`, which is the one outcome this change exists to prevent. Round 2 then
   falsified the fix's authority check as a character whitelist rather than a host check:
   `https://:/apps/620/header.jpg` (no host at all),
   `https://[invalid]/apps/620/header.jpg` and `…steamstatic.com:abc/apps/620/header.jpg`
   were all still accepted. Round 3 found two more, one of them a regression this change
   had introduced: a port above 65535 was accepted although no URL carries one, and a
   header whose bytes cannot be downloaded no longer fell back to the capsule beside it.
   Both are fixed and pinned; the second is why the parser returns an ordered list of
   candidates instead of one URL. Round 4 found that the query was discarded before
   validation — a NUL in `?x=…` was accepted although the downloader refuses that URI — and
   that `https://999.999.999.999/…` passed as a host. Both are fixed, which is what makes
   the whole-URI and address-literal rules above part of the specification rather than
   incidental. Round 5 found that "printable ASCII" is broader than "a URI character" — a
   `<` in the query was accepted although the downloader refuses that request — and that
   the agent followed redirects, so a 302 to another app's file was cached and returned as
   this app's cover. The first is why the rule above names a character set rather than a
   byte range; the second is rule 8. Round 8 falsified the escape ban as blunt rather than
   precise: it refused the live 620 artwork with its file name written `%68eader.jpg`,
   which the downloader fetches as the same 41,191 bytes. Round 9 then falsified the
   narrower version the same way — it still refused `header%2Ejpg`, where the escape only
   ever spells a dot *inside* a file name — so the rule now refuses a dot escape only when
   the decoded segment is itself `.` or `..`. A URL that cannot be read literally is a miss:
   it costs a cover, never correctness, but a rule that refuses a URL the store actually
   served is a rule with a price, and two rounds were spent measuring that price. Round 10
   closed the two escape bounds the reviewer who accepted `74761c0d` named as residual — a
   malformed `%` anywhere in the URI (`?x=%zz`, a trailing `%`) and an escape spelling a byte
   a file name cannot hold (`%c0%ae%c0%ae`, an overlong `.`) — and then falsified the sentence
   written for the first of them: both the comment and this spec claimed the downloader
   refuses such requests, and it does not. `http::Uri::try_from` returns `Ok` for all three,
   and a live request for `…?t=1790187113%zz` answered `200` with the same bytes as the clean
   URL. The bound stays, with the reason it actually has — the RFC's grammar — and with its
   cost stated rather than denied: it refuses URLs that work, at a measured cost of zero live
   candidates. The same round showed this code is *narrower* than the downloader in three more
   places (`|`, a backtick, braces), which the URI-character paragraph above now says.
5. The candidate URL's host must be the store's own: `steamstatic.com` itself or a subdomain
   of it, compared **case-insensitively** — host names are, exactly as the scheme is, and a
   reviewer falsified a case-sensitive version by uppercasing the host in the 620 fixture,
   from which the downloader fetched the identical 41,191-byte image. The path rule says
   which app id a URL claims to serve and the scheme rule says how, but neither says *who*
   serves it: round 6 used `https://evil.example/steam/apps/620/header.jpg`, whose path is
   shaped perfectly, and a later round an open image proxy,
   `https://wsrv.nl/apps/620/header.jpg?url=…`, whose path names app 620 while the URL in
   its query serves app 80's artwork — measured here as `200 image/jpeg`, 20,694 bytes, and
   the image is Counter-Strike: Condition Zero (app 80), against app 80's own header at
   27,592 bytes. Without the pin, either host's bytes would be written as this game's cover.
   What the pin costs today: nothing that was measured. Every appdetails artwork URL seen so
   far is on `shared.akamai.steamstatic.com`: 52 distinct URLs from my own 51-body sample and
   38 more from a second 20-body sample, plus 188, 192, 196 and 198 distinct URLs in the
   independent reviewers' samples of 274, 100, 100 and 100 bodies — none on any other host —
   while the URL the CDN rung builds is on `cdn.akamai.steamstatic.com`. Lookalikes
   fail whatever their case: `steamstatic.com.evil.example`, `notsteamstatic.com`,
   `Steamstatic.com.Evil.Example`, a host with a trailing dot. This is the rule that turns
   "the artwork of this app id" from a statement about a path into a statement about the
   store.
6. `header_image` comes before `capsule_image`, and the usable fields are returned as an
   ordered list: a field that is missing, not a string, or unusable contributes nothing and
   the next one is tried. The caller tries them in that order as well, so a header whose
   download fails still leaves the capsule beside it — the fallback the rung already had.
7. A body larger than the shared store response bound (4 MiB) is not parsed. Every
   refusal is a miss: the rung returns nothing, `fetch_steam` falls through to the
   later sources, and nothing surfaces as an error. The bound is pinned by two valid
   bodies, one of exactly 4 MiB (parsed, its URL returned) and one of 4 MiB and a byte
   (refused): padding a body with spaces, as the first version of that test did, proves
   nothing, because that is not JSON with or without a bound — a reviewer caught the
   test claiming more than it showed.
8. The downloader follows no redirects. A redirect is the one route to foreign bytes that
   this code cannot see — the URL that was checked and the file that arrives would differ —
   so a URL that answers with a redirect is a miss: the shared agent is built with
   `max_redirects(0)` and `http_get` accepts only `200`. It is not the only route overall: a
   foreign host whose path lies about the app id is the other one, and rule 5 is what closes
   it — an open image proxy was measured doing exactly that (`200 image/jpeg`, app 80's
   artwork under a path naming app 620).
   What was measured on 2026-09-27 is narrower than "no artwork URL redirects". On the Steam
   side: the store API and the GetItems API answered `200` directly; every candidate URL in
   my own swatches of live responses — 52 distinct header/capsule URLs out of 51 bodies, 26
   of which reported success, plus 38 distinct out of a second 20-body sample — answered
   `200` with no redirect; and the reviewers' samples of 100, 100 and 274 live bodies each
   found artwork hosts on `shared.akamai.steamstatic.com` only. An earlier draft of this
   rule credited 84 URLs to 26 responses, which is arithmetically impossible (26 responses
   carry at most 52 candidates); the 84 was my own corpus pooled with a reviewer's and is
   now counted per sample, as a reviewer pointed out.
   Not measured: URLs the *GetItems* rung builds — an independent sample found two `404`s
   there (the portrait URLs for apps 570 and 220, whose `cdn.akamai.steamstatic.com`
   fallback answers `200`, so the chain still finds a cover) — and the Heroic, GOG and Xbox
   sources, whose URLs this rule also governs through the shared agent. A reviewer could not
   measure those either (GOG's public endpoint returned no products that day), so "costs
   nothing today" is verified on the Steam side only; for the other sources the honest
   statement is that a redirect would now cost a cover instead of risking a wrong one. The
   in-tree pin is the agent's own configuration, asserted by
   `the_artwork_downloader_follows_no_redirects`; the behaviour behind it — a `200` URL is
   cached and the same file behind a `302` is a miss, with the origin seeing only the
   first request — was verified against a loopback server outside this suite, twice and
   independently, because a test that opens a socket is not one this suite keeps. This is
   the one rule here that reaches beyond the rung, because the agent and the fetcher are
   shared by every source in `ImageCache`. It is recorded as a deliberate widening rather
   than hidden: the fetcher seam hands back bytes with no notion of where they came from, so
   a redirect cannot be judged by the rung that asked for the bytes.
9. The request URL stays what it is today
   (`https://store.steampowered.com/api/appdetails?appids=<id>&filters=basic`), now
   built by `appdetails_url(appid)` so the test suite can assert it.
10. The rung keeps its place in the source order: local Steam `librarycache` portrait →
    store `GetItems` portrait → CDN `header.jpg` → `appdetails` → Heroic/GOG/Xbox →
    EXE icon.

## Not this change (limits, deliberately left alone)

Two things reviewers found are real, and are *not* fixed here, because each belongs to
another change with its own spec:

- `ImageCache::fetch` also has an `art_url` source that caches a caller-supplied URL under a
  name-derived stem without either identity check: a caller passing the name `appid_620`
  together with app 80's art URL writes `appid_620.jpg`. That predates this change and is
  the Heroic/GOG source's contract with local launcher manifests — a caller inside this
  program, not a network response — and the appdetails rung itself checks identity twice.
- The GetItems portrait rung (`store_item_portrait_url`) rests on a single identity source,
  the item's own `appid`, while building its URL from `asset_url_format`, so a GetItems body
  could name another app's asset path. That is that rung's own change and its own spec;
  folding it in here would put two unrelated changes in one diff.

## What changes in the pipeline

`ImageCache::fetch_steam` no longer parses the response inline. It asks the injected
fetcher for `appdetails_url(appid)`, passes the body to `appdetails_image_urls`, and
tries the returned candidates in order, caching the first one that downloads under
`appid_<id>` through the existing `download_and_cache` (bounded decode). When none
downloads, the rung returns nothing and the chain continues. `appdetails_image_url` is
the single-candidate view of the same rules (header first, capsule as the fallback) for
callers and tests that want one answer. The explicit `within_response_bound` check that
used to sit in `fetch_steam` is now inside the parser, where it also covers a caller that
is not this rung.

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
