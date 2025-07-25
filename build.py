#!/usr/bin/env python3
"""
Build script for creating standalone OptiScaler-GUI executable
"""

import subprocess
import sys
import os
import shutil
from pathlib import Path

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
    icon='assets/icons/xbox_logo.png' if os.path.exists('assets/icons/xbox_logo.png') else None,
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
