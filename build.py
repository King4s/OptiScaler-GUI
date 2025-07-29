#!/usr/bin/env python3
"""
OptiScaler GUI Build Script
Builds standalone executable using PyInstaller
"""

import os
import sys
import subprocess
import shutil
from pathlib import Path

def check_pyinstaller():
    """Check if PyInstaller is installed"""
    try:
        import PyInstaller
        print("✅ PyInstaller found")
        return True
    except ImportError:
        print("❌ PyInstaller not found. Installing...")
        try:
            subprocess.run([sys.executable, '-m', 'pip', 'install', 'pyinstaller'], check=True)
            print("✅ PyInstaller installed successfully")
            return True
        except subprocess.CalledProcessError:
            print("❌ Failed to install PyInstaller")
            return False

def clean_build():
    """Clean previous build artifacts"""
    print("🧹 Cleaning previous builds...")
    
    build_dirs = ['dist', 'build', '__pycache__']
    for build_dir in build_dirs:
        if Path(build_dir).exists():
            shutil.rmtree(build_dir)
    
    # Clean Python cache files
    for root, dirs, files in os.walk('.'):
        for d in dirs[:]:
            if d == '__pycache__':
                shutil.rmtree(Path(root) / d)
                dirs.remove(d)
        for f in files:
            if f.endswith('.pyc'):
                os.remove(Path(root) / f)
    
    print("✅ Cleanup completed")

def build_executable():
    """Build the executable using PyInstaller"""
    print("🔨 Building executable...")
    print("This may take several minutes...")
    
    try:
        # Build using the spec file
        subprocess.run([
            sys.executable, '-m', 'PyInstaller', 
            'build_executable.spec', 
            '--clean', 
            '--noconfirm'
        ], check=True)
        
        print("✅ Build completed successfully!")
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Build failed with error: {e}")
        return False

def show_results():
    """Show build results and instructions"""
    print("\n📁 Build Results:")
    
    single_file = Path('dist/OptiScaler-GUI.exe')
    portable_dir = Path('dist/OptiScaler-GUI-Portable')
    
    if single_file.exists():
        size_mb = single_file.stat().st_size / (1024 * 1024)
        print(f"   ✅ Single file: {single_file} ({size_mb:.1f} MB)")
    else:
        print("   ❌ Single file executable not found")
    
    if portable_dir.exists():
        print(f"   ✅ Portable:    {portable_dir}/")
    else:
        print("   ❌ Portable directory not found")
    
    print("\n🎯 Distribution Options:")
    print("   • Single file: Share OptiScaler-GUI.exe (slower startup)")
    print("   • Portable:    Share entire OptiScaler-GUI-Portable/ folder (faster startup)")
    print("\n💡 Tips:")
    print("   • Single file is easier to distribute but slower to start")
    print("   • Portable version starts faster but requires entire folder")
    print("   • Both versions include all dependencies - no Python installation required")

def main():
    """Main build process"""
    print("🚀 OptiScaler GUI Build Script")
    print("=" * 50)
    
    # Check if we're in the right directory
    if not Path('src/main.py').exists():
        print("❌ Error: Please run this script from the OptiScaler-GUI root directory")
        print("   Expected file: src/main.py")
        sys.exit(1)
    
    # Check dependencies
    if not check_pyinstaller():
        sys.exit(1)
    
    # Clean previous builds
    clean_build()
    
    # Build executable
    if not build_executable():
        sys.exit(1)
    
    # Show results
    show_results()
    
    print("\n🎉 Build process completed!")

if __name__ == "__main__":
    main()

