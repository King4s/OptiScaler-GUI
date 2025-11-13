@echo off
REM
REM Test Environment Initialization Script
REM Sets up test_env directory with all necessary subdirectories
REM Run this script after cloning the repository
REM

setlocal enabledelayedexpansion

echo.
echo ========================================
echo OptiScaler-GUI Test Environment Setup
echo ========================================
echo.

REM Check if we're in the right directory
if not exist ".github" (
    echo ERROR: Please run this script from the OptiScaler-GUI root directory
    echo.
    exit /b 1
)

REM Create main test_env directory
echo Creating test_env directory structure...
if not exist "test_env" (
    mkdir test_env
    echo [+] Created test_env/
) else (
    echo [*] test_env/ already exists
)

REM Create subdirectories
set dirs=test_env\fixtures test_env\fixtures\archives test_env\fixtures\ini_configs test_env\mock_games test_env\cache test_env\cache\optiscaler_downloads test_env\cache\extracted test_env\outputs test_env\outputs\logs test_env\outputs\reports

for %%D in (%dirs%) do (
    if not exist "%%D" (
        mkdir "%%D"
        echo [+] Created %%D/
    ) else (
        echo [*] %%D/ already exists
    )
)

REM Copy sample configuration if it doesn't exist
echo.
echo Adding sample configurations...
if exist "test_env\fixtures\OptiScaler.ini.sample" (
    echo [+] OptiScaler.ini.sample already exists
) else (
    echo [*] OptiScaler.ini.sample will be created on first file write
)

REM Create a .gitkeep file in empty directories to preserve structure
echo.
echo Preserving directory structure...
echo.> test_env\outputs\logs\.gitkeep
echo.> test_env\outputs\reports\.gitkeep
echo [+] Added .gitkeep files

REM Display summary
echo.
echo ========================================
echo Setup Complete!
echo ========================================
echo.
echo Test Environment Structure:
echo   test_env/
echo   ├── fixtures/           (Test data and samples)
echo   │   ├── archives/       (Test archives)
echo   │   ├── ini_configs/    (Sample INI files)
echo   │   └── OptiScaler.ini.sample
echo   ├── mock_games/         (Mock game directories)
echo   ├── cache/              (Cached downloads)
echo   │   ├── optiscaler_downloads/
echo   │   └── extracted/
echo   └── outputs/            (Test results)
echo       ├── logs/
echo       └── reports/
echo.
echo Next Steps:
echo   1. Copy test archives to test_env\fixtures\archives\
echo   2. Create mock game directories as needed
echo   3. Run tests: python test_archive_extractor.py
echo   4. Check results in test_env\outputs\
echo.
echo For more information, see test_env\README.md
echo.

endlocal
