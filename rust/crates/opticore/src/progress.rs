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
    /// The release payload was fetched so the INI editor's "Restore defaults"
    /// has a pristine OptiScaler.ini to compare against (None = fetch failed).
    DefaultsFetched {
        ini_path: Option<PathBuf>,
        message: String,
    },
    /// A play session ended (game processes gone) — minutes to credit.
    PlaySession { path_norm: String, minutes: u64 },
    /// Store metadata for the game page arrived (None = no store knows it).
    MetadataReady {
        path_norm: String,
        meta: Option<crate::metadata::GameMetadata>,
    },
    /// Background walk of the game folder finished.
    InstalledSize { path_norm: String, bytes: u64 },
    /// A store connect/refresh/logout finished; the auth file on disk is
    /// already updated — reload it and show the message.
    StoreAuthChanged { ok: bool, message: String },
    /// The GOG account library arrived.
    GogLibraryReady(Vec<crate::stores::gog::GogOwned>),
    /// The Epic account library arrived.
    EpicLibraryReady(Vec<crate::stores::epic::EpicOwned>),
    /// A log line for the log screen.
    Log(String),
}
