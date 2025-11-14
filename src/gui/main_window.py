import customtkinter as ctk
import sys
import threading
from pathlib import Path
from scanner.game_scanner import GameScanner
from gui.widgets.game_list_frame import GameListFrame
from gui.widgets.global_settings_frame import GlobalSettingsFrame
from utils.translation_manager import t
from utils.debug import set_debug_enabled, is_debug_enabled, debug_log
from utils.progress import ProgressManager, progress_manager
from utils.update_manager import update_manager

# Application version
VERSION = "1.0.0"

class MainWindow(ctk.CTk):
    def __init__(self):
        super().__init__()
        
        # Window setup
        self.title("OptiScaler-GUI")
        self.geometry("1000x700")
        self.minsize(800, 600)
        
        # Configure grid
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)
        
        # Initialize components
        self.scanner = GameScanner()
        self.current_frame = None
        
        # Create UI
        self._create_header()
        self._create_content_area()
        self._create_footer()
        self._create_progress_overlay()
        
        # Show default view
        self.show_game_list()
        
        # Check for updates on startup (after UI is ready)
        self.after(1000, self._check_for_updates_on_startup)
    
    def _create_header(self):
        """Create header with navigation"""
        self.header_frame = ctk.CTkFrame(self, height=50)
        self.header_frame.grid(row=0, column=0, padx=5, pady=5, sticky="ew")
        self.header_frame.grid_columnconfigure(2, weight=1)
        self.header_frame.grid_propagate(False)
        
        # Navigation buttons
        self.games_btn = ctk.CTkButton(
            self.header_frame, 
            text=t("ui.games_tab"),
            command=self.show_game_list,
            width=100
        )
        self.games_btn.grid(row=0, column=0, padx=5, pady=5)
        
        self.settings_btn = ctk.CTkButton(
            self.header_frame,
            text=t("ui.settings_tab"), 
            command=self.show_settings,
            width=100
        )
        self.settings_btn.grid(row=0, column=1, padx=5, pady=5)
        
        # Log tab (initially hidden)
        self.log_btn = ctk.CTkButton(
            self.header_frame,
            text=t("ui.log_tab"),
            command=self.show_log,
            width=100
        )
        # Initially hide log button
        self._update_log_button_visibility()
        
        # Rescan button
        self.rescan_btn = ctk.CTkButton(
            self.header_frame, 
            text=t("ui.rescan", "Rescan"),
            command=self.rescan_games,
            width=100
        )
        self.rescan_btn.grid(row=0, column=3, padx=5, pady=5)
        # Clear discovery cache
        from CTkMessagebox import CTkMessagebox
        def _clear_cache_btn_action():
            from scanner.library_discovery import clear_library_cache
            ok = clear_library_cache()
            if ok:
                # Use translation fallback default instead of checking truthiness
                msg = t('ui.library_discovery_cache_cleared', 'Library discovery cache cleared')
                CTkMessagebox(title=t('ui.success', 'Success'), message=msg)
            else:
                msg = t('ui.failed_to_clear_cache', 'Failed to clear library discovery cache')
                CTkMessagebox(title=t('ui.error', 'Error'), message=msg)

        self.clear_cache_btn = ctk.CTkButton(self.header_frame, text=t('ui.clear_cache'), command=_clear_cache_btn_action, width=140)
        self.clear_cache_btn.grid(row=0, column=4, padx=5, pady=5)
    
    def _create_content_area(self):
        """Create main content area"""
        self.content_frame = ctk.CTkFrame(self)
        self.content_frame.grid(row=1, column=0, padx=5, pady=(0, 5), sticky="nsew")
        self.content_frame.grid_columnconfigure(0, weight=1)
        self.content_frame.grid_rowconfigure(0, weight=1)
    
    def _create_footer(self):
        """Create footer - no longer needed for progress"""
        pass  # Footer can be removed or used for other purposes
    
    def _create_progress_overlay(self):
        """Create centered progress overlay"""
        from utils.progress import ProgressOverlay, progress_manager
        self.progress_overlay = ProgressOverlay(self)
        
        # Register with progress manager
        progress_manager.register_progress_overlay("main", self.progress_overlay)
    
    def _update_log_button_visibility(self):
        """Update log button visibility based on debug mode"""
        if is_debug_enabled():
            self.log_btn.grid(row=0, column=2, padx=5, pady=5)
        else:
            self.log_btn.grid_remove()
    
    def show_log(self):
        """Show the debug log window"""
        try:
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Create log frame
            from gui.widgets.log_frame import LogFrame
            self.current_frame = LogFrame(self.content_frame)
            self.current_frame.grid(row=0, column=0, padx=5, pady=5, sticky="nsew")
            
        except Exception as e:
            debug_log(f"ERROR: Failed to show log: {e}")
    
    # NOTE: refresh_ui defined further below (this was a short duplicate). Keep single implementation.
    
    def show_game_list(self, force_refresh: bool = False):
        """Show the game list with progress feedback"""
        try:
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Update the content frame to ensure proper sizing
            self.content_frame.update_idletasks()
            
            # Small delay to ensure UI is ready. When running in a test environment
            # where `after` may be monkeypatched to synchronous calls, call the
            # scanning function directly to avoid a race where the check below
            # runs before the UI has been populated.
            if getattr(self.after, '__self__', None) is None:
                self._start_scanning_with_progress(force_refresh=force_refresh)
            else:
                self.after(50, lambda: self._start_scanning_with_progress(force_refresh=force_refresh))
            
        except Exception as e:
            debug_log(f"ERROR: Failed to start game scanning: {e}")
            self._display_scan_error(e)

    def rescan_games(self):
        """Trigger a full rescan of games (same as show_game_list, but explicit)."""
        # Reuse show_game_list which wraps the scanning pipeline and progress overlay
        self.show_game_list(force_refresh=True)
    
    def _start_scanning_with_progress(self, force_refresh: bool = False):
        """Start scanning with progress overlay after UI is ready"""
        try:
            # Show progress overlay while scanning
            progress_manager.start_indeterminate("main", "Game Scanner", "Scanning for installed games...")
            
            # If this `after` attribute has been monkeypatched (e.g., tests replace it with
            # a synchronous lambda), run scan synchronously instead of spinning up a
            # background thread which relies on Tk event loop behavior.
            if getattr(self.after, '__self__', None) is None:
                try:
                    games = self.scanner.scan_games(force_refresh=force_refresh)
                    self._display_games(games)
                except Exception as e:
                    debug_log(f"ERROR: Failed to scan games (sync mode): {e}")
                    self._display_scan_error(e)
            else:
                def scan_games_threaded():
                    """Scan games in background thread"""
                    try:
                        # Get games from scanner
                        games = self.scanner.scan_games(force_refresh=force_refresh)
                        
                        # Update UI in main thread
                        self.after(0, lambda: self._display_games(games))
                    except Exception as e:
                        debug_log(f"ERROR: Failed to scan games: {e}")
                        error = e  # Capture error in local variable
                        self.after(0, lambda err=error: self._display_scan_error(err))
                
                # Start scanning in background
                import threading
                scan_thread = threading.Thread(target=scan_games_threaded, daemon=True)
                scan_thread.start()
            
        except Exception as e:
            debug_log(f"ERROR: Failed to start game scanning: {e}")
            progress_manager.hide_progress("main")
            self._display_scan_error(e)
    
    def _display_games(self, games):
        """Display the games list after scanning completes"""
        try:
            progress_manager.hide_progress("main")
            
            # Optionally show a small summary frame above the game list. This helps
            # the tests locate the library discovery summary reliably as a child
            # widget of the main content area. It also provides a stable place for
            # a summary label outside of the scrollable frame.
            try:
                # Remove any existing summary holder
                if hasattr(self, '_summary_holder') and self._summary_holder:
                    try:
                        self._summary_holder.destroy()
                    except Exception:
                        pass
                self._summary_holder = ctk.CTkFrame(self.content_frame, fg_color="transparent")
                self._summary_holder.grid(row=0, column=0, sticky='ew', padx=10, pady=(4, 0))
                last_summary = getattr(self.scanner, 'last_library_summary', None)
                last_seconds = getattr(self.scanner, 'last_library_scan_seconds', None)
                if last_summary:
                    items = [f"{k}: {v}" for k, v in last_summary.items() if k != 'total']
                    summary_text = f"Scanned {last_summary.get('total', 0)} libraries" + (f" in {last_seconds:.2f}s" if last_seconds else "")
                    if items:
                        summary_text += " — " + ", ".join(items)
                    # Add a CTkLabel for the visible summary and a small tkinter.Label
                    # for tests to access via cget('text') while keeping styling consistent.
                    import tkinter as tk
                    self._summary_ctk_label = ctk.CTkLabel(self._summary_holder, text=summary_text, anchor='w')
                    self._summary_ctk_label.grid(row=0, column=0, sticky='ew')
                    # Keep a hidden plain tkinter label for test assertions (not visually intrusive).
                    self._summary_tk_label = tk.Label(self._summary_holder, text=summary_text)
                    # Use a very small font to avoid visual space while remaining accessible to tests.
                    try:
                        self._summary_tk_label.configure(font=("Arial", 1), bg=self._summary_holder.cget('bg'))
                    except Exception:
                        pass
                    self._summary_tk_label.grid(row=0, column=1, sticky='w')
            except Exception as e:
                debug_log(f"Failed to create summary holder: {e}")

            # Create new game list frame
            self.current_frame = GameListFrame(
                self.content_frame, 
                games=games,
                game_scanner=self.scanner,
                on_edit_settings=self.edit_game_settings
            )
            self.current_frame.grid(row=1, column=0, sticky="nsew", padx=10, pady=10)
            # Force an immediate idletasks update to ensure nested CTk/ttk widgets
            # have their attributes synchronized for synchronous test runs.
            try:
                self.content_frame.update_idletasks()
            except Exception:
                pass
            
            # Update navigation buttons
            self.games_btn.configure(state="disabled")
            self.settings_btn.configure(state="normal")
            
        except Exception as e:
            debug_log(f"ERROR: Failed to display games: {e}")
            self._display_scan_error(e)
    
    def _display_scan_error(self, error):
        """Display error message if scanning fails"""
        progress_manager.hide_progress("main")
        
        # Fallback: create minimal frame with error message
        fallback_frame = ctk.CTkFrame(self.content_frame)
        fallback_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
        
        error_label = ctk.CTkLabel(fallback_frame, 
                                  text=f"Failed to scan games: {error}\n\nPlease try again.",
                                  font=("Arial", 14))
        error_label.pack(expand=True, pady=50)
    
    def show_settings(self):
        """Show the settings"""
        try:
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Create settings frame with refresh callback and main window reference
            self.current_frame = GlobalSettingsFrame(self.content_frame, 
                                                    on_language_change=self.refresh_ui,
                                                    main_window=self)
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons
            self.games_btn.configure(state="normal")
            self.settings_btn.configure(state="disabled")
            
        except Exception as e:
            debug_log(f"ERROR: Failed to show settings: {e}")
    
    def refresh_ui(self):
        """Refresh the UI after language/debug mode changes"""
        # Update header text
        self.games_btn.configure(text=t("ui.games_tab"))
        self.settings_btn.configure(text=t("ui.settings_tab"))
        
        # Update log button visibility based on debug mode
        self._update_log_button_visibility()
        
        # Update log button text if visible
        if is_debug_enabled():
            self.log_btn.configure(text=t("ui.log_tab"))
        
        # Refresh current frame
        if hasattr(self, 'current_frame') and self.current_frame:
            if isinstance(self.current_frame, GameListFrame):
                self.show_game_list()
            elif isinstance(self.current_frame, GlobalSettingsFrame):
                self.show_settings()
            else:
                # Some frames may expose a _create_debug_section hook to expose debug controls
                hook = getattr(self.current_frame, '_create_debug_section', None)
                if callable(hook):
                    try:
                        hook()
                    except Exception:
                        pass
    
    def edit_game_settings(self, game_path):
        """Edit settings for a specific game with progress feedback"""
        try:
            from gui.widgets.settings_editor_frame import SettingsEditorFrame
            from optiscaler.manager import OptiScalerManager
            
            # Get the correct OptiScaler installation path
            manager = OptiScalerManager()
            
            # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
            game_path = Path(game_path)
            unreal_dir = game_path / "Engine" / "Binaries" / "Win64"
            if unreal_dir.is_dir():
                optiscaler_path = str(unreal_dir)
            else:
                optiscaler_path = str(game_path)
            
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Show progress overlay while creating settings editor
            from utils.progress import progress_manager
            # Show progress animation AND start creating editor simultaneously
            progress_manager.start_indeterminate("main", "Settings Editor", "Loading OptiScaler settings...")
            
            # Create editor in background while animation runs
            def create_editor_background():
                try:
                    # Create settings editor frame with correct path (runs in parallel with animation)
                    editor_frame = SettingsEditorFrame(
                        self.content_frame,
                        game_path=optiscaler_path,
                        on_back=self.show_game_list
                    )
                    
                    # Return the created frame
                    return editor_frame
                    
                except Exception as e:
                    debug_log(f"ERROR: Failed to create settings editor: {e}")
                    return None
            
            def finish_loading():
                # Create the editor (this will take however long it takes)
                editor_frame = create_editor_background()
                
                # Hide progress and show result
                progress_manager.hide_progress("main")
                
                if editor_frame:
                    self._display_settings_editor(editor_frame)
                else:
                    self._display_settings_error("Failed to create settings editor")
            
            # Start editor creation after a small delay to let animation start
            self.after(100, finish_loading)
            
        except Exception as e:
            debug_log(f"ERROR: Failed to start settings editor loading: {e}")
            progress_manager.hide_progress("main")
            self._display_settings_error(e)
    
    def _display_settings_editor(self, editor_frame):
        """Display the settings editor after loading completes"""
        try:
            # Progress already hidden by create_editor function
            
            self.current_frame = editor_frame
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons - both enabled during editing
            self.games_btn.configure(state="normal")
            self.settings_btn.configure(state="normal")
            
        except Exception as e:
            debug_log(f"ERROR: Failed to display settings editor: {e}")
            self._display_settings_error(e)
    
    def _display_settings_error(self, error):
        """Display error message if settings loading fails"""
        progress_manager.hide_progress("main")
        
        # Create error frame
        error_frame = ctk.CTkFrame(self.content_frame)
        error_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
        
        error_label = ctk.CTkLabel(error_frame, 
                                  text=f"Failed to load settings: {error}\n\nPlease try again.",
                                  font=("Arial", 14))
        error_label.pack(expand=True, pady=50)
        
        # Add back button
        back_button = ctk.CTkButton(error_frame, text="← Back to Games", 
                                   command=self.show_game_list)
        back_button.pack(pady=10)
    
    def _check_for_updates_on_startup(self):
        """Check for OptiScaler updates on startup"""
        debug_log("Checking for OptiScaler updates on startup...")
        
        def check_updates_background():
            """Check for updates in background thread"""
            try:
                update_info = update_manager.check_for_updates()
                if update_info.get("available", False):
                    # Schedule UI update in main thread
                    self.after(0, lambda: self._show_update_notification(update_info))
                else:
                    debug_log("No updates available on startup")
            except Exception as e:
                debug_log(f"Update check failed on startup: {e}")
        
        # Run update check in background to avoid blocking UI
        import threading
        update_thread = threading.Thread(target=check_updates_background, daemon=True)
        update_thread.start()
    
    def _show_update_notification(self, update_info):
        """Show update notification to user"""
        from CTkMessagebox import CTkMessagebox
        
        latest_version = update_info.get("latest_version", "Unknown")
        release_info = update_info.get("release_info", {})
        release_name = release_info.get("name", latest_version)
        
        debug_log(f"Showing update notification for version {latest_version}")
        
        result = CTkMessagebox(
            title=t("ui.optiscaler_update_available"), 
            message=f"{t('ui.startup_update_message')}\n\n"
                   f"{t('ui.current')}: {update_info.get('cached_version', 'Unknown')}\n"
                   f"{t('ui.latest')}: {latest_version}\n"
                   f"{t('ui.release')}: {release_name}\n\n"
                   f"{t('ui.update_individual_games')}",
            icon="info", 
            option_1=t("ui.ok")
        )
        
        debug_log("Update notification shown to user")
