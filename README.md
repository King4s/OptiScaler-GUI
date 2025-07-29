# OptiScaler GUI 🎮🚀

[![OptiScaler](https://img.shields.io/badge/OptiScaler-Official%20Project-blue?logo=github)](https://github.com/optiscaler/OptiScaler)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://python.org)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://microsoft.com/windows)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**A user-friendly GUI wrapper for OptiScaler - making upscaling technologies accessible to everyone!**

Transform your gaming experience with AMD FSR, Intel XeSS, and NVIDIA DLSS through an intuitive interface.

**Version: 0.2.0** | **Status: Active Development**

## ✨ What is OptiScaler GUI?

OptiScaler GUI is a **powerful, user-friendly interface** for the official [OptiScaler](https://github.com/optiscaler/OptiScaler) project. It eliminates the complexity of manual installation and configuration, making advanced upscaling technologies accessible to all gamers.

### 🎯 Key Features

- **🔍 Automatic Game Detection**: Scans Steam library and detects installed games
- **📦 One-Click Installation**: Download and install OptiScaler with a single click
- **🔧 Intelligent Configuration**: Smart setup for AMD FSR, Intel XeSS, and NVIDIA DLSS
- **🛡️ Robust Architecture**: Multi-tier fallback systems for maximum compatibility
- **🌐 Cross-Platform Support**: Works regardless of system 7z installation
- **💾 Backup & Restore**: Safe installation with automatic backup of original files
- **📊 Real-Time Progress**: Visual feedback during downloads and installations
- **🔄 Update Management**: Automatic checking for latest OptiScaler releases

### 🎮 What is OptiScaler?

[OptiScaler](https://github.com/optiscaler/OptiScaler) is a DirectX proxy DLL that enables:
- **AMD FSR 1.0, 2.0, 3.0** - FidelityFX Super Resolution
- **Intel XeSS** - Xe Super Sampling  
- **NVIDIA DLSS** - Deep Learning Super Sampling

**This GUI makes OptiScaler accessible to everyone** - no more manual file copying or configuration editing!

## 🚀 Quick Start

### Prerequisites
- **Windows 10/11** 
- **Python 3.8+** ([Download Python](https://python.org))
- **Steam** (for automatic game detection)

### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/King4s/OptiScaler-GUI.git
   cd OptiScaler-GUI
   ```

2. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

3. **Check system requirements**:
   ```bash
   python check_requirements.py
   ```

4. **Launch the GUI**:
   ```bash
   python src/main.py
   ```
   Or use the convenient batch file:
   ```bash
   start_gui.bat
   ```

### 🎯 Usage

1. **Launch OptiScaler GUI**
2. **Scan for games** - Automatically detects your Steam library
3. **Select a game** from the detected list
4. **Choose OptiScaler version** (latest recommended)
5. **Click Install** - GUI handles everything automatically!
6. **Configure settings** for optimal performance
7. **Launch your game** and enjoy enhanced performance!

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
python test_archive_extractor.py    # Test archive extraction
python test_progress_simple.py      # Test progress systems
python check_requirements.py        # Validate environment
```

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