# Changelog

## [Unreleased]

### v0.5.2 - 2026-07-12

#### Performance
- Full game scan is dramatically faster (measured ~9.5–17 s → ~0.8 s on a real library): each Steam library is now scanned exactly once instead of 2–3 times, Steam manifests are parsed once per library instead of once per game folder (O(N²) → O(N)), and each game folder is walked once for all detections (game/engine/anti-cheat/OptiScaler) instead of 3–4 times.
- No network I/O during scanning — artwork loads in the background after the list renders.
- Game list renders in chunks so large libraries no longer freeze the UI; Steam name lookups use a token index instead of scanning the 50k-entry catalogue per game; cache-size enforcement moved off the scan path; app startup no longer blocks on manifest/catalogue parsing.

#### Fixes
- Duplicate Steam list entries removed — games could previously appear twice (proper name + raw folder name) due to the overlapping scans.
- GOG games without metadata files were tagged with platform "Epic".
- Anti-cheat detection could report a false positive when a folder wasn't readable, and now also detects top-level anti-cheat folders (e.g. `EasyAntiCheat/`).

### v0.5.1 - 2026-07-12

#### Fixes
- Heroic Launcher games are now detected (#3). The scanner previously expected a store file format Heroic doesn't use and always came up empty; it now parses Heroic's real store files for Epic (legendary), GOG, Amazon (nile), and sideloaded apps, including title lookup from the GOG/nile library caches and dedup of entries pointing at the same install directory.

### v0.5.0 - 2026-06-01

#### Compatibility
- Support for OptiScaler v0.9.2 (latest: XeSS 3.0.1 SDK, Depth Aware sharpening, FSR 4.1 + FSR-FG 4.0.0).
- Removed `DlssOverrides` from additional directories — this folder was removed upstream in v0.9.0.
- Removed `amd_fidelityfx_dx12_v2.dll` / `amd_fidelityfx_vk_v2.dll` from stale legacy files — these FFX 2.2 SDK DLLs are still shipped in v0.9.x payloads.
- Updated compatibility notes for v0.9.0–v0.9.2.

### v0.4.3 - 2026-04-30

#### Improvements
- Added SHA256 verification for GitHub release assets when the API provides digest metadata.
- Added runtime/build checks so current OptiScaler `.7z` extraction requires real `7z.exe`.
- Updates now reuse the target proxy filename recorded in the install manifest instead of defaulting to `nvngx.dll`.
- Existing `OptiScaler.ini` files are backed up with a timestamp before overwrite updates.
- Failed installs roll back files copied during the failed operation.
- Anti-cheat installs now require an explicit "continue at own risk" confirmation.
- Added end-to-end install/update/uninstall tests for manifest, target reuse, backup, and rollback.

### v0.4.2 - 2026-04-30

#### Compatibility
- Added support for the current OptiScaler v0.9.1 payload layout.
- Installations now copy v0.9+ bundled runtime files including `fakenvapi.dll`, `libxell.dll`, `libxess_fg.dll`, DLSSG-to-FSR3, and split FidelityFX upscaler/framegeneration DLLs.
- Installations now copy required payload directories such as `D3D12_Optiscaler` and `Licenses`.

#### Fixes
- Removed stale legacy OptiScaler files such as old `nvapi64.dll`/`nvngx.dll` during overwrite installs to match upstream v0.9 update requirements.
- Added explicit handling for OptiScaler archives that require real `7z.exe` because `py7zr` cannot extract BCJ2-compressed archives.
- Updated dynamic setup tests for the v0.7.9+ DLSS Inputs behavior where the GUI no longer creates `nvngx.dll`.

### v0.3.6 - 2025-11-14

#### Improvements
- UI performance: background image loading, reduce main-thread work, batch widget updates.
- Library discovery: PowerShell support, pure-Python fallback, path normalization, TTL caching, Dedup.
- UX: Rescan, Clear discovery cache moved to Settings, small debug indicator, Log back navigation.
- Stability: fixed Tk redraw race conditions and scheduled safer widget destruction.

#### Fixes
- Avoid segmented drawing on the main window by moving image processing off the main thread.
- Fixed syntax error in LibraryRootsFrame import callback.
- Avoid auto rescans when returning from Settings; initial startup forces a rescan.


### 🔄 **OptiScaler v0.7.9 Compatibility Update**

#### Compatibility
- **Expanded Support**: Updated to support OptiScaler v0.7.0 - v0.7.9 (previously v0.7.0 - v0.7.7-pre9)
- **FSR 4.0.2 Support**: Ready for FidelityFX SDK 2.0.0 and FSR 4.0.2/3.1.5 files
- **Game Quirks**: Aligned with latest OptiScaler game-specific workarounds

#### Important v0.7.9 Changes
- **DLSS Inputs Behavior**: No longer creates `nvngx.dll` when user selects "Yes" to DLSS Inputs (AMD/Intel)
  - Only modifies `Dxgi=false` when user selects "No"
  - Reduces installation complexity and redundant files
- **Configuration Updated**: Default config reflects latest OptiScaler structure

### 🔎 Library Discovery & Scanning Improvements

- **PowerShell Game Library Detection (Windows)**: Optional PowerShell-based PoC for enumerating launchers and library roots (used by default when available/enabled).
- **Pure-Python Fallback**: Registry-based fallback (Uninstall keys), WinRT Appx enumeration, psutil/drive heuristics, and VDF support for robust discovery when PowerShell isn't available.
- **Steam `libraryfolders.vdf` parsing**: Try vdf.loads, then JSON parse, with a KV fallback parser to handle old Valve KeyValue format.
- **Path normalization & deduplication**: Normalizes paths for case-insensitive deduplication while preserving original path casing in outputs.
- **Caching (TTL)**: Discovery results cached - configurable TTL under `library_discovery_cache_ttl` (default 86400s = 24 hours).
- **UI Updates**: 'Rescan' and 'Clear discovery cache' buttons added; summary displays 'Scanned X libraries' with breakdown and last-scan time.
- **Safety & Exclusions**: Drive exclusion config and shallow scans for registry/Appx enumerations to avoid unsafe deep scans.

#### Files
- Updated `README.md`: Version compatibility table now includes v0.7.9
- Updated `.github/copilot-instructions.md`: Added v0.7.9 compatibility notes
- Enhanced `OptiScalerManager`: Added v0.7.9+ file support and documentation

---

## 0.3.5 - 2025-07-30

### 🎯 **Ultra-Compact Release - 66% Size Reduction**

#### Size Optimization
- **Dramatic Size Reduction**: From 143 MB down to 48 MB (66% smaller!)
- **Dependency Optimization**: Removed heavy libraries (NumPy, SciPy, OpenBLAS)
- **Minimal Environment**: Created ultra-lean build environment with only essential packages
- **Cache Exclusion**: Prevented bundling of unnecessary cache files in portable build

#### Technical Improvements
- **Aggressive Excludes**: Removed 36+ MB OpenBLAS library and related dependencies
- **Essential-Only Build**: CustomTkinter, Requests, Pillow, CTkMessagebox, VDF, psutil
- **Smart PyInstaller Config**: Enhanced build_executable.spec with optimized excludes
- **Minimal Python Runtime**: Streamlined Python 3.12 environment

#### User Experience
- **Faster Downloads**: 66% smaller portable package
- **Reduced Storage**: Less disk space required
- **Same Functionality**: All features preserved while dramatically reducing size
- **Improved Distribution**: Easier to share and deploy

### Technical Details
- Built with minimal conda environment (optiscaler-minimal)
- Removed mathematical libraries not needed for GUI operations
- Optimized PyInstaller configuration for size-critical deployment
- Maintained full compatibility with all OptiScaler versions (v0.7.0 - v0.7.7-pre9)

---

## 0.2.0 - 2025-07-25

### Added
- **Debug Control System**: Toggle debug output via checkbox in header bar
- **Internationalization (i18n)**: Complete support for English, Danish, and Polish
- **Progress Bar System**: Real-time progress indicators in footer for downloads/installations  
- **Enhanced UI Structure**: Header with version info, language selector, and debug toggle
- **Uninstall Functionality**: Complete OptiScaler removal with smart detection
- **Standalone App Support**: PyInstaller build system with `build.py` and `build.bat`
- **Smart Installation Detection**: Dynamic install/uninstall buttons based on status
- **Enhanced Settings Editor**: Better layout with descriptions and proper navigation

### Fixed
- **Back Button Navigation**: Fixed blank window issue when returning from settings
- **Debug Output Control**: Debug messages now properly controlled via UI toggle
- **Settings Editor Layout**: Improved organization with frames and descriptions
- **Game List Refresh**: UI updates properly after install/uninstall operations

### Technical Improvements
- **Modular Architecture**: Clean separation with `utils/` for cross-cutting concerns
- **Debug System**: Monkey-patched print function with UI control
- **Translation System**: Easy to extend with new languages
- **Error Handling**: Comprehensive error dialogs and user feedback
- **Path Management**: Better handling for both script and executable modes

## 0.1.0 - 2025-07-24

### Added
- Initial project setup with modular folder structure.
- Game scanning for Steam, Epic Games, GOG, and Xbox (common paths).
- Basic GUI to display discovered games.
- OptiScaler installation functionality (downloads and extracts latest release).
- INI file editor with comment parsing and dynamic widget rendering based on setting types (boolean, options, string, int, float).
