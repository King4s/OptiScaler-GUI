import customtkinter as ctk
from PIL import Image
import os
from optiscaler.manager import OptiScalerManager
from CTkMessagebox import CTkMessagebox
from utils.translation_manager import t
from utils.progress import progress_manager

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
            game_frame.grid_columnconfigure(0, weight=0) # Image column, no stretching
            game_frame.grid_columnconfigure(1, weight=1) # Info frame column, takes available space

            # Game Image
            image_path = game.image_path
            if not image_path or not os.path.exists(image_path):
                image_path = self.game_scanner.fetch_game_image(game.name, game.appid)
                if image_path:
                    game.image_path = image_path

            # Define the target height for all images
            target_height = 80

            if game.image_path and os.path.exists(game.image_path):
                try:
                    img = Image.open(game.image_path)
                    
                    original_width, original_height = img.size

                    # Calculate new width to maintain aspect ratio with fixed height
                    new_width = int(original_width * (target_height / original_height))
                    new_height = target_height # Height is fixed

                    img = img.resize((new_width, new_height), Image.LANCZOS)

                    # Create CTkImage with the *actual* new dimensions of the resized image
                    ctk_image = ctk.CTkImage(light_image=img, dark_image=img, size=(new_width, new_height))
                    img_label = ctk.CTkLabel(game_frame, image=ctk_image, text="")
                    img_label.image = ctk_image # Keep a reference
                    img_label.grid(row=0, column=0, sticky="w")
                except Exception as e:
                    # Fallback to placeholder if image loading fails
                    default_aspect_ratio = 16/9 
                    placeholder_width = int(target_height * default_aspect_ratio)
                    placeholder_img = Image.new('RGB', (placeholder_width, target_height), color = 'gray')
                    ctk_placeholder_image = ctk.CTkImage(light_image=placeholder_img, dark_image=placeholder_img, size=(placeholder_width, target_height))
                    placeholder_label = ctk.CTkLabel(game_frame, image=ctk_placeholder_image, text=game.name, compound="center", font=("Arial", 10))
                    placeholder_label.image = ctk_placeholder_image # Keep a reference
                    placeholder_label.grid(row=0, column=0, sticky="w")
            else:
                # For placeholder, create a dynamically sized gray image based on a common aspect ratio
                # Use a common aspect ratio (e.g., 16:9) to determine placeholder width
                default_aspect_ratio = 16/9 
                placeholder_width = int(target_height * default_aspect_ratio)

                placeholder_img = Image.new('RGB', (placeholder_width, target_height), color = 'gray')
                ctk_placeholder_image = ctk.CTkImage(light_image=placeholder_img, dark_image=placeholder_img, size=(placeholder_width, target_height))
                placeholder_label = ctk.CTkLabel(game_frame, image=ctk_placeholder_image, text=game.name, compound="center", font=("Arial", 10))
                placeholder_label.image = ctk_placeholder_image # Keep a reference
                placeholder_label.grid(row=0, column=0, sticky="w")

            # Game Name and Path
            info_frame = ctk.CTkFrame(game_frame)
            info_frame.grid(row=0, column=1, padx=5, pady=5, sticky="ew")
            info_frame.grid_columnconfigure(0, weight=1)

            # Game name
            name_label = ctk.CTkLabel(info_frame, text=game.name, font=("Arial", 14, "bold"))
            name_label.grid(row=0, column=0, sticky="w")

            # Platform/Manager tag
            if hasattr(game, 'platform') and game.platform:
                tag_label = ctk.CTkLabel(info_frame, text=game.platform, font=("Arial", 10, "italic"),
                                         fg_color="#444", text_color="#fff", corner_radius=6, padx=6, pady=2)
                tag_label.grid(row=0, column=1, padx=8, sticky="w")

            # Game path
            path_label = ctk.CTkLabel(info_frame, text=game.path, font=("Arial", 10))
            path_label.grid(row=1, column=0, columnspan=2, sticky="w")

            # Buttons Frame
            buttons_frame = ctk.CTkFrame(game_frame, fg_color="transparent")
            buttons_frame.grid(row=0, column=2, padx=5, pady=5, sticky="e")
            buttons_frame.grid_columnconfigure(0, weight=1)

            # Check if OptiScaler is installed
            is_installed = self.optiscaler_manager.is_optiscaler_installed(game.path)
            print(f"DEBUG: OptiScaler installed check for {game.name} at {game.path}: {is_installed}")

            # Install/Uninstall Button (dynamic based on installation status)
            if is_installed:
                action_button = ctk.CTkButton(buttons_frame, text=t("ui.uninstall") + " OptiScaler", 
                                            fg_color="#d32f2f", hover_color="#b71c1c",
                                            command=lambda g=game: self._uninstall_optiscaler_for_game(g))
            else:
                action_button = ctk.CTkButton(buttons_frame, text=t("ui.install_optiscaler"),
                                            command=lambda g=game: self._install_optiscaler_for_game(g))
            action_button.grid(row=0, column=0, padx=5, pady=2, sticky="e")

            # Edit Settings Button (only show if installed)
            if is_installed:
                edit_settings_button = ctk.CTkButton(buttons_frame, text=t("ui.edit_settings"),
                                                   command=lambda g_path=game.path: self.on_edit_settings(g_path))
                edit_settings_button.grid(row=1, column=0, padx=5, pady=2, sticky="e")
                row_offset = 2
            else:
                row_offset = 1

            # Open Folder Button
            open_folder_button = ctk.CTkButton(buttons_frame, text=t("ui.open_folder"),
                                             command=lambda p=game.path: self._open_game_folder(p))
            open_folder_button.grid(row=row_offset, column=0, padx=5, pady=2, sticky="e")

    def _install_optiscaler_for_game(self, game):
        """Install OptiScaler for the selected game with progress feedback"""
        print(f"Installing OptiScaler for {game.name} at {game.path}")
        
        # Show indeterminate progress overlay
        progress_manager.start_indeterminate("main", "OptiScaler Installation", f"Installing OptiScaler for {game.name}...")
        
        def status_callback(message):
            """Update progress status"""
            print(f"Progress: {message}")
            # Update the overlay status message
            progress_manager.update_status("main", message)
            
        def done_callback(success):
            """Handle installation completion"""
            progress_manager.hide_progress("main")
            
            if success:
                print(f"Installation successful for {game.name}")
                CTkMessagebox(title="Success", message=f"OptiScaler installed successfully for {game.name}!")
                # Refresh the game list to update button states
                print("Refreshing game list display...")
                self._refresh_display()
            else:
                print(f"Installation failed for {game.name}")
                CTkMessagebox(title="Error", message=f"Failed to install OptiScaler for {game.name}.")
        
        # Use threaded installation with callbacks
        self.optiscaler_manager.install_optiscaler_threaded(
            game.path, 
            status_callback=status_callback,
            done_callback=done_callback
        )

    def _uninstall_optiscaler_for_game(self, game):
        """Uninstall OptiScaler from the selected game"""
        print(f"Uninstalling OptiScaler for {game.name} at {game.path}")
        
        # Show confirmation dialog
        result = CTkMessagebox(title="Confirm Uninstall", 
                              message=f"Are you sure you want to uninstall OptiScaler from {game.name}?\n\nThis will remove all OptiScaler files from the game directory.",
                              icon="question", option_1="Cancel", option_2="Uninstall")
        
        if result.get() == "Uninstall":
            # Show progress overlay during uninstall
            progress_manager.start_indeterminate("main", "OptiScaler Uninstall", f"Uninstalling OptiScaler from {game.name}...")
            
            def uninstall_threaded():
                """Uninstall in background thread"""
                try:
                    success, message = self.optiscaler_manager.uninstall_optiscaler(game.path)
                    # Update UI in main thread
                    self.after(0, lambda: self._handle_uninstall_result(game, success, message))
                except Exception as e:
                    print(f"ERROR: Uninstall failed: {e}")
                    self.after(0, lambda: self._handle_uninstall_result(game, False, str(e)))
            
            # Start uninstall in background
            import threading
            uninstall_thread = threading.Thread(target=uninstall_threaded, daemon=True)
            uninstall_thread.start()
    
    def _handle_uninstall_result(self, game, success, message):
        """Handle uninstall completion"""
        progress_manager.hide_progress("main")
        
        if success:
            print(f"Uninstallation successful for {game.name}")
            CTkMessagebox(title="Success", message=f"OptiScaler uninstalled successfully!\n\n{message}")
            # Refresh the game list to update button states
            print("Refreshing game list display after uninstall...")
            self._refresh_display()
        else:
            print(f"Uninstallation failed for {game.name}: {message}")
            CTkMessagebox(title="Error", message=f"Failed to uninstall OptiScaler: {message}")

    def _refresh_display(self):
        """Refresh the game list display to update button states"""
        # Clear current display
        for widget in self.winfo_children():
            widget.destroy()
        # Recreate the display
        self._display_games()

    def _open_game_folder(self, path):
        import subprocess
        if os.path.exists(path):
            subprocess.Popen(f'explorer \"{path}\"')
        else:
            CTkMessagebox(title="Error", message="Game folder not found.")