# OptiScaler GUI

[![Release](https://img.shields.io/github/v/release/King4s/OptiScaler-GUI)](https://github.com/King4s/OptiScaler-GUI/releases/latest)
[![CI](https://github.com/King4s/OptiScaler-GUI/actions/workflows/ci.yml/badge.svg)](https://github.com/King4s/OptiScaler-GUI/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-blue)](#)

**An unofficial Windows installer and manager for [OptiScaler](https://github.com/optiscaler/OptiScaler).**

All upscaling technology — FSR, XeSS, DLSS integration, frame generation, the in-game overlay — is the work of the [OptiScaler team](https://github.com/optiscaler/OptiScaler). This project does one thing: it makes installing their mod easy. It detects your games, downloads the latest official OptiScaler release, and copies the right files into the right place — no manual extraction, renaming, or INI editing.

> This is a community project, not affiliated with or endorsed by the OptiScaler developers. For questions about OptiScaler itself, use the [official repository](https://github.com/optiscaler/OptiScaler).

## Features

- **One-click install, update, and uninstall** of official OptiScaler releases
- **Game auto-detection** for Steam, Epic Games, GOG Galaxy, Xbox Game Pass, and Heroic Launcher — plus manual folder selection for everything else
- **Engine-aware installation** — detects Unreal Engine games and installs to `Engine/Binaries/Win64`, warns about known anti-cheat risks
- **Settings editor** for `OptiScaler.ini` (runtime tuning is still done in OptiScaler's own Insert-key overlay)
- **Safe updates** — SHA256 verification of downloads, config backup, and rollback on failed installs
- **Portable** — single ~14 MB ZIP with Python runtime and 7-Zip bundled, nothing to install
- **Languages:** English, Danish, Polish

## Getting started

1. Download the portable ZIP from the [latest release](https://github.com/King4s/OptiScaler-GUI/releases/latest)
2. Extract it and run `OptiScaler-GUI.exe`
3. Scan for games (or browse to a game folder manually), select a game, click **Install**
4. Launch the game and press **Insert** to configure upscaling in OptiScaler's overlay

Requires Windows 10/11. The GUI downloads OptiScaler exclusively from the official GitHub releases and works offline otherwise — no data collection.

## OptiScaler compatibility

The current release (v0.5.2) supports OptiScaler **v0.7.0 through v0.9.3** and always downloads the latest official release. When a new OptiScaler version changes the payload layout, a compatibility update is released — see the [changelog](CHANGELOG.md) and [releases](https://github.com/King4s/OptiScaler-GUI/releases) for history.

## Running from source

```bash
git clone https://github.com/King4s/OptiScaler-GUI.git
cd OptiScaler-GUI
pip install -r requirements.txt   # Python 3.8+
python src/main.py
```

Run the tests with `pytest -q`, and build the portable package with `python build.py` (requires 7-Zip). Implementation details are covered in the [technical overview](docs/TECHNICAL_OVERVIEW.md).

## Reporting issues

- [Bug report](https://github.com/King4s/OptiScaler-GUI/issues/new?template=bug_report.md)
- [Game compatibility issue](https://github.com/King4s/OptiScaler-GUI/issues/new?template=game_compatibility.md)
- [Feature request](https://github.com/King4s/OptiScaler-GUI/issues/new?template=feature_request.md)

## Credits

- The [**OptiScaler team**](https://github.com/optiscaler/OptiScaler) — for the upscaling technology this project exists to serve. If you find OptiScaler useful, consider [supporting them](https://github.com/sponsors/cdozdil).
- OptiScaler bundles further excellent work: [fakenvapi](https://github.com/optiscaler/fakenvapi), Nukem's [dlssg-to-fsr3](https://github.com/Nukem9/dlssg-to-fsr3), AMD FidelityFX SDK, and Intel XeSS SDK.

## License

MIT — see [LICENSE](LICENSE). OptiScaler and its bundled components are licensed by their respective authors.
