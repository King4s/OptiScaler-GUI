#!/usr/bin/env python3
"""
Internationalization (i18n) module for OptiScaler-GUI
Supports English (default), Danish, and Polish
"""

import json
import os
from pathlib import Path

class I18n:
    def __init__(self):
        self.current_language = "en"
        self.translations = {}
        self._load_translations()
    
    def _load_translations(self):
        """Load translation data"""
        
        # English translations (default)
        self.translations["en"] = {
            # Main Window
            "app_title": "OptiScaler GUI",
            "version": "Version",
            "thanks_to": "Thanks to",
            "games_tab": "Games",
            "settings_tab": "Settings",
            
            # Game List
            "install_optiscaler": "Install OptiScaler",
            "edit_settings": "Edit",
            "uninstall": "Uninstall",
            "open_folder": "Open Folder",
            "games_found": "games found",
            "no_games": "No games found",
            
            # Settings Editor
            "back": "← Back",
            "save_settings": "Save Settings",
            "settings_title": "OptiScaler Settings",
            "section": "Section",
            "setting": "Setting",
            "value": "Value",
            "description": "Description",
            
            # Status Messages  
            "status_scanning": "Scanning for games...",
            "status_installing": "Installing OptiScaler...",
            "status_downloading": "Downloading OptiScaler...",
            "status_ready": "Ready",
            
            # Progress
            "progress": "Progress",
            "download_progress": "Download Progress",
            "install_progress": "Installation Progress",
            
            # GPU Detection
            "nvidia_rtx_gpu": "NVIDIA RTX GPU",
            "nvidia_gtx_gpu": "NVIDIA GTX GPU", 
            "amd_rdna_gpu": "AMD RDNA GPU",
            "amd_older_gpu": "AMD Older GPU",
            "intel_arc_gpu": "Intel Arc GPU",
            "older_unknown_gpu": "Older/Unknown GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR", 
            "xess": "XeSS",
            
            # Global Settings
            "language": "Language",
            "english": "English",
            "danish": "Dansk",
            "polish": "Polski",
            "theme": "Theme",
            "light": "Light",
            "dark": "Dark",
            "debug_mode": "Debug Mode",
            
            # Status Messages
            "downloading": "Downloading...",
            "installing": "Installing...",
            "processing": "Processing...",
            "please_wait": "Please wait...",
            
            # Error Messages
            "error": "Error",
            "failed_to_save": "Failed to save settings:",
            "failed_to_load": "Failed to load settings:",
            "installation_failed": "Installation failed:",
            
            # Uninstall
            "uninstall_title": "Uninstall OptiScaler",
            "uninstall_confirm": "Are you sure you want to uninstall OptiScaler from this game?\n\nThis will restore the original game files.",
            "uninstalling": "Uninstalling...",
            "uninstall_success": "OptiScaler uninstalled successfully!",
            "uninstall_failed": "Failed to uninstall OptiScaler:",
            
            # Setting Descriptions for OptiScaler Settings
            "fsr_enabled_desc": "Enable AMD FidelityFX Super Resolution - improves performance with good visual quality",
            "fsr_quality_desc": "FSR Quality Mode - higher numbers = better performance, lower visual quality",
            "dlss_enabled_desc": "Enable NVIDIA DLSS - AI-powered upscaling for RTX graphics cards",
            "dlss_quality_desc": "DLSS Quality Mode - balances performance and visual fidelity", 
            "xess_enabled_desc": "Enable Intel XeSS - works on most graphics cards, best on Intel Arc",
            "xess_quality_desc": "XeSS Quality Mode - higher numbers = better performance",
            
            # OptiScaler Upscaler Settings
            "upscalers_dx11upscaler_desc": "DirectX 11 upscaling technology selection",
            "upscalers_dx12upscaler_desc": "DirectX 12 upscaling technology selection", 
            "upscalers_vulkanupscaler_desc": "Vulkan upscaling technology selection",
            
            # DLSS Settings
            "dlss_librarypath_desc": "Path to DLSS library files",
            "dlss_renderpresetoverride_desc": "Override automatic quality preset selection",
            "dlss_renderpresetforall_desc": "Apply single quality preset to all modes",
            
            # Frame Generation
            "framegen_fgtype_desc": "Frame generation technology selection",
            "optifg_enabled_desc": "Enable OptiFrame frame generation",
            "optifg_debugview_desc": "Show frame generation debug overlay",
            
            # Input Detection  
            "inputs_dlss_desc": "Enable DLSS input detection",
            "inputs_xess_desc": "Enable XeSS input detection",
            "inputs_fsr2_desc": "Enable FSR 2.x input detection",
            "inputs_fsr3_desc": "Enable FSR 3.x input detection",
            "inputs_enablehotswapping_desc": "Allow switching upscalers without restart",
            
            # FSR Settings
            "fsr_debugview_desc": "Show FSR debug information overlay",
            "fsr_upscalerindex_desc": "FSR upscaler variant selection",
            "fsr_velocityfactor_desc": "Motion vector scaling factor",
            
            # XeSS Settings  
            "xess_buildpipelines_desc": "Pre-build XeSS rendering pipelines",
            "xess_networkmodel_desc": "XeSS AI model selection",
            
            # Rendering
            "framerate_frameratelimit_desc": "Maximum frame rate limit",
            "outputscaling_enabled_desc": "Enable output resolution scaling",
            "outputscaling_multiplier_desc": "Output scaling multiplier",
            
            # Menu/Overlay
            "menu_overlaymenu_desc": "Enable in-game configuration menu",
            "menu_scale_desc": "Menu scaling factor",
            "menu_shortcutkey_desc": "Hotkey to open in-game menu",
            "menu_showfps_desc": "Display frame rate counter",
            
            # GPU Spoofing
            "spoofing_spoofedgpuname_desc": "GPU name to report to games",
            "spoofing_dxgi_desc": "Enable DirectX GPU spoofing",
            "spoofing_vulkan_desc": "Enable Vulkan GPU spoofing",
            
            # Logging
            "log_logfile_desc": "Log file path and name",
            "log_loglevel_desc": "Logging detail level",
            "log_logtoconsole_desc": "Display log messages in console",
            "log_logtofile_desc": "Save log messages to file",
            "log_openconsole_desc": "Open console window on startup",
        }
        
        # Danish translations
        self.translations["da"] = {
            # Main Window
            "app_title": "OptiScaler GUI",
            "version": "Version",
            "thanks_to": "Tak til",
            "games_tab": "Spil",
            "settings_tab": "Indstillinger",
            
            # Game List
            "install_optiscaler": "Installer OptiScaler",
            "edit_settings": "Rediger",
            "uninstall": "Afinstaller",
            "open_folder": "Åbn Mappe",
            "games_found": "spil fundet",
            "no_games": "Ingen spil fundet",
            
            # Settings Editor
            "back": "← Tilbage",
            "save_settings": "Gem Indstillinger",
            "settings_title": "OptiScaler Indstillinger",
            "section": "Sektion",
            "setting": "Indstilling",
            "value": "Værdi",
            "description": "Beskrivelse",
            
            # Status Messages
            "status_scanning": "Scanner efter spil...",
            "status_installing": "Installerer OptiScaler...",
            "status_downloading": "Downloader OptiScaler...",
            "status_ready": "Klar",
            
            # Progress
            "progress": "Fremskridt",
            "download_progress": "Download Fremskridt",
            "install_progress": "Installations Fremskridt",
            
            # GPU Detection
            "nvidia_rtx_gpu": "NVIDIA RTX GPU",
            "nvidia_gtx_gpu": "NVIDIA GTX GPU", 
            "amd_rdna_gpu": "AMD RDNA GPU",
            "amd_older_gpu": "AMD Ældre GPU",
            "intel_arc_gpu": "Intel Arc GPU", 
            "older_unknown_gpu": "Ældre/Ukendt GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR",
            "xess": "XeSS",
            
            # Global Settings
            "language": "Sprog",
            "english": "English",
            "danish": "Dansk",
            "polish": "Polski",
            "theme": "Tema",
            "light": "Lys",
            "dark": "Mørk",
            "debug_mode": "Debug Tilstand",
            
            # Status Messages
            "downloading": "Downloader...",
            "installing": "Installerer...",
            "processing": "Behandler...",
            "please_wait": "Vent venligst...",
            
            # Error Messages
            "error": "Fejl",
            "failed_to_save": "Kunne ikke gemme indstillinger:",
            "failed_to_load": "Kunne ikke indlæse indstillinger:",
            "installation_failed": "Installation fejlede:",
            
            # Uninstall
            "uninstall_title": "Afinstaller OptiScaler",
            "uninstall_confirm": "Er du sikker på at du vil afinstallere OptiScaler fra dette spil?\n\nDette vil gendanne de originale spil filer.",
            "uninstalling": "Afinstallerer...",
            "uninstall_success": "OptiScaler afinstalleret succesfuldt!",
            "uninstall_failed": "Kunne ikke afinstallere OptiScaler:",
            
            # Setting Descriptions for OptiScaler Settings
            "fsr_enabled_desc": "Aktiver AMD FidelityFX Super Resolution - forbedrer ydeevne med god visuel kvalitet",
            "fsr_quality_desc": "FSR Kvalitet Tilstand - højere tal = bedre ydeevne, lavere visuel kvalitet",
            "dlss_enabled_desc": "Aktiver NVIDIA DLSS - AI-drevet opskalering til RTX grafikkort",
            "dlss_quality_desc": "DLSS Kvalitet Tilstand - balancerer ydeevne og visuel kvalitet", 
            "xess_enabled_desc": "Aktiver Intel XeSS - virker på de fleste grafikkort, bedst på Intel Arc",
            "xess_quality_desc": "XeSS Kvalitet Tilstand - højere tal = bedre ydeevne",
            
            # OptiScaler Setting Descriptions
            "upscalers_dx11upscaler_desc": "DirectX 11 opskalering teknologi valg",
            "upscalers_dx12upscaler_desc": "DirectX 12 opskalering teknologi valg", 
            "upscalers_vulkanupscaler_desc": "Vulkan opskalering teknologi valg",
            
            # DLSS Settings
            "dlss_librarypath_desc": "Sti til DLSS bibliotek filer",
            "dlss_renderpresetoverride_desc": "Tilsidesæt automatisk kvalitet preset valg",
            "dlss_renderpresetforall_desc": "Anvend enkelt kvalitet preset til alle tilstande",
            
            # Frame Generation
            "framegen_fgtype_desc": "Frame generation teknologi valg",
            "optifg_enabled_desc": "Aktiver OptiFrame frame generation",
            "optifg_debugview_desc": "Vis frame generation debug overlay",
            
            # Input Detection  
            "inputs_dlss_desc": "Aktiver DLSS input detection",
            "inputs_xess_desc": "Aktiver XeSS input detection",
            "inputs_fsr2_desc": "Aktiver FSR 2.x input detection",
            "inputs_fsr3_desc": "Aktiver FSR 3.x input detection",
            "inputs_enablehotswapping_desc": "Tillad skift mellem opskalere uden genstart",
            
            # FSR Settings
            "fsr_debugview_desc": "Vis FSR debug information overlay",
            "fsr_upscalerindex_desc": "FSR opskalerer variant valg",
            "fsr_velocityfactor_desc": "Bevægelses vektor skalering faktor",
            
            # XeSS Settings  
            "xess_buildpipelines_desc": "Pre-byg XeSS rendering pipelines",
            "xess_networkmodel_desc": "XeSS AI model valg",
            
            # Rendering
            "framerate_frameratelimit_desc": "Maksimum frame rate grænse",
            "outputscaling_enabled_desc": "Aktiver output opløsning skalering",
            "outputscaling_multiplier_desc": "Output skalering multiplikator",
            
            # Menu/Overlay
            "menu_overlaymenu_desc": "Aktiver i-spillet konfiguration menu",
            "menu_scale_desc": "Menu skalering faktor",
            "menu_shortcutkey_desc": "Genvejstast til at åbne i-spillet menu",
            "menu_showfps_desc": "Vis frame rate tæller",
            
            # GPU Spoofing
            "spoofing_spoofedgpuname_desc": "GPU navn at rapportere til spil",
            "spoofing_dxgi_desc": "Aktiver DirectX GPU spoofing",
            "spoofing_vulkan_desc": "Aktiver Vulkan GPU spoofing",
            
            # Logging
            "log_logfile_desc": "Log fil sti og navn",
            "log_loglevel_desc": "Logging detalje niveau",
            "log_logtoconsole_desc": "Vis log beskeder i konsol",
            "log_logtofile_desc": "Gem log beskeder til fil",
            "log_openconsole_desc": "Åbn konsol vindue ved opstart",
        }
        
        # Polish translations
        self.translations["pl"] = {
            # Main Window
            "app_title": "OptiScaler GUI",
            "version": "Wersja",
            "thanks_to": "Podziękowania dla",
            "games_tab": "Gry",
            "settings_tab": "Ustawienia",
            
            # Game List
            "install_optiscaler": "Zainstaluj OptiScaler",
            "edit_settings": "Edytuj",
            "uninstall": "Odinstaluj",
            "open_folder": "Otwórz Folder",
            "games_found": "gier znaleziono",
            "no_games": "Nie znaleziono gier",
            
            # Settings Editor
            "back": "← Wstecz",
            "save_settings": "Zapisz Ustawienia",
            "settings_title": "Ustawienia OptiScaler",
            "section": "Sekcja",
            "setting": "Ustawienie",
            "value": "Wartość",
            "description": "Opis",
            
            # Status Messages
            "status_scanning": "Skanowanie gier...",
            "status_installing": "Instalowanie OptiScaler...",
            "status_downloading": "Pobieranie OptiScaler...",
            "status_ready": "Gotowy",
            
            # Progress  
            "progress": "Postęp",
            "download_progress": "Postęp Pobierania",
            "install_progress": "Postęp Instalacji",
            
            # GPU Detection
            "nvidia_rtx_gpu": "NVIDIA RTX GPU",
            "nvidia_gtx_gpu": "NVIDIA GTX GPU", 
            "amd_rdna_gpu": "AMD RDNA GPU",
            "amd_older_gpu": "AMD Starszy GPU",
            "intel_arc_gpu": "Intel Arc GPU",
            "older_unknown_gpu": "Starszy/Nieznany GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR",
            "xess": "XeSS",
            
            # Global Settings
            "language": "Język",
            "english": "English",
            "danish": "Dansk",
            "polish": "Polski",
            "theme": "Motyw",
            "light": "Jasny",
            "dark": "Ciemny",
            "debug_mode": "Tryb Debug",
            
            # Status Messages
            "downloading": "Pobieranie...",
            "installing": "Instalowanie...",
            "processing": "Przetwarzanie...",
            "please_wait": "Proszę czekać...",
            
            # Error Messages
            "error": "Błąd",
            "failed_to_save": "Nie udało się zapisać ustawień:",
            "failed_to_load": "Nie udało się załadować ustawień:",
            "installation_failed": "Instalacja nie powiodła się:",
            
            # Uninstall
            "uninstall_title": "Odinstaluj OptiScaler",
            "uninstall_confirm": "Czy na pewno chcesz odinstalować OptiScaler z tej gry?\n\nTo przywróci oryginalne pliki gry.",
            "uninstalling": "Odinstalowywanie...",
            "uninstall_success": "OptiScaler odinstalowany pomyślnie!",
            "uninstall_failed": "Nie udało się odinstalować OptiScaler:",
            
            # Setting Descriptions for OptiScaler Settings
            "fsr_enabled_desc": "Włącz AMD FidelityFX Super Resolution - poprawia wydajność przy dobrej jakości wizualnej",
            "fsr_quality_desc": "Tryb Jakości FSR - wyższe liczby = lepsza wydajność, niższa jakość wizualna",
            "dlss_enabled_desc": "Włącz NVIDIA DLSS - skalowanie wspomagane AI dla kart RTX",
            "dlss_quality_desc": "Tryb Jakości DLSS - równoważy wydajność i wierność wizualną",
            "xess_enabled_desc": "Włącz Intel XeSS - działa na większości kart graficznych, najlepiej na Intel Arc",
            "xess_quality_desc": "Tryb Jakości XeSS - wyższe liczby = lepsza wydajność",
            
            # OptiScaler Setting Descriptions  
            "upscalers_dx11upscaler_desc": "Wybór technologii skalowania DirectX 11",
            "upscalers_dx12upscaler_desc": "Wybór technologii skalowania DirectX 12", 
            "upscalers_vulkanupscaler_desc": "Wybór technologii skalowania Vulkan",
            
            # DLSS Settings
            "dlss_librarypath_desc": "Ścieżka do plików biblioteki DLSS",
            "dlss_renderpresetoverride_desc": "Zastąp automatyczny wybór presetów jakości",
            "dlss_renderpresetforall_desc": "Zastosuj jeden preset jakości dla wszystkich trybów",
            
            # Frame Generation
            "framegen_fgtype_desc": "Wybór technologii generowania klatek",
            "optifg_enabled_desc": "Włącz generowanie klatek OptiFrame",
            "optifg_debugview_desc": "Pokaż nakładkę debug generowania klatek",
            
            # Input Detection  
            "inputs_dlss_desc": "Włącz wykrywanie wejścia DLSS",
            "inputs_xess_desc": "Włącz wykrywanie wejścia XeSS",
            "inputs_fsr2_desc": "Włącz wykrywanie wejścia FSR 2.x",
            "inputs_fsr3_desc": "Włącz wykrywanie wejścia FSR 3.x",
            "inputs_enablehotswapping_desc": "Pozwól na przełączanie skalerów bez restartu",
            
            # FSR Settings
            "fsr_debugview_desc": "Pokaż nakładkę informacji debug FSR",
            "fsr_upscalerindex_desc": "Wybór wariantu skalera FSR",
            "fsr_velocityfactor_desc": "Współczynnik skalowania wektora ruchu",
            
            # XeSS Settings  
            "xess_buildpipelines_desc": "Wstępnie zbuduj potoki renderowania XeSS",
            "xess_networkmodel_desc": "Wybór modelu AI XeSS",
            
            # Rendering
            "framerate_frameratelimit_desc": "Maksymalny limit klatek na sekundę",
            "outputscaling_enabled_desc": "Włącz skalowanie rozdzielczości wyjściowej",
            "outputscaling_multiplier_desc": "Mnożnik skalowania wyjściowego",
            
            # Menu/Overlay
            "menu_overlaymenu_desc": "Włącz menu konfiguracji w grze",
            "menu_scale_desc": "Współczynnik skalowania menu",
            "menu_shortcutkey_desc": "Skrót klawiszowy do otwarcia menu w grze",
            "menu_showfps_desc": "Wyświetl licznik klatek na sekundę",
            
            # GPU Spoofing
            "spoofing_spoofedgpuname_desc": "Nazwa GPU do zgłaszania grom",
            "spoofing_dxgi_desc": "Włącz spoofing GPU DirectX",
            "spoofing_vulkan_desc": "Włącz spoofing GPU Vulkan",
            
            # Logging
            "log_logfile_desc": "Ścieżka i nazwa pliku dziennika",
            "log_loglevel_desc": "Poziom szczegółowości logowania",
            "log_logtoconsole_desc": "Wyświetl wiadomości dziennika w konsoli",
            "log_logtofile_desc": "Zapisz wiadomości dziennika do pliku",
            "log_openconsole_desc": "Otwórz okno konsoli przy uruchomieniu",
        }

    def set_language(self, language_code):
        """Set the current language"""
        if language_code in self.translations:
            self.current_language = language_code
        else:
            print(f"Language '{language_code}' not supported, using English")
            self.current_language = "en"

    def get(self, key, **kwargs):
        """Get translated text for given key"""
        # Try current language first
        if key in self.translations[self.current_language]:
            text = self.translations[self.current_language][key]
        # Fall back to English if not found
        elif key in self.translations["en"]:
            text = self.translations["en"][key]
        # Return key if no translation found
        else:
            text = key
        
        # Format with kwargs if provided
        try:
            return text.format(**kwargs)
        except (KeyError, ValueError):
            return text

    def get_available_languages(self):
        """Get list of available language codes"""
        return list(self.translations.keys())

# Global instance
i18n = I18n()

# Convenience function for translations
def t(key, **kwargs):
    """Shortcut function for getting translations"""
    return i18n.get(key, **kwargs)
