# OptiScaler-GUI v0.3.0 - Portable Version (moved to `docs/PORTABLE_README.md`)

![OptiScaler-GUI](https://img.shields.io/badge/Version-0.3.0-blue) ![License](https://img.shields.io/badge/License-MIT-green) ![Platform](https://img.shields.io/badge/Platform-Windows-lightgrey)

🚀 **Experimental Portable Version with Bundled Dependencies**

OptiScaler-GUI v0.3.0 er en tidlig udviklings-version med portable funktionalitet! Denne version inkluderer alle nødvendige dependencies og værktøjer, hvilket gør den komplet selvstændig og klar til at køre på ethvert Windows system uden at kræve Python eller yderligere installationer.

⚠️ **Bemærk: Dette er en tidlig udviklingsversion (0.3.0) - brug på eget ansvar og rapporter gerne bugs!**

## 📦 Hvad er nyt i v0.3.0

### ✅ Complete Portable Solution
- **Zero Dependencies**: No Python installation required
- **Self-Contained**: All libraries and tools bundled internally
- **Bundled 7z.exe**: Complete archive extraction support (~0.54MB)
- **Translation Support**: Multi-language interface works in portable mode
- **Enhanced Performance**: Optimized for standalone execution

### 🎮 Enhanced OptiScaler Detection
- **Unreal Engine Support**: Detects OptiScaler in `Engine/Binaries/Win64`
- **Improved Game Scanner**: More robust detection methods
- **Real-time UI Updates**: Interface refreshes after installation/uninstallation

### 🔧 Advanced Archive Extraction
- **Multi-Method Fallback**: System 7z → Bundled 7z → py7zr → zipfile
- **Complete Compatibility**: Handles all OptiScaler archive formats
- **Enhanced Error Handling**: Detailed feedback and recovery options

### 🌐 Translation System Improvements
- **PyInstaller Compatibility**: Fixed translation loading in portable mode
- **Proper Fallbacks**: Graceful handling of missing translations
- **Multi-language Support**: Danish, English, Polish included

## 📋 How to Use the Portable Version

### 🔽 Download and Setup
1. **Download**: Get `OptiScaler-GUI-v0.3.0-Portable.zip` from the releases page
2. **Extract**: Unzip the entire folder to your desired location
3. **Run**: Double-click `OptiScaler-GUI.exe` inside the extracted folder

⚠️ **Vigtigt: Dette er en tidlig udviklings-version - test grundigt før brug på vigtige spil!**

This file has been moved to `docs/PORTABLE_README.md` — see the docs folder for full details including folder structure and usage. 

### 📁 Folder Structure
```
OptiScaler-GUI/
├── OptiScaler-GUI.exe          # Main executable
├── 7z.exe                      # Bundled archive extraction tool
├── assets/                     # Icons and resources
├── cache/                      # Game images and downloads cache
├── _internal/                  # Python runtime and libraries
├── README.md                   # Documentation
├── CHANGELOG.md               # Version history
└── requirements.txt           # Dependencies (for reference)
```

### ⚠️ Important Notes
- **Keep Folder Together**: The entire `OptiScaler-GUI` folder must stay intact
- **Don't Move Executable**: Run `OptiScaler-GUI.exe` from inside its folder
- **Windows Only**: This portable version is designed for Windows systems
- **No Installation Required**: Just extract and run!

### 🎯 First Time Usage
1. **Launch the Application**: Run `OptiScaler-GUI.exe`
2. **Game Scanning**: The app will automatically scan for installed games
3. **OptiScaler Management**: Download, install, and manage OptiScaler versions
4. **Language Selection**: Choose your preferred language from the interface

## 🔧 Features

### 🎮 Game Management
- **Automatic Game Detection**: Scans Steam and other game libraries
- **OptiScaler Installation**: Download and install latest versions
- **Unreal Engine Support**: Enhanced detection for UE games
- **Version Tracking**: Keep track of installed OptiScaler versions

### 📦 Archive Support
- **Complete Extraction**: Handles .7z and .zip archives
- **Bundled Tools**: Includes 7z.exe for maximum compatibility
- **Fallback Methods**: Multiple extraction methods for reliability
- **Progress Tracking**: Real-time extraction progress

### 🌍 Multi-Language Support
- **Danish (Dansk)**: Full translation support
- **English**: Default language
- **Polish (Polski)**: Complete interface translation
- **Easy Switching**: Change language from the interface

### 🔍 Advanced Features
- **Game Image Cache**: Automatic game cover downloads
- **Update Checking**: Automatic OptiScaler update detection
- **Compatibility Checking**: Version compatibility verification
- **Debug Mode**: Comprehensive logging for troubleshooting

## 🛠️ Troubleshooting

### 📋 Common Issues

#### ❌ "Application failed to start"
- **Solution**: Ensure the entire `OptiScaler-GUI` folder is extracted
- **Verify**: Check that `_internal` folder exists alongside the executable

#### ❌ "Archive extraction failed"
- **Solution**: The bundled `7z.exe` handles most cases automatically
- **Alternative**: Install 7-Zip system-wide for additional support

#### ❌ "Game not detected"
- **Solution**: Use the manual scan feature or add games manually
- **Note**: Some game launchers may require specific detection methods

#### ❌ "Translation not showing"
- **Solution**: Restart the application to reload translations
- **Fallback**: English will be used if translations are unavailable

### 📝 Debug Information
If you encounter issues:
1. Check the `debug.log` file created next to the executable
2. Run from command line to see console output: `OptiScaler-GUI.exe`
3. Report issues on the GitHub repository with log files

## 🔗 Links and Resources

- **GitHub Repository**: [King4s/OptiScaler-GUI](https://github.com/King4s/OptiScaler-GUI)
- **OptiScaler Project**: Official OptiScaler releases and documentation
- **Issues & Support**: Report bugs and request features on GitHub

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues on the GitHub repository.

---

**🧪 Enjoy testing OptiScaler-GUI Portable v0.3.0!**

*Dette er en tidlig udviklings-version af OptiScaler management funktionaliteten. Rapporter gerne bugs og feedback på GitHub repository.*
