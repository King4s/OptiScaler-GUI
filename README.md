# OptiScaler GUI 🎮🚀

[![Release](https://img.shields.io/github/v/release/King4s/OptiScaler-GUI)](https://github.com/King4s/OptiScaler-GUI/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue)](#)
[![OptiScaler](https://img.shields.io/badge/OptiScaler-Official%20Project-blue?logo=github)](https://github.com/optiscaler/OptiScaler)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://python.org)

| Feature             | Status     |
|---------------------|------------|
| 📦 Portable Version    | ✅ v0.4.3   |
| 🎮 Steam Auto-Detection | ✅ Full    |
| 🎮 Epic Games Auto-Detection | ✅ Full    |
| 🎮 GOG Galaxy Auto-Detection | ✅ Full    |
| 🎮 Xbox Game Pass Auto-Detection | ✅ Full    |
| 📁 Manual Path Selection | ✅ All Platforms |
| 🚀 FSR / DLSS / XeSS   | ✅ All 3    |
| 🐍 Python Runtime      | ✅ Bundled  |
| 🌍 Multi-language UI   | ✅ DA / EN / PL |

**An easy-to-use installer and manager for OptiScaler - helps you install the graphics enhancement mod with just a few clicks!**

## 🤔 **What is this project?**

### 📸 **GUI Preview**
> *Coming soon: Animated GIF showing the interface in action*
> 
> **Quick Visual Guide:**
> 1. 🔍 **Scan Games** → Automatically detects supported game libraries
> 2. 🎯 **Select Game** → Choose from detected games list  
> 3. 📦 **Download OptiScaler** → Latest official release is used
> 4. ⚡ **One Click Install** → GUI handles everything!
> 5. 🎮 **Launch & Play** → Enhanced graphics ready!

### **How OptiScaler and OptiScaler-GUI Work Together**

**These are NOT competing tools - they work together!**

🎮 **OptiScaler** (the main project):
- Graphics enhancement mod that improves game performance
- Provides in-game overlay (Insert key) for runtime settings
- Adjust FSR/DLSS/XeSS settings while playing
- Configure upscaling in real-time

🛠️ **OptiScaler-GUI** (this installer):
- Makes installing OptiScaler easier for everyone
- Automatically downloads latest OptiScaler releases
- Detects your games and installs OptiScaler for you
- Manages OptiScaler across multiple games

**Simple workflow:** Use this GUI to install OptiScaler → Then use OptiScaler's in-game overlay to adjust settings

### **What this GUI does**
- **Installs OptiScaler** into your games (replaces manual file copying)
- **Downloads** latest OptiScaler releases automatically  
- **Detects** Steam, Epic Games, GOG, Xbox Game Pass, and manual game paths
- **Manages** OptiScaler installations across multiple games
- **Provides** user-friendly installation for non-technical users

### **What this GUI does NOT do**
- ❌ Replace OptiScaler's built-in overlay
- ❌ Change in-game upscaling settings  
- ❌ Modify OptiScaler's core functionality
- ❌ Work without the official OptiScaler files

**Think of it as**: A desktop installer/manager, like how Steam manages game installations.

**Version: 0.4.3** | **Status: Release** | **🚀 Release: v0.4.3**

## 🆕 What's New in v0.4.3

### 🛡️ **Release and Install Hardening**
- **📦 7z Guaranteed**: Portable releases include `7z.exe`, and release CI verifies it is present.
- **🔐 Download Verification**: GitHub asset SHA256 digests are verified when available.
- **🧾 Install Manifest**: Installs record copied files, folders, proxy filename, and OptiScaler version.
- **↩️ Safer Failure Handling**: Failed installs roll back copied files.
- **💾 Config Backup**: Existing `OptiScaler.ini` files are timestamp-backed-up before overwrite updates.
- **⚠️ Anti-Cheat Confirmation**: Anti-cheat installs require explicit "continue at own risk" confirmation.

### 🚀 **Download Options**

#### 🎯 **For End Users (Recommended)**
**[📥 Download Portable Version v0.4.3](https://github.com/King4s/OptiScaler-GUI/releases/tag/v0.4.3)**

| Package Type | Size | Checksum | Notes |
|-------------|------|----------|-------|
| 📦 Portable ZIP | ~14 MB | *See release page* | No installation required |
| 🛠️ Future: .EXE Installer | *TBD* | *Coming soon* | Auto-install + shortcuts |

**Features:**
- ✅ No Python installation required
- ✅ No dependencies to install  
- ✅ Just download, extract, and run!
- ✅ Bundled 7z.exe for archive extraction
- ✅ Includes Python runtime (3.8+)

#### 🛠️ **For Developers**
- Clone repository and run from source (see Development section below)

## ✨ What is OptiScaler GUI?

**This is an INSTALLATION MANAGER for OptiScaler - not a replacement for its built-in features!**

### 🎯 Key Features

- **🔍 Automatic Game Detection**: Scans supported launchers and detects installed games
- **📦 One-Click Installation**: Download and install OptiScaler with a single click
- **🔧 Intelligent Configuration**: Smart setup for AMD FSR, Intel XeSS, and NVIDIA DLSS
- **🛡️ Robust Architecture**: Multi-tier fallback systems for maximum compatibility
- **🌐 Complete Portable Support**: Bundled 7z.exe and all dependencies included
- **🎮 Unreal Engine Support**: Enhanced detection for UE games (Engine/Binaries/Win64)
- **🧹 Update Cleanup**: Removes stale legacy OptiScaler files during overwrite installs
- **📊 Real-Time Progress**: Visual feedback during downloads and installations
- **🔄 Update Management**: Automatic checking for latest OptiScaler releases
- **🌍 Multi-Language**: Support for Danish, English, and Polish

For implementation details, see [Technical Overview](docs/TECHNICAL_OVERVIEW.md).

### 🎮 What is OptiScaler?

[OptiScaler](https://github.com/optiscaler/OptiScaler) is a DirectX proxy DLL that enables:
- **AMD FSR 1.x/2.x/3.x/4.x** - FidelityFX Super Resolution and frame generation
- **Intel XeSS** - Xe Super Sampling  
- **NVIDIA DLSS** - Deep Learning Super Sampling

**This GUI makes OptiScaler installation accessible to everyone** - no more manual file copying or configuration editing!

### 🔄 **Workflow: Before vs After**

#### **Manual Installation (Before)**
1. Download OptiScaler `.7z` from GitHub releases
2. Extract archive using 7-Zip
3. Navigate to game installation directory
4. Manually copy `OptiScaler.dll` 
5. Rename it to correct proxy DLL name
6. Copy additional FSR/XeSS libraries
7. Create/edit `OptiScaler.ini` configuration
8. Test game launch and troubleshoot issues

**Time: 10-15 minutes per game + troubleshooting**

#### **GUI Installation (After)**
1. Launch OptiScaler-GUI.exe
2. Click "Scan for Games"
3. Select game from detected list
4. Click "Install OptiScaler"
5. Launch game and use Insert key for OptiScaler overlay

**Time: 30 seconds per game**

## 🚀 Quick Start

### 🎯 **Option 1: Portable Version (Recommended for Users)**

1. **[📥 Download the Portable Version](https://github.com/King4s/OptiScaler-GUI/releases/tag/v0.4.3)**
2. **Extract** the ZIP file to your desired location
3. **Run** `OptiScaler-GUI.exe` inside the extracted folder
4. **Done!** No installation or Python setup required

⚠️ **Note**: This is a stable release (0.4.3) - test thoroughly before using on important games!

### 🛠️ **Option 2: Development Setup**

#### Prerequisites
- **Windows 10/11** 
- **Python 3.8+** ([Download Python](https://python.org))
- **Steam, Epic Games, GOG, or Xbox Game Pass** (optional, for automatic game detection)

#### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/King4s/OptiScaler-GUI.git
   cd OptiScaler-GUI
   ```

2. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

3. **Check system requirements** (optional):
   ```bash
   python check_requirements.py
   ```

4. **Launch the GUI**:
   ```bash
   python src/main.py
   ```

### 🏃‍♂️ **Quick Launch Scripts**

For convenience, you can also use:
- **`start_gui.bat`** - Direct launch of the GUI
- **`run_progress_tests.bat`** - Run system tests

### 🎯 Usage

#### For Portable Version Users:
1. **Launch** `OptiScaler-GUI.exe` from the extracted folder
2. **Scan for games** - Automatically detects supported game libraries
3. **Select a game** from the detected list
4. **Click Install** - GUI downloads the latest official OptiScaler release and installs it
5. **Launch your game** and use OptiScaler's in-game overlay for runtime settings

#### Additional Features:
- **📁 Manual Path Selection**: Browse for games not automatically detected
- **🌍 Multi-Language Support**: Danish, English, Polish (auto-detection)
- **🔄 Update Checks**: OptiScaler release checking and per-game update flow
- **📋 Progress Tracking**: Real-time installation progress with detailed logs

## 🏗️ Architecture Highlights

### Robust Multi-Tier Systems
- **Archive Extraction**: Bundled/system 7z.exe for `.7z` → zipfile for `.zip`
- **Installation Methods**: Direct proxy DLL install with manifest-based uninstall/update tracking
- **Error Handling**: Comprehensive error detection and user guidance
- **Compatibility**: Portable release bundles required extraction tooling

### Staying Updated with OptiScaler

This project tracks the official OptiScaler releases. To check for updates to the main project:

```bash
git fetch upstream
git log --oneline upstream/master --since="1 week ago"
```

## 🛠️ Development

### Project Structure
```
OptiScaler-GUI/
├── src/
│   ├── gui/              # User interface components
│   ├── optiscaler/       # OptiScaler management
│   ├── scanner/          # Game detection system
│   ├── utils/            # Utility functions
│   └── translations/     # Multi-language support
├── cache/                # Downloaded files and cache
├── assets/               # GUI assets and icons
└── tests/                # Test suite
```

### Running Tests
```bash
pytest -q                           # Run the test suite
python check_requirements.py         # Validate environment
```

### Building Portable Version
```bash
# Build executable with PyInstaller
python build.py                     # Creates portable .exe in dist/
```

## 📦 Release Information

**Current Version**: 0.4.3 (Release)
- **Portable Version Available**: Self-contained executable with all dependencies
- **Size**: ~14 MB portable ZIP (includes Python runtime and bundled `7z.exe`)
- **Compatibility**: Windows 10/11, no Python installation required

### 🔄 **OptiScaler Version Compatibility**

| GUI Version | Compatible OptiScaler Versions | Status |
|-------------|--------------------------------|--------|
| **v0.4.3** | v0.7.0 - v0.9.1 | ✅ Current |
| v0.4.2 | v0.7.0 - v0.9.1 | 🟡 Legacy |
| v0.3.6 | v0.7.0 - v0.7.9 | 🟡 Legacy |
| v0.3.0 | v0.7.0 - v0.7.7-pre9 | 🟡 Legacy |
| v0.2.0 | v0.6.0 - v0.7.0 | 🟡 Legacy |
| v0.1.0 | v0.5.0 - v0.6.5 | ❌ Deprecated |

### 🔧 **Supported Installation Methods**

- **Proxy DLL Installation**: `dxgi.dll`, `winmm.dll`, `nvngx.dll`
- **Direct Installation**: `OptiScaler.dll` placement
- **Configuration Management**: Automatic `OptiScaler.ini` generation
- **Library Support**: FSR, XeSS, and DLSS companion files
- **Update Cleanup**: Removes stale legacy OptiScaler files on overwrite installs

### 🎮 **Tested Game Engines**

| Engine | Support Level | Notes |
|--------|---------------|-------|
| **Unreal Engine** | ✅ Full | Enhanced detection for UE4/UE5 games |
| **Unity** | ✅ Full | Standard proxy DLL installation |
| **Custom Engines** | ✅ Good | Manual path selection supported |
| **DirectX 11/12** | ✅ Full | Primary target platform |
| **Vulkan** | ✅ Good | OptiScaler handles API translation |

For full release notes and downloads, visit: [GitHub Releases](https://github.com/King4s/OptiScaler-GUI/releases)

## ❓ **Frequently Asked Questions**

### **Q: How is this different from OptiScaler's built-in overlay?**
**A:** OptiScaler has a built-in overlay (Insert key) for in-game configuration. This GUI is a **separate desktop application** that helps you **install** OptiScaler into your games. They serve different purposes:
- **OptiScaler overlay**: Configure FSR/DLSS settings while playing
- **This GUI**: Install OptiScaler files into game directories

### **Q: Is this officially supported by the OptiScaler team?**
**A:** No, this is a **community project** that makes OptiScaler installation easier. It's not affiliated with or endorsed by the official OptiScaler developers.

### **Q: Will this interfere with OptiScaler's functionality?**
**A:** No, this GUI only handles installation. Once installed, OptiScaler works exactly as designed - you still use the Insert key for the in-game overlay.

### **Q: What if I already installed OptiScaler manually?**
**A:** The GUI can detect existing installations and help you update or manage them. It won't break existing setups.

### **Q: Do I still need to configure OptiScaler settings?**
**A:** Yes! After installation, launch your game and press **Insert** to access OptiScaler's built-in configuration overlay. This GUI doesn't replace that functionality.

### **Q: Which OptiScaler versions are supported?**
**A:** Currently supports v0.7.0 through v0.9.1. The GUI automatically downloads from the official OptiScaler releases.

### **Q: Can I use this for games not on Steam?**
**A:** Yes. The GUI can auto-detect several launchers, and you can manually browse and select any game directory.

## 🛣️ **Project Status & Roadmap**

### **Current Status (v0.4.3)**
- ✅ **Stable Installation**: Reliable OptiScaler installation for most games
- ✅ **Portable Distribution**: Self-contained Windows package with bundled `7z.exe`
- ✅ **Multi-Launcher Detection**: Steam, Epic Games, GOG, Xbox Game Pass, and manual paths
- ✅ **Multi-Language**: Danish, English, and Polish translations
- ✅ **Error Recovery**: Robust fallback systems for edge cases

### **Known Limitations**
- 🟡 **Configuration Limited**: Basic INI generation only (use OptiScaler overlay for advanced settings)
- 🟡 **Windows Only**: No Linux/Mac support (follows OptiScaler platform limitations)
- 🟡 **Manual Updates**: GUI updates require manual download (no auto-updater)

### **Planned Features (Future Versions)**
- 🔮 **Backup Management**: Better backup/restore functionality with versioning
- 🔮 **Installation Profiles**: Save and reuse installation configurations
- 🔮 **OptiScaler INI Editor**: Basic configuration editor (complementary to overlay)
- 🔮 **Auto-Update System**: Automatic GUI updates and OptiScaler tracking

### **Community Feedback Needed**
- 📊 **Compatibility Reports**: Which games work well vs need fixes?
- 🐛 **Bug Reports**: Installation failures or edge cases
- 💡 **Feature Requests**: What would make installation easier?
- 🌍 **Language Support**: Additional translation requests

- **OptiScaler** is developed by the talented team at [optiscaler/OptiScaler](https://github.com/optiscaler/OptiScaler)
- **This GUI** is an independent installation manager created by [King4s](https://github.com/King4s)
- All credit for the actual upscaling technology goes to the OptiScaler team
- For OptiScaler support, issues, or questions, please visit the [official repository](https://github.com/optiscaler/OptiScaler)

### **Relationship to OptiScaler**

```
┌─────────────────────────────────────────────────────────────┐
│                     OptiScaler Ecosystem                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  📦 Official OptiScaler                                     │
│  ├── Core upscaling technology (FSR, DLSS, XeSS)           │
│  ├── In-game overlay (Insert key)                          │
│  ├── Runtime configuration                                 │
│  └── DirectX/Vulkan integration                            │
│                                                             │
│  🖥️ This GUI (Community Project)                           │
│  ├── Installation manager                                  │
│  ├── Multi-launcher game detection                         │
│  ├── File download/extraction                              │
│  └── Basic configuration setup                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 🤝 **Community & Support**

### 📢 **Get Help & Report Issues**
- 🐛 [**Bug Reports**](https://github.com/King4s/OptiScaler-GUI/issues/new?template=bug_report.md) - Found a problem? Let us know!
- 💡 [**Feature Requests**](https://github.com/King4s/OptiScaler-GUI/issues/new?template=feature_request.md) - Have an idea? Share it!
- 💬 [**Discussions**](https://github.com/King4s/OptiScaler-GUI/discussions) - General questions and community chat
- 📖 [**Documentation**](https://github.com/King4s/OptiScaler-GUI/wiki) - Guides and troubleshooting

### 🎯 **Supported Games & Compatibility**
- ✅ **Steam Games** - Automatic detection
- ✅ **Epic/GOG/Xbox Games** - Automatic detection where library metadata is available
- ✅ **Unreal Engine** - Enhanced support for UE games  
- ✅ **DirectX 11/12** - Full compatibility
- ⚠️ **Other launchers/custom installs** - Manual path selection may be required
- ❓ **Game not working?** - [Report compatibility issue](https://github.com/King4s/OptiScaler-GUI/issues/new?template=game_compatibility.md)

### 🔒 **Security & Trust**
- 📝 **Open Source** - All code is public and auditable
- 🛡️ **No Data Collection** - GUI works completely offline
- ✅ **Official Sources** - Downloads OptiScaler from official GitHub only
- 🔍 **Checksums** - Verifies GitHub SHA256 asset digests when available

---

## 📄 **License**

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 **Acknowledgments**

- [**OptiScaler Team**](https://github.com/optiscaler/OptiScaler) - For the incredible upscaling technology
- **Python Community** - For the amazing libraries that make this possible  
- **Gaming Community** - For feedback and testing

---

**Made with ❤️ for the gaming community**

*Making OptiScaler installation accessible to everyone, one click at a time.*
