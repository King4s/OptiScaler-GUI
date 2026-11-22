//! Background operations: one mpsc channel from worker threads to the GUI.
//! Workers request a repaint after each send so events render immediately.

use eframe::egui;
use opticore::cover_art::CoverCache;
use opticore::install::{self, InstallOptions, InstallStage, Installer};
use opticore::model::Game;
use opticore::progress::TaskEvent;
use opticore::scan::{scan_all, ScanConfig};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

pub struct Ops {
    pub tx: Sender<TaskEvent>,
    pub rx: Receiver<TaskEvent>,
    pub images: Arc<CoverCache>,
    inflight_images: HashSet<String>,
    image_generations: HashMap<String, u64>,
    scan_running: bool,
    cache_clear_running: bool,
}

/// Portable layout root: the exe's directory (cwd fallback in dev). The
/// cache/ subtree matches the Python app so caches and config carry over.
pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn cache_dir() -> PathBuf {
    base_dir().join("cache").join("game_images")
}

pub(crate) fn downloads_dir() -> PathBuf {
    base_dir().join("cache").join("optiscaler_downloads")
}

impl Ops {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let dir = cache_dir();
        Self {
            tx,
            rx,
            images: Arc::new(CoverCache::new(&dir)),
            inflight_images: HashSet::new(),
            image_generations: HashMap::new(),
            scan_running: false,
            cache_clear_running: false,
        }
    }

    pub fn scan_running(&self) -> bool {
        self.scan_running
    }

    pub fn scan_finished(&mut self) {
        self.scan_running = false;
    }

    pub fn spawn_scan(&mut self, ctx: &egui::Context, config: ScanConfig) {
        if self.scan_running {
            return;
        }
        self.scan_running = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let result = scan_all(&config);
            for message in result.notes.iter().chain(&result.warnings) {
                let _ = tx.send(TaskEvent::Log(message.clone()));
            }
            let _ = tx.send(TaskEvent::Log(format!(
                "Scan finished: {} games in {:.2}s",
                result.games.len(),
                t0.elapsed().as_secs_f32()
            )));
            let _ = tx.send(TaskEvent::ScanFinished {
                games: result.games,
                warnings: result.warnings,
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
        let generation = *self.image_generations.entry(key.clone()).or_default();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let images = self.images.clone();
        let game = game.clone();
        std::thread::spawn(move || {
            let event = match images.fetch(&game) {
                Some(path) => TaskEvent::ImageReady {
                    path_norm: key,
                    image_path: path,
                    generation,
                },
                None => TaskEvent::ImageMissing {
                    path_norm: key,
                    generation,
                },
            };
            let _ = tx.send(event);
            ctx.request_repaint();
        });
    }

    /// Allow re-requesting images (after AppListReady retry-reset).
    pub fn clear_inflight(&mut self) {
        self.inflight_images.clear();
    }

    pub fn reset_image(&mut self, key: &str) {
        self.inflight_images.remove(key);
        *self.image_generations.entry(key.to_string()).or_default() += 1;
    }

    pub fn clear_artwork_cache(&mut self, ctx: &egui::Context) {
        if self.cache_clear_running {
            return;
        }
        self.cache_clear_running = true;
        let images = self.images.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let error = images.clear_automatic_cache().err();
            let _ = tx.send(TaskEvent::ArtworkCacheCleared { error });
            ctx.request_repaint();
        });
    }

    pub fn cache_clear_finished(&mut self) {
        self.cache_clear_running = false;
    }

    pub fn image_is_current(&self, key: &str, generation: u64) -> bool {
        self.image_generations.get(key).copied().unwrap_or_default() == generation
    }

    fn downloads_dir(&self) -> PathBuf {
        downloads_dir()
    }

    pub fn spawn_fetch_defaults(&self, ctx: &egui::Context, game_ini: PathBuf, tag: String) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let downloads = self.downloads_dir();
        std::thread::spawn(move || {
            let installer = Installer::new(&downloads);
            let result = opticore::release_defaults::fetch_exact(&tag).and_then(|release| {
                let extracted = installer
                    .prepare_release(&release, |_| {})
                    .map_err(|e| e.to_string())?;
                opticore::ini::read_file(&extracted.join("OptiScaler.ini"))
                    .ok_or_else(|| "No readable OptiScaler.ini in release".into())
            });
            let _ = tx.send(TaskEvent::DefaultsFetched {
                game_ini,
                tag,
                result,
            });
            ctx.request_repaint();
        });
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

    /// Check for a newer GUI release (stable releases only — GitHub's
    /// releases/latest endpoint excludes pre-releases).
    pub fn spawn_gui_update_check(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Ok(release) =
                install::github::fetch_release_from(opticore::selfupdate::GUI_RELEASES_LATEST)
            {
                let latest = release.version_label();
                if install::is_update_available(opticore::VERSION, &latest) {
                    let _ = tx.send(TaskEvent::GuiUpdateAvailable {
                        version: latest,
                        url: release.html_url.unwrap_or_else(|| {
                            "https://github.com/King4s/OptiScaler-GUI/releases".to_string()
                        }),
                    });
                    ctx.request_repaint();
                }
            }
        });
    }

    /// Download + verify the new GUI exe next to the current one. The GUI
    /// shows a "Restart now" button when the Staged phase arrives.
    pub fn spawn_gui_update_download(&self, ctx: &egui::Context) {
        use opticore::progress::GuiUpdatePhase;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let send = |phase: GuiUpdatePhase, label: String| {
                let _ = tx.send(TaskEvent::GuiUpdateStatus { phase, label });
                ctx.request_repaint();
            };
            let Ok(exe) = std::env::current_exe() else {
                send(GuiUpdatePhase::Failed, "cannot locate own exe".into());
                return;
            };
            send(GuiUpdatePhase::Downloading, "0%".into());
            let release = match install::github::fetch_release_from(
                opticore::selfupdate::GUI_RELEASES_LATEST,
            ) {
                Ok(r) => r,
                Err(e) => {
                    send(GuiUpdatePhase::Failed, e.to_string());
                    return;
                }
            };
            let progress_tx = tx.clone();
            let progress_ctx = ctx.clone();
            let mut last_pct = 0;
            let result = opticore::selfupdate::download_update(&release, &exe, |done, total| {
                if let Some(pct) = (done * 100).checked_div(total).map(|p| p as u32) {
                    if pct != last_pct {
                        last_pct = pct;
                        let _ = progress_tx.send(TaskEvent::GuiUpdateStatus {
                            phase: GuiUpdatePhase::Downloading,
                            label: format!("{pct}%"),
                        });
                        progress_ctx.request_repaint();
                    }
                }
            });
            match result {
                Ok(_) => send(GuiUpdatePhase::Staged, release.version_label()),
                Err(e) => send(GuiUpdatePhase::Failed, e.to_string()),
            }
        });
    }

    /// Sequentially update OptiScaler in every outdated install (opt-in
    /// startup auto-update). One worker thread — parallel updates would race
    /// on the shared download cache.
    pub fn spawn_auto_updates(&self, ctx: &egui::Context, games: Vec<Game>) {
        if games.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let downloads = self.downloads_dir();
        std::thread::spawn(move || {
            let _ = tx.send(TaskEvent::Log(format!(
                "Auto-updating OptiScaler in {} game(s)…",
                games.len()
            )));
            for game in games {
                let key = game.key.path_norm.clone();
                let installer = Installer::new(&downloads);
                let progress_tx = tx.clone();
                let progress_ctx = ctx.clone();
                let progress_key = key.clone();
                let result = installer.update(&game.path, "auto", move |stage| {
                    let _ = progress_tx.send(TaskEvent::OpProgress {
                        path_norm: progress_key.clone(),
                        label: Self::stage_label(&stage),
                    });
                    progress_ctx.request_repaint();
                });
                let (ok, message) = match result {
                    Ok(m) => (
                        true,
                        format!("Auto-updated OptiScaler to {}", m.optiscaler_version),
                    ),
                    Err(e) => (false, e.to_string()),
                };
                let _ = tx.send(TaskEvent::OpFinished {
                    path_norm: key,
                    ok,
                    message,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_cover_change_rejects_an_older_automatic_result() {
        let (tx, rx) = channel();
        let mut ops = Ops {
            tx,
            rx,
            images: Arc::new(CoverCache::new(std::path::Path::new("unused-test-cache"))),
            inflight_images: HashSet::from(["game".to_string()]),
            image_generations: HashMap::new(),
            scan_running: false,
            cache_clear_running: false,
        };
        assert!(ops.image_is_current("game", 0));
        ops.reset_image("game");
        assert!(!ops.image_is_current("game", 0));
        assert!(ops.image_is_current("game", 1));
        assert!(!ops.inflight_images.contains("game"));
        assert!(ops.image_is_current("other", 0));
    }
}
