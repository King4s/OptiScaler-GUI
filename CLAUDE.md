# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

OptiScaler-GUI is a Windows desktop installer/manager for [OptiScaler](https://github.com/optiscaler/OptiScaler) — a DirectX proxy DLL that enables AMD FSR, Intel XeSS, and NVIDIA DLSS upscaling in games. This project is **not** the upscaling engine; it auto-detects games (Steam, Epic, GOG, Xbox Game Pass, Heroic), downloads official OptiScaler releases from GitHub, and installs the payload into game directories.

Unofficial community project, not affiliated with the OptiScaler team.

## IMPORTANT: two codebases live here

**The Rust app in `rust/` is the product.** It ships as a single portable native exe (`v2026.8.0` onward). Default to working there.

**The Python tree in `src/` is legacy.** It is the pre-rewrite Tkinter/CustomTkinter app, still present and still covered by its own CI, but it is not what users download. Only touch it when a change explicitly concerns the legacy app, or to keep a shared behavior in parity — and say which you're doing.

Both stacks implement the same install logic, so a behavioral fix often belongs in **both**, with the Rust side authoritative.

| | Rust (product) | Python (legacy) |
|---|---|---|
| Source | `rust/crates/` | `src/` |
| CI | `ci-rust.yml`, `release-rust.yml` | `ci.yml`, `release.yml` |
| Tests | `cargo test --workspace` | `pytest` (`tests/`) |
| Output | one native exe (~7.5 MB) | PyInstaller portable exe |

## Commands

### Rust (primary) — run from `rust/`

```bash
cargo run -p optiscaler-gui        # run the app
cargo test --workspace             # all tests
cargo fmt --all --check            # CI enforces this
cargo clippy --all-targets --workspace -- -D warnings   # CI enforces this
cargo build --release --workspace  # release build
```

CI gates on **fmt, clippy (`-D warnings`), tests, and a binary size budget** (warn >9 MB, fail >12 MB). Run fmt and clippy before committing or CI will reject the change.

There is also an ignored integration test that downloads a real OptiScaler release:

```bash
cargo test -p opticore extracts_real_optiscaler_archive -- --ignored
```

### Python (legacy)

```bash
python src/main.py                 # run from source
python -m pytest -q                # tests (testpaths = tests/)
python build.py                    # PyInstaller portable exe
```

## Architecture (Rust)

Workspace at `rust/`, two crates:

**`crates/opticore`** — all logic, no UI. Reusable and directly testable.

- `install/` — the core flow. `mod.rs` orchestrates install/update/uninstall; `github.rs` fetches releases and verifies SHA256; `payload.rs` handles marker removal, stale-legacy cleanup, config backup, payload copy, uninstaller script, rollback; `manifest.rs` reads/writes the install manifest that records copied files, directories, proxy filename, and OptiScaler version.
- `scan/` — game detection per store: `steam.rs`, `epic.rs`, `gog.rs`, `xbox.rs`, `heroic.rs`, plus `discovery.rs` (library roots), `folder_facts.rs` (engine/anti-cheat detection), `names.rs`.
- `archive.rs`, `ini.rs`, `config.rs`, `i18n.rs`, `images.rs`, `logging.rs`, `model.rs`, `progress.rs`, `launch.rs`, `selfupdate.rs`, `appids.rs`.

**`crates/optiscaler-gui`** — egui/wgpu GPU-rendered UI.

- `app.rs` (root), `screens/` (`games_grid.rs`, `ini_editor.rs`), `ops.rs` (background operations), `state.rs`, `theme.rs`, `chrome.rs`, `fx/` (animated backgrounds), `winutil.rs`.

Keep logic in `opticore` and UI in `optiscaler-gui` — the split is what makes the install path testable without a window.

## Key patterns and conventions

### Install path detection
If `Engine/Binaries/Win64` exists within the game folder → install there (Unreal Engine). Otherwise → game root. See `payload::determine_install_directory`.

### Proxy filenames
`PROXY_FILENAMES` in `payload.rs` is the supported list (`dxgi.dll` default). `nvngx.dll` is **legacy** — no longer offered for new installs (a game only loads it via the DLSS loader, so OptiScaler often never starts), but kept in `LEGACY_PROXY_FILENAMES` so uninstall still removes it and update migrates away from it.

### Stale/obsolete payload files
`STALE_LEGACY_FILES` (install path) and `LEGACY_UNINSTALL_FILES` (manifest-less uninstall) must stay consistent: a file the uninstaller treats as obsolete should generally also be cleaned on upgrade. Verify claims about payload contents against a real release archive rather than assuming — upstream changes what it ships.

### Safety on install
SHA256 verification of downloads, timestamped `OptiScaler.ini` backup before overwrite, and rollback of copied files on failure. Preserve all three when touching the install flow.

### i18n
English, Danish, Polish. Add new user-visible strings to every language, not just English.

### Anti-cheat
Anti-cheat-protected games require explicit user confirmation before install. Don't weaken or bypass that path.

## Testing guidance

- Rust tests live inline in `#[cfg(test)] mod tests` next to the code. Prefer `tempfile::tempdir()` over touching real game folders.
- Never make real network calls in unit tests; the one test that does is `#[ignore]`d and run explicitly in CI.
- For UI changes, test the underlying `opticore` logic rather than rendering egui.
- Python: `pytest` only collects `tests/` (see `pytest.ini`).

## Upstream OptiScaler compatibility

`.github/optiscaler-version.txt` tracks the last upstream release seen. The `check-optiscaler-release.yml` workflow compares it against the GitHub API daily and opens an issue on a new release — **do not bump that file manually ahead of doing the compatibility work**, or the only automated reminder is silenced.

When verifying compatibility with a new OptiScaler version, inspect the actual release archive's file list; don't infer payload contents from release notes.

## When to ask before proceeding

- A change touching distribution (release workflows, size budget, portable layout) — confirm which target to test.
- A new dependency — it affects the binary size budget, so confirm before adding.
- A change that would alter the Python legacy app's behavior independently of the Rust app.
