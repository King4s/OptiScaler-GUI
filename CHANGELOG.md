# Changelog

## [Unreleased]

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
