import customtkinter as ctk
import os
from ...optiscaler.manager import OptiScalerManager

class SettingsEditorFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, game_path, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.game_path = game_path
        self.optiscaler_manager = OptiScalerManager()
        self.ini_path = os.path.join(self.game_path, "OptiScaler.ini")
        self.settings = {}
        self.widgets = {}

        self._load_settings()
        self._create_widgets()

    def _load_settings(self):
        if os.path.exists(self.ini_path):
            self.settings = self.optiscaler_manager.read_optiscaler_ini(self.ini_path)
            print(f"Loaded settings: {self.settings}")
        else:
            print(f"OptiScaler.ini not found at {self.ini_path}")

    def _create_widgets(self):
        row = 0
        for section, keys in self.settings.items():
            section_label = ctk.CTkLabel(self, text=f"[{section}]", font=("Arial", 16, "bold"))
            section_label.grid(row=row, column=0, padx=10, pady=(10, 5), sticky="w")
            row += 1

            for key, data in keys.items():
                key_label = ctk.CTkLabel(self, text=key)
                key_label.grid(row=row, column=0, padx=20, pady=2, sticky="w")

                # Simple entry for now, will be replaced with more specific widgets
                entry = ctk.CTkEntry(self)
                entry.insert(0, data["value"])
                entry.grid(row=row, column=1, padx=10, pady=2, sticky="ew")
                self.widgets[f"{section}.{key}"] = entry
                row += 1
        
        save_button = ctk.CTkButton(self, text="Save Settings", command=self._save_settings)
        save_button.grid(row=row, column=0, columnspan=2, padx=10, pady=20)

    def _save_settings(self):
        for section, keys in self.settings.items():
            for key, data in keys.items():
                widget = self.widgets[f"{section}.{key}"]
                self.settings[section][key]["value"] = widget.get()
        
        self.optiscaler_manager.write_optiscaler_ini(self.ini_path, self.settings)
        ctk.CTkMessagebox(title="Settings Saved", message="OptiScaler settings saved successfully!")
