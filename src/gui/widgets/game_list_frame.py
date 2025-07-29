import customtkinter as ctk
from PIL import Image
from pathlib import Path
from optiscaler.manager import OptiScalerManager
from CTkMessagebox import CTkMessagebox
from utils.translation_manager import t
from utils.progress import progress_manager
from utils.debug import debug_log
from utils.update_manager import update_manager
from utils.robust_wrapper import robust_wrapper
from utils.compatibility_checker import compatibility_checker

class GameListFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, games, game_scanner, on_edit_settings, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.games = games
        self.game_scanner = game_scanner
        self.optiscaler_manager = OptiScalerManager()
        self.on_edit_settings = on_edit_settings
        
        # Cache update check result to avoid multiple API calls
        self._update_check_cache = None
        self._cache_timestamp = None

        self._display_games()

    def _get_update_info(self):
        """Get update information with caching to avoid multiple API calls"""
        import time
        
        # Cache for 5 minutes
        cache_duration = 300  # seconds
        current_time = time.time()
        
        # Return cached result if still valid
        if (self._update_check_cache is not None and 
            self._cache_timestamp is not None and 
            current_time - self._cache_timestamp < cache_duration):
            return self._update_check_cache
        
        # Perform new update check
        try:
            update_info = update_manager.check_for_updates()
            self._update_check_cache = update_info
            self._cache_timestamp = current_time
            debug_log(f"Update check completed: {update_info.get('available', False)}")
            return update_info
        except Exception as e:
            debug_log(f"Update check failed: {e}")
            # Return cached result if available, otherwise indicate no updates
            if self._update_check_cache is not None:
                return self._update_check_cache
            return {"available": False}

    def _display_games(self):
        for i, game in enumerate(self.games):
            game_frame = ctk.CTkFrame(self)
            game_frame.grid(row=i, column=0, padx=5, pady=5, sticky="ew")
            game_frame.grid_columnconfigure(0, weight=0) # Image column, no stretching
            game_frame.grid_columnconfigure(1, weight=1) # Info frame column, takes available space

            # Game Image
            image_path = game.image_path
            if not image_path or (image_path and not Path(image_path).exists()):
                image_path = self.game_scanner.fetch_game_image(game.name, game.appid)
                if image_path:
                    game.image_path = image_path

            # Define the target height for all images
            target_height = 80

            if game.image_path and Path(game.image_path).exists():
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

            # Check if OptiScaler is installed (using robust wrapper)
            success, message, is_installed = robust_wrapper.safe_operation("is_optiscaler_installed", game.path)
            if not success:
                # Fallback to basic check
                debug_log(f"Robust check failed for {game.name}, using fallback: {message}")
                is_installed = robust_wrapper._fallback_is_installed(game.path)
            
            debug_log(f"DEBUG: OptiScaler installed check for {game.name} at {game.path}: {is_installed}")

            # Install/Uninstall Button (dynamic based on installation status)
            if is_installed:
                action_button = ctk.CTkButton(buttons_frame, text=t("ui.uninstall") + " OptiScaler", 
                                            fg_color="#d32f2f", hover_color="#b71c1c",
                                            command=lambda g=game: self._uninstall_optiscaler_for_game(g))
            else:
                action_button = ctk.CTkButton(buttons_frame, text=t("ui.install_optiscaler"),
                                            command=lambda g=game: self._install_optiscaler_for_game(g))
            action_button.grid(row=0, column=0, padx=5, pady=2, sticky="e")

            # Check for updates if installed
            update_available = False
            button_row = 1
            
            if is_installed:
                # Check if update is available for this game (using cached result)
                try:
                    update_info = self._get_update_info()
                    update_available = update_info.get("available", False)
                except Exception as e:
                    debug_log(f"Failed to check updates for {game.name}: {e}")
                    update_available = False
                
                # Update Button (only show if update is available)
                if update_available:
                    update_button = ctk.CTkButton(buttons_frame, text=t("ui.update") + " OptiScaler",
                                                fg_color="#ff9800", hover_color="#f57c00",
                                                command=lambda g=game: self._update_optiscaler_for_game(g))
                    update_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")
                    button_row += 1
                
                # Edit Settings Button
                edit_settings_button = ctk.CTkButton(buttons_frame, text=t("ui.edit_settings"),
                                                   command=lambda g_path=game.path: self.on_edit_settings(g_path))
                edit_settings_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")
                button_row += 1
                
            row_offset = button_row

            # Open Folder Button
            open_folder_button = ctk.CTkButton(buttons_frame, text=t("ui.open_folder"),
                                             command=lambda p=game.path: self._open_game_folder(p))
            open_folder_button.grid(row=row_offset, column=0, padx=5, pady=2, sticky="e")

    def _install_optiscaler_for_game(self, game):
        """Install OptiScaler for the selected game with enhanced error handling"""
        debug_log(f"Installing OptiScaler for {game.name} at {game.path}")
        
        # Validate environment before installation
        env_valid, env_warnings = robust_wrapper.validate_operation_environment("install", game.path)
        
        if not env_valid:
            CTkMessagebox(
                title=t("ui.installation_error"),
                message=f"{t('ui.cannot_install')}:\n\n" + "\n".join(env_warnings)
            )
            return
        
        # Show environment warnings if any
        if env_warnings:
            warning_result = CTkMessagebox(
                title=t("ui.installation_warnings"),
                message=f"{t('ui.potential_issues')}:\n\n" + "\n".join(env_warnings) + f"\n\n{t('ui.continue_installation')}?",
                icon="warning",
                option_1=t("ui.cancel"),
                option_2=t("ui.continue")
            )
            
            if warning_result.get() == t("ui.cancel"):
                return
        
        # Show indeterminate progress overlay
        progress_manager.start_indeterminate("main", t("status.optiscaler_installation"), f"{t('status.installing_optiscaler_for')} {game.name}...")
        
        def progress_callback(stage, data=None):
            """Handle progress updates from installation with enhanced error reporting"""
            debug_log(f"Progress callback: stage='{stage}', data={data}")
            
            # Handle completion stages
            if stage == "install_complete":
                # Installation successful
                progress_manager.hide_progress("main")
                debug_log(f"Installation successful for {game.name}")
                message = data.get("message", "Installation completed successfully") if data else "Installation completed successfully"
                CTkMessagebox(title=t("ui.success"), message=f"{t('ui.optiscaler_installed')} {game.name}!\n\n{message}")
                self._refresh_display()
            elif stage == "install_error":
                # Installation failed - report to compatibility checker
                progress_manager.hide_progress("main")
                error_msg = data.get("message", "Unknown error") if data else "Unknown error"
                debug_log(f"Installation failed for {game.name}: {error_msg}")
                
                # Report the error for tracking
                compatibility_checker.report_installation_issue(
                    "current_version",  # We don't know the version being installed
                    error_msg,
                    {"operation": "install", "game": game.name, "path": game.path}
                )
                
                CTkMessagebox(title=t("ui.error"), message=f"{t('ui.failed_to_install')} {game.name}.\n\n{t('ui.error')}: {error_msg}")
            elif isinstance(stage, str):
                # Status update message
                debug_log(f"Progress: {stage}")
                progress_manager.update_status("main", stage)
            else:
                # Fallback for other data formats
                message = str(stage) if stage else "Processing..."
                debug_log(f"Progress: {message}")
                progress_manager.update_status("main", message)
        
        # Try robust installation first
        def install_threaded():
            """Installation with fallback mechanisms"""
            try:
                # Try primary installation method
                if robust_wrapper.optiscaler_manager:
                    thread = robust_wrapper.optiscaler_manager.install_optiscaler_threaded(
                        game.path, 
                        target_filename="nvngx.dll",
                        progress_callback=progress_callback
                    )
                    debug_log(f"Started primary installation thread for {game.name}")
                else:
                    # Use fallback installation
                    debug_log(f"Using fallback installation for {game.name}")
                    progress_manager.update_status("main", "Using fallback installation method...")
                    
                    success, message = robust_wrapper._fallback_install(game.path)
                    
                    if success:
                        progress_callback("install_complete", {"message": message})
                    else:
                        progress_callback("install_error", {"message": message})
                        
            except Exception as e:
                debug_log(f"Primary installation failed, trying fallback: {e}")
                progress_manager.update_status("main", "Primary method failed, trying fallback...")
                
                try:
                    success, message = robust_wrapper._fallback_install(game.path)
                    
                    if success:
                        progress_callback("install_complete", {"message": f"Fallback installation successful: {message}"})
                    else:
                        progress_callback("install_error", {"message": f"Both primary and fallback failed: {e}, {message}"})
                        
                except Exception as fallback_error:
                    progress_callback("install_error", {"message": f"All installation methods failed: {e}, {fallback_error}"})
        
        # Start installation in background
        import threading
        install_thread = threading.Thread(target=install_threaded, daemon=True)
        install_thread.start()

    def _update_optiscaler_for_game(self, game):
        """Update OptiScaler for the selected game with compatibility checking"""
        debug_log(f"Updating OptiScaler for {game.name} at {game.path}")
        
        # Use cached update info to avoid double-checking
        update_info = self._get_update_info()
        
        if not update_info.get("available", False):
            # No updates available
            CTkMessagebox(
                title=t("ui.no_updates"), 
                message=f"{t('ui.already_up_to_date')}\n\n{t('ui.current_version')}: {update_info.get('latest_version', 'Unknown')}"
            )
            return
        
        # Check compatibility and get recommendations
        latest_version = update_info.get("latest_version", "Unknown")
        current_version = update_info.get("cached_version", "Unknown")
        
        compatibility = compatibility_checker.check_version_compatibility(latest_version)
        recommendation = compatibility_checker.get_safe_update_recommendation(current_version, latest_version)
        
        # Build update message with warnings
        release_info = update_info.get("release_info", {})
        release_name = release_info.get("name", latest_version)
        
        message_parts = [
            f"{t('ui.new_version_available')}\n",
            f"{t('ui.current')}: {current_version}",
            f"{t('ui.latest')}: {latest_version}",
            f"{t('ui.release')}: {release_name}"
        ]
        
        # Add compatibility warnings
        if not compatibility["compatible"]:
            message_parts.append(f"\n{t('ui.compatibility_warning')}")
            message_parts.extend([f"• {issue}" for issue in compatibility["issues"]])
            message_parts.append(f"\n{t('ui.version_may_not_work')}")
        
        elif compatibility["warnings"]:
            message_parts.append(f"\n{t('ui.cautions')}")
            message_parts.extend([f"• {warning}" for warning in compatibility["warnings"]])
        
        # Add safety recommendations
        if recommendation["backup_recommended"]:
            message_parts.append(f"\n{t('ui.backup_recommendation')}")
        
        if recommendation["risk_level"] == "high":
            message_parts.append(f"\n{t('ui.high_risk')}")
        elif recommendation["risk_level"] == "medium":
            message_parts.append(f"\n{t('ui.medium_risk')}")
        
        message_parts.append(f"\n{t('ui.update_question')} {game.name}?")
        
        # Show enhanced confirmation dialog
        result = CTkMessagebox(
            title=t("ui.update_available"), 
            message="\n".join(message_parts),
            icon="question" if recommendation["safe_to_update"] else "warning",
            option_1=t("ui.cancel"), 
            option_2=t("ui.update_anyway") if not recommendation["safe_to_update"] else t("ui.update")
        )
        
        if result.get() in [t("ui.update"), t("ui.update_anyway")]:
            # Validate environment before updating
            env_valid, env_warnings = robust_wrapper.validate_operation_environment("install", game.path)
            
            if not env_valid:
                CTkMessagebox(
                    title=t("ui.environment_error"),
                    message=f"{t('ui.cannot_update')}:\n\n" + "\n".join(env_warnings)
                )
                return
            
            # Show environment warnings if any
            if env_warnings:
                warning_result = CTkMessagebox(
                    title=t("ui.environment_warnings"),
                    message=f"{t('ui.potential_issues')}:\n\n" + "\n".join(env_warnings) + f"\n\n{t('ui.continue_anyway')}?",
                    icon="warning",
                    option_1=t("ui.cancel"),
                    option_2=t("ui.continue")
                )
                
                if warning_result.get() == t("ui.cancel"):
                    return
            
            # Show progress overlay during update
            progress_manager.start_indeterminate("main", t("status.optiscaler_update"), f"{t('status.updating_optiscaler_for')} {game.name}...")
            
            def progress_callback(message):
                """Update progress status"""
                debug_log(f"Update progress: {message}")
                progress_manager.update_status("main", message)
            
            def update_threaded():
                """Update in background thread with error reporting"""
                try:
                    success, message = update_manager.update_optiscaler_for_game(
                        game.path, 
                        progress_callback=progress_callback
                    )
                    
                    # Report any issues to compatibility checker
                    if not success:
                        compatibility_checker.report_installation_issue(
                            latest_version,
                            message,
                            {"operation": "update", "game": game.name, "path": game.path}
                        )
                    
                    # Update UI in main thread
                    self.after(0, lambda: self._handle_update_result(game, success, message, latest_version))
                    
                except Exception as e:
                    debug_log(f"ERROR: Update failed: {e}")
                    
                    # Report exception to compatibility checker
                    compatibility_checker.report_installation_issue(
                        latest_version,
                        str(e),
                        {"operation": "update", "game": game.name, "path": game.path, "exception": True}
                    )
                    
                    self.after(0, lambda: self._handle_update_result(game, False, str(e), latest_version))
            
            # Start update in background
            import threading
            update_thread = threading.Thread(target=update_threaded, daemon=True)
            update_thread.start()
    
    def _handle_update_result(self, game, success, message, version):
        """Handle update completion"""
        progress_manager.hide_progress("main")
        
        if success:
            debug_log(f"Update successful for {game.name}")
            CTkMessagebox(
                title=t("ui.update_successful"), 
                message=f"{t('ui.optiscaler_updated')} {game.name}!\n\n"
                       f"{t('ui.update_version')}: {version}\n"
                       f"{t('ui.details')}: {message}"
            )
            # Refresh the game list to update button states
            debug_log("Refreshing game list display after update...")
            self._refresh_display()
        else:
            debug_log(f"Update failed for {game.name}: {message}")
            CTkMessagebox(
                title=t("ui.update_failed"), 
                message=f"{t('ui.failed_to_update')} {game.name}.\n\n"
                       f"{t('ui.error')}: {message}"
            )

    def _uninstall_optiscaler_for_game(self, game):
        """Uninstall OptiScaler from the selected game with robust methods"""
        debug_log(f"Uninstalling OptiScaler for {game.name} at {game.path}")
        
        # Show confirmation dialog
        result = CTkMessagebox(title=t("ui.confirm_uninstall"), 
                              message=f"{t('ui.sure_to_uninstall')} {game.name}?\n\n{t('ui.remove_all_files')}",
                              icon="question", option_1=t("ui.cancel"), option_2=t("ui.uninstall"))
        
        if result.get() == t("ui.uninstall"):
            # Show progress overlay during uninstall
            progress_manager.start_indeterminate("main", t("status.optiscaler_uninstall"), f"{t('status.uninstalling_optiscaler_from')} {game.name}...")
            
            def uninstall_threaded():
                """Uninstall with fallback mechanisms"""
                try:
                    # Try primary uninstall method
                    if robust_wrapper.optiscaler_manager:
                        success, message = robust_wrapper.optiscaler_manager.uninstall_optiscaler(game.path)
                    else:
                        # Use fallback immediately if no manager
                        success, message = robust_wrapper._fallback_uninstall(game.path)
                    
                    # Update UI in main thread
                    self.after(0, lambda: self._handle_uninstall_result(game, success, message))
                    
                except Exception as e:
                    debug_log(f"Primary uninstall failed, trying fallback: {e}")
                    
                    try:
                        # Try fallback uninstall
                        success, message = robust_wrapper._fallback_uninstall(game.path)
                        fallback_message = f"Primary method failed, fallback used: {message}"
                        self.after(0, lambda: self._handle_uninstall_result(game, success, fallback_message))
                        
                    except Exception as fallback_error:
                        error_message = f"Both primary and fallback failed: {e}, {fallback_error}"
                        debug_log(f"ERROR: {error_message}")
                        self.after(0, lambda: self._handle_uninstall_result(game, False, error_message))
            
            # Start uninstall in background
            import threading
            uninstall_thread = threading.Thread(target=uninstall_threaded, daemon=True)
            uninstall_thread.start()
    
    def _handle_uninstall_result(self, game, success, message):
        """Handle uninstall completion"""
        progress_manager.hide_progress("main")
        
        if success:
            debug_log(f"Uninstallation successful for {game.name}")
            CTkMessagebox(title=t("ui.success"), message=f"{t('ui.optiscaler_uninstalled')}\n\n{message}")
            # Refresh the game list to update button states
            debug_log("Refreshing game list display after uninstall...")
            self._refresh_display()
        else:
            debug_log(f"Uninstallation failed for {game.name}: {message}")
            CTkMessagebox(title=t("ui.error"), message=f"{t('ui.failed_to_uninstall')}: {message}")

    def _refresh_display(self):
        """Refresh the game list display to update button states"""
        # Clear update cache to force fresh check
        self._update_check_cache = None
        self._cache_timestamp = None
        
        # Clear current display
        for widget in self.winfo_children():
            widget.destroy()
        # Recreate the display
        self._display_games()

    def _open_game_folder(self, path):
        import subprocess
        if Path(path).exists():
            subprocess.Popen(f'explorer \"{path}\"')
        else:
            CTkMessagebox(title=t("ui.error"), message=t("ui.game_folder_not_found"))