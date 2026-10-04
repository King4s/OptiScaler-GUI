# Development guidance — Rust only

OptiScaler-GUI is an unofficial Windows 10/11 installer/manager, not an upscaling engine.
The legacy Python application, tests, requirements, PyInstaller build and Python CI are removed.
Historical release documents describe their original trees, not current instructions.

## Commands (from rust/)

```sh
cargo run --release --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked
```

Use a private CARGO_TARGET_DIR when another agent shares build caches. No global config changes.

## Architecture

- `rust/crates/opticore`: scanner, resolver, installer, archive decoder, hardware, advice, profiles, reports, INI, artwork and updates.
- `rust/crates/optiscaler-gui`: egui/wgpu app; ops and hardware_view send worker results via mpsc and request repaint. Never collect hardware or download on the UI thread.
- `rust/crates/opticore/data/translations`: embedded en/da/pl; add every new key in all languages.
- `rust/crates/opticore/data/community_verified_games.json`: preserved shared discovery data.
- `assets/`: preserved shared icons; Rust assets/fixtures/shaders must survive cleanup.

Resolver-approved executable and directory are confirmed before install and rechecked after download.
Only complete owned v2 manifests authorize updates/uninstall; hashes, foreign originals, rollback snapshots,
modified INI and unknown fields remain protected. Legacy v1 installs remain preserved, not guessed away.

Hardware is advisory. Never substitute the GUI adapter or first adapter for the game's rendering GPU.
Refresh invalidates per-game enumeration choices. Community RDNA2 runtime requires explicit RX 6000
selection and fresh opt-in, pinned archive + DLL verification, separate cache, private staging and manifest provenance.
RDNA3/4 retain official payload. No recommendation changes settings. Never bypass anti-cheat.
Reports remain preview-and-save only; do not add uploads or telemetry.

Write a failing regression first, exercise real file operations in disposable folders, then implement.
Keep release version and historical release evidence unchanged unless release work is explicitly requested.
See docs/TECHNICAL_OVERVIEW.md, docs/hardware-runtime.md and RELEASE.md.
