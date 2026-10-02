# Plan: finish 2026.9.0 release verification

Status: implementation, both fixes, targeted regressions, final Jev checks, package verification and independent review are complete. Only remote CI and publication remain. Stable publication is already authorized after green gates.

## Implemented scope

1. Store identity and separate title provenance with path-based deduplication and distinct installs preserved.
2. Exact-ID Steam local/GetItems portraits and guarded header/appdetails fallback.
3. Persisted custom roots with enable/disable/remove/rescan, bounded traversal and visible warnings.
4. Amazon local SQLite, Ubisoft registry, EA registry plus installerdata XML and Battle.net product.db discovery; [format evidence and limits](../docs/store-formats.md).
5. Per-install hashed `cover_art` cache, preserved legacy files, portrait upgrades and aspect-preserving cards.
6. Local override/reset/refresh/source controls with worker-thread disk/network operations.
7. Optional SteamGridDB with explicit consent, session-only key, explicit Steam app ID or title lookup and user game/image selection; no automatic SteamSpy title matching.

## Final sequence

1. Completed: Windows verbatim-path deduplication correction and green targeted regressions.
2. Completed: last-good-art preservation during a failed refresh and green targeted regressions.
3. Completed: final Jev turn 4 formatting, Clippy and locked workspace tests PASS.
4. Completed: final EXE/7z built; exact sizes/hashes, archive integrity and eight-second da/en/pl startup PASS recorded.
5. Completed: [verification record](../docs/releases/2026.9.0-verification.md) records source tree `26d8042f463393ac845e1c0f0f11957de30319ae` and exact final evidence; manual/authenticated gaps remain explicit.
6. Independent review approved; record green remote CI, then publish stable 2026.9.0 under the existing authorization and record the GitHub release URL.

Subsequent reconciliation changes are documentation-only. GitHub-built artifacts will carry their own `SHA256SUMS`. Read-only live discovery is positive for Ubisoft only; zero Amazon/EA/Battle.net results do not establish positive live validation.

## Constraints

No installer behavior changes, new mods, anti-cheat changes, telemetry or foreign-code copying. Preserve reviewed targets, explicit ambiguous-EXE selection, v2 ownership/rollback and legacy/foreign files. Reference projects remain format research, not copied code or implementation dependencies.

Do not claim universal discovery or compatibility. Fatekeeper's working user report remains valid; no fresh-version manual test is claimed or imposed as a special gate. No API key was available for authenticated SteamGridDB testing.

Keep all historical beta docs unchanged. The release body must stay concise and public-ready, using absolute GitHub `blob/v2026.9.0` documentation links; operational status belongs in tasks and verification evidence.
