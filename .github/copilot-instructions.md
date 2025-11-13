<!-- Copilot / AI agent instructions for working on OptiScaler-GUI -->

# OptiScaler-GUI — Quick context for AI coding agents

Be concise. This file summarizes the essential, discoverable patterns and workflows that help an AI agent be productive in this repository.

1) Quick entry points
 - Run from source: `python src/main.py` (see `src/main.py` for PyInstaller/frozen handling).
 - Fast dev launcher: `start_gui.bat` (project root).
 - Build portable exe: `python build.py` (uses PyInstaller - see `build.py` and `OptiScaler-GUI.spec`).
 - Install dev deps: `pip install -r requirements.txt`.
 - Tests: small unit scripts at repo root (e.g. `test_archive_extractor.py`, `test_progress_simple.py`) and `run_progress_tests.bat` for a larger suite.

2) Big-picture architecture (what to read first)
 - `src/main.py` — app entry, environment setup for source vs frozen executable.
 - `src/gui/main_window.py` — top-level UI, navigation, background-update patterns, progress overlay registration.
 - `src/optiscaler/manager.py` — central installation/download/extract logic (download → extract → copy → create uninstaller/config).
 - `src/scanner/game_scanner.py` — game detection entry (used by UI scanner).
 - `src/utils/` — shared helpers: `progress.py`, `archive_extractor.py`, `update_manager.py`, `translation_manager.py`, `debug.py`.

3) Key patterns & conventions (practical rules an AI should follow)
 - Long-running or blocking work runs in background threads (threading.Thread, daemon=True). UI code expects progress callbacks. Prefer using *_threaded helpers (e.g. `OptiScalerManager.install_optiscaler_threaded`) when simulating UI flows.
 - Progress and UX: use `progress_manager` and `ProgressOverlay` (see `src/gui/main_window.py` and `src/utils/progress.py`). When adding feedback, call progress callbacks with structured messages.
 - Translations: use the `t()` helper from `utils.translation_manager` for all visible strings. Language files live in `src/translations/*.json`.
 - Debugging/logging: use `utils.debug.debug_log()` and `utils.debug` toggles. UI exposes a debug-mode log button — don't print directly to stdout in GUI contexts.
 - File operations: code uses pathlib (`Path`) and writes Windows batch scripts for installers/uninstallers. When modifying install logic, respect the Unreal-Engine install path detection rule: install to `Engine/Binaries/Win64` if that folder exists (see `OptiScalerManager._determine_install_directory`).
 - Archive handling: extraction uses a robust multi-tier approach: system 7z.exe → `py7zr` → Python zip fallback. Reuse `utils/archive_extractor.py` for validation/extraction.
 - Network/IO: downloads use `requests` with timeouts and chunked writes; preserve progress callbacks and graceful failure handling (refer to `_download_latest_release` and `_download_file`).

4) Integration & external dependencies
 - OptiScaler releases are fetched from GitHub API (see `OptiScalerConfig.GITHUB_API_URL` in `src/optiscaler/manager.py`). Tests or mock work should patch network calls.
 - **v0.7.9 update**: DLSS Inputs (AMD/Intel) no longer creates `nvngx.dll` file. Only modifies `Dxgi=false` when user selects "No". Update setup logic accordingly.
 - 7-Zip: code tries standard install locations plus PATH; tests should mock `shutil.which` or the `_find_seven_zip` result.
 - PyInstaller: `src/main.py` checks `sys.frozen` / `sys._MEIPASS`. When editing startup logic, preserve both source and frozen behaviors.

5) Unit & integration testing guidance (what agents can add)
 - Small, focused tests live at repository root (e.g. `test_archive_extractor.py`). New tests should import and exercise `utils/archive_extractor`, `OptiScalerManager` (use temp dirs), and `progress_manager` behavior.
 - Avoid network in unit tests: patch `requests.get` and `OptiScalerManager._download_latest_release` return values.
 - For UI-related changes, prefer testing underlying logic (manager, extractor) rather than rendering Tk windows. When needed, ensure tests clean up created batch files and cache directories.

6) Files & code examples to cite when generating edits
 - Entry / frozen handling: `src/main.py` (setup_environment, requirements check UI fallback)
 - UI orchestration & async patterns: `src/gui/main_window.py` (use of `after`, threading, progress overlay)
 - Install flow & rules: `src/optiscaler/manager.py` (install_optiscaler, create_uninstaller_script, _determine_install_directory, install_optiscaler_threaded)
 - Archive extraction: `src/utils/archive_extractor.py` (used by manager for validation/extraction)
 - i18n files: `src/translations/en.json`, `da.json`, `pl.json`
 - Entry-level scripts: `start_gui.bat`, `build.py`, `build_executable.bat`, `run_progress_tests.bat`.

7) Safe code edit rules for AI agents
 - Preserve user-facing strings by using `t()` i18n helper; add new keys to `src/translations/*.json` when adding visible text.
 - Avoid introducing blocking calls on the main UI thread; if you add IO/HTTP, wrap it in a background thread and surface progress through `progress_manager`.
 - When changing install/uninstall behavior, update the uninstaller batch generation in `OptiScalerManager` to keep parity with current script expectations.
 - When adding new external calls (network or subprocess), follow existing patterns: timeouts, exception handling, and debug_log() usage.

8) When you need clarification from humans
 - If a change touches distribution (PyInstaller spec, bundled Python runtime, or portable ZIP layout), ask which release target to test (portable .zip vs .exe installer).
 - If a new dependency is required, confirm whether it should be bundled in the portable build or added to `requirements.txt` only.

Feedback request
 - I added concise, repo-specific guidance and examples. Please review and point out any missing files or developer workflows you want included (CI steps, release checklist, or additional test targets).
