import customtkinter as ctk
from ..scanner.game_scanner import GameScanner
from .widgets.game_list_frame import GameListFrame
from .widgets.settings_editor_frame import SettingsEditorFrame

class MainWindow(ctk.CTkFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self.game_scanner = GameScanner()
        self.games = self.game_scanner.scan_games()

        self.game_list_frame = GameListFrame(self, games=self.games, game_scanner=self.game_scanner, 
                                             on_edit_settings=self.show_settings_editor)
        self.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")

        self.settings_editor_frame = None

    def show_settings_editor(self, game_path):
        if self.settings_editor_frame:
            self.settings_editor_frame.destroy()
        
        self.game_list_frame.grid_forget() # Hide game list

        self.settings_editor_frame = SettingsEditorFrame(self, game_path=game_path)
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