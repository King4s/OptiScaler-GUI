//! Dev harness: run a full scan against the real machine and print the
//! results — used to verify M1 parity against the Python app's game list.
//!
//! Usage: cargo run --release -p opticore --example scan_cli

use opticore::scan::{scan_all, ScanConfig};
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let result = scan_all(&ScanConfig::default());
    let elapsed = t0.elapsed();

    let mut games = result.games;
    games.sort_by(|a, b| {
        (a.platform.label(), a.key.name_lower.clone())
            .cmp(&(b.platform.label(), b.key.name_lower.clone()))
    });

    let mut per_platform: BTreeMap<&str, usize> = BTreeMap::new();
    for g in &games {
        *per_platform.entry(g.platform.label()).or_default() += 1;
    }

    for g in &games {
        println!(
            "{}|{}|{}|engine={:?}|appid={}|optiscaler={}|anticheat={:?}",
            g.platform.label(),
            g.name,
            g.path.display(),
            g.engine,
            g.steam_appid.map(|a| a.to_string()).unwrap_or_default(),
            g.optiscaler_installed,
            g.anti_cheat,
        );
    }
    println!(
        "RESULT games={} time={:.2}s platforms={:?}",
        games.len(),
        elapsed.as_secs_f32(),
        per_platform
    );
}
