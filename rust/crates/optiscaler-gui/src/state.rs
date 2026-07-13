//! GUI state: screens, game list + filters, texture cache, log ring.

use eframe::egui::{self, TextureHandle};
use opticore::config::AppConfig;
use opticore::i18n::Translator;
use opticore::ini::{GpuVendor, IniDocument};
use opticore::library::Library;
use opticore::model::{Engine, Game, Platform};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Games,
    /// Full game page for `AppState::page_game`.
    GamePage,
    IniEditor,
    Settings,
    Log,
    About,
}

/// Open OptiScaler.ini editing session for one game.
pub struct EditorState {
    pub game_name: String,
    pub ini_path: PathBuf,
    pub doc: IniDocument,
    /// Pristine upstream OptiScaler.ini from the cached release payload,
    /// used for "Restore defaults" and per-key reset.
    pub defaults: Option<IniDocument>,
    pub dirty: bool,
    pub status: Option<String>,
    /// "Section.Key: old → new" lines from Auto Settings / Restore defaults.
    pub applied_changes: Vec<String>,
    pub search: String,
    /// Set when Back was clicked with unsaved changes (second click discards).
    pub discard_armed: bool,
    /// A background download of the release payload (for defaults) is running.
    pub fetching_defaults: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    NotStarted,
    Running,
    Done,
}

/// Games list sort column (persisted as config.sort_key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Platform,
    Engine,
    Optiscaler,
    LastPlayed,
    Playtime,
}

impl SortKey {
    pub const ALL: [SortKey; 6] = [
        SortKey::Name,
        SortKey::Platform,
        SortKey::Engine,
        SortKey::Optiscaler,
        SortKey::LastPlayed,
        SortKey::Playtime,
    ];

    pub fn code(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Platform => "platform",
            SortKey::Engine => "engine",
            SortKey::Optiscaler => "optiscaler",
            SortKey::LastPlayed => "last_played",
            SortKey::Playtime => "playtime",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "platform" => SortKey::Platform,
            "engine" => SortKey::Engine,
            "optiscaler" => SortKey::Optiscaler,
            "last_played" => SortKey::LastPlayed,
            "playtime" => SortKey::Playtime,
            _ => SortKey::Name,
        }
    }

    /// Translation key for the sort label.
    pub fn tr_key(self) -> &'static str {
        match self {
            SortKey::Name => "ui.sort_name",
            SortKey::Platform => "ui.sort_platform",
            SortKey::Engine => "ui.sort_engine",
            SortKey::Optiscaler => "ui.sort_optiscaler",
            SortKey::LastPlayed => "ui.sort_last_played",
            SortKey::Playtime => "ui.sort_playtime",
        }
    }
}

/// Games list presentation, Windows Explorer style (persisted as
/// config.view_mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    CardsLarge,
    CardsSmall,
    List,
    Details,
}

impl ViewMode {
    pub fn code(self) -> &'static str {
        match self {
            ViewMode::CardsLarge => "cards_large",
            ViewMode::CardsSmall => "cards_small",
            ViewMode::List => "list",
            ViewMode::Details => "details",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "cards_small" => ViewMode::CardsSmall,
            "list" => ViewMode::List,
            "details" => ViewMode::Details,
            _ => ViewMode::CardsLarge,
        }
    }
}

/// Artwork status per game (keyed by Game.key.path_norm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtState {
    Unknown,
    Fetching,
    Ready(PathBuf),
    Missing,
}

const TEXTURE_CACHE_CAP: usize = 128;
const LOG_CAP: usize = 2000;

/// Pristine OptiScaler.ini from the newest cached release payload
/// (cache/optiscaler_downloads/extracted/*/OptiScaler.ini).
fn load_default_ini() -> Option<IniDocument> {
    let extracted = crate::ops::base_dir()
        .join("cache")
        .join("optiscaler_downloads")
        .join("extracted");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&extracted)
        .ok()?
        .flatten()
        .map(|e| e.path().join("OptiScaler.ini"))
        .filter(|p| p.exists())
        .collect();
    candidates.sort();
    let newest = candidates.pop()?;
    opticore::ini::read_file(&newest)
}

