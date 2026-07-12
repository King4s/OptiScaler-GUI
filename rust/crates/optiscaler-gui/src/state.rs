//! GUI state: screens, game list + filters, texture cache, log ring.

use eframe::egui::{self, TextureHandle};
use opticore::model::{Game, Platform};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Games,
    Settings,
    Log,
    About,
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
            textures: HashMap::new(),
            texture_lru: VecDeque::new(),
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
