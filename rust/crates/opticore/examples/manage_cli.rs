//! Dev harness: real install/uninstall against a game folder.
//! Usage:
//!   cargo run --release -p opticore --example install_cli -- install <game_dir> [target.dll]
//!   cargo run --release -p opticore --example install_cli -- uninstall <game_dir>

use opticore::install::{uninstall, InstallOptions, InstallStage, Installer};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let action = args.get(1).map(String::as_str).unwrap_or("");
    let game_dir = PathBuf::from(args.get(2).expect("game dir argument"));

    match action {
        "install" => {
            let mut options = InstallOptions {
                overwrite: true,
                ..Default::default()
            };
            if let Some(target) = args.get(3) {
                options.target_filename = target.clone();
            }
            let downloads = std::env::temp_dir().join("opticore-downloads");
            let installer = Installer::new(&downloads);
            let result = installer.install(&game_dir, &options, |stage| match stage {
                InstallStage::FetchingRelease => println!("STAGE fetching release"),
                InstallStage::Downloading { done, total } => {
                    if total > 0 && done % (8 * 1024 * 1024) < 65536 {
                        println!("STAGE downloading {}%", done * 100 / total);
                    }
                }
                InstallStage::Extracting => println!("STAGE extracting"),
                InstallStage::CopyingPayload { done, total } => {
                    if done == total {
                        println!("STAGE payload {done}/{total}");
                    }
                }
                InstallStage::Finalizing => println!("STAGE finalizing"),
            });
            match result {
                Ok(manifest) => {
                    println!(
                        "INSTALL_OK version={} target={} files={} dirs={}",
                        manifest.optiscaler_version,
                        manifest.target_filename,
                        manifest.files.len(),
                        manifest.directories.len()
                    );
                }
                Err(e) => {
                    eprintln!("INSTALL_FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "update" => {
            let downloads = std::env::temp_dir().join("opticore-downloads");
            let installer = Installer::new(&downloads);
            match installer.update(&game_dir, "auto", |_| {}) {
                Ok(manifest) => println!(
                    "UPDATE_OK version={} target={}",
                    manifest.optiscaler_version, manifest.target_filename
                ),
                Err(e) => {
                    eprintln!("UPDATE_FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "uninstall" => match uninstall(&game_dir) {
            Ok((files, dirs)) => {
                println!("UNINSTALL_OK files={} dirs={}", files.len(), dirs.len());
            }
            Err(e) => {
                eprintln!("UNINSTALL_FAILED: {e}");
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("unknown action: {other}");
            std::process::exit(2);
        }
    }
}
