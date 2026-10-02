# Game discovery and artwork TODO

`2026.9.0-beta.2` is an unpublished local candidate on `codex/2026-9-beta2`. PRs #33, #34 and #35 are independently green but not merged. The appdetails-art guard is included in the candidate. See `tasks/handoff.md` and `docs/releases/2026.9.0-beta.2.md` for behavior and release limits.

## Included in the local candidate

- [x] T1/T2: store identity and discovery/title provenance; path-based dedup that merges repeated hits for one install while preserving available identity. Identity and title alone are not dedup keys.
- [x] T2b: central path-key normalization of case, separator spelling and trailing separator, without lexical `.`/`..` folding. Existing GPU choices have an exact prior-key read fallback; no rewrite or general path migration.
- [x] T3: exact-app-ID Steam local portrait lookup, after existing cached images.
- [x] T4: Steam GetItems portrait lookup with existing header/appdetails fallback, bounded and injectable fetches.
- [x] Appdetails-art guard: payload app ID and image URL path must both match the requested ID. This is included, not deferred.

## Release candidate validation still pending

- [x] Combined formatting, Clippy, workspace tests, release build and package startup smoke passed. See `docs/releases/2026.9.0-beta.2-verification.md`; independent review and remote candidate CI remain separate gates.
- [ ] Manually verify Fatekeeper with exact hardware, GUI/game/OptiScaler versions, overlay behavior and real-game install/update/uninstall. The earlier user report is not this evidence.
- [ ] Test packaged-app install/update/rollback/uninstall and legacy-install safety on supported Windows systems; record representative game layouts and any exceptions.
- [ ] Review license, privacy, image URL handling, scan bounds, install safety and DA/EN/PL UI before any publication decision. Do not promote beta.2 to stable based on synthetic tests.

## Future work, not beta.2 claims

- [ ] T5: persist user scan roots and opt-out settings; show scan provenance; bound traversal and reject out-of-root paths.
- [ ] T6-T9: one verified-format store adapter per PR for Amazon, Ubisoft, EA and Battle.net. Do not guess executables or implement against unverified formats.
- [ ] T10: per-store artwork provenance and collision-free cache keys while preserving existing cached images. Current cache precedence can keep a landscape image even when a portrait is available.
- [ ] T11/T12: optional user-approved SteamGridDB and cover-source/override UI with privacy, offline and DA/EN/PL checks.
- [ ] Revisit path-key read compatibility if a store changes the spelling of a persisted install path; the current old-key fallback is exact. Do not lexically fold `..` as a shortcut.

For every future task, record fixture and manual evidence separately, keep installer safety gates, and review the diff. Foreign repositories may be studied for formats and behavior, but their source code must not be copied, pasted, vendored or translated line-by-line.
