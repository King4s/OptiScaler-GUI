# Game discovery and artwork TODO

Status: T1, T2, T2b, T3 and T4 are **implemented, reviewed and pushed**; T0 is satisfied in
practice (the work ran on branches cut from `origin/main` with green CI). T1/T2/T2b are
`codex/game-identity-contracts` → PR #33, T3 is `codex/steam-local-art` → PR #34 and T4 is
`codex/steam-getitems-art` → PR #35. None is merged; every acceptance criterion and its
verification is recorded in its PR. `tasks/handoff.md` carries the current state, the open
findings and the decisions that are the user's.

For every task: write fixture tests first, keep the existing installer safety gate, run focused tests and review the diff. Complete only when acceptance criteria and verification are recorded in the PR.

- [x] **T0, XS, no dependency:** Rebase planning context onto current `origin/main`; document baseline and create a clean feature branch. Verify worktree status, branch, CI and beta tag before editing code. — Done in practice: the branches were cut from `origin/main` and every CI run on them is green; see hit handoff.
- [x] **T1, M, T0:** Define store identity, scan provenance and art candidate contracts in `opticore` without removing `steam_appid` or breaking legacy config/cache. Verify serialization/back-compat and same-title fixture tests. — Delivered in PR #33 (15 files, +1888/−106), criteria and verification recorded there.
- [x] **T2, S, T1:** Make dedup path/store-ID aware; retain two installs with the same name, while removing duplicate scan hits for one install. Verify fixture tests and `scan_all` behavior. — Delivered in PR #33; dedup now keys on `StoreIdentity` first, then a normalised path, then name+platform.
- [x] **T2b, S, T2:** Fold path spellings in the dedup key. `GameKey.path_norm` is only — Delivered in PR #33 as central path normalisation (`GameKey::path_key`).
  `path.to_string_lossy().to_lowercase()`, which folds case and nothing else, so one install
  reported as `C:\Games\X` by a store root and as `C:/Games/X` by Heroic stays two entries.
  Every source hands the path over exactly as its store metadata spells it, and the Python
  scanner folds separators through `os.path.normcase`, so the two scanners disagree on the same
  machine. Add one documented normalization function and use it everywhere a key is built,
  `Game::new` and `build_game`. It must fold both separator kinds and a single trailing
  separator centrally, so no caller has to remember either.
  Do not collapse `.`/`..` segments textually. A junction or symlink can make a purely textual
  shortening point at a different install, and merging two genuinely different installs is worse
  than showing one twice. Resolving paths through the filesystem is the thorough version, but it
  costs a stat per path on every scan; take the conservative half, record why, and leave the
  rest to a task that can pay for it.
  Treat this as a policy change, not a bug fix: separator spellings are not the only paths at
  stake. `profiles.json` persists the user's per-game GPU choice as
  `hardware.local.game_gpus: {"<path_norm>": "<gpu-id>"}`, and `busy_ops`, the art cache state
  and the observation/report lookups are keyed by `path_norm` too. A new normalization orphans
  every entry whose spelling changes, so decide and document read-compat for existing `game_gpus`
  values (accept the loss, or resolve the new key and the old one as well) before this lands.
  Verify: fixture tests for `\` vs `/`, mixed case and a trailing separator each collapsing to one
  entry; two genuinely different paths still separate; a `..` segment left alone rather than
  folded; a persisted `game_gpus` entry written under the old key still resolving; existing
  same-title and legacy fixtures unchanged.
  Decided when implemented: the key folds to backslashes, because that is what `os.path.normcase`
  gives on Windows and a natively spelled path then keeps the exact key older builds wrote;
  read-compat resolves the new key first and the old spelling second, so nothing is rewritten.
  The fallback is exact, so a store that changes its own spelling between versions still orphans
  its entry. See `GameKey::path_key`, `Game::legacy_path_norm` and `LocalProfiles::gpu_for`.
- [x] **T3, M, T1:** Add exact-app-ID local Steam portrait cache lookup. Verify offline fixture, no global recursive scan, bounded decode and icon fallback. — Delivered in PR #34 (5 files, +481/−10, CI green, 14 new tests in `tests/steam_local_art.rs`).
- [x] **T4, M, T3:** Add Steam `GetItems` portrait lookup, with `appdetails`/header fallback. Verify mock responses for missing assets, wrong app ID, HTTP failure, oversized and malformed images; no UI blocking. — Delivered in PR #35 (5 files, +662/−20, 20 new tests in `tests/steam_store_art.rs`).
- [ ] **T5, M, T1:** Persist user scan roots and opt-out settings, show provenance, bound traversal and protect against out-of-root paths. Verify settings roundtrip and fixture scan.
- [ ] **T6, M, T1+T5:** Amazon adapter. Verify local SQLite fixtures: installed/uninstalled, relocated, malformed/locked DB, no EXE guess.
- [ ] **T7, M, T1+T5:** Ubisoft adapter. Verify registry/manifest fixtures, multiple libraries, launcher exclusion and no EXE guess.
- [ ] **T8, M, T1+T5:** EA adapter. First record verified current format; then test installed, moved, malformed and unknown records. Do not implement from unverified assumptions.
- [ ] **T9, M, T1+T5:** Battle.net adapter. First record verified current format; test missing name/EXE mappings and keep ambiguous records manual-only.
- [ ] **T10, M, T1+T3:** Per-store cover provenance and collision-free cache keys. Verify two same-name games have separate covers, old cached images still load and no automatic deletion occurs.
- [ ] **T11, M, T10:** Optional SteamGridDB provider with BYO key, opt-in UI, rate/size limits and confirmed fuzzy match. Verify disabled mode sends no requests and key is never logged or exported.
- [ ] **T12, S, T10+T11:** Cover-source display, override/clear, loading/fallback states and DA/EN/PL strings. Verify keyboard/UI smoke and offline operation.
- [ ] **T13, M, T2-T12:** Docs, full CI, security/privacy review, packaged EXE/7z check and manual store tests. Beta only after evidence; stable promotion separate.

Checkpoints: after T2, T4, each of T6-T9, T11 and T13 run the complete Rust test/lint/build gate. Keep each store adapter in its own PR. Stop a store adapter if its current format cannot be verified; mark it blocked in the checklist and continue with independent work.
