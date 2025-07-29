@echo off
echo ========================================
echo  Progress Bar Testing Suite
echo ========================================
echo.
echo Choose a test to run:
echo.
echo 1. Minimal Test (pure tkinter - no dependencies)
echo 2. Simple Test (CustomTkinter with progress overlay)
echo 3. Advanced Test (Full Settings Editor simulation)
echo 4. Run All Tests
echo.
set /p choice="Enter your choice (1-4): "

if "%choice%"=="1" goto minimal
if "%choice%"=="2" goto simple
if "%choice%"=="3" goto advanced
if "%choice%"=="4" goto all
goto end

:minimal
echo.
echo Running Minimal Test...
python test_progress_minimal.py
goto end

:simple
echo.
echo Running Simple Test...
python test_progress_simple.py
goto end

:advanced
echo.
echo Running Advanced Test...
python test_progress_advanced.py
goto end

:all
echo.
echo Running all tests...
echo.
echo === Test 1: Minimal ===
start "Minimal Test" python test_progress_minimal.py
timeout /t 2 >nul
echo.
echo === Test 2: Simple ===
start "Simple Test" python test_progress_simple.py
timeout /t 2 >nul
echo.
echo === Test 3: Advanced ===
start "Advanced Test" python test_progress_advanced.py
goto end

:end
echo.
echo Test completed!
pause