pub struct AppState {
    pub screen: Screen,
    pub games: Vec<Game>,
    pub scan_state: ScanState,
    pub search: String,
    pub platform_filter: Option<Platform>,
    /// None = all engines.
    pub engine_filter: Option<Engine>,
    /// None = all; Some(true) = OptiScaler installed; Some(false) = not.
    pub optiscaler_filter: Option<bool>,
    /// None = all; Some(true) = has anti-cheat; Some(false) = none detected.
    pub anticheat_filter: Option<bool>,
    /// Show games the user has hidden (session-only toggle).
    pub show_hidden: bool,
    /// Per-game launcher data (favorites/hidden/playtime/launch options).
    pub library: Library,
    pub library_path: PathBuf,
    pub sort_key: SortKey,
    pub sort_ascending: bool,
    pub view_mode: ViewMode,
    pub selected: Option<String>, // path_norm of selected game
    /// Game shown on Screen::GamePage (path_norm).
    pub page_game: Option<String>,
    /// Store metadata per game: absent = not requested, Some(None) = no
    /// store knows the game, Some(Some(..)) = ready.
    pub metadata: HashMap<String, Option<opticore::metadata::GameMetadata>>,
    /// Installed size per game (bytes), filled by a background walk.
    pub sizes: HashMap<String, u64>,
    pub art: HashMap<String, ArtState>,
    pub log: VecDeque<String>,
    /// Latest OptiScaler release tag (for update badges), once checked.
    pub latest_release: Option<String>,
    /// Games with an install/uninstall running: path_norm → progress label.
    pub busy_ops: HashMap<String, String>,
    /// Last finished operation result per game: (ok, message).
    pub op_results: HashMap<String, (bool, String)>,
    /// Anti-cheat confirmation checkbox state in the detail panel.
    pub anticheat_confirmed: bool,
    /// Selected proxy filename in the install flow.
    pub proxy_choice: String,
    /// Open INI editing session (Screen::IniEditor).
    pub editor: Option<EditorState>,
    /// GPU vendor from the wgpu adapter, for Auto Settings.
    pub gpu_vendor: GpuVendor,
    /// System accessibility: animations disabled → effects stay off.
    pub reduced_motion: bool,
    /// Newer GUI release available: (version, url).
    pub gui_update: Option<(String, String)>,
    /// Persisted app configuration (cache/config.json, shared with Python).
    pub config: AppConfig,
    /// Path the config is saved to.
    pub config_path: PathBuf,
    /// Active translations.
    pub i18n: Translator,
    /// On-disk app log (logs/optiscaler-gui.log next to the exe).
    pub file_log: Option<opticore::logging::FileLog>,
    textures: HashMap<String, TextureHandle>,
    texture_lru: VecDeque<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::Games,
            games: Vec::new(),
            scan_state: ScanState::NotStarted,
            search: String::new(),
            platform_filter: None,
            engine_filter: None,
            optiscaler_filter: None,
            anticheat_filter: None,
            show_hidden: false,
            library: Library::default(),
            library_path: PathBuf::new(),
            sort_key: SortKey::Name,
            sort_ascending: true,
            view_mode: ViewMode::CardsLarge,
            selected: None,
            page_game: None,
            metadata: HashMap::new(),
            sizes: HashMap::new(),
            art: HashMap::new(),
            log: VecDeque::new(),
            latest_release: None,
            busy_ops: HashMap::new(),
            op_results: HashMap::new(),
            anticheat_confirmed: false,
            proxy_choice: "dxgi.dll".to_string(),
            editor: None,
            gpu_vendor: GpuVendor::Unknown,
            reduced_motion: false,
            gui_update: None,
            config: AppConfig::default(),
            config_path: PathBuf::new(),
            i18n: Translator::default(),
            file_log: None,
            textures: HashMap::new(),
            texture_lru: VecDeque::new(),
        }
    }
}

impl AppState {
    pub fn dark(&self) -> bool {
        self.config.theme != "light"
    }

    /// Effects render only when enabled and the system allows animations.
    pub fn effects_active(&self) -> bool {
        self.config.effects_enabled && !self.reduced_motion
    }

    /// Open the INI editor for a game (install dir resolved like the installer).
    pub fn open_editor(&mut self, game: &Game) {
        let install_dir = opticore::install::payload::determine_install_directory(&game.path);
        let ini_path = install_dir.join("OptiScaler.ini");
        if let Some(doc) = opticore::ini::read_file(&ini_path) {
            self.editor = Some(EditorState {
                game_name: game.name.clone(),
                ini_path,
                doc,
                defaults: load_default_ini(),
                dirty: false,
                status: None,
                applied_changes: Vec::new(),
                search: String::new(),
                discard_armed: false,
                fetching_defaults: false,
            });
            self.screen = Screen::IniEditor;
        } else {
            self.push_log(format!("Could not read {}", ini_path.display()));
        }
    }
}

