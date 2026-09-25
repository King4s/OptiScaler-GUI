# Plan: Game discovery and artwork after 2026.9.0-beta.1

Status: proposed, not implemented. Scope: the Windows Rust Standard edition.

## Goal and constraints

Find more installed games and show reliable covers without weakening the existing safe-install flow. Keep all hardware and library data local by default. No DLSS 5/AMD-NR edition, bulk installation, automatic anti-cheat changes, telemetry, or silent installation into uncertain folders.

Reference projects may be cloned to study documented formats, observable behavior, and test cases. Do not copy, translate line-by-line, or adapt their source code, including MIT/Apache code. Write a short independent specification first, implement from that specification and official format/API documentation, and retain source links in the PR. GPL/AGPL projects are research only. Do not add a reference project as a dependency without a separate license and maintenance review.

## Baseline and design decisions

- Start each implementation PR from current `origin/main`; the clean `codex/2026-9-beta` worktree is a planning reference, not necessarily the latest branch. The root checkout is an older dirty Python tree. Preserve all user changes.
- `opticore::scan` already handles Steam, Epic, GOG, Heroic, Xbox and manual folders. `ScanConfig` currently only has excluded drives. `Game` stores `steam_appid` but no general store identifier. Add stable store identity without breaking existing config or caches.
- `ImageCache` already has fixed Steam `header.jpg` then Store `appdetails` fallback, Heroic/GOG/Xbox sources, and EXE icon fallback. The actual gap is local Steam `librarycache`, portrait assets from `IStoreBrowseService/GetItems`, provenance, and non-Steam cover matching.
- Scanner output is discovery evidence, never permission to install. The common EXE resolver and user-choice requirement remain authoritative. Unknown/ambiguous games remain selectable but not auto-installable.
- Prefer local metadata and offline behavior. SteamGridDB is optional, disabled by default, and requires the user's own key plus clear notice that game identity is sent to that service. Do not embed an IGDB client secret.
- Keep artwork and store metadata separate from compatibility evidence. A cover or DLL name never marks an upscaler as active or the game as compatible.

## Implementation order

### Phase 0: Baseline and contracts

1. Confirm current `origin/main`, CI, beta release state, and existing tests; make a clean `codex/` branch. Record current scanner output and cover behavior for fixture games. No migration at startup.
2. Specify `StoreIdentity { platform, store_id }`, discovery provenance/confidence, and `ArtCandidate { source, identifier, kind }` before editing scanners. Preserve legacy `steam_appid` and name-keyed cache reads. Decide identity and dedup rules for same-name games, multiple stores and moved installs.
3. Add deterministic fixture tests for duplicate names, same store ID in two paths, missing metadata, corrupt manifests and out-of-root paths. Fix the present name-plus-platform dedup only after the expected behavior is specified.

Checkpoint: `cargo test --workspace --locked` and existing resolver/transaction safety tests pass; no installer or manifest behavior changes.

### Phase 1: Steam artwork, local first

4. Locate Steam's actual library cache per discovered Steam installation. Accept only the exact app ID's known portrait files; avoid recursive disk-wide image scans. Cache a decoded, size-bounded portrait in the existing app cache, with provenance.
5. Add a small, independently implemented Store `GetItems` asset parser for `library_capsule_2x`/`library_capsule`. Validate the response and HTTPS asset URL; retain current `appdetails` and fixed-header fallbacks. Keep network I/O on a worker, bounded by timeout/size and cancellable where practical.
6. Preserve old cached images on upgrade; use atomic cache writes and distinguish portrait from landscape so a cached header does not permanently block a better portrait. Add fixture-based API parsing, offline, 404, invalid-image and wrong-app-ID tests.

Checkpoint: Steam cover appears from local cache offline; new Steam games can use hosted portrait art; failure still shows existing fallback; no UI stalls.

### Phase 2: Safer, broader discovery

7. Add persisted user-selected scan roots and per-root on/off controls. Bound traversal depth and work; reject or flag system roots, symlink escapes and paths outside a chosen root. Show scan source and allow rescan/remove. Keep manual EXE addition intact.
8. Add one adapter per PR, in this order unless fixtures change the cost/benefit: Amazon (local install SQLite), Ubisoft (registry/manifests), EA (installed-game metadata), Battle.net (local product database plus explicit unknown-name handling). Confirm current formats from original launcher files or official documentation; third-party repos provide leads, not authority. Each adapter returns a stable store ID, install root and confidence, not a guessed game EXE.
9. Make per-adapter failures non-fatal and visible. Deduplicate by verified store identity and normalized install path, preserving separately installed copies. Do not infer identity from title alone. Limit scans and cache unchanged library metadata where measurable.

