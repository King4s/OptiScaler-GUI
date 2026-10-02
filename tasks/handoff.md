# Beta.2 candidate handoff: game discovery and artwork

Use branch `codex/2026-9-beta2` for this release candidate. It combines PR #33 identity changes with the appdetails-art guard, including PRs #34 and #35. The original three PRs were independently green but are not merged into main. `2026.9.0-beta.1` is a public prerelease; `2026.9.0-beta.2` is not published. Preserve unrelated worktrees and user changes.

## What the candidate does

- `scan::dedup_games` keys on normalized install path. Hits for one install merge; same-title installs at different paths remain separate. A better title wins, while missing store identity, artwork and Steam app ID are filled from the other hit. Store identity is provenance, not the dedup key, a compatibility claim or installation authority.
- `TitleSource` ranks title evidence separately from `DiscoverySource`. Store metadata can supply a better title than a folder name even when both hits describe one install.
- `GameKey::path_key` folds case, slash direction and trailing separators. It does not lexically collapse `.` or `..` or resolve links. Persisted GPU selections try the new key then the prior spelling; there is no settings rewrite. The fallback is exact, so a store changing its own path spelling between versions can still orphan a setting.
- Image lookup first returns an existing cached image, then tries the exact-app-ID Steam local portrait, Steam GetItems portrait, CDN header, appdetails, and existing non-Steam/icon fallbacks. A cached landscape image still wins. There is no cache migration, no new store scanners and no SteamGridDB integration.
- The appdetails guard ignores response object keys but requires both `data.steam_appid` and the image URL's `apps/<appid>` path to match the requested app. It rejects malformed or mismatched URLs and can try a capsule when a header is unusable. This is in the local candidate, not an open follow-up.

## Evidence and pending work

- Final combined formatting, Clippy and workspace tests passed through Jev MCP run `20261002-161555`, turn 1. The release build, 7z integrity/extraction, SHA256 comparison and eight-second startup in da/en/pl passed on 2026-10-02. See [verification](../docs/releases/2026.9.0-beta.2-verification.md). Independent review and remote candidate CI are tracked separately from these local results.
- Fatekeeper was reported working by the user on 2026-09-25, but this run did not verify exact hardware, GUI/game/OptiScaler versions, overlay result or real-game install/update/uninstall. Packaged-app install/update/rollback/uninstall, legacy-install safety and representative game layouts also need hands-on validation before broader claims.
- Preserve the existing safety model: explicit EXE choice when ambiguous, reviewed install target, v2 ownership and rollback, conservative handling of legacy/manifest-less installs, no inferred compatibility from images or DLL names, and no anti-cheat modification.

## Follow-up boundaries

T5 onward in `tasks/todo.md` remains future work, not beta.2 functionality. Existing image cache precedence and per-name cache collisions are separate future work; do not imply source-specific cache keys or automatic deletion are present. A store path spelling change beyond the exact legacy-key fallback also remains open. Keep new tests and release evidence labeled by the tree and artifact actually checked.

The user permits studying foreign reference repositories but forbids copying their source code, including line-by-line translations and pasted snippets. Record observed formats, public API contracts and expected behavior independently, then write original code. Do not vendor foreign files or infer a license exception from a permissive license.
