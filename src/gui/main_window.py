import customtkinter as ctk
import sys
import os
import threading
from scanner.game_scanner import GameScanner
from gui.widgets.game_list_frame import GameListFrame
from gui.widgets.settings_editor_frame import SettingsEditorFrame
from utils.config import config

class DebugRedirector:
    def __init__(self, widget, log_file_path):
        self.widget = widget
        self.stdout = sys.stdout
        self.stderr = sys.stderr
        self.log_file = open(log_file_path, "w", encoding="utf-8", buffering=1) # Line buffering

    def write(self, text):
        self.widget.insert(ctk.END, text)
        self.widget.see(ctk.END)
        self.stdout.write(text) # Also print to original stdout
        self.log_file.write(text) # Write to log file
        self.log_file.flush() # Force flush after each write

    def flush(self):
        self.stdout.flush()
        self.log_file.flush()

import customtkinter as ctk
from gui.widgets.game_list_frame import GameListFrame
from gui.widgets.settings_editor_frame import SettingsEditorFrame
from scanner.game_scanner import GameScanner
from utils.i18n import i18n, t
from utils.progress import ProgressFrame, progress_manager
import webbrowser

VERSION = "v1.0.0"

class MainWindow(ctk.CTk):
    def __init__(self, parent=None):
        super().__init__()
        self.title(f"{t('app_title')} {VERSION}")
        self.geometry("1200x800")
        self.minsize(800, 600)
        
        # Configure grid
        self.grid_rowconfigure(1, weight=1)  # Main content area
        self.grid_columnconfigure(0, weight=1)
        
        # Create header frame
        self._create_header()
        
        # Create main content area
        self._create_main_content()
        
        # Create footer with progress bar
        self._create_footer()
        
        # Initialize game scanner and load games
        self._load_games()
        
    def _create_header(self):
        """Create header with version and thanks"""
        self.header_frame = ctk.CTkFrame(self)
        self.header_frame.grid(row=0, column=0, padx=10, pady=(10, 5), sticky="ew")
        self.header_frame.grid_columnconfigure(1, weight=1)
        
        # Version info
        version_label = ctk.CTkLabel(self.header_frame, 
                                   text=f"{t('app_title')} {VERSION}",
                                   font=("Arial", 16, "bold"))
        version_label.grid(row=0, column=0, padx=10, pady=5, sticky="w")
        
        # Thanks link (clickable)
        thanks_frame = ctk.CTkFrame(self.header_frame, fg_color="transparent")
        thanks_frame.grid(row=0, column=2, padx=10, pady=5, sticky="e")
        
        thanks_label = ctk.CTkLabel(thanks_frame, text=f"{t('thanks_to')} ")
        thanks_label.grid(row=0, column=0)
        
        link_label = ctk.CTkLabel(thanks_frame, 
                                text="OptiScaler Project",
                                text_color="#1f6aa5",
                                cursor="hand2")
        link_label.grid(row=0, column=1)
        link_label.bind("<Button-1>", lambda e: webbrowser.open("https://github.com/optiscaler/OptiScaler"))
        
        # Language selector
        self.language_var = ctk.StringVar(value="English")
        language_menu = ctk.CTkOptionMenu(self.header_frame,
                                        values=list(i18n.get_languages().values()),
                                        variable=self.language_var,
                                        command=self._on_language_change,
                                        width=100)
        language_menu.grid(row=0, column=3, padx=10, pady=5)
        
    def _create_main_content(self):
        """Create main content area with tabs"""
        # Create tabview
        self.tabview = ctk.CTkTabview(self)
        self.tabview.grid(row=1, column=0, padx=10, pady=5, sticky="nsew")
        
        # Games tab
        self.games_tab = self.tabview.add(t("games_tab"))
        self.games_tab.grid_rowconfigure(0, weight=1)
        self.games_tab.grid_columnconfigure(0, weight=1)
        
        # Settings tab (placeholder for now)
        self.settings_tab = self.tabview.add(t("settings_tab"))
        
        # Initialize frames
        self.game_list_frame = None
        self.settings_editor_frame = None
        
    def _create_footer(self):
        """Create footer with progress bar"""
        self.footer_frame = ctk.CTkFrame(self)
        self.footer_frame.grid(row=2, column=0, padx=10, pady=(5, 10), sticky="ew")
        self.footer_frame.grid_columnconfigure(0, weight=1)
        
        # Progress bar
        self.progress_frame = ProgressFrame(self.footer_frame)
        self.progress_frame.grid(row=0, column=0, padx=5, pady=5, sticky="ew")
        
        # Register with progress manager
        progress_manager.register_progress_frame("main", self.progress_frame)
        
    def _on_language_change(self, language_name):
        """Handle language change"""
        # Get language code from name
        languages = i18n.get_languages()
        language_code = None
        for code, name in languages.items():
            if name == language_name:
                language_code = code
                break
                
        if language_code and i18n.set_language(language_code):
            # Update UI texts
            self._update_ui_texts()
            
    def _update_ui_texts(self):
        """Update all UI texts after language change"""
        self.title(f"{t('app_title')} {VERSION}")
        
        # Update tab names
        # Note: CTkTabview doesn't support changing tab names after creation
        # This would need a more complex solution with recreating tabs
        
    def _load_games(self):
        """Load games in background"""
        def load_games_worker():
            progress_manager.show_progress("main", t("processing"))
            
            self.game_scanner = GameScanner()
            self.games = self.game_scanner.scan_games()
            
            # Create game list frame on main thread
            self.after(0, self._create_game_list)
            
        import threading
        thread = threading.Thread(target=load_games_worker, daemon=True)
        thread.start()
        
    def _create_game_list(self):
        """Create game list frame (called on main thread)"""
        progress_manager.hide_progress("main")
        
        self.game_list_frame = GameListFrame(self.games_tab, 
                                           games=self.games, 
                                           game_scanner=self.game_scanner,
                                           on_edit_settings=self.show_settings_editor)
        self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")
        
        print(f"GameScanner returned {len(self.games)} games.")

    def _start_game_scan(self):
        self.game_scanner = GameScanner()
        self.games = self.game_scanner.scan_games()
        self.after(100, self._update_gui_after_scan) # Schedule GUI update on main thread

    def _update_gui_after_scan(self):
        self.progress_frame.destroy() # Remove progress bar and status label

        self.game_list_frame = GameListFrame(self.games_tab, games=self.games, game_scanner=self.game_scanner, 
                                             on_edit_settings=self.show_settings_editor)
        self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")

        print(f"GameScanner returned {len(self.games)} games.")

    def show_settings_editor(self, game_path):
        print(f"DEBUG: Opening settings editor for game path: {game_path}")
        
        if self.settings_editor_frame:
            self.settings_editor_frame.destroy()
        
        self.game_list_frame.grid_forget() # Hide game list

        try:
            self.settings_editor_frame = SettingsEditorFrame(self.games_tab, game_path=game_path, 
                                                           on_back=self.show_game_list)
            self.settings_editor_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")
            print("DEBUG: Settings editor created successfully")
        except Exception as e:
            print(f"ERROR: Failed to create settings editor: {e}")
            # Show game list again if there's an error
            self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")
            try:
                from CTkMessagebox import CTkMessagebox
                CTkMessagebox(title="Fejl", message=f"Kan ikke åbne indstillinger:\n{str(e)}")
            except:
                print(f"Error opening settings: {e}")

    def show_game_list(self):
        """Show the game list and hide settings editor"""
        print("DEBUG: Showing game list...")
        if self.settings_editor_frame:
            self.settings_editor_frame.destroy()
            self.settings_editor_frame = None
        
        # Make sure the games tab is properly configured and the frame is visible
        self.games_tab.grid_rowconfigure(0, weight=1)
        self.games_tab.grid_columnconfigure(0, weight=1)
        self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")
        print("DEBUG: Game list shown successfully")


class App(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("OptiScaler GUI")
        self.geometry("1024x768")
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self.main_window = MainWindow(self)
        self.main_window.grid(row=0, column=0, sticky="nsew")

if __name__ == "__main__":
    app = App()
    app.mainloop()