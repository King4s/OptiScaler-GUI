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
| T4 | Steam store portrait via `GetItems`, with fallback, bounds and an injectable fetch | `codex/steam-getitems-art` | #35 | open, mergeable, 6 files, head `3e3f683d`, base is #34 |

Evidence per branch, all re-run on the committed tree:

- PR #33: 170 tests pass, fmt clean, clippy `-D warnings` exit 0. Three independent
  adversarial reviews, all `accept`.
- PR #34: 138 tests pass on that branch (14 new in `tests/steam_local_art.rs`), fmt and
  clippy clean; two reviews (`accept_with_findings`), five findings closed and
  mutation-verified.
- PR #35: 158 tests pass (20 new in `tests/steam_store_art.rs`), fmt and clippy clean;
  four reviews (`accept_with_findings`), every finding closed, and the last round's
  remaining item is the stale message of an earlier commit - see the loop note.

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
- **Workspace test runs can lie while the shared build cache is dirty** (found and
  fixed, but it will recur). The worktree builds into
  `C:/Users/marci/.codex/cargo-target/opticore`; when several checkouts of this package
  build there, cargo can serve a `libopticore` built from other source. Verified cause:
  the copies made for mutation and review runs carry this worktree's `.cargo/config.toml`
  (which names that shared directory) and hold source predating the appdetails bound, so
  any `cargo` call in them without an explicit `CARGO_TARGET_DIR` wrote there. The symptom
  is one test failing in
  `cargo test --workspace` while passing under `-p opticore --test`. `cargo clean -p
  opticore` cleared it and the same command was green (158 passed / 0 failed / 1
  ignored). When a check result looks impossible, verify it against a fresh
  `CARGO_TARGET_DIR` before believing it — and give any reviewing subagent a fresh target
  directory rather than this shared one.

## Loop note (process, not code)

The work was driven by jev-loop runs `20260925-235232` (identity), `20260926-011909`
(T3) and `20260926-021849` (T4). All three hit the same server-side limitation: the
review phase was offered only while `p_done` was high or when the *same* role re-ran with
unchanged green checks, so a run that alternates roles after a review could never be sent
back to review, and the second and later verdicts were subagent reviews the runs could
not register.

That is fixed, not worked around: a `review_turns` fact (default 3) now takes a run back
to a reviewer once a recorded review has had that many executor turns since, Jev still
answering the `review_now` question. The change is **committed and live** — an earlier
Claude Code session committed this work's uncommitted patch together with its own
`unreviewed` offer as `b3d5d2d`; the four skill lines this work added followed as `2a57bb7`,
which is still in the history but has since been superseded by the reorganisation into
`skill/jev` (see the tooling section). `loop_decide` answered `review` / `why: revisit`
twice in run `20260926-021849`. Any further edit to `jev_mcp.py` needs the MCP server
restarted before it takes effect.

Run `20260926-021849` is deliberately **left open** at turn 11 (all checks green,
`consecutive_fails` 0, `cfg.review_turns` 1). Its last registered verdict is
`accept_with_findings` on wording only: the fourth review verified all four statements of
its predecessor true and filed three low findings, of which the spec's `i64` range and the
PR body's wording are fixed in `3e3f683d`, and the last one — the stale messages of
`fd871a9d` and `8f598200` — needs the user's decision, because correcting them means
rewriting pushed commits. Closing the run as `goal_met` is likewise the user's call; the
alternative is one more review round, which by now finds word choices rather than
behaviour. Check state with `loop_status` rather than assuming.

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

## Tooling: the loop skill was replaced by Jev-AI-Skill

The loop tooling this work used has been replaced. The project (formerly `jev-loop`,
formerly `jev`) is now **`King4s/Jev-AI-Skill`**; the installed skill is named **`jev`** and
covers Loop, Route and Git. The MCP identity deliberately did **not** change: the server is
still `jev-loop` and its five tools (`loop_start`, `loop_decide`, `loop_record_turn`,
`loop_record_review`, `loop_status`) behave as before, so an older session and a new one can
both drive the same run.

Installed and verified on 2026-09-27 with the repository's own `install.ps1`:

- Skill copies: Claude Code `~/.claude/skills/jev`, Codex `~/.agents/skills/jev`, Hermes
  `C:/Users/marci/AppData/Local/hermes/skills/jev`. The legacy `jev-loop` skill folders were
  migrated away by the installer's `--legacy` handling.
- MCP re-registered for all three clients; Hermes reported **5 tools connected and enabled**.
- `python jev_mcp.py --check` → `jev-loop 2026.09.27.0056: OK (model jev-1.13.0, p=0.98)`.

Two things a next session must know:

