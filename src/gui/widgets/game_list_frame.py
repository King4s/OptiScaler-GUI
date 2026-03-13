import customtkinter as ctk
from PIL import Image, ImageDraw, ImageFont
# Determine correct LANCZOS constant across Pillow versions without static attribute
LANCZOS = getattr(getattr(Image, 'Resampling', Image), 'LANCZOS', None)
if LANCZOS is None:
    # Fallback to a commonly available resampling filter (BICUBIC or NEAREST if not present)
    LANCZOS = getattr(Image, 'BICUBIC', getattr(Image, 'NEAREST', None))
from pathlib import Path
import tkinter as tk
from optiscaler.manager import OptiScalerManager
from CTkMessagebox import CTkMessagebox
import concurrent.futures
from utils.config import config as app_config
from utils.translation_manager import t
from utils.progress import progress_manager
from utils.debug import debug_log
from utils.update_manager import update_manager
from utils.compatibility_checker import compatibility_checker

# PyInstaller-aware import system
import sys
import os

# Detect if running in PyInstaller bundle
RUNNING_IN_PYINSTALLER = getattr(sys, 'frozen', False)

if RUNNING_IN_PYINSTALLER:
    debug_log("Running in PyInstaller - using simplified game management")
    # Simplified wrapper for PyInstaller
    class SimpleGameWrapper:
        def __init__(self):
            # Add optiscaler_manager for compatibility
            try:
                from optiscaler.manager import OptiScalerManager
                self.optiscaler_manager = OptiScalerManager()
                debug_log("OptiScalerManager initialized in SimpleGameWrapper")
            except Exception as e:
                debug_log(f"Failed to initialize OptiScalerManager in SimpleGameWrapper: {e}")
                self.optiscaler_manager = None
        
        def safe_operation(self, operation, path):
            if operation == "is_optiscaler_installed":
                # Simple file-based check
                try:
                    from pathlib import Path
                    game_path = Path(path)
                    # Look for common OptiScaler files
                    optiscaler_files = ['nvngx_dlss.dll', 'nvngx_dlssg.dll', 'OptiScaler.dll']
                    for file in optiscaler_files:
                        if (game_path / file).exists():
                            return True, "OptiScaler detected", True
                    return True, "OptiScaler not detected", False
                except Exception as e:
                    return False, f"Check failed: {e}", False
            return False, "Operation not supported in PyInstaller mode", False
        
        def _fallback_is_installed(self, path):
            # Same simple check
            try:
                from pathlib import Path
                game_path = Path(path)
                optiscaler_files = ['nvngx_dlss.dll', 'nvngx_dlssg.dll', 'OptiScaler.dll']
                for file in optiscaler_files:
                    if (game_path / file).exists():
                        return True
                return False
            except:
                return False
        
        def validate_operation_environment(self, operation, path):
            return True, []  # Always valid in simple mode
        
        def _fallback_install(self, path):
            return False, "Installation not supported in PyInstaller mode - please use development version"
        
        def _fallback_uninstall(self, path):
            return False, "Uninstallation not supported in PyInstaller mode - please use development version"
    
    robust_wrapper = SimpleGameWrapper()
    debug_log("Using SimpleGameWrapper for PyInstaller compatibility")
else:
    # Normal development mode
    try:
        from utils.robust_wrapper import robust_wrapper
        debug_log("Successfully imported robust_wrapper (development mode)")
    except ImportError as e:
        debug_log(f"Failed to import robust_wrapper: {e}")
        # Create a minimal fallback
        class FallbackRobustWrapper:
            def safe_operation(self, operation, path):
                return False, "robust_wrapper not available", False
            def _fallback_is_installed(self, path):
                return False
            def validate_operation_environment(self, operation, path):
                # Default: assume environment is valid (no additional checks available)
                return True, []
            def _fallback_install(self, path):
                return False, "Installation not supported in this environment"
            def _fallback_uninstall(self, path):
                return False, "Uninstallation not supported in this environment"
            def __init__(self):
                # Provide the attribute that other code expects to exist
                self.optiscaler_manager = None
        robust_wrapper = FallbackRobustWrapper()