Checkpoint after each adapter: synthetic fixtures for installed, moved, uninstalled, malformed, permission-denied, multiple installs and launcher-only entries; existing store counts unchanged. No unsafe auto-install target appears.

### Phase 3: Covers beyond Steam

10. Use store-supplied/local artwork first for Heroic, Epic, GOG, Xbox and new stores when a stable ID or exact local metadata exists. Introduce provenance-aware cache keys so same-name games from different stores cannot overwrite one another. Preserve and gradually migrate legacy cache reads without deleting user art.
11. Add optional SteamGridDB lookup with a user-supplied key, explicit network/privacy UI, conservative rate limiting, image-size limits and local cache. Prefer exact external/store IDs; title search results require user confirmation before becoming a persistent match. Allow correction, clearing and per-game artwork override. Do not add IGDB in this version.
12. Show cover origin and useful fallback states in grid/list UI. Do not let art fetching block scanning, game selection or installation.

Checkpoint: offline launch works; no credentials or game names leave the machine unless the user enables the provider; same-name fixtures keep distinct covers; the UI works in Danish, English and Polish.

### Phase 4: Verification and release

13. Add focused docs for supported stores, source precedence, SteamGridDB opt-in, troubleshooting and privacy. Update README and changelog only for shipped behavior. Include precise source links and no copied source snippets.
14. Run Windows CI: `cargo fmt --all --check`, `cargo clippy --all-targets --workspace --locked -- -D warnings`, `cargo test --workspace --locked`, release build, binary-size check and GUI startup smoke. Verify packaged EXE and 7z. Manual test at least one real installed game per available store; mark untested stores as unverified, not supported by assertion alone.
15. Release beta first. Promote to stable only after installer/update/uninstall rollback checks, cover/privacy checks and manual game verification. Publishing GitHub releases is a separate explicit step.

## Risks and decisions

- Third-party launcher formats can change without notice. Isolate adapters, fail softly, and do not guess when metadata is missing.
- Local Steam art and Store APIs are best-effort sources, not guaranteed contracts. Preserve icon/legacy fallbacks.
- Online image search can return wrong games or inappropriate art. Require confirmation for fuzzy matches and provide local override.
- New IDs and cache keys affect saved data. Read old settings/caches; never delete or silently rewrite them on startup.
- Keep PRs small: contract/tests, Steam local art, Steam API, custom roots, then one store per PR, then optional online art and UI.

## Reference map

- Steam artwork: https://github.com/beeradmoore/dlss-swapper and https://github.com/SpecialKO/SKIF (behavioral references only).
- Store discovery: https://github.com/PhilipK/BoilR, https://github.com/JosefNemec/Playnite, https://github.com/tekgator/GameLib.NET, https://github.com/EqualGames/game-scanner (archived), https://github.com/Optiscaler-Client/Optiscaler-Client.
- Art provider: https://www.steamgriddb.com/api/v2. IGDB https://api-docs.igdb.com/ is deferred because its documented client-secret flow is unsuitable for an embedded desktop credential.
- The remaining projects from the research are comparison/background material, not implementation dependencies: https://github.com/Recol/DLSS-Updater, https://github.com/Wimukthi/OptiScalerInstaller, https://github.com/onehoon/OptiClick, https://github.com/Spexxl/OptiTux, https://github.com/Skynizz/Prism, https://github.com/xXJSONDeruloXx/Decky-Framegen, https://github.com/SteamGridDB/steam-rom-manager, https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher, https://github.com/lutris/lutris, https://github.com/kra-mo/cartridges, https://github.com/mmatyas/pegasus-frontend, https://github.com/ShadowBlip/OpenGamepadUI, https://github.com/legendary-gl/legendary, https://github.com/RareDevs/Rare, https://github.com/sharkwouter/minigalaxy, https://github.com/tkashkin/GameHub (archived), https://github.com/WilliamVenner/steamlocate-rs, https://github.com/Rolv-Apneseth/lib_game_detector, https://codeberg.org/CosmicHarper/vdf-rs, https://github.com/erri120/GameFinder, https://github.com/ValvePython/vdf, https://github.com/gogcom/galaxy-integrations-python-api.
