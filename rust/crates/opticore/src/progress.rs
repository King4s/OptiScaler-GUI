//! Events flowing from core worker threads to the GUI.
//! The GUI drains one mpsc receiver at the top of each frame; workers hold
//! Sender clones and request a repaint after sending.

use crate::model::Game;
use std::path::PathBuf;

#[derive(Debug)]
pub enum TaskEvent {
    /// A full scan finished (the scan itself takes ~0.2s, so results arrive
    /// in one batch rather than streamed per platform).
    ScanFinished { games: Vec<Game> },
    /// Artwork for a game is ready on disk.
    ImageReady {
        path_norm: String,
        image_path: PathBuf,
    },
    /// Artwork lookup gave up (no appid / download failed) — stop showing a spinner.
    ImageMissing { path_norm: String },
    /// The SteamSpy catalogue finished loading — retry missing artwork.
    AppListReady,
    /// Install/update/uninstall progress for one game.
    OpProgress { path_norm: String, label: String },
    /// Install/update/uninstall finished for one game.
    OpFinished {
        path_norm: String,
        ok: bool,
        message: String,
    },
    /// Latest OptiScaler release tag (for update badges).
    LatestRelease { version: String },
    /// A newer GUI release exists (version, html url).
    GuiUpdateAvailable { version: String, url: String },
    /// A log line for the log screen.
    Log(String),
}
