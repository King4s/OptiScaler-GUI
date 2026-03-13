# -*- mode: python ; coding: utf-8 -*-

import os
from pathlib import Path

# Get the current directory
current_dir = Path(os.getcwd())
src_dir = current_dir / 'src'

a = Analysis(
    [str(src_dir / 'main.py')],  # Main entry point
    pathex=[str(current_dir), str(src_dir)],  # Additional paths
    binaries=[],
    datas=[
        # Translations and assets
        (str(src_dir / 'translations'), 'src/translations'),
        ('assets', 'assets'),
        ('requirements.txt', '.'),
        ('README.md', '.'),
    ],
    hiddenimports=[
        # Third-party GUI / system deps that analysis may miss
        'customtkinter',
        'tkinter',
        '_tkinter',
        'PIL',
        'CTkMessagebox',
        'requests',
        'winreg',
        'vdf',
        'psutil',
        'concurrent.futures',
    ],
    # Collect all app packages as plain .py source files on disk.
    # This prevents them from being embedded in the PYZ archive where
    # PyInstaller 6.15 / Python 3.13 drops sub-module registrations,
    # and lets Python's standard PathFinder import them from _MEIPASS.
    module_collection_mode={
        'scanner':    'py',
        'utils':      'py',
        'gui':        'py',
        'optiscaler': 'py',
    },
    hookspath=['.'],
    hooksconfig={},
    runtime_hooks=['rthook_diagnostics.py'],
    excludes=[
        # Exclude heavy unnecessary modules to reduce size dramatically
        'matplotlib',
        'numpy',
        'scipy',
        'pandas',
        'IPython',
        'jupyter',
        'notebook',
        'pytest',
        'unittest',
        'test',
        'tests',
        'py7zr',
        'beautifulsoup4',
        'lxml',
        'numba',
        'torch',
        'tensorflow',
        'sklearn',
        'openpyxl',
        # Specifically exclude OpenBLAS and related math libraries
        'numpy.core',
        'numpy.linalg',
        'numpy.random',
        'numpy.fft',
        'numpy.distutils',
        'numpy.testing',
        'scipy.sparse',
        'scipy.linalg',
        'scipy.optimize',
        'openblas',
        'mkl',
        'blas',
        'lapack',
        # More aggressive excludes
        'pydoc',
        'doctest',
        'pickle',
        'multiprocessing',
    ],
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,  # binaries/datas go into COLLECT (_internal/), not embedded in EXE
    name='OptiScaler-GUI',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=None,  # No icon for now - can be added later with proper .ico file
    version_file=None,
)

# Create a COLLECT for directory distribution (alternative to single file)
coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='OptiScaler-GUI-Portable',
)
