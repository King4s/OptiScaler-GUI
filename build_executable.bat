@echo off
echo 🚀 Building OptiScaler GUI Standalone Executable
echo ===============================================

REM Check if PyInstaller is available
python -c "import PyInstaller" 2>nul
if errorlevel 1 (
    echo ❌ PyInstaller not found. Installing...
    pip install pyinstaller
    if errorlevel 1 (
        echo ❌ Failed to install PyInstaller
        pause
        exit /b 1
    )
)

echo ✅ PyInstaller found

REM Clean previous builds
echo 🧹 Cleaning previous builds...
if exist "dist" rmdir /s /q "dist"
if exist "build" rmdir /s /q "build"

REM Build the executable
echo 🔨 Building executable...
echo This may take several minutes...

python -m PyInstaller build_executable.spec --clean --noconfirm

if errorlevel 1 (
    echo ❌ Build failed!
    pause
    exit /b 1
)

echo ✅ Build completed successfully!
echo.
echo 📁 Executable location:
echo    Single file: dist\OptiScaler-GUI.exe
echo    Portable:    dist\OptiScaler-GUI-Portable\
echo.
echo 🎯 To distribute:
echo    - Single file: Share dist\OptiScaler-GUI.exe
echo    - Portable:    Share entire dist\OptiScaler-GUI-Portable\ folder
echo.

pause