impl AppState {
    pub fn push_log(&mut self, line: String) {
        if let Some(file_log) = &self.file_log {
            file_log.append(&line);
        }
        if self.log.len() >= LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    /// True when any filter beyond the defaults is active.
    pub fn filters_active(&self) -> bool {
        !self.search.is_empty()
            || self.platform_filter.is_some()
            || self.engine_filter.is_some()
            || self.optiscaler_filter.is_some()
            || self.anticheat_filter.is_some()
            || self.show_hidden
    }

    pub fn clear_filters(&mut self) {
        self.search.clear();
        self.platform_filter = None;
        self.engine_filter = None;
        self.optiscaler_filter = None;
        self.anticheat_filter = None;
        self.show_hidden = false;
    }

    /// Set the sort column; a second click on the same column flips the
    /// direction (Windows Explorer behaviour). Persisted via config.
    pub fn set_sort(&mut self, key: SortKey) {
        if self.sort_key == key {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_key = key;
            self.sort_ascending = true;
        }
        self.config.sort_key = self.sort_key.code().to_string();
        self.config.sort_ascending = self.sort_ascending;
        let _ = self.config.save(&self.config_path);
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        self.config.view_mode = mode.code().to_string();
        let _ = self.config.save(&self.config_path);
    }

    /// Indices of games matching every active filter, in the current sort
    /// order (name is always the tiebreaker so equal keys stay stable).
    pub fn filtered_indices(&self) -> Vec<usize> {
        let needle = self.search.to_lowercase();
        let mut indices: Vec<usize> = self
            .games
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                (needle.is_empty() || g.key.name_lower.contains(&needle))
                    && (self.show_hidden || !self.library.entry(&g.key.path_norm).hidden)
                    && self.platform_filter.is_none_or(|p| g.platform == p)
                    && self.engine_filter.is_none_or(|e| g.engine == e)
                    && self
                        .optiscaler_filter
                        .is_none_or(|want| g.optiscaler_installed == want)
                    && self
                        .anticheat_filter
                        .is_none_or(|want| g.anti_cheat.is_empty() != want)
            })
            .map(|(i, _)| i)
            .collect();
        indices.sort_by(|&a, &b| {
            let (ga, gb) = (&self.games[a], &self.games[b]);
            let (la, lb) = (
                self.library.entry(&ga.key.path_norm),
                self.library.entry(&gb.key.path_norm),
            );
            let name = ga.key.name_lower.cmp(&gb.key.name_lower);
            let ordering = match self.sort_key {
                SortKey::Name => name,
                SortKey::Platform => ga.platform.label().cmp(gb.platform.label()).then(name),
                SortKey::Engine => ga.engine.label().cmp(gb.engine.label()).then(name),
                SortKey::Optiscaler => gb
                    .optiscaler_installed
                    .cmp(&ga.optiscaler_installed)
                    .then(name),
                // ISO timestamps compare lexicographically; ascending shows
                // most recently played / most played first (like Optiscaler
                // shows installed-first).
                SortKey::LastPlayed => lb.last_played.cmp(&la.last_played).then(name),
                SortKey::Playtime => lb.playtime_minutes.cmp(&la.playtime_minutes).then(name),
            };
            let ordering = if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            };
            // Favorites always float to the top, whatever the sort
            lb.favorite.cmp(&la.favorite).then(ordering)
        });
        indices
    }

    /// Number of games the hidden-filter is currently suppressing.
    pub fn hidden_count(&self) -> usize {
        self.games
            .iter()
            .filter(|g| self.library.entry(&g.key.path_norm).hidden)
            .count()
    }

    pub fn save_library(&mut self) {
        if let Err(e) = self.library.save(&self.library_path) {
            self.push_log(format!("Failed to save library: {e}"));
        }
    }

    /// Bookkeeping after a successful game launch: stamp "last played".
    pub fn note_launched(&mut self, path_norm: &str) {
        self.library.touch_last_played(path_norm);
        self.save_library();
    }

    pub fn art_state(&self, path_norm: &str) -> ArtState {
        self.art
            .get(path_norm)
            .cloned()
            .unwrap_or(ArtState::Unknown)
    }

    /// Texture for an artwork file, loading + LRU-evicting as needed.
    pub fn texture_for(&mut self, ctx: &egui::Context, image_path: &Path) -> Option<TextureHandle> {
        let key = image_path.to_string_lossy().to_string();
        if let Some(handle) = self.textures.get(&key) {
            // refresh LRU position
            self.texture_lru.retain(|k| k != &key);
            self.texture_lru.push_back(key);
            return Some(handle.clone());
        }
        let bytes = std::fs::read(image_path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let handle = ctx.load_texture(&key, color, egui::TextureOptions::LINEAR);
        self.textures.insert(key.clone(), handle.clone());
        self.texture_lru.push_back(key);
        while self.texture_lru.len() > TEXTURE_CACHE_CAP {
            if let Some(evict) = self.texture_lru.pop_front() {
                self.textures.remove(&evict);
            }
        }
        Some(handle)
    }
}
