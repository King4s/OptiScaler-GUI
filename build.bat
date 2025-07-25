@echo off
echo ========================================
echo OptiScaler-GUI Standalone Builder
echo ========================================
echo.

REM Check if Python is available
python --version >nul 2>&1
if errorlevel 1 (
    echo Error: Python is not installed or not in PATH
    echo Please install Python 3.8+ and try again
    pause
    exit /b 1
)

echo Building standalone OptiScaler-GUI...
echo.

REM Run the build script
python build.py

echo.
echo Build process completed!
pause
