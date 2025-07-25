# Changelog

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
