# OptiScaler-GUI Function Reference

## Main Application Functions

### `src/main.py`

#### `setup_environment()`
**Purpose**: Configure environment for both script and standalone execution
**Parameters**: None
**Returns**: None
**Description**: Handles PyInstaller bundle detection, sets working directory, and configures Python path for imports

#### `main()`
**Purpose**: Main application entry point with error handling
**Parameters**: None  
**Returns**: None
**Description**: Sets up environment, imports modules, configures CustomTkinter, creates main window, and handles startup errors

---

## Main Window (`src/gui/main_window.py`)

### Core Window Management

#### `MainWindow.__init__()`
**Purpose**: Initialize main application window
**Parameters**: Inherits from `ctk.CTk`
**Returns**: MainWindow instance
**Description**: Sets up window properties, creates header/content/footer, initializes game scanner

#### `_create_header()`
**Purpose**: Create header with version, language selector, and debug toggle
**Parameters**: `self`
**Returns**: None
**Description**: Creates version label, OptiScaler credit link, language dropdown, and debug checkbox

#### `_create_main_content()`
**Purpose**: Create main tabbed interface
**Parameters**: `self`
**Returns**: None
**Description**: Creates Games and Settings tabs, initializes tab content areas

#### `_create_footer()`
**Purpose**: Create footer with progress bars
**Parameters**: `self`
**Returns**: None
**Description**: Sets up progress bar display area for download/install feedback

### Navigation Functions

#### `show_game_list()`
**Purpose**: Display game list and hide settings editor
**Parameters**: `self`
**Returns**: None
**Description**: Destroys settings editor, ensures game list exists and is visible, configures grid layout

#### `show_settings_editor(game_path)`
**Purpose**: Open settings editor for specific game
**Parameters**: 
- `game_path` (str): Path to game directory
**Returns**: None
**Description**: Hides game list, creates SettingsEditorFrame for game, handles errors gracefully

### Event Handlers

#### `_on_debug_toggle()`
**Purpose**: Handle debug checkbox state change
**Parameters**: `self`
**Returns**: None
**Description**: Sets global debug state via `set_debug_enabled()`, logs state change

#### `_on_language_change(language_name)`
**Purpose**: Handle language selection change
**Parameters**:
- `language_name` (str): Display name of selected language
**Returns**: None
**Description**: Maps language name to code, calls `i18n.set_language()`, updates UI texts

---

## Game Management (`src/gui/widgets/game_list_frame.py`)

### Game Display

#### `GameListFrame.__init__(master, games, game_scanner, on_edit_settings)`
**Purpose**: Initialize game list display
**Parameters**:
- `master`: Parent widget
- `games` (list): List of Game objects
- `game_scanner`: GameScanner instance
- `on_edit_settings` (callable): Callback for settings editor
**Returns**: GameListFrame instance
**Description**: Creates scrollable frame with game cards, handles image display and button layout

#### `_display_games()`
**Purpose**: Create visual cards for each game
**Parameters**: `self`
**Returns**: None
**Description**: Creates frame for each game with image, name, path, and action buttons

### Game Actions

#### `_install_optiscaler_for_game(game)`
**Purpose**: Install OptiScaler for specific game
**Parameters**:
- `game`: Game object with name and path
**Returns**: None
**Description**: Calls OptiScalerManager.install_optiscaler(), shows success/error dialog, refreshes display

#### `_uninstall_optiscaler_for_game(game)`
**Purpose**: Uninstall OptiScaler from specific game
**Parameters**:
- `game`: Game object with name and path
**Returns**: None
**Description**: Shows confirmation dialog, calls OptiScalerManager.uninstall_optiscaler(), refreshes display

#### `_refresh_display()`
**Purpose**: Refresh game list to update button states
**Parameters**: `self`
**Returns**: None
**Description**: Destroys current widgets and recreates display to reflect installation status changes

---

## Settings Management (`src/gui/widgets/settings_editor_frame.py`)

### Settings Loading

#### `SettingsEditorFrame.__init__(master, game_path, on_back)`
**Purpose**: Initialize settings editor for game
**Parameters**:
- `master`: Parent widget
- `game_path` (str): Path to game directory
- `on_back` (callable): Callback for back navigation
**Returns**: SettingsEditorFrame instance
**Description**: Loads OptiScaler.ini, creates dynamic form widgets based on setting types

#### `_load_settings()`
**Purpose**: Load settings from OptiScaler.ini file
**Parameters**: `self`
**Returns**: None
**Description**: Reads INI file via OptiScalerManager, handles missing files gracefully

### Dynamic Widget Creation

#### `_create_widgets()`
**Purpose**: Create form widgets based on loaded settings
**Parameters**: `self`
**Returns**: None
**Description**: Creates different widget types (checkbox, dropdown, entry) based on setting data types

#### `_create_settings_view()`
**Purpose**: Create organized settings interface
**Parameters**: `self`
**Returns**: None
**Description**: Groups settings by section, adds descriptions, creates save/back buttons

### Settings Actions

#### `_save_settings()`
**Purpose**: Save current form values to INI file
**Parameters**: `self`
**Returns**: None
**Description**: Collects widget values, writes to OptiScaler.ini via OptiScalerManager

#### `_apply_auto_settings()`
**Purpose**: Apply optimized settings based on detected GPU
**Parameters**: `self`
**Returns**: None
**Description**: Detects GPU type, applies appropriate FSR/DLSS/XeSS settings

---

## OptiScaler Management (`src/optiscaler/manager.py`)

### Installation Functions

