# Hardware/RDNA2 and Rust-only feature handoff

## Completed release and issue-review follow-up

Public download: [v2026.10.0](https://github.com/King4s/OptiScaler-GUI/releases/tag/v2026.10.0), tag commit `9ad183c017279d53a526b6106e14b130e59b3679`, release workflow `37200607827` passed. Downloaded EXE/7z checksums, archive integrity, extracted-EXE identity and published-binary startup da/en/pl passed. Exact hashes and evidence are in `docs/releases/2026.10.0-verification.md`.

All three open issues (#30 Palworld, #31 Hogwarts, #38 v0.9.4 review) were reviewed and answered with verified scope and retest instructions; no game compatibility claim or issue closure. Additional synthetic target regression passed after respecting explicit selection for ambiguous layouts. Both real-artifact canaries were rerun and passed (144.71 seconds; first 120-second tool window was insufficient).

Git Bash was already present. User-scoped `jqlang.jq` 1.8.2 installed with winget; `jq --version` and JSON generation passed via `%LOCALAPPDATA%/Microsoft/WinGet/Links/jq.exe`. Existing shells may need reopening for the PATH alias. Original dirty project checkout remains untouched. No gameplay/manual visual/anti-cheat test; Palworld and Hogwarts await reporter confirmation.

Release follow-up: 2026.10.0 was authorized after this feature handoff. See docs/releases/2026.10.0-verification.md and GitHub Releases for release evidence; the no-release statements below describe the original feature phase.

Branch: `hermes/hardware-recommendations-rdna2`.
Checkout: `C:/Users/marci/AppData/Local/hermes/cache/scratch/optiscaler-github-hardware`.
Parent publication is on branch `hermes/hardware-recommendations-rdna2`; GitHub PR status is recorded below after remote verification. No version bump, tags or new binary release is included. Original dirty checkout was not modified.

## GitHub publication

- PR: https://github.com/King4s/OptiScaler-GUI/pull/37, targeting `main`.
- Feature commit: `99f35ae76fddd868ed3c424c727a70fa4f7d548a`.
- GitHub CI run `37165895417`: lint, Windows tests and release build **passed**; optional extraction canary **skipped**. Actual RDNA2 canaries were executed locally by the parent, not by that remote job.
- Repository description updated and read back: “Unofficial Rust Windows GUI for OptiScaler: game discovery, upscaler installation, local hardware profiles and game settings.”
- No new tag or GitHub release/binary publication. Merge state must be checked on the PR; this record does not claim a merge before it happens.

## Parent verification and review

- Parent independently reran locked workspace tests, all-target strict Clippy and formatting successfully before the review fix, plus both real-artifact canaries (**2 passed**, actual official/community DLLs).
- Independent review found one stale cached-hardware authorization defect; a separate fix implemented current-session validity and regression tests.
- Parent reran all four hardware lifecycle regression tests after the fix: **4 passed**; formatting and diff checks passed.
- Independent re-review passed with no security concerns or logic errors. Two additional test-strengthening suggestions are nonblocking.
- Fix agent's complete post-fix suite: **291 passed, 0 failed, 4 ignored**; no parent claim of gameplay, live network DLL download or released-binary verification.
`tasks/handoff.md` and `docs/releases/*` remain unchanged historical evidence.

## Implemented

- App's first update starts hardware collection on a worker regardless of update-check settings. Always-visible bottom panel offers family recommendations and manual refresh on every screen; Settings retains detailed Windows/driver/RAM/VRAM facts and explicit deletion.
- Refresh invalidates snapshot-local game GPU choices and current-session validity; cached/pending/failed snapshots cannot authorize runtime installation or populate the game GPU selector. Validity starts false and becomes true only when polling receives a fresh collection. Cached facts remain displayable after failure. Delete invalidates validity and cancels the receiver so late results cannot resurrect deleted hardware. Unreadable profile files stay preserved until explicit deletion. Next startup collects again.
- Recommendations are localized en/da/pl and advisory only: known RX 6000 → optional community fix, RX 7000/9000 → official FSR 4, RTX → native DLSS, Arc → XeSS, older Radeon RX/GTX → conditional XeSS/FSR 3.1, unknown/APU → conservative. Family identification is name-based, not measured capability.
- Explicit per-game rendering GPU remains mandatory. Neither first/default adapter nor GUI rendering adapter becomes the game GPU. Even a single RX 6000 is not auto-selected. Mixed hardware requires explicit eligible choice.
- Community RDNA2 installation is a fresh per-operation Yes/No opt-in with community/online/anti-cheat warning. No new settings are applied by recommendations. Existing anti-cheat confirmation and executable/target review remain in force; no bypass is added.
- Separate pinned archive cache, fresh extraction and exact member/DLL SHA256+size checks; private revalidated DLL staging precedes game mutations. Official extraction never changes. Transactional file replacements now stage beside destinations and atomically persist, preserving hard-link and original-backup safety.
- Manifest `fsr_runtime` records source/asset/archive hash/DLL hash+size/consent/selected GPU name. It is provenance, not proof of loaded DLL or actual rendering adapter.
- Normal owned update/uninstall tracks the community DLL and preserves/restores foreign originals. Automatic updates refuse existing community installs; manual update requires fresh choice/consent. Current installed games have explicit community and Restore official runtime actions. Official restoration refuses a payload lacking a replacement DLL rather than clearing provenance while leaving the patch behind.

Pins and user steps are in [the feature guide](../docs/hardware-runtime.md).

## Rust-only cleanup

54 tracked `.py` files removed; 74 obsolete implementation/tooling/documentation files removed overall (inventory below).
This includes all legacy `src/` Python implementation and root Python tests, requirements/pytest config, PyInstaller
spec/hooks/build/launch/test setup scripts, Python-only release helpers, ci.yml/release.yml/windows-tests.yml,
and obsolete Python test-environment/PyInstaller/robust-architecture instructions.

Before deletion, traced every Rust include and runtime resource reference. Preserved and relocated:
- `src/translations/{en,da,pl}.json` → `rust/crates/opticore/data/translations/` (extended with new strings).
- `src/data/community_verified_games.json` → `rust/crates/opticore/data/community_verified_games.json` (unchanged data).
- Rust i18n/scanner and Rust advice tests now embed the new paths. Missing-path compilation was observed before fixes; all resource/translation/scanner tests now pass.
- `assets/` icons, Rust shaders/data/tests/fixtures, including historical python_manifest.json ownership fixture, remain intact.

Six older overview/status documents moved under `docs/history/` and labeled historical where nonempty:
COMMUNITY_ENGAGEMENT, CRITICAL_ISSUES, LANGUAGE_STANDARDIZATION, MULTI_LANGUAGE_SYNC_COMPLETE,
PROGRESS_IMPLEMENTATION_COMPLETE and V0_7_9_UPDATE_SUMMARY. Historical release notes/checksums were not rewritten.
Older release-plan/parity notes are labeled historical rather than treated as current publication authorization.
README, CLAUDE, Copilot instructions, release/build/technical/portable/demo guidance, troubleshooting and changelog
now describe Rust and distinguish unreleased additions from published binaries.

CI uses `rust/**` resource paths. Removed unnecessary Python setup from docs deployment.
Upstream notification uses github-script/API objects instead of Python shell parsing, points to the current official
repository and Rust review paths, deduplicates against open/closed issues, and no longer commits/pushes tracking changes.
No workflow was remotely dispatched or release published here.

## Verification

Observed targeted RED→GREEN cycles for RX 6000/conservative classification, eligibility/consent failure,
all-language recommendation keys, pending snapshot/deletion cancellation, missing moved includes, official restore guard,
and real archive member layout (`4.1.1b/…`, not root). Added correct-size/wrong-hash preservation coverage.

Review defect follow-up (only `rust/crates/optiscaler-gui/src/hardware_view.rs` and this handoff modified):
- Strict RED before production changes: disconnected-channel cached RX 6700 XT regressions both failed as expected (`current_gpu` returned stale hardware; the rendered game selector showed the stale GPU). Fresh-success→refresh→disconnect lifecycle regression also failed on stale authorization.
- GREEN: `cargo test --manifest-path rust/Cargo.toml -p optiscaler-gui --locked hardware_view::tests -- --nocapture`: **4 passed**. Exact tests: `disconnected_refresh_cannot_authorize_cached_rx6000`, `disconnected_refresh_selector_rejects_cached_rx6000_choice`, `fresh_result_authorizes_until_refresh_or_delete`, `pending_snapshot_cannot_authorize_runtime_and_delete_cancels_late_result` (all under `hardware_view::tests`).
- Post-fix verification used the absolute checkout-private `CARGO_TARGET_DIR=C:/Users/marci/AppData/Local/hermes/cache/scratch/optiscaler-github-hardware/target-private`; workspace locked tests and all-target locked clippy passed. No commits/push, gameplay, cache cleanup or unrelated policy changes.

Final Windows checks:
- `cargo fmt --all --check`: PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `cargo test --workspace --locked`: post-review **291 passed, 0 failed, 4 ignored**; full local output at `target-private/review-stale-hardware-tests.log` (run with `--manifest-path rust/Cargo.toml`). Post-review clippy output: `target-private/review-stale-hardware-clippy.log`; fmt check also passed. Earlier pre-review output remains at `target-private/verification-tests.log`.
- Both ignored RDNA2 real-artifact tests explicitly run: **2 passed**, including pinned cache reuse with corrupted extracted-cache repair; real official 0.9.4 + pinned community archives install→community update→official restore→uninstall and foreign-original restoration. Corrupt runtime preserves existing manifest/DLL and official extraction stays byte-identical.
- `cargo build --workspace --locked`: PASS (debug native EXE at `target-private/debug/OptiScaler-GUI.exe`).
- da/en/pl native startup smoke, 8 seconds each: all alive, expected startup log, no crash log; with `check_updates=false`, fresh local hardware profile collected and RX 6700 XT detected, game GPU map empty. No actual game was modified by smoke.
- `git diff --check`: PASS. No remaining tracked Python implementation exists in the working tree; no active Python build/run instructions outside clearly historical material.

All final Cargo targets are checkout-private `target-private/` (ignored). One early command used a root-relative target
incorrectly and created a sibling scratch build cache; that unused cache was removed. No cargo clean/global Cargo
configuration or dependency versions/lockfile changes. Existing tempfile dependency promoted from dev to runtime
for private staging/atomic persistence.

## Limits / parent publication gates

No fresh gameplay test, anti-cheat safety guarantee, measured in-game rendering-GPU detection, manual visual QA,
live community network-download test or new release artifact claim. Real canaries used already downloaded pinned
archives; downloader digest behavior is exercised separately in existing Rust tests. Name guidance is conservative
and intentionally not a complete GPU capability database. Existing official download/extraction cache policy is
unchanged. Parent should independently review the new code/UI and cleanup, run remote Rust CI, then publish only
after approval. No new release or version bump is part of this request.

## Proposed short GitHub description

Unofficial Rust Windows OptiScaler manager with local hardware guidance, safe installs and opt-in pinned RDNA2 FSR runtime.

## Removed obsolete files (shared resource moves excluded)

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.github/workflows/windows-tests.yml`
- `PYINSTALLER_BUILD_GUIDE.md`
- `ROBUST_ARCHITECTURE.md`
- `TEST_ENV_CHECKLIST.md`
- `TEST_ENV_IMPLEMENTATION.md`
- `TEST_ENV_QUICK_REF.md`
- `TEST_ENV_SETUP.md`
- `build.py`
- `build_executable.bat`
- `build_executable.spec`
- `check_requirements.py`
- `cleanup_project.py`
- `cleanup_repository.py`
- `hook-numpy.py`
- `pytest.ini`
- `requirements.txt`
- `requirements_minimal.txt`
- `rthook_diagnostics.py`
- `run_progress_tests.bat`
- `scripts/prepare_release.ps1`
- `scripts/publish_release.ps1`
- `setup_test_env.bat`
- `setup_test_env.ps1`
- `src/__version__.py`
- `src/gui/__init__.py`
- `src/gui/main_window.py`
- `src/gui/main_window_backup.py`
- `src/gui/widgets/__init__.py`
- `src/gui/widgets/dynamic_setup_frame.py`
- `src/gui/widgets/game_list_frame.py`
- `src/gui/widgets/global_settings_frame.py`
- `src/gui/widgets/library_roots_frame.py`
- `src/gui/widgets/log_frame.py`
- `src/gui/widgets/settings_editor_frame.py`
- `src/gui/widgets/settings_editor_frame_new.py`
- `src/gui/widgets/settings_editor_frame_old.py`
- `src/main.py`
- `src/optiscaler/__init__.py`
- `src/optiscaler/manager.py`
- `src/optiscaler/manager_backup.py`
- `src/optiscaler/manager_fixed.py`
- `src/optiscaler/manager_new.py`
- `src/optiscaler/manager_old.py`
- `src/scanner/__init__.py`
- `src/scanner/debug_steam_name.py`
- `src/scanner/game_scanner.py`
- `src/scanner/library_discovery.py`
- `src/test_install.py`
- `src/test_install_detailed.py`
- `src/utils/__init__.py`
- `src/utils/archive_extractor.py`
- `src/utils/cache_manager.py`
- `src/utils/compatibility_checker.py`
- `src/utils/config.py`
- `src/utils/debug.py`
- `src/utils/i18n.py`
- `src/utils/i18n_corrupted.py`
- `src/utils/logging_utils.py`
- `src/utils/performance.py`
- `src/utils/progress.py`
- `src/utils/robust_wrapper.py`
- `src/utils/system_requirements.py`
- `src/utils/translation_manager.py`
- `src/utils/update_manager.py`
- `standalone_main.py`
- `start_gui.bat`
- `tests/conftest.py`
- `tests/test_dynamic_setup.py`
- `tests/test_game_platform_tags.py`
- `tests/test_heroic_scanner.py`
- `tests/test_scan_perf_fixes.py`
- `tests/test_steam_vdf_parsing.py`
