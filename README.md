# OptiScaler GUI

[![OptiScaler](https://img.shields.io/badge/OptiScaler-Official%20Project-blue?logo=github)](https://github.com/optiscaler/OptiScaler)
[![Python](https://img.shields.io/badge/Python-3.12+-green.svg)](https://python.org)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A graphical user interface for managing the OptiScaler project.

**Version: 0.1.0**

## About

This project is a GUI wrapper for the official [OptiScaler](https://github.com/optiscaler/OptiScaler) project. OptiScaler GUI provides an easy-to-use interface for installing, configuring, and managing OptiScaler across your game library.

## Relationship to OptiScaler

- **Official OptiScaler Project**: https://github.com/optiscaler/OptiScaler
- **What is OptiScaler**: A DirectX proxy DLL that enables AMD FSR, Intel XeSS, and NVIDIA DLSS upscaling technologies in games
- **This GUI Project**: Provides a user-friendly interface to download, install, and configure OptiScaler for your games

This GUI automatically downloads the latest OptiScaler releases from the official repository and handles installation, configuration, and game detection for you.

### Staying Updated with OptiScaler

This project tracks the official OptiScaler releases. To check for updates to the main project:

```bash
git fetch upstream
git log --oneline upstream/master --since="1 week ago"
```