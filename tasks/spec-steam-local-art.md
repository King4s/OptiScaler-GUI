# Specification: local Steam portrait art (T3)

Scope: use the portrait art Steam has already downloaded for the exact app id,
from disk, before any network source. Out of scope here: Steam's `GetItems` API
(T4), provenance-aware cache keys, and re-deciding an image that a previous
version already cached.

## Evidence this is built on

Observed on a real Steam install (`C:\Program Files (x86)\Steam`, 2026-09-26;
485 entries in `appcache/librarycache`):

| File | Where | Count |
| --- | --- | --- |
| `library_600x900.jpg` | `<librarycache>/<appid>/` | 133 |
| `header.jpg` | `<librarycache>/<appid>/` | 225 |
| `library_hero.jpg` | `<librarycache>/<appid>/` | 123 |
| `logo.png` | `<librarycache>/<appid>/` | 108 |
| `library_600x900.jpg` | `<librarycache>/<appid>/<40-hex>/` | 18 |
| `<40-hex>.jpg` | `<librarycache>/<appid>/` | content-addressed |
| flat `<appid>_<name>.jpg` | - | 0, that layout is retired |

Largest portrait in that cache: 575 KB. Smallest: 14 KB. No `library_600x900_2x.jpg`
exists there, so it is not a candidate.

Behavioural references only, no code reused: `beeradmoore/dlss-swapper` and
`SpecialKO/SKIF`. The plan this task comes from lives on the
`codex/game-identity-contracts` branch (PR #33) as `tasks/plan.md`; this branch
carries only this specification. The layout above is the primary evidence; those
projects are not authoritative for it.

## Rules

1. One library cache (a Steam install's `appcache/librarycache`) and one app id
   resolve to, in order: `<appid>/library_600x900.jpg`, then exactly one
   `<appid>/<entry>/library_600x900.jpg` where `<entry>` is a directory directly
   inside the app's own directory. Zero or two or more nested matches: none.
2. Nothing deeper than that is considered, and the cache as a whole is never
   walked. Cost is one directory listing of the app's own directory per install
   root, plus a stat for the direct path.
3. Only the portrait file name. `header.jpg`, `library_hero.jpg` and `logo.png`
   are landscape or logo art; returning one of those as a portrait would be a
   guess, and the CDN/API fallbacks already cover that job.
4. Every Steam install root on the machine is searched, because `appcache` lives
   per installation rather than per library folder.
5. Read bound: a file over 4 MB is refused (7x the largest observed portrait),
   and decoding carries an explicit pixel limit (4096x4096, 64 MB allocation),
   so a corrupt or hostile file cannot exhaust memory. A refused candidate is a
   miss, not an error.
6. A miss leaves the existing pipeline unchanged: cached image, then Steam CDN
   header, Store `appdetails`, store art URL, GOG search, Xbox local logo, exe
   icon.