def install_pyinstaller():
    """Install PyInstaller if not already installed"""
    try:
        import PyInstaller
        print("✅ PyInstaller already installed")
    except ImportError:
        print("📦 Installing PyInstaller...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "pyinstaller"])

def create_spec_file():
    """Create PyInstaller spec file for OptiScaler-GUI"""
    spec_content = '''# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

# Collect all data files
datas = [
    ('assets', 'assets'),
    ('cache', 'cache'),
    ('src/translations', 'src/translations'),
]

# Hidden imports for all dependencies
hiddenimports = [
    'customtkinter',
    'PIL',
    'PIL._tkinter_finder',
    'PIL.Image',
    'PIL.ImageTk',
    'requests',
    'CTkMessagebox',
    'psutil',
    'tkinter',
    'tkinter.ttk',
    'tkinter.filedialog',
    'tkinter.messagebox',
    'utils.debug',
    'utils.i18n',
    'utils.progress',
    'utils.cache_manager',
    'utils.config',
    'utils.logging_utils',
    'utils.performance',
    'scanner.game_scanner',
    'optiscaler.manager',
    'gui.main_window',
    'gui.widgets.game_list_frame',
    'gui.widgets.settings_editor_frame',
    'gui.widgets.dynamic_setup_frame',
]

a = Analysis(
    ['src/main.py'],
    pathex=[],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='OptiScaler-GUI',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,  # Set to True for debug console
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='OptiScaler-GUI',
)
'''
    
    with open('OptiScaler-GUI.spec', 'w', encoding='utf-8') as f:
        f.write(spec_content)
    print("✅ Created OptiScaler-GUI.spec")

def build_executable():
    """Build the standalone executable"""
    print("🔨 Building standalone executable...")
    
    # Clean previous builds
    if os.path.exists('dist'):
        shutil.rmtree('dist')
    if os.path.exists('build'):
        shutil.rmtree('build')
    
    # Build with PyInstaller
    cmd = [sys.executable, "-m", "PyInstaller", "--clean", "OptiScaler-GUI.spec"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode == 0:
        print("✅ Build completed successfully!")
        print(f"📁 Executable location: {os.path.abspath('dist/OptiScaler-GUI')}")
        
        # Copy additional files
        dist_dir = Path('dist/OptiScaler-GUI')
        
        # Copy assets if not already included
        if not (dist_dir / 'assets').exists() and os.path.exists('assets'):
            shutil.copytree('assets', dist_dir / 'assets')
            print("✅ Copied assets folder")
        
        # Create cache directory
        cache_dir = dist_dir / 'cache'
        cache_dir.mkdir(exist_ok=True)
        print("✅ Created cache directory")
        
        # Copy important files
        important_files = ['README.md', 'requirements.txt', 'CHANGELOG.md']
        for file in important_files:
            if os.path.exists(file):
                shutil.copy2(file, dist_dir / file)
                print(f"✅ Copied {file}")
        
        # Copy 7-Zip executable for complete archive extraction support
        seven_zip_paths = [
            r'C:\Program Files\7-Zip\7z.exe',
            r'C:\Program Files (x86)\7-Zip\7z.exe'
        ]
        
        for seven_zip_path in seven_zip_paths:
            if os.path.exists(seven_zip_path):
                shutil.copy2(seven_zip_path, dist_dir / '7z.exe')
                print("✅ Copied 7z.exe for complete archive support")
                break
        else:
            print("⚠️ 7-Zip not found - archive extraction will use Python fallback only")
        
        print(f"\\n🎉 Standalone OptiScaler-GUI is ready!")
        print(f"📁 Location: {dist_dir.absolute()}")
        print(f"🚀 Run: {dist_dir / 'OptiScaler-GUI.exe'}")
        
    else:
        print("❌ Build failed!")
        print("STDOUT:", result.stdout)
        print("STDERR:", result.stderr)
        return False
    
    return True

def optimize_for_standalone():
    """Optimize the codebase for standalone deployment"""
    print("🎯 Optimizing for standalone deployment...")
    
    # Create optimized entry point
    entry_point_content = '''#!/usr/bin/env python3
"""
Standalone entry point for OptiScaler-GUI
Optimized for PyInstaller deployment
"""

import sys
import os
from pathlib import Path

# Add src directory to path when running as script
if getattr(sys, 'frozen', False):
    # Running as compiled executable
    application_path = Path(sys.executable).parent
    src_path = application_path / 'src'
else:
    # Running as script
    application_path = Path(__file__).parent
    src_path = application_path / 'src'

# Ensure src is in Python path
if str(src_path) not in sys.path:
    sys.path.insert(0, str(src_path))

# Set working directory to application path
os.chdir(application_path)

# Import and run the main application
try:
    from main import main
    if __name__ == "__main__":
        main()
except ImportError as e:
    import tkinter as tk
    from tkinter import messagebox
    
    root = tk.Tk()
    root.withdraw()
    messagebox.showerror("Import Error", 
                        f"Failed to import main application: {e}\\n\\n"
                        f"Please ensure all required dependencies are installed.")
    sys.exit(1)
except Exception as e:
    import tkinter as tk
    from tkinter import messagebox
    
    root = tk.Tk()
    root.withdraw()
    messagebox.showerror("Application Error", 
                        f"Application failed to start: {e}")
    sys.exit(1)
'''
    
    with open('standalone_main.py', 'w', encoding='utf-8') as f:
        f.write(entry_point_content)
    print("✅ Created optimized entry point")

def main():
    """Main build process"""
    print("🚀 OptiScaler-GUI Standalone Builder")
    print("=" * 40)
    
    # Check if we're in the right directory
    if not os.path.exists('src/main.py'):
        print("❌ Error: Please run this script from the OptiScaler-GUI root directory")
        sys.exit(1)
    
    try:
        # Step 1: Install PyInstaller
        install_pyinstaller()
        
        # Step 2: Optimize for standalone
        optimize_for_standalone()
        
        # Step 3: Create spec file
        create_spec_file()
        
        # Step 4: Build executable
        success = build_executable()
        
        if success:
            print("\\n🎊 Build process completed successfully!")
            print("\\n📋 Next steps:")
            print("   1. Test the executable in dist/OptiScaler-GUI/")
            print("   2. Distribute the entire OptiScaler-GUI folder")
            print("   3. Users can run OptiScaler-GUI.exe directly")
        else:
            print("\\n❌ Build process failed!")
            sys.exit(1)
            
    except Exception as e:
        print(f"\\n❌ Build process failed with error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
