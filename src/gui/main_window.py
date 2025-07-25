import customtkinter as ctk
import sys
import os
import threading
from scanner.game_scanner import GameScanner
from gui.widgets.game_list_frame import GameListFrame
from gui.widgets.global_settings_frame import GlobalSettingsFrame
from utils.translation_manager import t
from utils.debug import set_debug_enabled, is_debug_enabled
from utils.progress import ProgressManager

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
        """Create minimalistic footer with progress bar"""
        self.footer_frame = ctk.CTkFrame(self, height=25, fg_color="transparent")
        self.footer_frame.grid(row=2, column=0, padx=5, pady=(0, 5), sticky="ew")
        self.footer_frame.grid_columnconfigure(0, weight=1)
        self.footer_frame.grid_propagate(False)
        
        # Progress bar (initially hidden)
        from utils.progress import ProgressFrame, progress_manager
        self.progress_frame = ProgressFrame(self.footer_frame)
        # Don't grid it immediately - let it stay hidden
        
        # Register with progress manager
        progress_manager.register_progress_frame("main", self.progress_frame)
    
    def _toggle_debug(self):
        """Toggle debug output"""
        enabled = self.debug_var.get()
        set_debug_enabled(enabled)
        print(f"Debug {'enabled' if enabled else 'disabled'}")
    
    def show_game_list(self):
        """Show the game list"""
        try:
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Get games from scanner
            games = self.scanner.scan_games()
            
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
            print(f"ERROR: Failed to show game list: {e}")
            # Fallback: create minimal frame
            fallback_frame = ctk.CTkFrame(self.content_frame)
            fallback_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            error_label = ctk.CTkLabel(fallback_frame, text=f"Error loading games: {e}")
            error_label.pack(pady=20)
    
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
        self.games_btn.configure(text=t("games"))
        self.settings_btn.configure(text=t("settings"))
        self.debug_checkbox.configure(text=t("debug"))
        
        # Refresh current frame
        if hasattr(self, 'current_frame') and self.current_frame:
            if isinstance(self.current_frame, GameListFrame):
                self.show_game_list()
            elif isinstance(self.current_frame, GlobalSettingsFrame):
                self.show_settings()
    
    def edit_game_settings(self, game_path):
        """Edit settings for a specific game"""
        try:
            from gui.widgets.settings_editor_frame import SettingsEditorFrame
            
            # Clear current content
            for widget in self.content_frame.winfo_children():
                widget.destroy()
            
            # Create settings editor frame
            self.current_frame = SettingsEditorFrame(
                self.content_frame,
                game_path=game_path,
                on_back=self.show_game_list
            )
            self.current_frame.grid(row=0, column=0, sticky="nsew", padx=10, pady=10)
            
            # Update navigation buttons - both enabled during editing
            self.games_btn.configure(state="normal")
            self.settings_btn.configure(state="normal")
            
        except Exception as e:
            print(f"ERROR: Failed to show game settings editor: {e}")
