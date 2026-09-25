//! Explicit, minimal public report. Never serialize local profile or observation types directly.
use crate::hardware::{GpuProfile, HardwareProfile};
use crate::observations::GameObservation;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserTestResult {
    Passed,
    Failed,
    #[default]
    NotRun,
}

#[derive(Debug, Serialize)]
pub struct PublicReport {
    schema_version: u8,
    hardware: ReportHardware,
    game: ReportGame,
    test_result: UserTestResult,
}

#[derive(Debug, Serialize)]
struct ReportHardware {
    windows_build: Option<String>,
    ram_bytes: Option<u64>,
    gpu: Option<ReportGpu>,
}

#[derive(Debug, Serialize)]
struct ReportGpu {
    model: Option<String>,
    vendor: Option<String>,
    dedicated_bytes: Option<u64>,
    shared_bytes: Option<u64>,
    driver: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportGame {
    platform: Option<String>,
    store_id: Option<String>,
    build_version: Option<String>,
    optiscaler_version: Option<String>,
    installed: bool,
    loaded: Option<bool>,
    user_tested: bool,
}

fn safe_label(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() <= 80
        && !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'-' | b'(' | b')' | b'&')))
    .then(|| value.to_string())
}

fn numeric_version(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() <= 40
        && !value.is_empty()
        && value.bytes().any(|b| b.is_ascii_digit())
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-')))
    .then(|| value.to_string())
}

fn optiscaler_version(value: &str) -> Option<String> {
    let value = value.trim();
    let suffix = value.strip_prefix('v')?;
    (suffix.len() <= 40
        && suffix.bytes().filter(|b| *b == b'.').count() >= 2
        && suffix.bytes().any(|b| b.is_ascii_digit())
        && suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')))
    .then(|| value.to_string())
}

pub fn build_report(
    hardware: &HardwareProfile,
    selected_gpu: Option<&GpuProfile>,
    observation: &GameObservation,
    test_result: UserTestResult,
) -> PublicReport {
    let gpu = selected_gpu.map(|gpu| ReportGpu {
        model: gpu.name.as_deref().and_then(safe_label),
        vendor: gpu.vendor.as_deref().and_then(|value| match value {
            "AMD" | "Intel" | "NVIDIA" => Some(value.to_string()),
            _ => None,
        }),
        dedicated_bytes: gpu.dedicated_bytes,
        shared_bytes: gpu.shared_bytes,
        driver: gpu.driver.as_deref().and_then(numeric_version),
    });
    let platform = match observation.platform.as_str() {
        "Steam" | "Epic" | "GOG" | "Xbox" | "Heroic" | "Manual" | "Installed" => {
            Some(observation.platform.clone())
        }
        _ => None,
    };
    PublicReport {
        schema_version: 1,
        hardware: ReportHardware {
            windows_build: hardware.windows_build.as_deref().and_then(numeric_version),
            ram_bytes: hardware.ram_bytes,
            gpu,
        },
        game: ReportGame {
            platform,
            store_id: observation.store_id.as_deref().and_then(|value| {
                (observation.platform == "Steam"
                    && !value.is_empty()
                    && value.len() <= 20
                    && value.bytes().all(|b| b.is_ascii_digit()))
                .then(|| value.to_string())
            }),
            build_version: observation
                .build_version
                .as_deref()
                .and_then(numeric_version),
            optiscaler_version: observation
                .optiscaler_version
                .as_deref()
                .and_then(optiscaler_version),
            installed: observation.installed,
            loaded: observation.loaded_from_log,
            user_tested: test_result != UserTestResult::NotRun,
        },
        test_result,
    }
}

pub fn preview_json(report: &PublicReport) -> serde_json::Result<String> {
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    Ok(json)
}

/// Save only after the user chose the destination and reviewed the preview.
pub fn write_report(path: &Path, report: &PublicReport) -> std::io::Result<()> {
    let bytes = preview_json(report)
        .map_err(std::io::Error::other)?
        .into_bytes();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}
