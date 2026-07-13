//! OptiScaler-GUI core: game scanning, OptiScaler install management, config.
//! This crate has zero GUI dependencies so all logic is testable headless.

pub mod appids;
pub mod archive;
pub mod config;
pub mod i18n;
pub mod images;
pub mod ini;
pub mod install;
pub mod launch;
pub mod library;
pub mod logging;
pub mod metadata;
pub mod model;
pub mod procwatch;
pub mod progress;
pub mod scan;

/// App version (CalVer), single source of truth for the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
