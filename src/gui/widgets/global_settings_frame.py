import customtkinter as ctk
from pathlib import Path
import shutil
from utils.translation_manager import t, get_translation_manager, get_languages, set_language
from utils.debug import debug_log
from utils.config import get_config_value, set_config_value
from utils.archive_extractor import archive_extractor
import webbrowser

class GlobalSettingsFrame(ctk.CTkScrollableFrame):
    """Global application settings interface"""
    
    def __init__(self, master, on_language_change=None, main_window=None, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.on_language_change = on_language_change
        self.main_window = main_window
        
        self._create_settings_interface()
    
    def _create_settings_interface(self):
        """Create the global settings interface"""
        
        # Title
        title_label = ctk.CTkLabel(self, text=t("ui.global_settings_title"), 
                                 font=("Arial", 20, "bold"))
        title_label.grid(row=0, column=0, padx=20, pady=(20, 10), sticky="w")
        
        # Language Settings Section
        self._create_language_section()
        
        # Application Settings Section  
        self._create_app_settings_section()
        
        # Cache Settings Section
        self._create_cache_section()
        
        # About Section
        self._create_about_section()
    
    def _create_language_section(self):
        """Create language settings section"""
        lang_frame = ctk.CTkFrame(self)
        lang_frame.grid(row=1, column=0, padx=20, pady=10, sticky="ew")
        lang_frame.grid_columnconfigure(1, weight=1)
        
        # Section title
        lang_title = ctk.CTkLabel(lang_frame, text=t("ui.language_settings"), 
                                font=("Arial", 16, "bold"))
        lang_title.grid(row=0, column=0, columnspan=2, padx=15, pady=(15, 5), sticky="w")
        
        # Language selector
        lang_label = ctk.CTkLabel(lang_frame, text=t("ui.interface_language"))
        lang_label.grid(row=1, column=0, padx=15, pady=5, sticky="w")
        
        self.language_var = ctk.StringVar()
        tm = get_translation_manager()
        current_lang = tm.current_language
        language_names = get_languages()
        self.language_var.set(language_names.get(current_lang, "English"))
        
        language_menu = ctk.CTkOptionMenu(lang_frame,
                                        values=list(language_names.values()),
                                        variable=self.language_var,
                                        command=self._on_language_change)
        language_menu.grid(row=1, column=1, padx=15, pady=5, sticky="w")
        
        # Language info
        lang_info = ctk.CTkLabel(lang_frame, 
                               text=t("ui.language_restart_note"),
                               font=("Arial", 10),
                               text_color="gray")
        lang_info.grid(row=2, column=0, columnspan=2, padx=15, pady=(0, 15), sticky="w")
    
    def _create_app_settings_section(self):
        """Create application settings section"""
        app_frame = ctk.CTkFrame(self)
        app_frame.grid(row=2, column=0, padx=20, pady=10, sticky="ew")
        app_frame.grid_columnconfigure(1, weight=1)
        
        # Section title
        app_title = ctk.CTkLabel(app_frame, text=t("ui.app_settings"), 
                               font=("Arial", 16, "bold"))
        app_title.grid(row=0, column=0, columnspan=2, padx=15, pady=(15, 5), sticky="w")
        
        # Debug mode setting
        debug_label = ctk.CTkLabel(app_frame, text=t("ui.debug_mode"))
        debug_label.grid(row=1, column=0, padx=15, pady=5, sticky="w")
        
        from utils.config import get_config_value
        self.debug_var = ctk.BooleanVar()
        # Persisted debug setting takes precedence; otherwise use current runtime state
        from utils.debug import is_debug_enabled
        persisted_debug = bool(get_config_value('debug', is_debug_enabled()))
        self.debug_var.set(persisted_debug)
        debug_switch = ctk.CTkSwitch(app_frame, 
                                   text=t("ui.enable_debug"),
                                   variable=self.debug_var,
                                   command=self._on_debug_toggle)
        debug_switch.grid(row=1, column=1, padx=15, pady=5, sticky="w")
        # Add a button inside Settings to open the debug Log frame; disabled when not in debug mode.
        self.view_log_btn = ctk.CTkButton(app_frame, text=t("ui.view_log", "View Log"), command=self._open_log)
        self.view_log_btn.grid(row=1, column=2, padx=15, pady=5, sticky="w")
        self.view_log_btn.configure(state='normal' if self.debug_var.get() else 'disabled')
        
        # Auto-update checking
        update_label = ctk.CTkLabel(app_frame, text=t("ui.auto_update_check"))
        update_label.grid(row=2, column=0, padx=15, pady=5, sticky="w")
        
        self.update_var = ctk.BooleanVar(value=True)
        update_switch = ctk.CTkSwitch(app_frame,
                                    text=t("ui.check_for_updates"),
                                    variable=self.update_var)
        update_switch.grid(row=2, column=1, padx=15, pady=5, sticky="w")
        
        # Theme setting
        theme_label = ctk.CTkLabel(app_frame, text=t("ui.appearance_theme"))
        theme_label.grid(row=3, column=0, padx=15, pady=5, sticky="w")
        
        self.theme_var = ctk.StringVar(value="System")
        theme_menu = ctk.CTkOptionMenu(app_frame,
                                     values=["Light", "Dark", "System"],
                                     variable=self.theme_var,
                                     command=self._on_theme_change)
        theme_menu.grid(row=3, column=1, padx=15, pady=(5, 15), sticky="w")

        # Prefer system 7z setting
        self.prefer_7z_var = ctk.BooleanVar(value=bool(get_config_value('prefer_system_7z', True)))
        prefer_7z_label = ctk.CTkLabel(app_frame, text=t("ui.prefer_system_7z"))
        prefer_7z_label.grid(row=4, column=0, padx=15, pady=5, sticky="w")
        prefer_7z_switch = ctk.CTkSwitch(app_frame,
                                        text=t("ui.prefer_system_7z_switch"),
                                        variable=self.prefer_7z_var,
                                        command=self._on_prefer_7z_toggle)
        prefer_7z_switch.grid(row=4, column=1, padx=15, pady=(5, 15), sticky="w")

        # Max worker count (threadpool size)
        workers_label = ctk.CTkLabel(app_frame, text=t("ui.max_workers"))
        workers_label.grid(row=5, column=0, padx=15, pady=5, sticky="w")
        from utils.config import config as app_config
        self.workers_var = ctk.IntVar(value=getattr(app_config, 'max_workers', 4))
        workers_entry = ctk.CTkEntry(app_frame, textvariable=self.workers_var, width=80)
        workers_entry.grid(row=5, column=1, padx=15, pady=(5, 15), sticky="w")
        # Update config on focus out or enter key
        def _on_workers_change(event=None):
            try:
                value = int(self.workers_var.get())
                from utils.config import config as app_config
                app_config.max_workers = value
                set_config_value('max_workers', int(value))
                debug_log(f"Set max_workers to {value}")
            except Exception as e:
                debug_log(f"Invalid max_workers value: {e}")
        workers_entry.bind('<FocusOut>', _on_workers_change)
        workers_entry.bind('<Return>', _on_workers_change)

        # PowerShell discovery toggle
        self.powershell_discovery_var = ctk.BooleanVar(value=bool(get_config_value('use_powershell_discovery', True)))
        powershell_label = ctk.CTkLabel(app_frame, text='PowerShell Library Discovery')
        powershell_label.grid(row=6, column=0, padx=15, pady=5, sticky='w')
        powershell_switch = ctk.CTkSwitch(app_frame, text='Use PowerShell for library discovery (Windows only)', variable=self.powershell_discovery_var, command=self._on_powershell_discovery_toggle)
        powershell_switch.grid(row=6, column=1, padx=15, pady=(5, 15), sticky='w')

        # Button to view discovered library roots (opens a new frame)
        view_roots_btn = ctk.CTkButton(app_frame, text='View detected library roots', command=self._view_library_roots)
        view_roots_btn.grid(row=7, column=1, padx=15, pady=(5, 15), sticky='w')

        # Library discovery cache TTL
        ttl_label = ctk.CTkLabel(app_frame, text='Library discovery cache TTL (seconds)')
        ttl_label.grid(row=8, column=0, padx=15, pady=5, sticky='w')
        from utils.config import config as app_config
        self.library_cache_ttl_var = ctk.IntVar(value=get_config_value('library_discovery_cache_ttl', getattr(app_config, 'library_discovery_cache_ttl', 86400)))
        ttl_entry = ctk.CTkEntry(app_frame, textvariable=self.library_cache_ttl_var, width=80)
        ttl_entry.grid(row=8, column=1, padx=15, pady=(5, 15), sticky='w')

        def _on_ttl_change(event=None):
            try:
                value = int(self.library_cache_ttl_var.get())
                set_config_value('library_discovery_cache_ttl', int(value))
                debug_log(f"Set library discovery cache TTL to {value}s")
            except Exception as e:
                debug_log(f"Invalid TTL value: {e}")
        ttl_entry.bind('<FocusOut>', _on_ttl_change)
        ttl_entry.bind('<Return>', _on_ttl_change)

        # NOTE: The library summary UI option was removed. Discovery logging is
        # still available through the Log viewer and the scanner's last_library_summary.

        # Engine support toggles
        engine_label = ctk.CTkLabel(app_frame, text='Engine Support')
        engine_label.grid(row=9, column=0, padx=15, pady=5, sticky='w')
        from utils.compatibility_checker import compatibility_checker
        # Keep an internal map of vars so we can read next
        self.engine_vars = {}
        known_engines = ['Unreal', 'Unity', 'Godot', 'Prism3D']
        for idx, engine in enumerate(known_engines, start=10):
            var = ctk.BooleanVar(value=compatibility_checker.is_engine_supported(engine))
            self.engine_vars[engine] = var
            eng_switch = ctk.CTkSwitch(app_frame, text=engine, variable=var, command=lambda e=engine, v=var: self._on_engine_toggle(e, v))
            eng_switch.grid(row=idx, column=1, padx=15, pady=(2, 8), sticky='w')

        # Filters
        filter_title = ctk.CTkLabel(app_frame, text='Game List Filters')
        filter_title.grid(row=14, column=0, padx=15, pady=5, sticky='w')
        self.filter_verified_var = ctk.BooleanVar(value=bool(get_config_value('filter_show_verified_only', False)))
        filter_verified_switch = ctk.CTkSwitch(app_frame, text='Show only community-verified games', variable=self.filter_verified_var, command=self._on_filter_verified_toggle)
        filter_verified_switch.grid(row=14, column=1, padx=15, pady=(5, 15), sticky='w')
        self.filter_supported_var = ctk.BooleanVar(value=bool(get_config_value('filter_show_supported_only', False)))
        filter_supported_switch = ctk.CTkSwitch(app_frame, text='Show only supported engine games', variable=self.filter_supported_var, command=self._on_filter_supported_toggle)
        filter_supported_switch.grid(row=15, column=1, padx=15, pady=(5, 15), sticky='w')

        # Excluded drives (comma-separated letters) - optional
        exclude_label = ctk.CTkLabel(app_frame, text='Exclude Drives (comma-separated, e.g., D,E)')
        exclude_label.grid(row=16, column=0, padx=15, pady=5, sticky='w')
        self.excluded_drives_var = ctk.StringVar(value=get_config_value('excluded_drives', ''))
        exclude_entry = ctk.CTkEntry(app_frame, textvariable=self.excluded_drives_var)
        exclude_entry.grid(row=16, column=1, padx=15, pady=(5, 15), sticky='w')

        def _on_excluded_drives_change(event=None):
            try:
                s = self.excluded_drives_var.get() or ''
                # Normalize to uppercase letters separated by commas
                parts = [p.strip().upper() for p in s.split(',') if p.strip()]
                set_config_value('excluded_drives', ','.join(parts))
                debug_log(f"Set excluded drives to: {parts}")
            except Exception as e:
                debug_log(f"Failed to set excluded drives: {e}")
        exclude_entry.bind('<FocusOut>', _on_excluded_drives_change)
        exclude_entry.bind('<Return>', _on_excluded_drives_change)
    
    def _create_cache_section(self):
        """Create cache management section"""
        cache_frame = ctk.CTkFrame(self)
        cache_frame.grid(row=3, column=0, padx=20, pady=10, sticky="ew")
        cache_frame.grid_columnconfigure(1, weight=1)
        
        # Section title
        cache_title = ctk.CTkLabel(cache_frame, text=t("ui.cache_settings"), 
                                 font=("Arial", 16, "bold"))
        cache_title.grid(row=0, column=0, columnspan=2, padx=15, pady=(15, 5), sticky="w")
        
        # Cache info
        cache_info = self._get_cache_info()
        info_label = ctk.CTkLabel(cache_frame, text=cache_info)
        info_label.grid(row=1, column=0, columnspan=2, padx=15, pady=5, sticky="w")
        
        # Clear cache button
        clear_button = ctk.CTkButton(cache_frame,
                                   text=t("ui.clear_cache"),
                                   command=self._clear_cache)
        clear_button.grid(row=2, column=0, padx=15, pady=(5, 15), sticky="w")
        
        # Open cache folder button
        open_button = ctk.CTkButton(cache_frame,
                                  text=t("ui.open_cache_folder"),
                                  command=self._open_cache_folder)
        open_button.grid(row=2, column=1, padx=15, pady=(5, 15), sticky="w")
        # Clear discovery cache button
        clear_lib_cache_button = ctk.CTkButton(cache_frame,
                       text=t('ui.clear_library_discovery_cache', 'Clear library discovery cache'),
                       command=self._clear_library_discovery_cache)
        clear_lib_cache_button.grid(row=3, column=0, padx=15, pady=(5, 15), sticky='w')
    
    def _create_about_section(self):
        """Create about section"""
        about_frame = ctk.CTkFrame(self)
        about_frame.grid(row=4, column=0, padx=20, pady=10, sticky="ew")
        about_frame.grid_columnconfigure(0, weight=1)
        
        # Section title
        about_title = ctk.CTkLabel(about_frame, text=t("ui.about"), 
                                 font=("Arial", 16, "bold"))
        about_title.grid(row=0, column=0, padx=15, pady=(15, 5), sticky="w")
        
        # App info
        from gui.main_window import VERSION
        app_info = f"{t('ui.app_title')} v{VERSION}"
        info_label = ctk.CTkLabel(about_frame, text=app_info, font=("Arial", 12, "bold"))
        info_label.grid(row=1, column=0, padx=15, pady=5, sticky="w")
        
        # Description
        desc_text = t("ui.app_description")
        desc_label = ctk.CTkLabel(about_frame, text=desc_text, wraplength=400)
        desc_label.grid(row=2, column=0, padx=15, pady=5, sticky="w")
        
        # Links frame
        links_frame = ctk.CTkFrame(about_frame, fg_color="transparent")
        links_frame.grid(row=3, column=0, padx=15, pady=10, sticky="w")
        
        # GitHub link
        github_button = ctk.CTkButton(links_frame,
                                    text=t("ui.view_on_github"),
                                    command=lambda: webbrowser.open("https://github.com/optiscaler/OptiScaler"))
        github_button.grid(row=0, column=0, padx=(0, 10))
        
        # OptiScaler link
        optiscaler_button = ctk.CTkButton(links_frame,
                                        text=t("ui.optiscaler_project"),
                                        command=lambda: webbrowser.open("https://github.com/optiscaler/OptiScaler"))
        optiscaler_button.grid(row=0, column=1, padx=(10, 0))
        
        # Copyright
        copyright_label = ctk.CTkLabel(about_frame, 
                                     text="© 2025 OptiScaler-GUI",
                                     font=("Arial", 10),
                                     text_color="gray")
        copyright_label.grid(row=4, column=0, padx=15, pady=(5, 15), sticky="w")
    
    def _on_language_change(self, language_name):
        """Handle language change"""
        debug_log(f"DEBUG: Language change requested to: {language_name}")
        languages = get_languages()
        for code, name in languages.items():
            if name == language_name:
                debug_log(f"DEBUG: Setting language to code: {code}")
                set_language(code)
                debug_log(f"DEBUG: Language set, current language is now: {get_translation_manager().current_language}")
                # Try different callback approaches
                if self.on_language_change:
                    debug_log(f"DEBUG: Callback function: {self.on_language_change}")
                    debug_log(f"DEBUG: Callback type: {type(self.on_language_change)}")
                    debug_log(f"DEBUG: Callback method name: {getattr(self.on_language_change, '__name__', 'Unknown')}")
                    debug_log("DEBUG: Calling refresh callback directly")
                    try:
                        # Test if the callback is callable
                        if callable(self.on_language_change):
                            debug_log("DEBUG: Callback is callable, invoking...")
                            # Try both with and without explicit call to see if there's a difference
                            debug_log("DEBUG: About to call callback method...")
                            result = self.on_language_change()
                            debug_log(f"DEBUG: Callback result: {result}")
                            debug_log("DEBUG: Callback execution completed")
                            debug_log("DEBUG: Refresh callback completed successfully")
                        else:
                            debug_log("DEBUG: Callback is not callable!")
                    except Exception as e:
                        debug_log(f"DEBUG: Error in refresh callback: {e}")
                        import traceback
                        traceback.print_exc()
                else:
                    debug_log("DEBUG: No callback function available")
                break
    
    def _on_debug_toggle(self):
        """Handle debug mode toggle"""
        from utils.debug import set_debug_enabled
        set_debug_enabled(self.debug_var.get())
        # Persist debug setting across restarts
        try:
            from utils.config import set_config_value
            set_config_value('debug', bool(self.debug_var.get()))
        except Exception:
            pass
        debug_log(f"Debug mode {'enabled' if self.debug_var.get() else 'disabled'} from settings")
        
        # Update main window UI (show/hide log tab)
        if self.main_window and hasattr(self.main_window, 'refresh_ui'):
            self.main_window.refresh_ui()
        # Enable/disable the view log button accordingly
        try:
            self.view_log_btn.configure(state='normal' if self.debug_var.get() else 'disabled')
        except Exception:
            pass

    def _open_log(self):
        """Open the debug log viewer using the main window if available."""
        if self.main_window and hasattr(self.main_window, 'show_log'):
            try:
                # If possible, provide a callback to return to Settings
                try:
                    self.main_window.show_log(on_back=self.main_window.show_settings)
                except TypeError:
                    # Fallback for older callers that do not accept on_back
                    self.main_window.show_log()
            except Exception as e:
                debug_log(f"ERROR: Failed to open log window: {e}")
    
    def _on_theme_change(self, theme):
        """Handle theme change"""
        theme_map = {"Light": "light", "Dark": "dark", "System": "system"}
        ctk_theme = theme_map.get(theme, "system")
        ctk.set_appearance_mode(ctk_theme)
        debug_log(f"Theme changed to: {theme}")

    def _on_prefer_7z_toggle(self):
        """Handle prefer system 7z toggle"""
        prefer_value = bool(self.prefer_7z_var.get())
        archive_extractor.set_prefer_system_7z(prefer_value)
        set_config_value('prefer_system_7z', prefer_value)
        debug_log(f"Prefer system 7z set to: {prefer_value}")

    def _on_powershell_discovery_toggle(self):
        """Handle toggle for using PowerShell library discovery"""
        val = bool(self.powershell_discovery_var.get())
        set_config_value('use_powershell_discovery', val)
        debug_log(f"PowerShell library discovery toggled to: {val}")

    def _on_engine_toggle(self, engine, var):
        from utils.compatibility_checker import compatibility_checker
        val = bool(var.get())
        if val:
            compatibility_checker.supported_engines.add(engine)
        else:
            compatibility_checker.supported_engines.discard(engine)
        # persist the updated list into cache via compatibility checker
        compatibility_checker._save_compatibility_data()
        debug_log(f"Engine support for {engine} toggled to {val}")

    def _on_filter_verified_toggle(self):
        try:
            value = bool(self.filter_verified_var.get())
            set_config_value('filter_show_verified_only', value)
            debug_log(f"Filter: show verified only = {value}")
        except Exception as e:
            debug_log(f"Failed to set filter_show_verified_only: {e}")

    def _on_filter_supported_toggle(self):
        try:
            value = bool(self.filter_supported_var.get())
            set_config_value('filter_show_supported_only', value)
            debug_log(f"Filter: show supported only = {value}")
        except Exception as e:
            debug_log(f"Failed to set filter_show_supported_only: {e}")

    # Previously there was a _on_show_summary_toggle handler for the deleted option
    
    def _get_cache_info(self):
        """Get cache directory information"""
        cache_dir = Path("cache")
        if cache_dir.exists():
            total_size = 0
            file_count = 0
            for file_path in cache_dir.rglob("*"):
                if file_path.is_file():
                    total_size += file_path.stat().st_size
                    file_count += 1
            
            size_mb = total_size / (1024 * 1024)
            return f"{file_count} files, {size_mb:.1f} MB"
        return "Cache directory not found"
    
    def _clear_cache(self):
        """Clear the cache directory"""
        import shutil
        from CTkMessagebox import CTkMessagebox
        
        result = CTkMessagebox(title=t("ui.confirm_clear_cache"),
                             message=t("ui.clear_cache_warning"),
                             icon="question",
                             option_1=t("ui.cancel"),
                             option_2=t("ui.clear"))
        
        if result.get() == t("ui.clear"):
            try:
                cache_dir = Path("cache")
                if cache_dir.exists():
                    # Keep the directory structure, just remove files
                    for file_path in cache_dir.rglob("*"):
                        if file_path.is_file() and file_path.name != ".gitkeep":
                            file_path.unlink()
                
                CTkMessagebox(title=t("ui.success"), message=t("ui.cache_cleared"))
                # Refresh cache info
                self._create_cache_section()
                # Also clear scanner cached game results if available to ensure next view triggers a rescan
                try:
                    if self.main_window and hasattr(self.main_window, 'scanner') and hasattr(self.main_window.scanner, 'clear_cached_games'):
                        self.main_window.scanner.clear_cached_games()
                except Exception:
                    pass
            except Exception as e:
                CTkMessagebox(title=t("ui.error"), message=f"{t('ui.failed_to_clear_cache')}: {e}")
    
    def _open_cache_folder(self):
        """Open cache folder in file explorer"""
        import subprocess
        cache_dir = Path("cache").resolve()
        if cache_dir.exists():
            subprocess.Popen(f'explorer "{cache_dir}"')
        else:
            from CTkMessagebox import CTkMessagebox
            CTkMessagebox(title=t("ui.error"), message=t("ui.cache_folder_not_found"))

    def _clear_library_discovery_cache(self):
        """Clear the library discovery cache file"""
        from scanner.library_discovery import clear_library_cache
        from CTkMessagebox import CTkMessagebox
        ok = clear_library_cache()
        if ok:
            CTkMessagebox(title=t('ui.success', 'Success'), message=t('ui.library_discovery_cache_cleared', 'Library discovery cache cleared'))
        else:
            CTkMessagebox(title=t('ui.error', 'Error'), message=t('ui.failed_to_clear_cache', 'Failed to clear library discovery cache'))

    def _view_library_roots(self):
        # Import lazily to avoid circular imports
        from scanner.library_discovery import get_game_libraries
        from gui.widgets.library_roots_frame import LibraryRootsFrame
        def _rescan_callback():
            return get_game_libraries(use_powershell=bool(self.powershell_discovery_var.get()))
        libs = _rescan_callback()
        # Clear content and show library roots frame
        for widget in self.master.winfo_children():
            widget.destroy()
        frame = LibraryRootsFrame(self.master, libraries=libs, on_rescan=_rescan_callback)
        frame.grid(row=0, column=0, sticky='nsew', padx=10, pady=10)
