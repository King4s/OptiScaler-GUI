#!/usr/bin/env python3
"""
OptiScaler-GUI - Main Entry Point
A user-friendly GUI for managing OptiScaler installations
"""

import sys
import os
from pathlib import Path
from __version__ import __version__

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
        from utils.system_requirements import requirements_checker
        from gui.main_window import MainWindow
        
        # Check system requirements before starting GUI
        report = requirements_checker.generate_user_report()
        
        # If critical requirements are missing, show error and exit
        if report["overall_status"] == "not_ready":
            try:
                from CTkMessagebox import CTkMessagebox
                
                # Build error message
                missing_items = []
                for name, req in report["requirements"].items():
                    if req["required"] and not req["status"]:
                        missing_items.append(req["message"])
                
                error_msg = "Critical requirements missing:\n\n" + "\n".join(missing_items)
                
                if report["recommendations"]:
                    error_msg += "\n\nRequired actions:"
                    for rec in report["recommendations"]:
                        if rec["priority"] == "critical":
                            error_msg += f"\n• {rec['action']}"
                
                # Show error dialog
                CTkMessagebox(
                    title="System Requirements Not Met",
                    message=error_msg,
                    icon="cancel"
                )
                
            except ImportError:
                # Fallback to console if GUI not available
                print("❌ Critical system requirements not met!")
                for name, req in report["requirements"].items():
                    if req["required"] and not req["status"]:
                        print(f"  Missing: {req['message']}")
                
                print("\nPlease install missing requirements and try again.")
            
            sys.exit(1)
        
        # Show warning for functional but not optimal setup
        if report["overall_status"] == "functional":
            try:
                from CTkMessagebox import CTkMessagebox
                
                warnings = []
                for rec in report["recommendations"]:
                    if rec["priority"] == "optional":
                        warnings.append(f"• {rec['description']}: {rec['action']}")
                
                if warnings:
                    warning_msg = "OptiScaler GUI will work but performance could be improved:\n\n" + "\n".join(warnings)
                    CTkMessagebox(
                        title="Performance Notice",
                        message=warning_msg,
                        icon="warning"
                    )
            except ImportError:
                pass  # Skip warning if GUI not available
        
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