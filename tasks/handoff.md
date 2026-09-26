# AI handoff: game discovery and artwork

Read `tasks/plan.md` and `tasks/todo.md` first. This file records what is **done and
verified**, where the evidence is, and what is still open. `tasks/` lives on the branch
`codex/game-identity-contracts` (PR #33) — the T3 and T4 branches were cut from
`origin/main` and stacked, so they do not carry these documents yet.

## Status (updated after T1–T4)

| task | what | branch | PR | state |
|---|---|---|---|---|
| T1/T2 | `StoreIdentity { platform, store_id }` + identity- and path-aware dedup, title quality | `codex/game-identity-contracts` | #33 | open, mergeable, 15 files, +1717/−106 |
| T2b | central path normalisation (`GameKey::path_key`) | same branch | #33 | same PR |
| T3 | Steam portrait from the local `librarycache`, by exact app id | `codex/steam-local-art` | #34 | open, mergeable, 5 files, +481/−10, CI green |
| T4 | Steam store portrait via `GetItems`, with fallback, bounds and an injectable fetch | `codex/steam-getitems-art` | #35 | open, mergeable, 5 files, +633/−20, base is #34 |

Evidence per branch, all re-run on the committed tree:

- PR #33: 170 tests pass, fmt clean, clippy `-D warnings` exit 0. Three independent
  adversarial reviews, all `accept`.
- PR #34: 138 tests pass on that branch (14 new in `tests/steam_local_art.rs`), fmt and
  clippy clean; two reviews (`accept_with_findings`), five findings closed and
  mutation-verified.
- PR #35: 157 tests pass (19 new in `tests/steam_store_art.rs`), fmt and clippy clean;
  two reviews (`accept_with_findings`), six findings closed, and five further low
  findings from the second review fixed in the same tree.

Reviews are subagent reviews with no shared context, running against the working tree
in copies they may not modify (`deleg_8f83dc56`, `deleg_6ecd1d1c`, `deleg_1918aa67`,
`deleg_15e04e34`). They are **not** registered in the jev-loop runs that produced the
work — see the loop note below.

## Repository state at handoff

- Product: MIT-licensed OptiScaler-GUI Standard for Windows, Rust `opticore` + egui GUI.
  Public `2026.9.0-beta.1` exists, but check remote state again before work.
- Implementation worktree: `C:/Users/marci/.codex/worktrees/release-2026-9-beta/OptiScaler-GUI`.
  Branches: `codex/game-identity-contracts` (#33, carries `tasks/`),
  `codex/steam-local-art` (#34, cut from `origin/main` `223e9722`),
  `codex/steam-getitems-art` (#35, stacked on #34 — both edit `images.rs`).
  `#34` and `#35` have **zero file overlap** with `#33` and can land in any order; `#35`
  must be retargeted to `main` after `#34`.
- Original workspace `F:/e/D/VSC Projekt mappe/OptiScaler-GUI` is an older Python
  checkout with many user changes. Another `.../OptiScaler-GUI-dlssnr` worktree exists.
  Do not reset, clean, delete or merge those changes casually.
- Relevant files: `rust/crates/opticore/src/{model.rs,steam_art.rs,images.rs,scan/mod.rs,scan/steam.rs}`;
  GUI `rust/crates/optiscaler-gui/src/ops.rs`, `screens/games_grid.rs`; tests
  `rust/crates/opticore/tests/`; CI `.github/workflows/ci-rust.yml`.
- The Rust scanner supports Steam, Epic, GOG, Heroic, Xbox and manual. Dedup now keys on
  `StoreIdentity` first (so two same-name installs of different stores survive), falls
  back to a normalised path key, and only then to name-plus-platform.
- The image pipeline now has six sources in order: local Steam `librarycache` portrait
  (exact app id), Steam store `GetItems` portrait, CDN `header.jpg`, `appdetails`
  (`header_image`/`capsule_image`), then Heroic/GOG/Xbox sources and the icon fallback.
  Every network body and every decoded image is bounded, and the fetch is injectable.
- `game-scanner` and GameHub are archived. IGDB requires a Twitch client secret and is
  deferred. Reference repos are not project dependencies.

## Open, known and deliberately not fixed

- **The `appdetails` rung is broken in production** (pre-existing, unchanged by #35). It
  looks the response up by `appid.to_string()`, but a live response is normally keyed by
  a different id (`appids=620` → key `323180`, `730` → `2678630`, `220` → `323140`,
  `440` → `629330`, `570` → `2120612`, `400` → `622640`, `70` → `632440`). When the key
  *does* match the payload can belong to another app: for `appids=100` the rung returns
  app 80's artwork. It deserves its own change and its own live verification.
- **The GOG search body keeps the same response bound without a test of its own** — one
  shared helper, proven out of tree, not pinned in the suite.
- **T2b's fallback is exact**: a store that changes the spelling of its own path between
  versions matches neither the new nor the old key. A third candidate would cover it.
- **#34's note, still open**: an already-cached landscape header wins over a new local
  portrait; per-source cache keys belong to a later task.
- **`cache_local_image` (Xbox) still reads without a bound** — pre-existing, untouched.
- **Low findings carried forward, not blocking**: Steam's `installdir` name join;
  `Platform::Amazon` missing from `games_grid.rs` and `report.rs`; `fill_gaps` does not
  re-derive `community_verified`; `game_at` never sets `discovery_source`; correction C
  does not follow Windows symlinks; the Rust scanner diverges from
  `src/scanner/game_scanner.py`.
- **One unexplained test run**: one run reported four failures in `steam_store_art.rs`
  against unmutated code; four later runs and the full gate were green and the panic
  text was lost to a filter. Cause unknown. Do not treat it as fixed.

## Loop note (process, not code)

The work was driven by jev-loop runs `20260925-235232` (identity), `20260926-011909`
(T3) and `20260926-021849` (T4). All three hit the same server-side limitation: the
review phase is offered only while `p_done` is high or when the *same* role re-runs with
unchanged green checks, so a run that alternates roles after a review can never be sent
back to review — the second and later verdicts are therefore subagent reviews the runs
could not register. A fix (a `review_turns` fact plus skill notes) is written, tested and
**uncommitted** in `F:/AI-Projekter/jev-loop`; it takes effect only after that MCP server
restarts. Verify loop state with `loop_status` rather than assuming these runs are
closed.

## Non-negotiable instructions

The user says cloning reference repositories for study is allowed, but copying their code
is not. Clone only into an isolated research directory if necessary; never vendor their
files, paste snippets, or translate implementation line-by-line. Record observed file
formats, public API contracts and expected behaviors in an independent spec, then write
original Rust. Even permissively licensed sources are subject to this user restriction.
Retain attribution links for research and check any new dependency's license separately.

Preserve the 2026.9 safety model: common EXE resolver, explicit choice under ambiguity,
transactional backups/rollback, conservative legacy manifests, no inferred compatibility
from images or DLL names, no anti-cheat modification, no extra mods. Offline-first,
privacy-first. No DLSS 5/AMD-NR branch.

## First actions for the next AI

1. Inspect `git status`, `git worktree list`, current `origin/main`, release/CI status and
   this plan; choose a clean `codex/` feature branch. Avoid editing the dirty original
   checkout.
2. Land or rebase #33, #34 and #35 (no file overlap), retargeting #35 to `main` after #34.
3. Then T5 and one store adapter per PR. Give the broken `appdetails` rung its own change
   with live verification, since the chain's later sources depend on it.
4. Keep `tasks/todo.md` and this file updated with completed tasks and test evidence. Test
   real games only where available and label missing manual coverage accurately — every
   number in this file comes from synthetic fixtures, temp dirs and injected fetchers.
5. Before release, review license, privacy, image URL handling, scan bounds, install
   safety and DA/EN/PL UI. Do not publish or promote a release based only on synthetic
   tests.

## User-facing success definition

More installed games are found with reliable store IDs and sources, Steam covers improve
offline and online, non-Steam covers have controlled fallback and optional user-approved
online lookup, and **no newly discovered game is installed into a guessed location**. The
user can see where each game and cover came from and correct mistakes.
