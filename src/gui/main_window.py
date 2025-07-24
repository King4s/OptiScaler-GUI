import customtkinter as ctk
import sys
import os

class DebugRedirector:
    def __init__(self, widget, log_file_path):
        self.widget = widget
        self.stdout = sys.stdout
        self.stderr = sys.stderr
        self.log_file = open(log_file_path, "w", encoding="utf-8")

    def write(self, text):
        self.widget.insert(ctk.END, text)
        self.widget.see(ctk.END)
        self.stdout.write(text) # Also print to original stdout
        self.log_file.write(text) # Write to log file

    def flush(self):
        self.stdout.flush()
        self.log_file.flush()

class MainWindow(ctk.CTkFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self.tabview = ctk.CTkTabview(self)
        self.tabview.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")

        self.games_tab = self.tabview.add("Games")
        self.debug_tab = self.tabview.add("Debug")

        # Configure Games tab
        self.games_tab.grid_columnconfigure(0, weight=1)
        self.games_tab.grid_rowconfigure(0, weight=1)

        # Configure Debug tab
        self.debug_tab.grid_columnconfigure(0, weight=1)
        self.debug_tab.grid_rowconfigure(0, weight=1)
        self.debug_textbox = ctk.CTkTextbox(self.debug_tab)
        self.debug_textbox.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")

        # Define log file path
        log_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'cache')
        os.makedirs(log_dir, exist_ok=True)
        log_file_path = os.path.join(log_dir, "optiscaler_gui_debug.log")

        # Redirect stdout and stderr to the debug textbox and log file
        sys.stdout = DebugRedirector(self.debug_textbox, log_file_path)
        sys.stderr = DebugRedirector(self.debug_textbox, log_file_path)

        print("Initializing GameScanner...")
        self.game_scanner = GameScanner()
        print("Scanning games...")
        self.games = self.game_scanner.scan_games()
        print(f"GameScanner returned {len(self.games)} games.")

        self.game_list_frame = GameListFrame(self.games_tab, games=self.games, game_scanner=self.game_scanner, 
                                             on_edit_settings=self.show_settings_editor)
        self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")

        self.settings_editor_frame = None

    def show_settings_editor(self, game_path):
        if self.settings_editor_frame:
            self.settings_editor_frame.destroy()
        
        self.game_list_frame.grid_forget() # Hide game list

        self.settings_editor_frame = SettingsEditorFrame(self.games_tab, game_path=game_path)
        self.settings_editor_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")


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