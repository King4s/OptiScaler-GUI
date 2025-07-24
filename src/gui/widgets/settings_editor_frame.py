import customtkinter as ctk
import os
from optiscaler.manager import OptiScalerManager

class SettingsEditorFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, game_path, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1) # Allow entry to expand
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
            section_label.grid(row=row, column=0, columnspan=2, padx=10, pady=(10, 5), sticky="w")
            row += 1

            for key, data in keys.items():
                key_label = ctk.CTkLabel(self, text=key, font=("Arial", 12, "bold"))
                key_label.grid(row=row, column=0, padx=20, pady=(5, 0), sticky="w")
                row += 1

                if data["comment"]:
                    comment_label = ctk.CTkLabel(self, text=data["comment"], wraplength=400, justify="left", font=("Arial", 10))
                    comment_label.grid(row=row, column=0, columnspan=2, padx=25, pady=(0, 5), sticky="w")
                    row += 1

                widget = None
                if data["type"] == "bool_options":
                    options_list = ["true", "false", "auto"]
                    widget = ctk.CTkOptionMenu(self, values=options_list)
                    widget.set(data["value"].lower())
                    widget.grid(row=row, column=0, columnspan=2, padx=20, pady=(0, 10), sticky="w")
                elif data["type"] == "options":
                    # Ensure 'auto' is handled correctly if it's a valid option
                    display_options = list(data["options"].values())
                    widget = ctk.CTkOptionMenu(self, values=display_options)
                    
                    # Set initial value based on the current value
                    current_value_found = False
                    for k, v in data["options"].items():
                        if v == selected_option_text:
                            widget.set(v)
                            current_value_found = True
                            break
                    if not current_value_found and data["value"].lower() == "auto" and "auto" in display_options:
                        widget.set("auto")

                    widget.grid(row=row, column=0, columnspan=2, padx=20, pady=(0, 10), sticky="ew")
                else: # int, float, string
                    widget = ctk.CTkEntry(self)
                    widget.insert(0, data["value"])
                    widget.grid(row=row, column=0, columnspan=2, padx=20, pady=(0, 10), sticky="ew")
                
                self.widgets[f"{section}.{key}"] = widget
                row += 1
        
        save_button = ctk.CTkButton(self, text="Save Settings", command=self._save_settings)
        save_button.grid(row=row, column=0, columnspan=2, padx=10, pady=20)

    def _save_settings(self):
        for section, keys in self.settings.items():
            for key, data in keys.items():
                widget = self.widgets[f"{section}.{key}"]
                if data["type"] == "bool_options":
                    self.settings[section][key]["value"] = widget.get()
                elif data["type"] == "options":
                    selected_option_text = widget.get()
                    # Find the key (numeric value) corresponding to the selected text
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
        ctk.CTkMessagebox(title="Settings Saved", message="OptiScaler settings saved successfully!")
