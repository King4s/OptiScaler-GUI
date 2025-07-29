# OptiScaler GUI 🎮🚀

[![OptiScaler](https://img.shields.io/badge/OptiScaler-Official%20Project-blue?logo=github)](https://github.com/optiscaler/OptiScaler)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://python.org)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://microsoft.com/windows)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/King4s/OptiScaler-GUI)](https://github.com/King4s/OptiScaler-GUI/releases)

**A user-friendly GUI wrapper for OptiScaler - making upscaling technologies accessible to everyone!**

Transform your gaming experience with AMD FSR, Intel XeSS, and NVIDIA DLSS through an intuitive interface.

**Version: 0.3.0** | **Status: Early Development** | **🚀 NEW: Portable Version Available!**

## 🆕 What's New in v0.3.0

### 📦 **Portable Version - No Installation Required!**
- **🎯 Download & Run**: Get the portable ZIP, extract, and run - no Python installation needed!
- **📋 Complete Package**: All dependencies bundled including Python runtime and 7z.exe
- **🔧 Self-Contained**: Works on any Windows system without additional software
- **💿 Size**: ~143 MB portable package

### 🚀 **Download Options**

#### 🎯 **For End Users (Recommended)**
**[📥 Download Portable Version v0.3.0](https://github.com/King4s/OptiScaler-GUI/releases/tag/v0.3.0)**
- ✅ No Python installation required
- ✅ No dependencies to install  
- ✅ Just download, extract, and run!

#### 🛠️ **For Developers**
- Clone repository and run from source (see Development section below)

## ✨ What is OptiScaler GUI?

OptiScaler GUI is a **powerful, user-friendly interface** for the official [OptiScaler](https://github.com/optiscaler/OptiScaler) project. It eliminates the complexity of manual installation and configuration, making advanced upscaling technologies accessible to all gamers.

### 🎯 Key Features

- **🔍 Automatic Game Detection**: Scans Steam library and detects installed games
- **📦 One-Click Installation**: Download and install OptiScaler with a single click
- **🔧 Intelligent Configuration**: Smart setup for AMD FSR, Intel XeSS, and NVIDIA DLSS
- **🛡️ Robust Architecture**: Multi-tier fallback systems for maximum compatibility
- **🌐 Complete Portable Support**: Bundled 7z.exe and all dependencies included
- **🎮 Unreal Engine Support**: Enhanced detection for UE games (Engine/Binaries/Win64)
- **💾 Backup & Restore**: Safe installation with automatic backup of original files
- **📊 Real-Time Progress**: Visual feedback during downloads and installations
- **🔄 Update Management**: Automatic checking for latest OptiScaler releases
- **🌍 Multi-Language**: Support for Danish, English, and Polish

### 🎮 What is OptiScaler?

[OptiScaler](https://github.com/optiscaler/OptiScaler) is a DirectX proxy DLL that enables:
- **AMD FSR 1.0, 2.0, 3.0** - FidelityFX Super Resolution
- **Intel XeSS** - Xe Super Sampling  
- **NVIDIA DLSS** - Deep Learning Super Sampling

**This GUI makes OptiScaler accessible to everyone** - no more manual file copying or configuration editing!

## 🚀 Quick Start

### 🎯 **Option 1: Portable Version (Recommended for Users)**

1. **[📥 Download the Portable Version](https://github.com/King4s/OptiScaler-GUI/releases/tag/v0.3.0)**
2. **Extract** the ZIP file to your desired location
3. **Run** `OptiScaler-GUI.exe` inside the extracted folder
4. **Done!** No installation or Python setup required

⚠️ **Note**: This is an early development version (0.3.0) - test thoroughly before using on important games!

### 🛠️ **Option 2: Development Setup**

#### Prerequisites
- **Windows 10/11** 
- **Python 3.8+** ([Download Python](https://python.org))
- **Steam** (for automatic game detection)

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
2. **Scan for games** - Automatically detects your Steam library
3. **Select a game** from the detected list
4. **Choose OptiScaler version** (latest recommended)
5. **Click Install** - GUI handles everything automatically!
6. **Configure settings** for optimal performance
7. **Launch your game** and enjoy enhanced performance!

#### Additional Features:
- **📁 Manual Path Selection**: Browse for games not automatically detected
- **🌍 Multi-Language Support**: Danish, English, Polish (auto-detection)
- **🔄 Auto-Updates**: OptiScaler version checking and updates
- **📋 Progress Tracking**: Real-time installation progress with detailed logs

## 🏗️ Architecture Highlights

### Robust Multi-Tier Systems
- **Archive Extraction**: System 7z.exe → py7zr → zipfile fallback
- **Installation Methods**: Direct install → backup & restore → fallback modes
- **Error Handling**: Comprehensive error detection and user guidance
- **Compatibility**: Works on all Windows systems regardless of installed tools

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
# Test core functionality
python test_archive_extractor.py    # Test archive extraction
python test_progress_simple.py      # Test progress systems
python check_requirements.py        # Validate environment

# Run comprehensive tests
./run_progress_tests.bat            # Full test suite
```

### Building Portable Version
```bash
# Build executable with PyInstaller
python build.py                     # Creates portable .exe in dist/
```

## 📦 Release Information

**Current Version**: 0.3.0 (Development Release)
- **Portable Version Available**: Self-contained executable with all dependencies
- **Size**: ~143 MB (includes Python runtime and all libraries)
- **Compatibility**: Windows 10/11, no Python installation required

For full release notes and downloads, visit: [GitHub Releases](https://github.com/King4s/OptiScaler-GUI/releases)

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **OptiScaler Team** - For the incredible upscaling technology
- **Python Community** - For the amazing libraries that make this possible
- **Gaming Community** - For feedback and testing

---

**Made with ❤️ for the gaming community**

*Bringing cutting-edge upscaling technology to everyone, one click at a time.*