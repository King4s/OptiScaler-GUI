import customtkinter as ctk
from PIL import ImageTk, Image
import os
from optiscaler.manager import OptiScalerManager

class GameListFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, games, game_scanner, on_edit_settings, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.games = games
        self.game_scanner = game_scanner
        self.optiscaler_manager = OptiScalerManager()
        self.on_edit_settings = on_edit_settings

        self._display_games()

    def _display_games(self):
        for i, game in enumerate(self.games):
            game_frame = ctk.CTkFrame(self)
            game_frame.grid(row=i, column=0, padx=5, pady=5, sticky="ew")
            game_frame.grid_columnconfigure(1, weight=1)

            # Game Image
            image_path = game.image_path
            if not image_path or not os.path.exists(image_path):
                image_path = self.game_scanner.fetch_game_image(game.name)
                if image_path:
                    game.image_path = image_path

            if game.image_path and os.path.exists(game.image_path):
                img = Image.open(game.image_path)
                img = img.resize((60, 80), Image.LANCZOS)
                img_tk = ImageTk.PhotoImage(img)
                img_label = ctk.CTkLabel(game_frame, image=img_tk, text="")
                img_label.image = img_tk  # Keep a reference!
                img_label.grid(row=0, column=0, padx=5, pady=5, sticky="w")
            else:
                placeholder_label = ctk.CTkLabel(game_frame, text="No Image")
                placeholder_label.grid(row=0, column=0, padx=5, pady=5, sticky="w")

            # Game Name and Path
            info_frame = ctk.CTkFrame(game_frame)
            info_frame.grid(row=0, column=1, padx=5, pady=5, sticky="ew")
            info_frame.grid_columnconfigure(0, weight=1)

            name_label = ctk.CTkLabel(info_frame, text=game.name, font=("Arial", 14, "bold"))
            name_label.grid(row=0, column=0, sticky="w")

            path_label = ctk.CTkLabel(info_frame, text=game.path, font=("Arial", 10))
            path_label.grid(row=1, column=0, sticky="w")

            # Buttons Frame
            buttons_frame = ctk.CTkFrame(game_frame, fg_color="transparent")
            buttons_frame.grid(row=0, column=2, padx=5, pady=5, sticky="e")
            buttons_frame.grid_columnconfigure(0, weight=1)

            # Install Button
            install_button = ctk.CTkButton(buttons_frame, text="Install OptiScaler",
                                           command=lambda g=game: self._install_optiscaler_for_game(g))
            install_button.grid(row=0, column=0, padx=5, pady=2, sticky="e")

            # Edit Settings Button
            edit_settings_button = ctk.CTkButton(buttons_frame, text="Edit Settings",
                                                 command=lambda g_path=game.path: self.on_edit_settings(g_path))
            edit_settings_button.grid(row=1, column=0, padx=5, pady=2, sticky="e")

    def _install_optiscaler_for_game(self, game):
        print(f"Installing OptiScaler for {game.name} at {game.path}")
        success = self.optiscaler_manager.install_optiscaler(game.path)
        if success:
            ctk.CTkMessagebox(title="Success", message=f"OptiScaler installed successfully for {game.name}!")
        else:
            ctk.CTkMessagebox(title="Error", message=f"Failed to install OptiScaler for {game.name}.")