#### `install_optiscaler(game_path)`
**Purpose**: Download and install OptiScaler to game directory
**Parameters**:
- `game_path` (str): Target game directory path
**Returns**: `bool` - Success status
**Description**: Downloads latest release, extracts files, auto-detects Unreal Engine, copies files to correct location

#### `uninstall_optiscaler(game_path)`
**Purpose**: Remove OptiScaler files from game directory
**Parameters**:
- `game_path` (str): Game directory to clean
**Returns**: `tuple(bool, str)` - Success status and message
**Description**: Auto-detects installation directory, removes all OptiScaler files and folders

#### `is_optiscaler_installed(game_path)`
**Purpose**: Check if OptiScaler is installed in game directory
**Parameters**:
- `game_path` (str): Game directory to check
**Returns**: `bool` - Installation status
**Description**: Checks for key OptiScaler files (OptiScaler.dll, OptiScaler.ini)

### File Management

#### `_download_latest_release()`
**Purpose**: Download latest OptiScaler release archive
**Parameters**: `self`
**Returns**: `str` - Path to downloaded archive, or None if failed
**Description**: Fetches GitHub API, finds .7z asset, downloads with validation, caches locally

#### `_extract_release(archive_path, game_path)`
**Purpose**: Extract OptiScaler archive
**Parameters**:
- `archive_path` (str): Path to downloaded .7z file
- `game_path` (str): Target game directory
**Returns**: `str` - Path to extracted files
**Description**: Uses 7-Zip to extract archive to cache directory

### Settings Functions

#### `read_optiscaler_ini(ini_path)`
**Purpose**: Parse OptiScaler INI file with comments
**Parameters**:
- `ini_path` (str): Path to OptiScaler.ini file
**Returns**: `dict` - Structured settings data with types and comments
**Description**: Parses INI format, extracts comments, infers setting types for UI generation

#### `write_optiscaler_ini(ini_path, settings)`
**Purpose**: Write settings back to INI file
**Parameters**:
- `ini_path` (str): Path to OptiScaler.ini file
- `settings` (dict): Settings data structure
**Returns**: `bool` - Success status
**Description**: Writes INI format preserving comments and structure

---

## Game Detection (`src/scanner/game_scanner.py`)

#### `scan_games()`
**Purpose**: Discover installed games from multiple platforms
**Parameters**: `self`
**Returns**: `list` - List of Game objects
**Description**: Scans Steam, Epic Games, GOG, Xbox directories for installed games

#### `fetch_game_image(game_name, app_id)`
**Purpose**: Download and cache game cover images
**Parameters**:
- `game_name` (str): Game display name
- `app_id` (str): Platform-specific game ID
**Returns**: `str` - Path to cached image file
**Description**: Downloads game images from Steam API, caches locally, handles errors gracefully

---

## Utility Functions (`src/utils/`)

### Debug System (`debug.py`)

#### `set_debug_enabled(enabled)`
**Purpose**: Enable or disable debug output globally
**Parameters**:
- `enabled` (bool): Debug state
**Returns**: None
**Description**: Sets global debug flag, affects all debug_log() calls and DEBUG: print statements

#### `debug_log(message)`
**Purpose**: Output debug message if debug is enabled
**Parameters**:
- `message` (str): Debug message to output
**Returns**: None
**Description**: Prints message with DEBUG: prefix only if debug_enabled is True

### Internationalization (`i18n.py`)

#### `t(key, *args)`
**Purpose**: Translate text key to current language
**Parameters**:
- `key` (str): Translation key
- `*args`: Format arguments for string interpolation
**Returns**: `str` - Translated text
**Description**: Looks up translation in current language, falls back to English, supports formatting

#### `set_language(language_code)`
**Purpose**: Change application language
**Parameters**:
- `language_code` (str): Language code ('en', 'da', 'pl')
**Returns**: `bool` - Success status
**Description**: Sets current language for all future t() calls

### Progress Management (`progress.py`)

#### `show_progress(task_id, message)`
**Purpose**: Display progress indicator in footer
**Parameters**:
- `task_id` (str): Unique identifier for this progress task
- `message` (str): Status message to display
**Returns**: None
**Description**: Shows progress bar and message in main window footer

#### `hide_progress(task_id)`
**Purpose**: Hide progress indicator
**Parameters**:
- `task_id` (str): Task identifier to hide
**Returns**: None
**Description**: Removes progress display from footer when task completes

---

## Build System (`build.py`)

#### `install_pyinstaller()`
**Purpose**: Install PyInstaller if not present
**Parameters**: None
**Returns**: None
**Description**: Checks for PyInstaller import, installs via pip if missing

#### `create_spec_file()`
**Purpose**: Generate PyInstaller specification file
**Parameters**: None
**Returns**: None
**Description**: Creates optimized .spec file with all dependencies and data files

#### `build_executable()`
**Purpose**: Build standalone executable
**Parameters**: None
**Returns**: `bool` - Build success status
**Description**: Runs PyInstaller with generated spec, copies additional files, reports status

## Function Call Flow

### Application Startup
```
main() → setup_environment() → MainWindow.__init__() → 
_create_header() → _create_main_content() → _create_footer() → _load_games()
```

### Game Installation
```
_install_optiscaler_for_game() → install_optiscaler() → 
_download_latest_release() → _extract_release() → file copying → _refresh_display()
```

### Settings Editing
```
show_settings_editor() → SettingsEditorFrame.__init__() → 
_load_settings() → read_optiscaler_ini() → _create_widgets() → _create_settings_view()
```

### Debug Output
```
print("DEBUG: message") → conditional_print() → [check debug_enabled] → 
[if enabled] original_print() OR [if disabled] ignore
```

This comprehensive function reference covers all major operations in OptiScaler-GUI.
