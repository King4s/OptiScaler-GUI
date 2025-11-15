#!/usr/bin/env python3
"""
OptiScaler-GUI - Main Entry Point
A user-friendly GUI for managing OptiScaler installations
"""

import sys
import os
import threading
import traceback
from utils.logging_utils import log_exception, log_message
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
        # Use getattr to safely access _MEIPASS if present in PyInstaller runtime
        bundle_dir = Path(getattr(sys, '_MEIPASS', application_path))
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
        # Install a global excepthook to capture uncaught exceptions and write crash logs
        def _handle_uncaught_exceptions(exc_type, exc_value, exc_traceback):
            # Write the traceback to a log
            try:
                filename = log_exception((exc_type, exc_value, exc_traceback))
                if filename:
                    log_message(f"Unhandled exception written to {filename}")
            except Exception:
                pass
            # Fallback: call default excepthook
            sys.__excepthook__(exc_type, exc_value, exc_traceback)

        sys.excepthook = _handle_uncaught_exceptions

        # Threading hook (Python 3.8+)
        try:
            def _threading_excepthook(args):
                # args has .exc_type, .exc_value, .exc_traceback
                try:
                    log_exception((args.exc_type, args.exc_value, args.exc_traceback))
                except Exception:
                    pass
            threading.excepthook = _threading_excepthook
        except Exception:
            # Older Python may not support threading.excepthook
            pass
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
        
        # Set debug enabled from persisted configuration (if any)
        try:
            from utils.config import get_config_value
            from utils.debug import set_debug_enabled
            set_debug_enabled(bool(get_config_value('debug', False)))
        except Exception:
            pass
        # Set appearance mode and theme
        ctk.set_appearance_mode("system")
        ctk.set_default_color_theme("blue")
        
        from utils.performance import performance_monitor
        # Start background performance monitoring
        performance_monitor.start_monitoring()

        # Create and run the application
        app = MainWindow()
        app.mainloop()
        
    except ImportError as e:
        import tkinter as tk
        from tkinter import messagebox
        
        # Log the exception
        log_exception()
        root = tk.Tk()
        root.withdraw()
        messagebox.showerror("Import Error",
                   f"Failed to import required modules: {e}\n\n"
                   f"Please ensure all dependencies are installed:\n"
                   f"pip install -r requirements.txt")
        sys.exit(1)
    except Exception as e:
        # Log the exception
        log_exception()
        import tkinter as tk
        from tkinter import messagebox
        
        root = tk.Tk()
        root.withdraw()
        messagebox.showerror("Application Error",
                           f"OptiScaler-GUI failed to start: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()