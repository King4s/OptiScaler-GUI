# OptiScaler GUI

[![Release](https://img.shields.io/github/v/release/King4s/OptiScaler-GUI)](https://github.com/King4s/OptiScaler-GUI/releases/latest)
[![CI](https://github.com/King4s/OptiScaler-GUI/actions/workflows/ci.yml/badge.svg)](https://github.com/King4s/OptiScaler-GUI/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-blue)](#)

**An unofficial Windows installer and manager for [OptiScaler](https://github.com/optiscaler/OptiScaler).**

All upscaling technology — FSR, XeSS, DLSS integration, frame generation, the in-game overlay — is the work of the [OptiScaler team](https://github.com/optiscaler/OptiScaler). This project does one thing: it makes installing their mod easy. It detects your games, downloads the latest official OptiScaler release, and copies the right files into the right place — no manual extraction, renaming, or INI editing.

> **This is a community project, not affiliated with or endorsed by the OptiScaler developers.**
> Support and bug reports for this tool go to [this repository's issues](https://github.com/King4s/OptiScaler-GUI/issues) only — **please don't ask the OptiScaler team about this tool.** For questions about OptiScaler itself, use the [official repository](https://github.com/optiscaler/OptiScaler).

![Game library — auto-detected games with artwork and one-click play](docs/screenshots/library-cards.png)

## Features

- **One-click install, update, and uninstall** of official OptiScaler releases
- **Game auto-detection** for Steam, Epic Games, GOG Galaxy, Xbox Game Pass, and Heroic Launcher — plus manual folder selection for everything else
- **Launch games directly** — with or without the OptiScaler proxy (Steam games via the Steam client, Game Pass via the bundled launch helper)
- **Reviewed install targets** — resolves the selected game executable to a target folder, shows both paths before install, and requires explicit selection when the layout is ambiguous; includes the documented Satisfactory target exception
- **Settings editor** for `OptiScaler.ini` with per-key reset and change preview (runtime tuning is still done in OptiScaler's own Insert-key overlay)
- **Transactional install management** — SHA256 verification, v2 ownership manifest, preservation of pre-existing files, and rollback/recovery snapshots
- **Local guidance and report preview** — inspect detected GPU facts and source-linked advice, then review and explicitly save a limited JSON report with your test result
- **Explorer-style library** — large/small cards, list and details views, full sorting and filtering, artwork for every store
- **GPU-rendered UI** (egui/wgpu) with selectable animated backgrounds — and zero idle cost when disabled
- **Portable** — one ~7.5 MB native exe, nothing to install, no runtime dependencies
- **Languages:** English, Danish, Polish

![Details view — sortable columns, filters, and animated background](docs/screenshots/library-details.png)

## Getting started

1. For a publicly released build, download `OptiScaler-GUI.exe` from the [latest release](https://github.com/King4s/OptiScaler-GUI/releases/latest). The local `2026.9.0 Standard` beta is not a public release.
2. Run it — no installation or extraction needed
3. Scan for games (or browse to a game folder manually), select a game, click **Install**
4. Launch the game and press **Insert** (**Alt+Insert** on non-US keyboard layouts) to configure upscaling in OptiScaler's overlay

Requires Windows 10/11. The GUI downloads OptiScaler exclusively from the official GitHub releases. Local hardware and install observations are used for on-device guidance. Reports are previewed in the game panel and saved only after you choose a file; review the JSON before sharing it yourself.

## OptiScaler compatibility

The current release supports OptiScaler **v0.7.0 through v0.9.4** and always downloads the latest official release. When a new OptiScaler version changes the payload layout, a compatibility update is released — see the [releases](https://github.com/King4s/OptiScaler-GUI/releases) for history.

## Project status

The Rust `2026.9.0 Standard` work is offered first as **2026.9.0-beta.1**, a prerelease rather than a stable compatibility claim. Hardware inventory, deterministic advice, local install/runtime observations, an application log, an OptiScaler INI change preview and a limited JSON report preview/export are included. The INI preview is not a settings compatibility guarantee; DLL hints and log timestamps are evidence only, and a detected `Init done` line can be stale. The user reported Fatekeeper working on 2026-09-25, but exact versions, hardware, overlay behavior and uninstall were not recorded. This beta does not establish compatibility with all games.

| Track | Where | Status |
|---|---|---|
| Rust app (CalVer `2026.x`) | `rust/` | Stable users should stay on the latest stable release. The `2026.9.0-beta.1` prerelease is for feedback and has not been validated for general compatibility. |
| Python app (v0.x) | `src/` | ⚠️ **Legacy — phased out.** Final release is [v0.5.2](https://github.com/King4s/OptiScaler-GUI/releases/tag/v0.5.2); security/compatibility fixes only. Existing Python installs with old manifests are preserved, not automatically migrated or removed by this beta. |

## Running from source

```bash
git clone https://github.com/King4s/OptiScaler-GUI.git
cd OptiScaler-GUI/rust
cargo run --release
```

Run the tests with `cargo test --workspace`. The legacy Python app still runs from `src/` (`pip install -r requirements.txt && python src/main.py`); its implementation details are covered in the [technical overview](docs/TECHNICAL_OVERVIEW.md).

## Reporting issues

- [Bug report](https://github.com/King4s/OptiScaler-GUI/issues/new?template=bug_report.md)
- [Game compatibility issue](https://github.com/King4s/OptiScaler-GUI/issues/new?template=game_compatibility.md)
- [Feature request](https://github.com/King4s/OptiScaler-GUI/issues/new?template=feature_request.md)

## Credits

- The [**OptiScaler team**](https://github.com/optiscaler/OptiScaler) — for the upscaling technology this project exists to serve. If you find OptiScaler useful, consider [supporting them](https://github.com/sponsors/cdozdil).
- OptiScaler bundles further excellent work: [fakenvapi](https://github.com/optiscaler/fakenvapi), Nukem's [dlssg-to-fsr3](https://github.com/Nukem9/dlssg-to-fsr3), AMD FidelityFX SDK, and Intel XeSS SDK.

## License

MIT — see [LICENSE](LICENSE). OptiScaler and its bundled components are licensed by their respective authors.