- **Windows: set `HERMES_HOME` before running the installer.** `install.ps1` writes Hermes's
  skill to `$HERMES_HOME` when set and to `~/.hermes` otherwise — this installation lives in
  `C:/Users/marci/AppData/Local/hermes`, so run it as
  `HERMES_HOME="C:/Users/marci/AppData/Local/hermes" powershell -File install.ps1`, or the
  skill lands outside Hermes's real home. Re-run the installer after every `git pull`.
- **A restart is what makes it take effect** — a new Hermes session, and restarted Claude
  Code and Codex. The Hermes MCP entry now reads `command: python` (it used to be an absolute
  `Python313\python.exe`); both interpreters have `mcp` installed and `python` resolves to
  the Hermes venv first. No `env` block was added: the server reads the TypeSafe key from
  `~/.config/jev-loop/typesafe_api_key`, which is what the passing check exercised.

The local clone is still the directory `F:/AI-Projekter/jev-loop` with the remote
`git@github.com:King4s/Jev-AI-Skill.git`. Pre-install copies of the Hermes `config.yaml` and
`~/.codex/config.toml` are in
`C:/Users/marci/AppData/Local/hermes/cache/scratch/jev-preinstall-20260927-142232/`.

## First actions for the next AI

0. Start a fresh session so the `jev` skill and the re-registered MCP server are loaded,
   then read `loop_status` for run `20260926-021849` (open, all checks green, one verdict on
   wording) instead of assuming its state.
1. Inspect `git status`, `git worktree list`, current `origin/main`, release/CI status and
   this plan; choose a clean `codex/` feature branch. Avoid editing the dirty original
   checkout.
2. Land or rebase #33, #34 and #35 (no file overlap), retargeting #35 to `main` after #34.
   Five decisions are the user's and are not to be taken silently: which side wins the five
   files that conflict with `feature/dlssnr-editions`; whether run `20260926-021849` is
   closed as `goal_met` or given one more review round; whether the stale messages of
   `fd871a9d` and `8f598200` are rewritten; whether CI run #84 on `main` (Python job: 23
   tests pass, then the interpreter segfaults; flaky - the same job passed on #33) is simply
   re-run or gets the `faulthandler` line; and who commits anything further in the loop
   repository.
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

## Working alongside Claude Code on this machine

Another agent (Claude Code) works here too, and the two of you share the `jev-loop`
repository. On 2026-09-26 one of its sessions committed the whole of `jev_mcp.py` while
this work had an uncommitted patch in that file, so commit `b3d5d2d` ("Offer a reviewer to
an unreviewed run of green turns") carries both agents' work: the `revisit` path described
above and that session's `unreviewed` offer. Nothing was lost and the file is a superset,
but it is exactly the hazard of a dirty tree in a shared repository.

What was agreed afterwards: each agent owns its own checkout; commits in `jev-loop` carry
a distinct author identity so the history stays readable (this work commits as
`Hermes Agent <hermes+ai@seaaid.me>`, the repository default is the user's own identity);
and no agent commits another's uncommitted files. The four skill lines this work added
were committed as `2a57bb7` once the user approved, so that repository is clean again.

Claude Code's answers to the coordination questions came through the user, because its
CLI could not be used from Hermes: `claude auth status` reports an expired login and a
print-mode run fails with `OAuth session expired and could not be refreshed`. It confirmed
it did not build into the shared cargo target directory, did not touch the uncommitted
skill lines, and has nothing half-finished in `jev-loop`.

## Merge readiness (verified, not assumed)

The three branches merge into `main` in the order 33, 34, 35 with no conflicts: a trial
merge in a throwaway worktree produced `101f9e66`, and on that tree - built with a fresh,
private `CARGO_TARGET_DIR` so no stale artifact could flatter it - `cargo fmt --all
--check` is clean, `cargo clippy --all-targets --workspace --locked -- -D warnings` exits
0, and `cargo test --workspace --locked --no-fail-fast` is **204 passed / 0 failed / 1
pre-existing ignored** across 12 binaries. The branches are green on their own too (158 on
the T4 tree, 20 of them in `steam_store_art.rs`). Merging is the user's call; nothing is
known to block it.

## The other release branch conflicts with #33

Another branch, `feature/dlssnr-editions` (Claude Code's, 7 commits ahead of `origin/main`
and touching 42 files), diverges from `codex/game-identity-contracts` - 12 commits one way,
7 the other - and the two **conflict on five files**:

- `rust/crates/opticore/src/profiles.rs`
- `rust/crates/opticore/tests/advice_profiles.rs`
- `rust/crates/optiscaler-gui/src/advice_view.rs`
- `rust/crates/optiscaler-gui/src/hardware_view.rs`
- `rust/crates/optiscaler-gui/src/screens/games_grid.rs`

A trial merge shows #34 and #35 apply on top of that branch cleanly, so the conflict is
#33's alone; #33 also merges into `origin/main` cleanly. Which side wins in those five
files is not a mechanical resolution and is the user's decision.
