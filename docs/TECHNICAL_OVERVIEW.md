# Technical Overview

## Installation Flow

1. Fetch the latest release metadata from `optiscaler/OptiScaler`.
2. Select the first `.7z` or `.zip` release asset.
3. Download the asset into `cache/optiscaler_downloads`.
4. Extract `.7z` with bundled/system `7z.exe`; extract `.zip` with Python `zipfile`.
5. Copy `OptiScaler.dll` to the selected proxy filename, for example `dxgi.dll`.
6. Copy the remaining release payload dynamically, preserving relative paths.
7. Write `.optiscaler-gui-install.json` with installed files, directories, selected proxy filename, and OptiScaler release version.

## Why 7z.exe Is Required

Current OptiScaler releases can use 7z compression filters such as BCJ2 that `py7zr` cannot extract reliably. The portable release therefore bundles `7z.exe`, and release CI verifies that the ZIP contains it.

## Uninstall And Update Safety

New installs write an install manifest. Uninstall first uses that manifest to remove only files the GUI installed. Legacy installs without a manifest still use the older broad OptiScaler-file detection path for compatibility.

## Game Discovery

The scanner supports Steam, Epic Games, GOG, Xbox Game Pass, Heroic metadata, and manual paths. Library roots are cached to avoid repeated expensive scans. Steam image lookup uses local manifests first and then background SteamSpy data.

## Release Process

Create releases by pushing an annotated `v*` tag. The GitHub release workflow runs tests, builds the portable package on Windows, verifies `7z.exe` is included, and uploads the ZIP to the GitHub release.
