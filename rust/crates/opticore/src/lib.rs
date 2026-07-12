//! OptiScaler-GUI core: game scanning, OptiScaler install management, config.
//! This crate has zero GUI dependencies so all logic is testable headless.

pub mod archive;
pub mod model;

/// App version (CalVer), single source of truth for the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
