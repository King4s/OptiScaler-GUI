#!/usr/bin/env python3
"""
OptiScaler-GUI - Main Entry Point
A user-friendly GUI for managing OptiScaler installations
"""

import sys
import os
from pathlib import Path

def setup_environment():
    """Setup environment for both script and standalone execution"""
    # Handle PyInstaller bundle
    if getattr(sys, 'frozen', False):
        # Running as compiled executable
        application_path = Path(sys.executable).parent
        
        # Set working directory to executable location
        os.chdir(application_path)
        
        # Add paths for imports
        bundle_dir = Path(sys._MEIPASS) if hasattr(sys, '_MEIPASS') else application_path
        sys.path.insert(0, str(bundle_dir))
    else:
        # Running as script - add src to path if needed
        current_dir = Path(__file__).parent
        if current_dir.name != 'src':
            src_dir = current_dir / 'src'
            if src_dir.exists():
                sys.path.insert(0, str(src_dir))

def main():
    """Main application entry point"""
    try:
        # Setup paths and environment
        setup_environment()
        
        # Import after path setup
        import customtkinter as ctk
        from utils import debug  # Import debug system first to patch print
        from gui.main_window import MainWindow
        
        # Set appearance mode and theme
        ctk.set_appearance_mode("system")
        ctk.set_default_color_theme("blue")
        
        # Create and run the application
        app = MainWindow()
        app.mainloop()
        
    except ImportError as e:
        import tkinter as tk
        from tkinter import messagebox
        
        root = tk.Tk()
        root.withdraw()
        messagebox.showerror("Import Error", 
                           f"Failed to import required modules: {e}\n\n"
                           f"Please ensure all dependencies are installed:\n"
                           f"pip install -r requirements.txt")
        sys.exit(1)
    except Exception as e:
        import tkinter as tk
        from tkinter import messagebox
        
        root = tk.Tk()
        root.withdraw()
        messagebox.showerror("Application Error", 
                           f"OptiScaler-GUI failed to start: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()