def create_placeholder_pil_image(width, height, text=None):
    """Module-level helper to create a simple placeholder PIL image (testable)
    This function is intentionally module-level so tests can call it directly.
    """
    try:
        img = Image.new('RGB', (width, height), color='#444444')
        if text:
            draw = ImageDraw.Draw(img)
            try:
                font = ImageFont.truetype("arial.ttf", size=max(10, int(height / 6)))
            except Exception:
                font = ImageFont.load_default()
            try:
                bbox = draw.textbbox((0, 0), text, font=font)
                text_w = bbox[2] - bbox[0]
                text_h = bbox[3] - bbox[1]
            except Exception:
                # Fallback: approximate text size based on character count and font size
                text_w = len(text) * max(6, int(height / 8))
                text_h = max(12, int(height / 6))
            text_x = (width - text_w) / 2
            text_y = (height - text_h) / 2
            draw.text((text_x, text_y), text, fill=(255, 255, 255), font=font)
        return img
    except Exception:
        try:
            return Image.new('RGB', (width, height), color='gray')
        except Exception:
            return Image.new('RGB', (1, 1), color='gray')

class GameListFrame(ctk.CTkScrollableFrame):

    def __init__(self, master, games, game_scanner, on_edit_settings, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        # Apply cached filters from config
        from utils.config import get_config_value
        self.games = list(games) or []
        show_verified = bool(get_config_value('filter_show_verified_only', False))
        show_supported = bool(get_config_value('filter_show_supported_only', False))
        if show_verified:
            self.games = [g for g in self.games if getattr(g, 'community_verified', False)]
        if show_supported:
            self.games = [g for g in self.games if getattr(g, 'engine_supported', True)]
        self.game_scanner = game_scanner
        self.optiscaler_manager = OptiScalerManager()
        self.on_edit_settings = on_edit_settings
        # ThreadPoolExecutor for background image fetching and other short tasks
        self._executor = concurrent.futures.ThreadPoolExecutor(max_workers=getattr(app_config, 'max_workers', 4))
        self._image_label_map = {}  # Map game.path -> label widget for later updates
        
        # Cache update check result to avoid multiple API calls
        self._update_check_cache = None
        self._cache_timestamp = None

        self._display_games()
        self._schedule_fetch_game_images()

        # When the background SteamSpy download finishes, retry thumbnails for
        # any games that still have the placeholder (no_image or grey square).
        def _on_app_list_ready():
            debug_log("Steam app list ready callback — scheduling thumbnail retry")
            try:
                self.after(0, self._retry_missing_images)
            except Exception as e:
                debug_log(f"Thumbnail retry schedule error: {e}")

        try:
            game_scanner.on_app_list_ready = _on_app_list_ready
        except Exception:
            pass

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
        debug_log(f"_display_games: rendering {len(self.games)} games")
        for i, game in enumerate(self.games):
            game_frame = ctk.CTkFrame(self)
            game_frame.grid(row=i, column=0, padx=5, pady=5, sticky="ew")
            game_frame.grid_columnconfigure(0, weight=0)
            game_frame.grid_columnconfigure(1, weight=1)

            # Placeholder image — replaced in background by _schedule_fetch_game_images
            target_height = 80
            placeholder_width = int(target_height * 16 / 9)
            placeholder_img = self._create_placeholder_pil_image(placeholder_width, target_height)
            ctk_placeholder_image = ctk.CTkImage(light_image=placeholder_img, dark_image=placeholder_img,
                                                  size=(placeholder_width, target_height))
            placeholder_label = ctk.CTkLabel(game_frame, image=ctk_placeholder_image)
            setattr(placeholder_label, '_ctk_image_ref', ctk_placeholder_image)
            placeholder_label.grid(row=0, column=0, sticky="w")
            self._image_label_map[game.path] = placeholder_label

            # Info frame
            info_frame = ctk.CTkFrame(game_frame)
            info_frame.grid(row=0, column=1, padx=(10, 15), pady=5, sticky="ew")
            info_frame.grid_columnconfigure(0, weight=1)

            if i < 5 or i % 10 == 0:
                debug_log(f"Displaying game [{i}]: {game.name}, "
                          f"community_verified={getattr(game, 'community_verified', False)}, "
                          f"optiscaler_installed={getattr(game, 'optiscaler_installed', False)}")

            name_label = ctk.CTkLabel(info_frame, text=game.name, font=("Arial", 14, "bold"))
            name_label.grid(row=0, column=0, sticky="w")

            if hasattr(game, 'platform') and game.platform:
                tag_label = ctk.CTkLabel(info_frame, text=game.platform, font=("Arial", 10, "italic"),
                                         fg_color="#444", text_color="#fff", corner_radius=6, padx=6, pady=2)
                tag_label.grid(row=0, column=1, padx=(10, 0), sticky="w")

            if getattr(game, 'community_verified', False):
                verified_label = ctk.CTkLabel(info_frame, text=t('ui.community_verified', 'Verified'),
                                              font=("Arial", 10), fg_color="#2e7d32", text_color="#fff",
                                              corner_radius=6, padx=6, pady=2)
                verified_label.grid(row=0, column=2, padx=(10, 0), sticky="w")
            elif getattr(game, 'anti_cheat_list', None):
                ac_text = ", ".join(game.anti_cheat_list)
                ac_label = ctk.CTkLabel(info_frame, text=f"{t('ui.anti_cheat', 'Anti-cheat')}: {ac_text}",
                                        font=("Arial", 10), fg_color="#ffa000", text_color="#000",
                                        corner_radius=6, padx=6, pady=2)
                ac_label.grid(row=0, column=2, padx=(10, 0), sticky="w")

            engine = getattr(game, 'engine', None)
            engine_supported = getattr(game, 'engine_supported', True)
            if engine and not engine_supported:
                engine_label = ctk.CTkLabel(info_frame, text=f"{engine} ({t('ui.engine_unsupported')})",
                                             font=("Arial", 10), fg_color="#d32f2f", text_color="#fff",
                                             corner_radius=6, padx=6, pady=2)
                engine_label.grid(row=0, column=3, padx=(10, 0), sticky="w")
            elif engine:
                engine_label = ctk.CTkLabel(info_frame, text=f"{engine}", font=("Arial", 10),
                                             fg_color="#666", text_color="#fff", corner_radius=6, padx=6, pady=2)
                engine_label.grid(row=0, column=3, padx=(10, 0), sticky="w")

            path_label = ctk.CTkLabel(info_frame, text=game.path, font=("Arial", 10))
            path_label.grid(row=1, column=0, columnspan=4, sticky="w")

            # Buttons
            buttons_frame = ctk.CTkFrame(game_frame, fg_color="transparent")
            buttons_frame.grid(row=0, column=2, padx=5, pady=5, sticky="e")
            buttons_frame.grid_columnconfigure(0, weight=1)

            is_installed = getattr(game, 'optiscaler_installed', False)
            debug_log(f"OptiScaler installed for {game.name}: {is_installed}")

            button_row = 0
            if is_installed:
                action_button = ctk.CTkButton(
                    buttons_frame, text=t("ui.uninstall") + " OptiScaler",
                    fg_color="#d32f2f", hover_color="#b71c1c",
                    command=lambda g=game: self._uninstall_optiscaler_for_game(g))
            else:
                action_button = ctk.CTkButton(
                    buttons_frame, text=t("ui.install_optiscaler"),
                    command=lambda g=game: self._install_optiscaler_for_game(g))
            action_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")
            button_row += 1

            if is_installed:
                try:
                    update_info = self._get_update_info()
                    if update_info.get("available", False):
                        update_button = ctk.CTkButton(
                            buttons_frame, text=t("ui.update") + " OptiScaler",
                            fg_color="#ff9800", hover_color="#f57c00",
                            command=lambda g=game: self._update_optiscaler_for_game(g))
                        update_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")
                        button_row += 1
                except Exception as e:
                    debug_log(f"Update check failed for {game.name}: {e}")

                edit_settings_button = ctk.CTkButton(
                    buttons_frame, text=t("ui.edit_settings"),
                    command=lambda g_path=game.path: self.on_edit_settings(g_path))
                edit_settings_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")
                button_row += 1

            open_folder_button = ctk.CTkButton(
                buttons_frame, text=t("ui.open_folder"),
                command=lambda p=game.path: self._open_game_folder(p))
            open_folder_button.grid(row=button_row, column=0, padx=5, pady=2, sticky="e")

        debug_log(f"_display_games: done rendering {len(self.games)} games")

    def _create_placeholder_pil_image(self, width, height, text=None):
        """Return a plain PIL image used as a placeholder before the real thumbnail loads."""
        try:
            img = Image.new('RGB', (width, height), color='#444444')
            if text:
                draw = ImageDraw.Draw(img)
                try:
                    font = ImageFont.truetype("arial.ttf", size=max(10, int(height / 6)))
                except Exception:
                    font = ImageFont.load_default()
                try:
                    bbox = draw.textbbox((0, 0), text, font=font)
                    text_w = bbox[2] - bbox[0]
                    text_h = bbox[3] - bbox[1]
                except Exception:
                    text_w = len(text) * max(6, int(height / 8))
                    text_h = max(12, int(height / 6))
                draw.text(((width - text_w) / 2, (height - text_h) / 2),
                          text, fill=(255, 255, 255), font=font)
            return img
        except Exception:
            try:
                return Image.new('RGB', (width, height), color='gray')
            except Exception:
                return Image.new('RGB', (1, 1), color='gray')

    def _schedule_fetch_game_images(self):
        """Schedule background tasks to fetch images for games that are missing one."""
        target_height = 80

        def fetch_and_update(game):
            try:
                image_path = self.game_scanner.fetch_game_image(game.name, game.appid)
                if image_path:
                    game.image_path = image_path
                    # If the game scanner returned the configured placeholder, log for diagnostics
                    try:
                        if image_path == str(self.game_scanner.no_image_path):
                            debug_log(f"No image available for {game.name}; using configured placeholder.")
                    except Exception:
                        pass

                    def _update():
                        img_label = self._image_label_map.get(game.path)
                        if not img_label or not Path(image_path).exists():
                            return
                        try:
                            img = Image.open(image_path)
                            original_width, original_height = img.size
                            # protect against dividing by zero or unexpected sizes
                            if not original_height or not original_width:
                                raise ValueError("Invalid image dimensions")
                            new_width = int(original_width * (target_height / original_height))
                            if new_width <= 0:
                                new_width = target_height
                            resampler = getattr(Image, 'Resampling', None)
                            if resampler is not None:
                                lanczos = resampler.LANCZOS
                            else:
                                lanczos = LANCZOS
                            img = img.resize((new_width, target_height), lanczos)
                            ctk_image = ctk.CTkImage(light_image=img, dark_image=img, size=(new_width, target_height))
                            img_label.configure(image=ctk_image, text='')
                            setattr(img_label, '_ctk_image_ref', ctk_image)
                        except Exception as e:
                            debug_log(f"Failed to update game image for {game.path}: {e}")
                            # On failure, keep fallback placeholder but ensure it's sized correctly
                            try:
                                img_label = self._image_label_map.get(game.path)
                                if img_label:
                                    placeholder_img = self._create_placeholder_pil_image(int(target_height * (16/9)), target_height)
                                    ctk_placeholder = ctk.CTkImage(light_image=placeholder_img, dark_image=placeholder_img, size=(int(target_height * (16/9)), target_height))
                                    img_label.configure(image=ctk_placeholder, text='')
                                    setattr(img_label, '_ctk_image_ref', ctk_placeholder)
                            except Exception:
                                pass

                    self.after(0, _update)
            except Exception as e:
                debug_log(f"Error fetching image for {game.name}: {e}")
                # If fetching fails, ensure we set a consistent placeholder image on the UI
                def _set_placeholder():
                    try:
                        img_label = self._image_label_map.get(game.path)
                        if img_label:
                            placeholder_img = self._create_placeholder_pil_image(int(target_height * (16/9)), target_height)
                            ctk_placeholder = ctk.CTkImage(light_image=placeholder_img, dark_image=placeholder_img, size=(int(target_height * (16/9)), target_height))
                            img_label.configure(image=ctk_placeholder, text='')
                            setattr(img_label, '_ctk_image_ref', ctk_placeholder)
                    except Exception:
                        pass
                try:
                    self.after(0, _set_placeholder)
                except Exception:
                    _set_placeholder()

        for game in self.games:
            # Always schedule image load/optimize in background to avoid main-thread workload
            try:
                self._executor.submit(fetch_and_update, game)
            except Exception:
                # If executor fails, continue without blocking
                continue

    def _retry_missing_images(self):
        """Re-fetch thumbnails for games that still have no resolved image.
        Called after the SteamSpy app list finishes downloading so games that
        previously got only the placeholder now get a real Steam CDN image."""
        no_img = str(self.game_scanner.no_image_path)
        retry_games = [
            g for g in self.games
            if not g.image_path or g.image_path == no_img or not Path(g.image_path).exists()
        ]
        debug_log(f"Thumbnail retry: {len(retry_games)} games without images")
        if not retry_games:
            return

        target_height = 80

        def fetch_and_update_retry(game):
            try:
                # Clear cached image_path so fetch_game_image tries again
                game.image_path = None
                image_path = self.game_scanner.fetch_game_image(game.name, game.appid)
                if not image_path or image_path == no_img:
                    return
                game.image_path = image_path

                def _update():
                    img_label = self._image_label_map.get(game.path)
                    if not img_label or not Path(image_path).exists():
                        return
                    try:
                        img = Image.open(image_path)
                        ow, oh = img.size
                        if not oh or not ow:
                            return
                        nw = max(1, int(ow * target_height / oh))
                        resampler = getattr(Image, 'Resampling', None)
                        lanczos = resampler.LANCZOS if resampler else LANCZOS
                        img = img.resize((nw, target_height), lanczos)
                        ctk_img = ctk.CTkImage(light_image=img, dark_image=img, size=(nw, target_height))
                        img_label.configure(image=ctk_img, text='')
                        setattr(img_label, '_ctk_image_ref', ctk_img)
                        debug_log(f"Retry thumbnail updated: {game.name}")
                    except Exception as e:
                        debug_log(f"Retry thumbnail update failed for {game.name}: {e}")

                self.after(0, _update)
            except Exception as e:
                debug_log(f"Retry fetch failed for {game.name}: {e}")

        for game in retry_games:
            try:
                self._executor.submit(fetch_and_update_retry, game)
            except Exception:
                continue

    def _install_optiscaler_for_game(self, game):
        """Install OptiScaler for the selected game with enhanced error handling"""
        debug_log(f"Installing OptiScaler for {game.name} at {game.path}")
        
        # Analyze game safety: anti-cheat, engine, community verification
        safety = self.game_scanner.analyze_game_safety(game)
        anti_cheat_list = safety.get('anti_cheat_list', []) if safety else []
        engine = safety.get('engine', 'Unknown') if safety else 'Unknown'
        community_verified = safety.get('community_verified', False) if safety else False

        # Warn user if anti-cheat is present
        if anti_cheat_list:
            ac_text = ', '.join(anti_cheat_list)
            res = CTkMessagebox(
                title=t('ui.anti_cheat_warning_title'),
                message=t('ui.anti_cheat_warning_message', 'Anti-cheat detected: {ac}').format(ac=ac_text),
                icon='warning',
                option_1=t('ui.cancel'),
                option_2=t('ui.continue')
            )
            if res.get() == t('ui.cancel'):
                return

        # Warn about unknown or unsupported engine if not community verified
        if (engine == 'Unknown' or not compatibility_checker.is_engine_supported(engine)) and not community_verified:
            res2 = CTkMessagebox(
                title=t('ui.engine_unknown_warning_title'),
                message=t('ui.engine_unknown_warning_message', 'Unknown engine. Proceed?'),
                icon='warning',
                option_1=t('ui.cancel'),
                option_2=t('ui.continue')
            )
            if res2.get() == t('ui.cancel'):
                return

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
        update_info = self._get_update_info() or {}
        if not isinstance(update_info, dict):
            update_info = {}

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
        # Ensure we pass strings to the compatibility checker
        latest_str = str(latest_version) if latest_version is not None else "Unknown"
        current_str = str(current_version) if current_version is not None else "Unknown"

        compatibility = compatibility_checker.check_version_compatibility(latest_str)
        recommendation = compatibility_checker.get_safe_update_recommendation(current_str, latest_str)
        
        # Build update message with warnings
        release_info = update_info.get("release_info", {}) if isinstance(update_info, dict) else {}
        release_name = release_info.get("name", latest_version) if isinstance(release_info, dict) else latest_version
        
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
                            latest_str,
                            message,
                            {"operation": "update", "game": game.name, "path": game.path}
                        )
                    
                    # Update UI in main thread
                    self.after(0, lambda: self._handle_update_result(game, success, message, latest_str))
                    
                except Exception as e:
                    debug_log(f"ERROR: Update failed: {e}")
                    
                    # Report exception to compatibility checker
                    compatibility_checker.report_installation_issue(
                        latest_str,
                        str(e),
                        {"operation": "update", "game": game.name, "path": game.path, "exception": True}
                    )
                    
                    self.after(0, lambda: self._handle_update_result(game, False, str(e), latest_str))
            
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
        
        # Update OptiScaler status for all games to get current state
        for game in self.games:
            try:
                game.optiscaler_installed = self.game_scanner._detect_optiscaler(game.path)
                debug_log(f"Updated OptiScaler status for {game.name}: {game.optiscaler_installed}")
            except Exception as e:
                debug_log(f"Failed to update OptiScaler status for {game.name}: {e}")
                game.optiscaler_installed = False
        
        # Clear current display; schedule destruction to avoid mid-draw Tcl errors
        for widget in self.winfo_children():
            try:
                self.after(0, lambda w=widget: w.destroy())
            except Exception:
                try:
                    widget.destroy()
                except Exception:
                    pass
        # Recreate the display
        self._display_games()
        # Schedule image fetch/update for the recreated display
        try:
            self._schedule_fetch_game_images()
        except Exception:
            pass

    def _open_game_folder(self, path):
        import subprocess
        if Path(path).exists():
            subprocess.Popen(f'explorer \"{path}\"')
        else:
            CTkMessagebox(title=t("ui.error"), message=t("ui.game_folder_not_found"))

    def destroy(self):
        """Ensure executor is shutdown when frame is destroyed to prevent background threads from lingering."""
        try:
            if hasattr(self, '_executor') and self._executor:
                self._executor.shutdown(wait=False)
        except Exception as e:
            debug_log(f"Error shutting down executor: {e}")
        super().destroy()