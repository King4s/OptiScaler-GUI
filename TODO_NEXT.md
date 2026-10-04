> Historical 2026.9.0 / rewrite record, not current feature or publication authorization. See tasks/handoff-hardware.md for current unreleased scope.

# 2026.9.0: final review, remote CI and publication

The full original discovery/artwork implementation is complete: four new local store adapters, configurable scan roots, aspect-preserving cards, per-install cover caching with legacy preservation and portrait upgrades, local override/reset/refresh, and optional SteamGridDB with consent, session-only key and explicit game/image selection. SteamGridDB lookup sends an explicit Steam app ID or title; automatic SteamSpy matching is removed.

Stable 2026.9.0 publication is explicitly authorized after green checks. Publication remains pending until the GitHub release exists.

## Release checklist

- [x] Windows verbatim-path deduplication fix complete; targeted regressions passed.
- [x] Last-good-art refresh fix complete; targeted regressions passed.
- [x] Final Jev turn 4 formatting, Clippy and locked workspace tests PASS.
- [x] Final EXE/7z and exact hashes recorded; integrity and eight-second da/en/pl startup PASS.
- [x] Final [verification record](docs/releases/2026.9.0-verification.md) reconciled with build JSON.
- [x] Independent final review approved; Jev stopped with `goal_met` after six executor turns.
- [ ] Record green remote CI.
- [ ] Publish under the existing authorization and record the GitHub release URL.

## Recorded evidence

Final tested source tree: `26d8042f463393ac845e1c0f0f11957de30319ae`. EXE: 8,933,888 bytes; 7z: 3,324,820 bytes. Subsequent reconciliation changes are documentation-only. GitHub-built binaries will have their own `SHA256SUMS`. Audit reported zero vulnerabilities with the existing unmaintained `ttf-parser` warning.

Read-only discovery smoke found two Ubisoft games, zero Amazon/EA/Battle.net games and three warnings. Fixture coverage is not positive live evidence. Manual visual/gameplay and authenticated SteamGridDB testing were not performed; no API key was available. Fatekeeper's earlier working user report remains valid.

The scope changes no installer behavior and adds no mods. Preserve reviewed targets, ownership/rollback, legacy/foreign files and the ban on foreign-code copying. Historical beta evidence remains separate and unchanged. See [plan](tasks/plan.md), [checklist](tasks/todo.md) and [release notes](docs/releases/2026.9.0.md).
