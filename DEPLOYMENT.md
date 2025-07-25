# OptiScaler-GUI Standalone Deployment Guide

## Building Standalone Executable

### Quick Build (Windows)
1. Double-click `build.bat` 
2. Wait for build to complete
3. Find executable in `dist/OptiScaler-GUI/`

### Manual Build
```bash
# Install PyInstaller
pip install pyinstaller

# Run build script
python build.py
```

## Distribution

### What to Distribute
- The entire `dist/OptiScaler-GUI/` folder
- Contains the executable and all dependencies
- Users run `OptiScaler-GUI.exe` directly

### System Requirements
- Windows 10/11
- No Python installation required
- Internet connection for game images and OptiScaler downloads

## Build Optimization

### Reducing File Size
The build script includes UPX compression. For smaller builds:
1. Install UPX: https://upx.github.io/
2. Place `upx.exe` in PATH
3. Rebuild with `python build.py`

### Debug Mode
To enable console for debugging:
1. Edit `OptiScaler-GUI.spec`
2. Change `console=False` to `console=True`
3. Rebuild

## Troubleshooting

### Build Fails
- Ensure all dependencies are installed: `pip install -r requirements.txt`
- Check Python version (3.8+ required)
- Run from OptiScaler-GUI root directory

### Runtime Issues
- Missing DLL errors: Install Visual C++ Redistributable
- Antivirus warnings: Add exclusion for OptiScaler-GUI folder
- Permissions: Run as administrator if needed

### Performance
- First run may be slower (Windows Defender scanning)
- Subsequent runs should be faster
- Cache directory stores downloaded files

## Development Notes

### File Structure
```
dist/OptiScaler-GUI/
├── OptiScaler-GUI.exe     # Main executable
├── assets/                # UI assets
├── cache/                 # Download cache
├── _internal/            # Python runtime and dependencies
└── README.md             # Documentation
```

### Updating
To update the standalone version:
1. Update source code
2. Run build script again
3. Distribute new `dist/OptiScaler-GUI/` folder
