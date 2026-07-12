//! Background operations: one mpsc channel from worker threads to the GUI.
//! Workers request a repaint after each send so events render immediately.

use eframe::egui;
use opticore::appids::AppIdResolver;
use opticore::images::ImageCache;
use opticore::install::{self, InstallOptions, InstallStage, Installer};
use opticore::model::Game;
use opticore::progress::TaskEvent;
use opticore::scan::{scan_all, ScanConfig};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

pub struct Ops {
    pub tx: Sender<TaskEvent>,
    pub rx: Receiver<TaskEvent>,
    pub resolver: Arc<AppIdResolver>,
    pub images: Arc<ImageCache>,
    inflight_images: HashSet<String>,
    scan_running: bool,
}

/// Portable layout: cache lives next to the exe (falls back to cwd in dev),
/// same `cache/game_images` naming as the Python app so caches carry over.
fn cache_dir() -> PathBuf {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("cache").join("game_images")
}

impl Ops {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let dir = cache_dir();
        Self {
            tx,
            rx,
            resolver: Arc::new(AppIdResolver::new(&dir)),
            images: Arc::new(ImageCache::new(&dir)),
            inflight_images: HashSet::new(),
            scan_running: false,
        }
    }

    /// Load the SteamSpy catalogue in the background (skipped if cache fresh).
    pub fn spawn_catalogue_load(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let resolver = self.resolver.clone();
        std::thread::spawn(move || {
            if resolver.load_steamspy_catalogue() {
                let _ = tx.send(TaskEvent::Log("SteamSpy catalogue loaded".into()));
            }
            let _ = tx.send(TaskEvent::AppListReady);
            ctx.request_repaint();
        });
    }

    pub fn scan_running(&self) -> bool {
        self.scan_running
    }

    pub fn scan_finished(&mut self) {
        self.scan_running = false;
    }

    pub fn spawn_scan(&mut self, ctx: &egui::Context) {
        if self.scan_running {
            return;
        }
        self.scan_running = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let result = scan_all(&ScanConfig::default());
            let _ = tx.send(TaskEvent::Log(format!(
                "Scan finished: {} games in {:.2}s",
                result.games.len(),
                t0.elapsed().as_secs_f32()
            )));
            let _ = tx.send(TaskEvent::ScanFinished {
                games: result.games,
            });
            ctx.request_repaint();
        });
    }

    /// Fetch artwork for one game (deduped per path).
    pub fn request_image(&mut self, ctx: &egui::Context, game: &Game) {
        let key = game.key.path_norm.clone();
        if !self.inflight_images.insert(key.clone()) {
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let resolver = self.resolver.clone();
        let images = self.images.clone();
        let name = game.name.clone();
        let appid = game.steam_appid;
        std::thread::spawn(move || {
            let appid = appid.or_else(|| resolver.lookup(&name));
            let event = match images.fetch(&name, appid) {
                Some(path) => TaskEvent::ImageReady {
                    path_norm: key,
                    image_path: path,
                },
                None => TaskEvent::ImageMissing { path_norm: key },
            };
            let _ = tx.send(event);
            ctx.request_repaint();
        });
    }

    /// Allow re-requesting images (after AppListReady retry-reset).
    pub fn clear_inflight(&mut self) {
        self.inflight_images.clear();
    }

    fn downloads_dir(&self) -> PathBuf {
        // sibling of cache/game_images → cache/optiscaler_downloads (Python layout)
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("cache").join("optiscaler_downloads")
    }

    /// Check the latest OptiScaler release for update badges.
    pub fn spawn_release_check(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Ok(release) = install::github::fetch_latest_release() {
                let _ = tx.send(TaskEvent::LatestRelease {
                    version: release.version_label(),
                });
                ctx.request_repaint();
            }
        });
    }

    fn stage_label(stage: &InstallStage) -> String {
        match stage {
            InstallStage::FetchingRelease => "Fetching release…".into(),
            InstallStage::Downloading { done, total } if *total > 0 => {
                format!("Downloading… {}%", done * 100 / total)
            }
            InstallStage::Downloading { .. } => "Downloading…".into(),
            InstallStage::Extracting => "Extracting…".into(),
            InstallStage::CopyingPayload { done, total } => {
                format!("Installing files… {done}/{total}")
            }
            InstallStage::Finalizing => "Finalizing…".into(),
        }
    }

    /// Install or update OptiScaler for one game on a worker thread.
    pub fn spawn_install(&self, ctx: &egui::Context, game: &Game, options: InstallOptions) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let key = game.key.path_norm.clone();
        let game_path = game.path.clone();
        let downloads = self.downloads_dir();
        std::thread::spawn(move || {
            let installer = Installer::new(&downloads);
            let progress_tx = tx.clone();
            let progress_ctx = ctx.clone();
            let progress_key = key.clone();
            let result = installer.install(&game_path, &options, move |stage| {
                let _ = progress_tx.send(TaskEvent::OpProgress {
                    path_norm: progress_key.clone(),
                    label: Self::stage_label(&stage),
                });
                progress_ctx.request_repaint();
            });
            let (ok, message) = match result {
                Ok(m) => (
                    true,
                    format!("Installed OptiScaler {}", m.optiscaler_version),
                ),
                Err(e) => (false, e.to_string()),
            };
            let _ = tx.send(TaskEvent::OpFinished {
                path_norm: key,
                ok,
                message,
            });
            ctx.request_repaint();
        });
    }

    /// Uninstall OptiScaler from one game on a worker thread.
    pub fn spawn_uninstall(&self, ctx: &egui::Context, game: &Game) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let key = game.key.path_norm.clone();
        let game_path = game.path.clone();
        std::thread::spawn(move || {
            let (ok, message) = match install::uninstall(&game_path) {
                Ok((files, dirs)) => (
                    true,
                    format!("Removed {} files, {} directories", files.len(), dirs.len()),
                ),
                Err(e) => (false, e.to_string()),
            };
            let _ = tx.send(TaskEvent::OpFinished {
                path_norm: key,
                ok,
                message,
            });
            ctx.request_repaint();
        });
    }
}
