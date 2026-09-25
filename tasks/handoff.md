# AI handoff: game discovery and artwork

Read `tasks/plan.md` and `tasks/todo.md` first. The user requested a plan and handoff, **not implementation in this turn**. No listed TODO is done.

## Repository state at handoff

- Product: MIT-licensed OptiScaler-GUI Standard for Windows, Rust `opticore` + egui GUI. Public `2026.9.0-beta.1` exists, but check remote state again before work.
- Planning worktree: `C:/Users/marci/.codex/worktrees/release-2026-9-beta/OptiScaler-GUI`, branch `codex/2026-9-beta`, clean before these three docs were added. Its HEAD was `742bb4d9`; local `origin/main` was `223e9722` at inspection. Do not assume these remain current.
- Original workspace `F:/e/D/VSC Projekt mappe/OptiScaler-GUI` is an older Python checkout with many user changes. Another `.../OptiScaler-GUI-dlssnr` worktree exists. Do not reset, clean, delete or merge those changes casually. Start implementation in a fresh clean branch/worktree from current `origin/main` or first reconcile these planning docs there.
- Relevant files: `rust/crates/opticore/src/model.rs`, `scan/mod.rs`, `scan/discovery.rs`, `scan/steam.rs`, `images.rs`, `resolver.rs`, `install/transaction.rs`; GUI `rust/crates/optiscaler-gui/src/ops.rs`, `screens/games_grid.rs`, `state.rs`; tests `rust/crates/opticore/tests/`; CI `.github/workflows/ci-rust.yml`.
- Current Rust scanner supports Steam, Epic, GOG, Heroic, Xbox and manual. `ScanConfig` only has `excluded_drives`. `Game` has `steam_appid` and optional `art_url`; dedup includes a name-plus-platform rule that may discard separate same-name installs. Confirm against current main before changing.
- Current image pipeline has Steam fixed `header.jpg` **and** `appdetails` fallback, Heroic/GOG/Xbox sources and icon fallback. It does not yet inspect local Steam `librarycache` or `GetItems` portraits. Do not repeat the false claim that it only uses a fixed header URL.
- `game-scanner` and GameHub are archived. IGDB requires a Twitch client secret and is deferred. Reference repos are not project dependencies.

## Non-negotiable instructions

The user says cloning reference repositories for study is allowed, but copying their code is not. Clone only into an isolated research directory if necessary; never vendor their files, paste snippets, or translate implementation line-by-line. Record observed file formats, public API contracts and expected behaviors in an independent spec, then write original Rust. Even permissively licensed sources are subject to this user restriction. Retain attribution links for research and check any new dependency's license separately.

Preserve the 2026.9 safety model: common EXE resolver, explicit choice under ambiguity, transactional backups/rollback, conservative legacy manifests, no inferred compatibility from images or DLL names, no anti-cheat modification, no extra mods. Offline-first, privacy-first. No DLSS 5/AMD-NR branch.

## First actions for the next AI

1. Inspect `git status`, `git worktree list`, current `origin/main`, release/CI status and this plan; choose a clean `codex/` feature branch. Avoid editing the dirty original checkout.
2. Implement T1/T2 as a small contract-and-fixture PR; do not start all store adapters in parallel because they share `Game`, `ScanConfig`, dedup and UI state.
3. Implement T3/T4 as the first user-visible slice, preserving current cover fallbacks and cache compatibility. Then T5 and one store adapter per PR.
4. Keep `tasks/todo.md` updated with completed tasks and test evidence. Use Windows CI commands from `tasks/plan.md`; test real games only where available and label missing manual coverage accurately.
5. Before release, review license, privacy, image URL handling, scan bounds, install safety and DA/EN/PL UI. Do not publish or promote a release based only on synthetic tests.

## User-facing success definition

More installed games are found with reliable store IDs and sources, Steam covers improve offline and online, non-Steam covers have controlled fallback and optional user-approved online lookup, and **no newly discovered game is installed into a guessed location**. The user can see where each game and cover came from and correct mistakes.
