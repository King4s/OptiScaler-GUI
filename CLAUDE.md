# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

OptiScaler-GUI is a Windows desktop installer/manager for [OptiScaler](https://github.com/cdozdil/OptiScaler) — a DirectX proxy DLL that enables AMD FSR, Intel XeSS, and NVIDIA DLSS upscaling in games. The GUI is **not** the upscaling engine itself; it auto-detects games (Steam, Epic, GOG, Xbox Game Pass), downloads OptiScaler releases from GitHub, and installs DLLs into game directories.

- **Language/framework**: Python 3.8+, Tkinter + CustomTkinter (modern dark-themed wrapper)
- **Target platform**: Windows 10/11 only
- **Distribution**: PyInstaller portable .exe

## Commands

```bash
# Run from source
python src/main.py

# Fast dev launcher (Windows)
start_gui.bat

# Install dependencies
pip install -r requirements.txt

# Build portable exe
python build.py

# Validate system requirements
python check_requirements.py

# Run tests (interactive menu)
run_progress_tests.bat

# Run a specific test directly
python test_archive_extractor.py
python test_progress_simple.py
```

Tests live at the repo root (e.g. `test_archive_extractor.py`, `test_progress_simple.py`). Use pytest for new ones.

## Architecture

**Entry → GUI → Manager → Utils** is the layered flow:

- **`src/main.py`** — Entry point. Handles PyInstaller frozen vs. source execution (`sys.frozen` / `sys._MEIPASS`), installs global exception hooks for crash logging, checks system requirements, launches MainWindow.
- **`src/gui/main_window.py`** — Top-level UI orchestration, navigation between frames, progress overlay registration, background-update threading patterns.
- **`src/gui/widgets/`** — Individual UI frames: `game_list_frame.py` (largest, ~46KB), `settings_editor_frame.py`, `global_settings_frame.py`, `library_roots_frame.py`, `log_frame.py`.
- **`src/optiscaler/manager.py`** — Central install logic: download → extract → copy → create uninstaller batch + config. Key methods: `install_optiscaler`, `install_optiscaler_threaded`, `_determine_install_directory`, `create_uninstaller_script`.
- **`src/scanner/game_scanner.py`** (~55KB) — Detects games across Steam/Epic/GOG/Xbox. Reads Steam VDF files, Windows registry, game metadata, AppIDs, engine detection, anti-cheat tracking.
- **`src/scanner/library_discovery.py`** — Discovers game library roots via PowerShell (preferred), registry, or pure-Python fallback. Results are TTL-cached.
- **`src/utils/`** — Shared helpers (see key patterns below).
- **`src/translations/*.json`** — i18n language files (en, da, pl).

## Key patterns and conventions

### Threading
Long-running or blocking work runs in background threads (`threading.Thread, daemon=True`). UI code expects progress callbacks. Use `*_threaded` helpers (e.g. `OptiScalerManager.install_optiscaler_threaded`) when simulating UI flows. Use Tkinter's `.after()` for any UI update from a background thread — never touch widgets directly from non-main threads.

### Progress and UX
Use `progress_manager` and `ProgressOverlay` (`src/utils/progress.py`, registered in `main_window.py`). Call progress callbacks with structured messages.

### Translations (i18n)
Use the `t()` helper from `utils.translation_manager` for **all** visible strings. When adding new visible text, add keys to all three files in `src/translations/*.json`.

### Logging and debug
Use `utils.debug.debug_log()` — never `print()` in GUI contexts. The UI exposes a debug-mode log button that captures this output. Crash logs go to `logs/`.

### Archive extraction
Prefer system `7z.exe` (fastest/most reliable) → `py7zr` (Python fallback) → `zipfile` (last resort, `.zip` only). Always go through `src/utils/archive_extractor.py`. Set `archive_extractor.prefer_system_7z = True` to enforce the preference.

### Install path detection
In `OptiScalerManager._determine_install_directory`: if `Engine/Binaries/Win64` exists within the game folder → install there (Unreal Engine games). Otherwise → game root.

### Network/IO
Downloads use `requests` with timeouts and chunked writes. OptiScaler releases are fetched from GitHub API (`OptiScalerConfig.GITHUB_API_URL` in `manager.py`). Patch `requests.get` in unit tests to avoid real network calls.

### File operations
Use `pathlib.Path` throughout. Uninstaller/installer batch scripts are generated as `.bat` files. When changing install/uninstall behavior, keep the batch script generation in `OptiScalerManager` in sync.

### PyInstaller (frozen) handling
`src/main.py` has setup logic for both source and frozen modes. Preserve both code paths whenever editing startup logic. When touching the spec file or portable ZIP layout, confirm which release target to test.

## Testing guidance

- Unit tests live at the repo root. New tests should exercise `utils/archive_extractor`, `OptiScalerManager` (use temp dirs), and `progress_manager` behavior.
- Avoid real network calls in unit tests: patch `requests.get` and `OptiScalerManager._download_latest_release`.
- For UI changes, test the underlying logic (manager, extractor) rather than rendering Tk windows.
- Clean up any created batch files and cache directories after tests.

## When to ask before proceeding

- If a change touches distribution (PyInstaller spec, bundled runtime, portable ZIP layout) — ask which release target to test.
- If a new dependency is needed — confirm whether it should be bundled in the portable build or only in `requirements.txt`.

## Notable implementation detail (v0.7.9)

DLSS Inputs (AMD/Intel) no longer creates `nvngx.dll`. It only modifies `Dxgi=false` when the user selects "No". Update setup logic accordingly if touching that flow.
