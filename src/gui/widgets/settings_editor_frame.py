import customtkinter as ctk
import os
from optiscaler.manager import OptiScalerManager
from utils.i18n import t
from utils.debug import debug_log

class SettingsEditorFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, game_path, on_back=None, **kwargs):
        debug_log(f"Creating SettingsEditorFrame for path: {game_path}")
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)
        self.game_path = game_path
        self.on_back = on_back  # Callback function for going back
        self.optiscaler_manager = OptiScalerManager()
        self.ini_path = os.path.join(self.game_path, "OptiScaler.ini")
        self.settings = {}
        self.widgets = {}

        debug_log(f"Looking for ini file at: {self.ini_path}")
        self._load_settings()
        debug_log(f"Settings loaded, creating widgets...")
        self._create_widgets()
        debug_log(f"SettingsEditorFrame created successfully")

    def _load_settings(self):
        if os.path.exists(self.ini_path):
            try:
                self.settings = self.optiscaler_manager.read_optiscaler_ini(self.ini_path)
                debug_log(f"Loaded settings: {len(self.settings)} sections")
            except Exception as e:
                print(f"ERROR: Failed to read ini file: {e}")
                self.settings = {}
        else:
            debug_log(f"OptiScaler.ini not found at {self.ini_path}")
            debug_log(f"You need to install OptiScaler first")
            self.settings = {}

    def _create_widgets(self):
        if not self.settings:
            # No settings loaded - OptiScaler not installed
            self._create_not_installed_view()
            return
        
        # Normal settings display with improved layout
        self._create_settings_view()
        
    def _create_not_installed_view(self):
        """Create view when OptiScaler is not installed"""
        title_label = ctk.CTkLabel(self, text=t("optiscaler_not_installed"), 
                                 font=("Arial", 20, "bold"))
        title_label.grid(row=0, column=0, columnspan=2, padx=10, pady=20, sticky="ew")
        
        info_label = ctk.CTkLabel(self, 
                                text=t("optiscaler_not_installed_msg"),
                                font=("Arial", 12),
                                justify="center")
        info_label.grid(row=1, column=0, columnspan=2, padx=20, pady=20, sticky="ew")
        
        back_button = ctk.CTkButton(self, text=t("back_to_games"), 
                                  command=self._go_back,
                                  font=("Arial", 14))
        back_button.grid(row=2, column=0, columnspan=2, padx=20, pady=20, sticky="ew")
        
    def _create_settings_view(self):
        """Create the main settings view with improved layout"""
        row = 0
        
        # Title
        title_label = ctk.CTkLabel(self, text="OptiScaler Settings", 
                                 font=("Arial", 20, "bold"))
        title_label.grid(row=row, column=0, columnspan=2, padx=10, pady=(10, 20), sticky="ew")
        row += 1
        
        for section, keys in self.settings.items():
            # Section header
            section_frame = ctk.CTkFrame(self, fg_color=("gray85", "gray25"))
            section_frame.grid(row=row, column=0, columnspan=2, padx=10, pady=(20, 10), sticky="ew")
            section_frame.grid_columnconfigure(0, weight=1)
            
            section_label = ctk.CTkLabel(section_frame, text=f"[{section}]", 
                                       font=("Arial", 16, "bold"))
            section_label.grid(row=0, column=0, padx=15, pady=10, sticky="w")
            row += 1

            for key, data in keys.items():
                # Create frame for each setting
                setting_frame = ctk.CTkFrame(self, fg_color=("gray90", "gray20"))
                setting_frame.grid(row=row, column=0, columnspan=2, padx=15, pady=5, sticky="ew")
                setting_frame.grid_columnconfigure(0, weight=1)
                
                # Setting name
                key_label = ctk.CTkLabel(setting_frame, text=key, 
                                       font=("Arial", 12, "bold"))
                key_label.grid(row=0, column=0, padx=15, pady=(10, 5), sticky="w")

                # Setting description - ALWAYS show something for debugging
                # Description
                description = self._get_setting_description(section, key)
                if description:
                    desc_text = description
                else:
                    desc_text = t("no_description_available", default="[No description available]")
                
                desc_label = ctk.CTkLabel(setting_frame, text=desc_text, 
                                        wraplength=300, justify="left", 
                                        font=("Arial", 10))
                desc_label.grid(row=1, column=0, padx=15, pady=(0, 5), sticky="w")

                # Setting widget
                widget = self._create_setting_widget(setting_frame, data)
                widget.grid(row=2, column=0, padx=15, pady=(5, 15), sticky="ew")
                
                # Store widget reference
                self.widgets[f"{section}.{key}"] = widget
                row += 1
        
        # Buttons section
        self._create_buttons(row)
        
    def _get_setting_description(self, section, key):
        """Get user-friendly description for a setting"""
        setting_key = f"{section.lower()}_{key.lower()}_desc"
        
        # Try to get translation, fallback to None if not found
        desc = t(setting_key)
        
        if desc == setting_key:  # Translation not found
            return None
        
        return desc
        
    def _create_setting_widget(self, parent, data):
        """Create appropriate widget for setting data"""
        if data["type"] == "bool_options":
            options_list = ["true", "false", "auto"]
            widget = ctk.CTkOptionMenu(parent, values=options_list)
            widget.set(data["value"].lower())
            return widget
        elif data["type"] == "options":
            display_options = list(data["options"].values())
            widget = ctk.CTkOptionMenu(parent, values=display_options)
            
            current_value_found = False
            for k, v in data["options"].items():
                if k == data["value"]:
                    widget.set(v)
                    current_value_found = True
                    break
            if not current_value_found and data["value"].lower() == "auto" and "auto" in display_options:
                widget.set("auto")
            if not current_value_found and display_options:
                widget.set(display_options[0])
            return widget
        else:
            widget = ctk.CTkEntry(parent)
            widget.insert(0, data["value"])
            return widget
            
    def _create_buttons(self, row):
        """Create the button section"""
        # Knapper sektion
        button_frame = ctk.CTkFrame(self)
        button_frame.grid(row=row, column=0, columnspan=2, padx=10, pady=20, sticky="ew")
        button_frame.grid_columnconfigure(0, weight=1)
        button_frame.grid_columnconfigure(1, weight=1)
        button_frame.grid_columnconfigure(2, weight=1)
        
        back_button = ctk.CTkButton(button_frame, text=t("back"), command=self._go_back)
        back_button.grid(row=0, column=0, padx=5, pady=10, sticky="ew")
        
        auto_button = ctk.CTkButton(button_frame, text=t("auto_settings"), command=self._apply_auto_settings)
        auto_button.grid(row=0, column=1, padx=5, pady=10, sticky="ew")
        
        save_button = ctk.CTkButton(button_frame, text=t("save_settings"), command=self._save_settings)
        save_button.grid(row=0, column=2, padx=5, pady=10, sticky="ew")

    def _detect_gpu(self):
        """Detect the user's GPU and return GPU info"""
        try:
            import subprocess
            import json
            
            # Hent GPU info fra Windows
            cmd = ['powershell', '-Command', '''
                Get-WmiObject -Class Win32_VideoController | 
                Where-Object {$_.Name -notlike "*Basic*" -and $_.Name -notlike "*Generic*"} |
                Select-Object Name, AdapterRAM |
                ConvertTo-Json
            ''']
            
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            
            if result.returncode == 0 and result.stdout.strip():
                gpu_data = json.loads(result.stdout)
                if not isinstance(gpu_data, list):
                    gpu_data = [gpu_data]
                
                # Find det bedste GPU
                for gpu in gpu_data:
                    name = gpu.get('Name', '').strip()
                    if not name:
                        continue
                        
                    name_upper = name.upper()
                    vram_bytes = gpu.get('AdapterRAM', 0)
                    vram_gb = 0
                    if vram_bytes and vram_bytes > 0:
                        vram_gb = vram_bytes / (1024**3)
                    
                    # Tjek om det er et gaming GPU
                    if any(brand in name_upper for brand in ['RTX', 'GTX', 'RX', 'VEGA', 'ARC']):
                        return {'name': name, 'vram_gb': vram_gb, 'name_upper': name_upper}
                    elif any(brand in name_upper for brand in ['NVIDIA', 'AMD', 'RADEON']):
                        return {'name': name, 'vram_gb': vram_gb, 'name_upper': name_upper}
                        
            return None
        except Exception as e:
            print(f"GPU detection error: {e}")
            return None

    def _apply_auto_settings(self):
        """Apply optimal settings based on detected GPU"""
        try:
            # Detect GPU first
            gpu_info = self._detect_gpu()
            
            if not gpu_info:
                try:
                    from CTkMessagebox import CTkMessagebox
                    CTkMessagebox(title="Fejl", message="Kan ikke finde dit grafikkort.\nKan ikke anvende auto indstillinger.")
                except:
                    print("Cannot detect GPU for auto settings")
                return
            
            gpu_name = gpu_info['name']
            gpu_name_upper = gpu_info['name_upper']
            vram_gb = gpu_info['vram_gb']
            
            # Determine GPU vendor and optimal settings
            if any(nvidia in gpu_name_upper for nvidia in ['RTX', 'GTX', 'NVIDIA']):
                # NVIDIA GPU settings
                optimal_settings = {
                    "Dlss.Enabled": "true",
                    "Dlss.QualityMode": "2",  # Quality mode
                    "Fsr.Enabled": "auto",   # Backup option
                    "Fsr.QualityMode": "2",
                    "Xess.Enabled": "auto",
                }
                gpu_type = "NVIDIA"
                primary_tech = "DLSS"
                
                if 'RTX' in gpu_name_upper:
                    recommendations = "• DLSS aktiveret (perfekt til RTX)\n• FSR og XeSS som backup\n• Quality mode valgt"
                else:
                    recommendations = "• DLSS aktiveret\n• FSR som backup (anbefalet for ældre NVIDIA)\n• Quality mode valgt"
                    
            elif any(amd in gpu_name_upper for amd in ['RX', 'VEGA', 'AMD', 'RADEON']):
                # AMD GPU settings
                optimal_settings = {
                    "Fsr.Enabled": "true",
                    "Fsr.QualityMode": "2",  # Quality mode
                    "Fsr.Sharpness": "0.5",
                    "Dlss.Enabled": "false",  # Doesn't work on AMD
                    "Xess.Enabled": "auto",   # Backup option
                    "Xess.QualityMode": "2",
                }
                gpu_type = "AMD"
                primary_tech = "FSR"
                recommendations = "• FSR aktiveret (perfekt til AMD)\n• XeSS som backup\n• DLSS deaktiveret (virker ikke på AMD)\n• Quality mode valgt"
                
            elif 'ARC' in gpu_name_upper:
                # Intel Arc GPU settings
                optimal_settings = {
                    "Xess.Enabled": "true",
                    "Xess.QualityMode": "2",  # Quality mode
                    "Fsr.Enabled": "auto",   # Backup option
                    "Fsr.QualityMode": "2",
                    "Dlss.Enabled": "false", # Doesn't work on Intel
                }
                gpu_type = "Intel Arc"
                primary_tech = "XeSS"
                recommendations = "• XeSS aktiveret (perfekt til Intel Arc)\n• FSR som backup\n• DLSS deaktiveret\n• Quality mode valgt"
                
            else:
                # Unknown/older GPU - conservative settings
                optimal_settings = {
                    "Fsr.Enabled": "true",
                    "Fsr.QualityMode": "3",  # Performance mode for older GPUs
                    "Dlss.Enabled": "false",
                    "Xess.Enabled": "false",
                }
                gpu_type = "Ældre/Ukendt"
                primary_tech = "FSR"
                recommendations = "• FSR aktiveret (bedste kompatibilitet)\n• Performance mode (bedre til ældre GPU)\n• DLSS og XeSS deaktiveret"
            
            # Common performance settings for all GPUs
            optimal_settings.update({
                "Performance.EnableAsyncCompute": "true",
                "Performance.EnableFP16": "true" if vram_gb >= 4 else "false",
                "Performance.AutoExposure": "true",
                "Output.DisplayResolution": "auto",
                "Output.RenderingResolution": "auto",
                "Advanced.MotionVectors": "true",
                "Advanced.ExposureScale": "1.0",
                "Advanced.AutoBias": "true"
            })
            
            # Apply VRAM-specific settings
            if vram_gb >= 8:
                optimal_settings["Performance.EnableFP16"] = "true"
                vram_note = "• FP16 aktiveret (nok VRAM)"
            elif vram_gb >= 4:
                optimal_settings["Performance.EnableFP16"] = "true"  
                vram_note = "• FP16 aktiveret"
            else:
                optimal_settings["Performance.EnableFP16"] = "false"
                vram_note = "• FP16 deaktiveret (begrænset VRAM)"
            
            # Apply settings to widgets
            changes_made = 0
            for setting_key, recommended_value in optimal_settings.items():
                if setting_key in self.widgets:
                    widget = self.widgets[setting_key]
                    
                    # Set recommended value
                    if hasattr(widget, 'set'):  # OptionMenu
                        widget.set(recommended_value)
                        changes_made += 1
                    elif hasattr(widget, 'delete') and hasattr(widget, 'insert'):  # Entry
                        widget.delete(0, 'end')
                        widget.insert(0, recommended_value)
                        changes_made += 1
            
            # Show results
            if changes_made > 0:
                vram_text = f" ({vram_gb:.1f}GB VRAM)" if vram_gb > 0 else ""
                try:
                    from CTkMessagebox import CTkMessagebox
                    CTkMessagebox(title=t("auto_settings_title"), 
                                message=f"{t('optimized_for')} {gpu_type} GPU!\n\n"
                                       f"🎮 {gpu_name}{vram_text}\n\n"
                                       f"{t('primary_tech')}: {primary_tech}\n\n"
                                       f"{recommendations}\n"
                                       f"{vram_note}\n\n"
                                       f"{t('settings_changed', changes_made)}")
                except:
                    print(f"Auto settings applied for {gpu_type}: {changes_made} settings changed")
            else:
                try:
                    from CTkMessagebox import CTkMessagebox
                    CTkMessagebox(title="Info", message=t("no_settings_changed"))
                except:
                    print("No settings were changed")
                    
        except Exception as e:
            try:
                from CTkMessagebox import CTkMessagebox
                CTkMessagebox(title=t("error"), message=f"Auto settings error:\n{str(e)}")
            except:
                print(f"Error applying auto settings: {e}")

    def _go_back(self):
        """Go back to game list"""
        debug_log("Going back to game list...")
        try:
            if self.on_back:
                # Use the callback function provided by MainWindow
                self.on_back()
            else:
                # Fallback to old method
                debug_log("No callback provided, using fallback method...")
                current = self
                main_window = None
                
                # Traverse up the widget hierarchy to find MainWindow
                while current:
                    current = current.master
                    if hasattr(current, 'game_list_frame') and hasattr(current, 'settings_editor_frame'):
                        main_window = current
                        break
                
                if main_window and hasattr(main_window, 'show_game_list'):
                    main_window.show_game_list()
                elif main_window:
                    # Old method
                    if main_window.settings_editor_frame:
                        main_window.settings_editor_frame.destroy()
                        main_window.settings_editor_frame = None
                    main_window.game_list_frame.grid(row=0, column=0, padx=10, pady=10, sticky="nsew")
                else:
                    print("ERROR: Could not find main window")
                    self.destroy()
                    
        except Exception as e:
            print(f"ERROR in _go_back: {e}")
            try:
                self.destroy()
            except:
                pass

    def _save_settings(self):
        try:
            for section, keys in self.settings.items():
                for key, data in keys.items():
                    widget = self.widgets[f"{section}.{key}"]
                    if data["type"] == "bool_options":
                        self.settings[section][key]["value"] = widget.get()
                    elif data["type"] == "options":
                        selected_option_text = widget.get()
                        found_key = False
                        for k, v in data["options"].items():
                            if v == selected_option_text:
                                self.settings[section][key]["value"] = k
                                found_key = True
                                break
                        if not found_key and selected_option_text.lower() == "auto":
                            self.settings[section][key]["value"] = "auto"
                    else:
                        self.settings[section][key]["value"] = widget.get()
            
            self.optiscaler_manager.write_optiscaler_ini(self.ini_path, self.settings)
            
            try:
                from CTkMessagebox import CTkMessagebox
                CTkMessagebox(title=t("settings_saved"), message=t("settings_saved_msg"))
            except:
                print("Settings saved successfully!")
                
        except Exception as e:
            try:
                from CTkMessagebox import CTkMessagebox
                CTkMessagebox(title=t("error"), message=f"{t('failed_to_save')}\n{str(e)}")
            except:
                print(f"Error saving settings: {e}")
