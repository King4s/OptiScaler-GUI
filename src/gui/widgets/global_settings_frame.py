import customtkinter as ctk
import os
from utils.translation_manager import t, get_translation_manager, get_languages, set_language
from utils.debug import debug_log
from utils.config import get_config_value, set_config_value
import webbrowser

class GlobalSettingsFrame(ctk.CTkScrollableFrame):
    """Global application settings interface"""
    
    def __init__(self, master, on_language_change=None, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.on_language_change = on_language_change
        
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
        
        self.debug_var = ctk.BooleanVar()
        debug_switch = ctk.CTkSwitch(app_frame, 
                                   text=t("ui.enable_debug"),
                                   variable=self.debug_var,
                                   command=self._on_debug_toggle)
        debug_switch.grid(row=1, column=1, padx=15, pady=5, sticky="w")
        
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
        print(f"DEBUG: Language change requested to: {language_name}")
        languages = get_languages()
        for code, name in languages.items():
            if name == language_name:
                print(f"DEBUG: Setting language to code: {code}")
                set_language(code)
                print(f"DEBUG: Language set, current language is now: {get_translation_manager().current_language}")
                # Try different callback approaches
                if self.on_language_change:
                    print(f"DEBUG: Callback function: {self.on_language_change}")
                    print(f"DEBUG: Callback type: {type(self.on_language_change)}")
                    print(f"DEBUG: Callback method name: {getattr(self.on_language_change, '__name__', 'Unknown')}")
                    print("DEBUG: Calling refresh callback directly")
                    try:
                        # Test if the callback is callable
                        if callable(self.on_language_change):
                            print("DEBUG: Callback is callable, invoking...")
                            # Try both with and without explicit call to see if there's a difference
                            print("DEBUG: About to call callback method...")
                            result = self.on_language_change()
                            print(f"DEBUG: Callback result: {result}")
                            print("DEBUG: Callback execution completed")
                            print("DEBUG: Refresh callback completed successfully")
                        else:
                            print("DEBUG: Callback is not callable!")
                    except Exception as e:
                        print(f"DEBUG: Error in refresh callback: {e}")
                        import traceback
                        traceback.print_exc()
                else:
                    print("DEBUG: No callback function available")
                break
    
    def _on_debug_toggle(self):
        """Handle debug mode toggle"""
        from utils.debug import set_debug_enabled
        set_debug_enabled(self.debug_var.get())
        debug_log(f"Debug mode {'enabled' if self.debug_var.get() else 'disabled'} from settings")
    
    def _on_theme_change(self, theme):
        """Handle theme change"""
        theme_map = {"Light": "light", "Dark": "dark", "System": "system"}
        ctk_theme = theme_map.get(theme, "system")
        ctk.set_appearance_mode(ctk_theme)
        debug_log(f"Theme changed to: {theme}")
    
    def _get_cache_info(self):
        """Get cache directory information"""
        cache_dir = "cache"
        if os.path.exists(cache_dir):
            total_size = 0
            file_count = 0
            for root, dirs, files in os.walk(cache_dir):
                for file in files:
                    file_path = os.path.join(root, file)
                    if os.path.exists(file_path):
                        total_size += os.path.getsize(file_path)
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
                cache_dir = "cache"
                if os.path.exists(cache_dir):
                    # Keep the directory structure, just remove files
                    for root, dirs, files in os.walk(cache_dir):
                        for file in files:
                            if file != ".gitkeep":  # Keep .gitkeep files
                                os.remove(os.path.join(root, file))
                
                CTkMessagebox(title=t("ui.success"), message=t("ui.cache_cleared"))
                # Refresh cache info
                self._create_cache_section()
            except Exception as e:
                CTkMessagebox(title=t("ui.error"), message=f"{t('ui.failed_to_clear_cache')}: {e}")
    
    def _open_cache_folder(self):
        """Open cache folder in file explorer"""
        import subprocess
        cache_dir = os.path.abspath("cache")
        if os.path.exists(cache_dir):
            subprocess.Popen(f'explorer "{cache_dir}"')
        else:
            from CTkMessagebox import CTkMessagebox
            CTkMessagebox(title=t("ui.error"), message=t("ui.cache_folder_not_found"))
