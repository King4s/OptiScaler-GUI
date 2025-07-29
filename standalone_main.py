#!/usr/bin/env python3
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
                        f"Failed to import main application: {e}\n\n"
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
