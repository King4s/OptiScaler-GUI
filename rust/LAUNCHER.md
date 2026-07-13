# Launcher branch — Heroic-class features (own Rust implementation)

Status of the `launcher` branch (not merged to main; no releases are built
from it). Everything below is implemented natively in this codebase — no
code copied from Heroic, legendary, or gogdl; store protocols are
implemented from their public behavior.

## What's in

### Library & launcher core (L0–L1)
- `cache/library.json`: per-game favorites, hidden flag, playtime,
  last-played, launch options. All-default entries are pruned on save.
- Favorite ⭐ (cards + detail + game page), favorites always sort first.
- Hide/unhide games, "Show hidden (N)" filter.
- Playtime tracking: after a launch, a watcher polls the process list
  (direct Toolhelp32/kernel32 externs, sub-millisecond sweep every 15 s,
  no extra crates) until the game's processes are gone, then credits the
  minutes. Handles Steam-client indirection; 24 h runaway guard.
- Sort by last played / playtime.

### Game page (L2)
- Hero artwork, title, genre badges, facts grid (developer, publisher,
  release date, size on disk, playtime, last played, OptiScaler state).
- Description + store link from Steam appdetails / GOG products, cached
  in `cache/metadata/` with a 7-day TTL.
- Installed size from one background folder walk per session.

### Launch options (L3)
- Per-game extra arguments (quote-aware split), exe override (missing
  file is an error, not a silent fallback), env vars (KEY=VALUE lines).
- Steam launches pass args via `steam://run/<id>//<args>/`.

### Stores (L4–L5)
- **GOG**: browser OAuth code login (public embed client), token
  refresh, owned-games listing, in-app installs: offline installer
  downloaded (streaming, resumable, MD5-verified against GOG's checksum
  XML) and run silently (InnoSetup flags) into a user-chosen folder.
- **Epic**: authorization-code login, token refresh, owned-games
  listing (Windows assets + catalog titles), installs hand off to the
  Epic launcher protocol.
- Installed Epic/Heroic games with an `.egstore` AppName now launch via
  `com.epicgames.launcher://` (EOS auth, cloud saves) instead of the
  raw exe.

## Security / privacy notes
- Store login happens in the user's browser; the app never sees the
  password — only the OAuth code the user pastes back.
- Tokens are stored in `cache/store_auth.json` (plain file, same model
  as other launchers) and are never written to logs. Logout deletes them.
- Downloads only ever cover games the signed-in account owns; no DRM is
  circumvented (GOG offline installers are DRM-free by design; Epic
  installs go through Epic's own launcher).

## Known gaps / future work
- **Epic native downloads**: Epic's chunked binary manifest CDN format
  is not implemented; installs open the Epic launcher instead. The
  stores module is structured so a `stores::epic::download` engine can
  slot in later.
- GOG installers decide some things themselves (registry entries, DLC
  selection); we pass `/DIR` and silent flags only.
- Store libraries are text lists (no cover art grid yet).
- Playtime is credited in whole minutes at session end; sessions
  shorter than a minute are dropped.
- `cache/store_auth.json` is not OS-encrypted (DPAPI would be a nice
  upgrade).

## Verification
- 99 unit tests green (`cargo test --workspace`), clippy `-D warnings`
  clean, `cargo fmt --check` clean.
- Store protocol parsers are covered by canned-response tests; network
  paths are thin wrappers around the tested parsers.
- Real-account end-to-end testing (GOG login → install; Epic login →
  library) requires the user's accounts and is the next manual step.
