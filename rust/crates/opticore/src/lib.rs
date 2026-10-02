//! OptiScaler-GUI core: game scanning, OptiScaler install management, config.
//! This crate has zero GUI dependencies so all logic is testable headless.

pub mod advice;
pub mod appids;
pub mod archive;
pub mod config;
pub mod cover_art;
pub mod hardware;
pub mod i18n;
pub mod images;
pub mod ini;
pub mod install;
pub mod launch;
pub mod logging;
pub mod model;
pub mod observations;
pub mod profiles;
pub mod progress;
pub mod report;
pub mod resolver;
pub mod scan;
pub mod selfupdate;
pub mod steam_art;

/// App version (CalVer), single source of truth for the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
