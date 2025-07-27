import customtkinter as ctk
import sys
import os
import threading
from scanner.game_scanner import GameScanner
from gui.widgets.game_list_frame import GameListFrame
from gui.widgets.global_settings_frame import GlobalSettingsFrame
from utils.translation_manager import t
from utils.debug import set_debug_enabled, is_debug_enabled
from utils.progress import ProgressManager, progress_manager

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
    
    def _create_header(self):
        """Create header with navigation and debug toggle"""
        self.header_frame = ctk.CTkFrame(self, height=50)
        self.header_frame.grid(row=0, column=0, padx=5, pady=5, sticky="ew")
        self.header_frame.grid_columnconfigure(1, weight=1)
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
        self.settings_btn.grid(row=0, column=1, padx=5, pady=5, sticky="w")
        
        # Debug toggle
        self.debug_var = ctk.BooleanVar()
        self.debug_var.set(is_debug_enabled())
        self.debug_checkbox = ctk.CTkCheckBox(
            self.header_frame,
            text=t("global_settings.debug_mode"),
            variable=self.debug_var,
            command=self._toggle_debug
        )
        self.debug_checkbox.grid(row=0, column=2, padx=5, pady=5, sticky="e")
    
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
    
    def _toggle_debug(self):
        """Toggle debug output"""
        enabled = self.debug_var.get()
        set_debug_enabled(enabled)
        print(f"Debug {'enabled' if enabled else 'disabled'}")
    
    def show_game_list(self):
        """Show the game list with progress feedback"""
        try:
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Show progress overlay while scanning
            progress_manager.start_indeterminate("main", "Game Scanner", "Scanning for installed games...")
            
            def scan_games_threaded():
                """Scan games in background thread"""
                try:
                    # Get games from scanner
                    games = self.scanner.scan_games()
                    
                    # Update UI in main thread
                    self.after(0, lambda: self._display_games(games))
                except Exception as e:
                    print(f"ERROR: Failed to scan games: {e}")
                    self.after(0, lambda: self._display_scan_error(e))
            
            # Start scanning in background
            import threading
            scan_thread = threading.Thread(target=scan_games_threaded, daemon=True)
            scan_thread.start()
            
        except Exception as e:
            print(f"ERROR: Failed to start game scanning: {e}")
            progress_manager.hide_progress("main")
            self._display_scan_error(e)
    
    def _display_games(self, games):
        """Display the games list after scanning completes"""
        try:
            progress_manager.hide_progress("main")
            
            # Create new game list frame
            self.current_frame = GameListFrame(
                self.content_frame, 
                games=games,
                game_scanner=self.scanner,
                on_edit_settings=self.edit_game_settings
            )
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons
            self.games_btn.configure(state="disabled")
            self.settings_btn.configure(state="normal")
            
        except Exception as e:
            print(f"ERROR: Failed to display games: {e}")
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
            
            # Create settings frame with refresh callback
            self.current_frame = GlobalSettingsFrame(self.content_frame, on_language_change=self.refresh_ui)
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons
            self.games_btn.configure(state="normal")
            self.settings_btn.configure(state="disabled")
            
        except Exception as e:
            print(f"ERROR: Failed to show settings: {e}")
    
    def refresh_ui(self):
        """Refresh the UI after language change"""
        # Update header text
        self.games_btn.configure(text=t("ui.games_tab"))
        self.settings_btn.configure(text=t("ui.settings_tab"))
        self.debug_checkbox.configure(text=t("global_settings.debug_mode"))
        
        # Refresh current frame
        if hasattr(self, 'current_frame') and self.current_frame:
            if isinstance(self.current_frame, GameListFrame):
                self.show_game_list()
            elif isinstance(self.current_frame, GlobalSettingsFrame):
                self.show_settings()
    
    def edit_game_settings(self, game_path):
        """Edit settings for a specific game with progress feedback"""
        try:
            from gui.widgets.settings_editor_frame import SettingsEditorFrame
            from optiscaler.manager import OptiScalerManager
            
            # Get the correct OptiScaler installation path
            manager = OptiScalerManager()
            
            # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
            unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
            if os.path.isdir(unreal_dir):
                optiscaler_path = unreal_dir
            else:
                optiscaler_path = game_path
            
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Show progress overlay while generating settings editor
            progress_manager.start_indeterminate("main", "Settings Editor", "Loading OptiScaler settings...")
            
            def load_settings_threaded():
                """Load settings editor in background thread"""
                try:
                    # Create settings editor frame with correct path
                    editor_frame = SettingsEditorFrame(
                        self.content_frame,
                        game_path=optiscaler_path,  # Use the actual OptiScaler installation path
                        on_back=self.show_game_list
                    )
                    
                    # Update UI in main thread
                    self.after(0, lambda: self._display_settings_editor(editor_frame))
                except Exception as e:
                    print(f"ERROR: Failed to create settings editor: {e}")
                    self.after(0, lambda: self._display_settings_error(e))
            
            # Start loading in background
            import threading
            load_thread = threading.Thread(target=load_settings_threaded, daemon=True)
            load_thread.start()
            
        except Exception as e:
            print(f"ERROR: Failed to start settings editor loading: {e}")
            progress_manager.hide_progress("main")
            self._display_settings_error(e)
    
    def _display_settings_editor(self, editor_frame):
        """Display the settings editor after loading completes"""
        try:
            progress_manager.hide_progress("main")
            
            self.current_frame = editor_frame
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons - both enabled during editing
            self.games_btn.configure(state="normal")
            self.settings_btn.configure(state="normal")
            
        except Exception as e:
            print(f"ERROR: Failed to display settings editor: {e}")
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
