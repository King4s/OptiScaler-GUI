# 2026.9.0 completion checklist

The full discovery/artwork implementation is complete. Stable publication is explicitly authorized after green final checks. Publication remains pending until the GitHub release exists.

## Implemented scope

These checkmarks record implementation, not manual verification.

- [x] T1/T2: identity/title provenance and path-based deduplication; Windows verbatim-path review correction tracked below.
- [x] T3/T4: exact-ID Steam local/GetItems portraits and guarded header/appdetails fallback.
- [x] T5: persisted custom roots with enable/disable/remove/rescan and bounded traversal.
- [x] T6-T9: Amazon SQLite, Ubisoft registry, EA registry plus installerdata XML, Battle.net product.db.
- [x] T10: per-install hashed cover cache, legacy preservation, portrait upgrades and aspect-preserving cards.
- [x] T11/T12: local override/reset/refresh/source controls and consent-gated SteamGridDB with session-only key, explicit Steam ID or title lookup, and user game/image selection.

## Release checklist

- [x] Windows verbatim-path deduplication fix complete; targeted regressions passed.
- [x] Last-good-art refresh fix complete; targeted regressions passed.
- [x] Final Jev turn 4 formatting, Clippy and locked workspace tests PASS.
- [x] Final EXE/7z built; sizes, hashes, archive integrity and eight-second da/en/pl startup PASS.
- [x] Exact final build evidence recorded in the [verification record](../docs/releases/2026.9.0-verification.md); manual/authenticated limits remain explicit.
- [x] Independent final review: `done=true`, no missing items. Jev stopped with `goal_met` after six executor turns.
- [ ] Record green remote CI.
- [ ] Publish stable 2026.9.0 once green under the existing authorization and record the GitHub release URL.

## Recorded evidence

Final source tree: `26d8042f463393ac845e1c0f0f11957de30319ae`. Verified EXE: 8,933,888 bytes; 7z: 3,324,820 bytes. Subsequent reconciliation changes are documentation-only. GitHub-built binaries will have their own `SHA256SUMS`. Audit reported zero vulnerabilities with the existing unmaintained `ttf-parser` warning.

Read-only store smoke found two Ubisoft games, zero Amazon/EA/Battle.net games and three warnings. The zero-result stores have fixture coverage, not positive live discovery evidence. No manual visual/gameplay or authenticated SteamGridDB test is claimed; no API key was available. Fatekeeper's earlier working user report remains valid.

Keep installer behavior/safety intact, add no mods, preserve legacy/foreign files and copy no foreign implementation code. Historical beta documents remain unchanged.
