//! GUI state: screens, game list + filters, texture cache, log ring.

use eframe::egui::{self, TextureHandle};
use opticore::config::AppConfig;
use opticore::i18n::Translator;
use opticore::ini::{GpuVendor, IniDocument};
use opticore::model::{Game, Platform};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Games,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    NotStarted,
    Running,
    Done,
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
    pub selected: Option<String>, // path_norm of selected game
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
            selected: None,
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
            });
            self.screen = Screen::IniEditor;
        } else {
            self.push_log(format!("Could not read {}", ini_path.display()));
        }
    }
}

impl AppState {
    pub fn push_log(&mut self, line: String) {
        if self.log.len() >= LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    /// Indices of games matching the current search + platform filter.
    pub fn filtered_indices(&self) -> Vec<usize> {
        let needle = self.search.to_lowercase();
        self.games
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                (needle.is_empty() || g.key.name_lower.contains(&needle))
                    && self.platform_filter.is_none_or(|p| g.platform == p)
            })
            .map(|(i, _)| i)
            .collect()
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
