# 2026.9.0 release handoff

The full original discovery/artwork implementation is complete in the shared release worktree. The user explicitly authorized stable 2026.9.0 publication after green checks; no new publication approval is needed. Record publication only after the GitHub release exists.

## Remaining work

Both fixes and targeted regressions are complete. Final formatting, Clippy and locked workspace tests passed through Jev turn 6. Final EXE/7z verification is complete; exact JSON values are recorded. Independent follow-up returned `done=true` with no missing items; Jev stopped with `goal_met` after six executor turns. Only remote CI and publication remain.

The [checklist](todo.md) tracks the remaining gates. The [verification record](../docs/releases/2026.9.0-verification.md) contains the exact final build JSON values and coordinator-reported test evidence. [Public release notes](../docs/releases/2026.9.0.md) are the workflow's GitHub release body and use absolute `blob/v2026.9.0` documentation links.

## Evidence and limits

Final source tree: `26d8042f463393ac845e1c0f0f11957de30319ae`. EXE: 8,933,888 bytes; 7z: 3,324,820 bytes. Archive integrity and eight-second da/en/pl startup passed. Subsequent reconciliation changes are documentation-only. GitHub-built binaries will have their own `SHA256SUMS`. Audit found zero vulnerabilities and the existing unmaintained `ttf-parser` warning.

Read-only store smoke: Ubisoft two positive entries; Amazon, EA and Battle.net zero; three warnings total. Fixtures cover the zero-result adapters, without positive live evidence. No manual visual/gameplay or authenticated SteamGridDB test is claimed; no API key was available. Fatekeeper's working user report remains valid without a new game-specific publication gate.

SteamGridDB sends an explicit Steam app ID or title after consent; the user chooses the game and image. Preserve session-only credentials and explicit selection. All four stores, custom roots and the full artwork controls/cache are implemented scope.

No installer behavior changes, new mods or foreign-code copying. Preserve other agents' files, source-evidence documents and all historical beta documentation.
