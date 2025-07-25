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
            "upscalers_dx11upscaler_desc": "DirectX 11 opskalering teknologi valg",
            "upscalers_dx12upscaler_desc": "DirectX 12 opskalering teknologi valg", 
            "upscalers_vulkanupscaler_desc": "Vulkan opskalering teknologi valg",
            
            # DLSS Settings
            "dlss_enabled_desc": "Aktiver NVIDIA DLSS - AI-drevet opskalering til RTX grafikkort",
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
            
            # Legacy settings for compatibility
            "upscaler_enabled_desc": "Aktiver/deaktiver opskalering teknologi",
            "upscaler_quality_desc": "Opskalering kvalitet niveau - påvirker ydeevne vs visuel kvalitet",
            "upscaler_sharpness_desc": "Billedskarphed intensitet - højere værdier = skarpere billede",
            "renderscale_desc": "Intern rendering opløsning skala - lavere = bedre ydeevne",
            "motioncutoff_desc": "Bevægelses vektor tærskel for opskalering - påvirker bevægelsesklarhed",
            "debug_desc": "Aktiver debug output og overlays til fejlfinding",
            "logtofile_desc": "Gem debug information til log filer",
            "autoexposure_desc": "Automatisk eksponering justering for bedre billedkvalitet",
            "hdr_desc": "High Dynamic Range support til kompatible skærme",
            "colorspace_desc": "Farverum konfiguration for nøjagtig farve reproduction",
        self.load_translations()
    
    def load_translations(self):
        """Load all translation files"""
        translations_dir = Path(__file__).parent.parent.parent / "translations"
        
        # Default English translations (embedded)
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
            "auto_settings": "🎯 Auto Settings", 
            "save_settings": "💾 Save Settings",
            "settings_saved": "Settings Saved",
            "settings_saved_msg": "OptiScaler settings saved successfully!",
            "optiscaler_not_installed": "OptiScaler Not Installed",
            "optiscaler_not_installed_msg": "OptiScaler is not installed in this game yet.\n\nClick the 'Install OptiScaler' button first to\ndownload and install OptiScaler for this game.\n\nThen you can edit the settings here.",
            "back_to_games": "← Back to Game List",
            
            # Auto Settings
            "auto_settings_title": "Auto Settings",
            "optimized_for": "✅ Optimized for your",
            "primary_tech": "🎯 Primary technology",
            "settings_changed": "Changed {0} settings.\nClick 'Save Settings' to save.",
            "no_settings_changed": "No settings were changed.",
            "gpu_detection_failed": "Cannot detect your graphics card.\nCannot apply auto settings.",
            
            # GPU Types
            "nvidia_gpu": "NVIDIA GPU",
            "amd_gpu": "AMD GPU", 
            "intel_arc_gpu": "Intel Arc GPU",
            "older_unknown_gpu": "Older/Unknown GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR",
            "xess": "XeSS",
            
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
            
            # Progress
            "progress": "Progress",
            "download_progress": "Download Progress",
            "install_progress": "Installation Progress",
            
            # Uninstall
            "uninstall_title": "Uninstall OptiScaler",
            "uninstall_confirm": "Are you sure you want to uninstall OptiScaler from this game?\n\nThis will restore the original game files.",
            "uninstalling": "Uninstalling...",
            "uninstall_success": "OptiScaler uninstalled successfully!",
            "uninstall_failed": "Failed to uninstall OptiScaler:",
            
            # Global Settings
            "global_settings_title": "Application Settings",
            "language_settings": "Language Settings",
            "interface_language": "Interface Language:",
            "language_restart_note": "Some changes may require restarting the application",
            "app_settings": "Application Settings", 
            "debug_mode": "Debug Mode:",
            "enable_debug": "Enable debug output",
            "auto_update_check": "Updates:",
            "check_for_updates": "Check for updates automatically",
            "appearance_theme": "Theme:",
            "cache_settings": "Cache Management",
            "clear_cache": "Clear Cache",
            "open_cache_folder": "Open Cache Folder",
            "confirm_clear_cache": "Clear Cache",
            "clear_cache_warning": "This will delete all cached game images and downloads.\n\nAre you sure?",
            "clear": "Clear",
            "cancel": "Cancel",
            "success": "Success",
            "cache_cleared": "Cache cleared successfully!",
            "failed_to_clear_cache": "Failed to clear cache",
            "cache_folder_not_found": "Cache folder not found",
            "about": "About",
            "app_description": "A user-friendly GUI for managing OptiScaler installations across your game library.",
            "view_on_github": "View on GitHub",
            "optiscaler_project": "OptiScaler Project",
            
            # Setting Descriptions
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
            "dlss_enabled_desc": "Enable NVIDIA DLSS - AI-powered upscaling for RTX graphics cards",
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
            "auto_settings": "🎯 Auto Indstillinger",
            "save_settings": "💾 Gem Indstillinger", 
            "settings_saved": "Indstillinger Gemt",
            "settings_saved_msg": "OptiScaler indstillinger gemt succesfuldt!",
            "optiscaler_not_installed": "OptiScaler ikke installeret",
            "optiscaler_not_installed_msg": "OptiScaler er ikke installeret i dette spil endnu.\n\nTryk på 'Installer OptiScaler' knappen først for at\ndownloade og installere OptiScaler til dette spil.\n\nDerefter kan du redigere indstillingerne her.",
            "back_to_games": "← Tilbage til Spil Listen",
            
            # Auto Settings
            "auto_settings_title": "Auto Indstillinger",
            "optimized_for": "✅ Optimeret til dit",
            "primary_tech": "🎯 Primær teknologi",
            "settings_changed": "Ændret {0} indstillinger.\nTryk 'Gem Indstillinger' for at gemme.",
            "no_settings_changed": "Ingen indstillinger blev ændret.",
            "gpu_detection_failed": "Kan ikke finde dit grafikkort.\nKan ikke anvende auto indstillinger.",
            
            # GPU Types
            "nvidia_gpu": "NVIDIA GPU",
            "amd_gpu": "AMD GPU",
            "intel_arc_gpu": "Intel Arc GPU", 
            "older_unknown_gpu": "Ældre/Ukendt GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR",
            "xess": "XeSS",
            
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
            
            # Progress
            "progress": "Fremskridt",
            "download_progress": "Download Fremskridt",
            "install_progress": "Installations Fremskridt",
            
            # Uninstall
            "uninstall_title": "Afinstaller OptiScaler",
            "uninstall_confirm": "Er du sikker på at du vil afinstallere OptiScaler fra dette spil?\n\nDette vil gendanne de originale spil filer.",
            "uninstalling": "Afinstallerer...",
            "uninstall_success": "OptiScaler afinstalleret succesfuldt!",
            "uninstall_failed": "Kunne ikke afinstallere OptiScaler:",
            
            # Global Settings
            "global_settings_title": "Program Indstillinger",
            "language_settings": "Sprog Indstillinger",
            "interface_language": "Interface Sprog:",
            "language_restart_note": "Nogle ændringer kræver måske genstart af programmet",
            "app_settings": "Program Indstillinger",
            "debug_mode": "Debug Mode:",
            "enable_debug": "Aktiver debug output",
            "auto_update_check": "Opdateringer:",
            "check_for_updates": "Tjek automatisk for opdateringer",
            "appearance_theme": "Tema:",
            "cache_settings": "Cache Styring",
            "clear_cache": "Ryd Cache",
            "open_cache_folder": "Åbn Cache Mappe",
            "confirm_clear_cache": "Ryd Cache",
            "clear_cache_warning": "Dette vil slette alle gemte spil billeder og downloads.\n\nEr du sikker?",
            "clear": "Ryd",
            "cancel": "Annuller",
            "success": "Succes",
            "cache_cleared": "Cache ryddet succesfuldt!",
            "failed_to_clear_cache": "Kunne ikke rydde cache",
            "cache_folder_not_found": "Cache mappe ikke fundet",
            "about": "Om",
            "app_description": "En brugervenlig GUI til styring af OptiScaler installationer på tværs af dit spil bibliotek.",
            "view_on_github": "Se på GitHub",
            "optiscaler_project": "OptiScaler Projekt",
            
            # Setting Descriptions
            "fsr_enabled_desc": "Aktiver AMD FidelityFX Super Resolution - forbedrer ydeevne med god visuel kvalitet",
            "fsr_quality_desc": "FSR Kvalitets Mode - højere tal = bedre ydeevne, lavere visuel kvalitet",
            "dlss_enabled_desc": "Aktiver NVIDIA DLSS - AI-dreven opskalering til RTX grafikkort",
            "dlss_quality_desc": "DLSS Kvalitets Mode - balancerer ydeevne og visuel kvalitet",
            "xess_enabled_desc": "Aktiver Intel XeSS - virker på de fleste grafikkort, bedst på Intel Arc",
            "xess_quality_desc": "XeSS Kvalitets Mode - højere tal = bedre ydeevne",
            
            # Common OptiScaler Settings
            "upscaler_enabled_desc": "Aktiver/deaktiver opskaleringsteknologi",
            "upscaler_quality_desc": "Opskalerings kvalitetsniveau - påvirker ydeevne vs visuel kvalitet",
            "upscaler_sharpness_desc": "Billedskarphed intensitet - højere værdier = skarpere billede",
            "renderscale_desc": "Intern rendering opløsning skala - lavere = bedre ydeevne",
            "motioncutoff_desc": "Bevægelses vektor tærskel for opskalering - påvirker bevægelsesklarhed",
            "debug_desc": "Aktiver debug output og overlays til fejlfinding",
            "logtofile_desc": "Gem debug information til log filer",
            "autoexposure_desc": "Automatisk eksponering justering for bedre billedkvalitet",
            "hdr_desc": "High Dynamic Range support til kompatible skærme",
            "colorspace_desc": "Farverum konfiguration for nøjagtig farve reproduction",
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
            "games_found": "znalezionych gier",
            "no_games": "Nie znaleziono gier",
            
            # Settings Editor
            "back": "← Wstecz",
            "auto_settings": "🎯 Auto Ustawienia",
            "save_settings": "💾 Zapisz Ustawienia",
            "settings_saved": "Ustawienia Zapisane",
            "settings_saved_msg": "Ustawienia OptiScaler zostały pomyślnie zapisane!",
            "optiscaler_not_installed": "OptiScaler Nie Zainstalowany",
            "optiscaler_not_installed_msg": "OptiScaler nie jest jeszcze zainstalowany w tej grze.\n\nNaciśnij przycisk 'Zainstaluj OptiScaler' aby\npobrać i zainstalować OptiScaler dla tej gry.\n\nNastępnie możesz edytować ustawienia tutaj.",
            "back_to_games": "← Powrót do Listy Gier",
            
            # Auto Settings
            "auto_settings_title": "Auto Ustawienia",
            "optimized_for": "✅ Zoptymalizowano dla twojej",
            "primary_tech": "🎯 Podstawowa technologia",
            "settings_changed": "Zmieniono {0} ustawień.\nKliknij 'Zapisz Ustawienia' aby zapisać.",
            "no_settings_changed": "Żadne ustawienia nie zostały zmienione.",
            "gpu_detection_failed": "Nie można wykryć karty graficznej.\nNie można zastosować auto ustawień.",
            
            # GPU Types
            "nvidia_gpu": "GPU NVIDIA",
            "amd_gpu": "GPU AMD",
            "intel_arc_gpu": "GPU Intel Arc",
            "older_unknown_gpu": "Starsze/Nieznane GPU",
            
            # Technologies
            "dlss": "DLSS",
            "fsr": "FSR", 
            "xess": "XeSS",
            
            # Status Messages
            "downloading": "Pobieranie...",
            "installing": "Instalowanie...",
            "processing": "Przetwarzanie...",
            "please_wait": "Proszę czekać...",
            
            # Error Messages
            "error": "Błąd",
            "failed_to_save": "Nie udało się zapisać ustawień:",
            "failed_to_load": "Nie udało się wczytać ustawień:",
            "installation_failed": "Instalacja nie powiodła się:",
            
            # Progress
            "progress": "Postęp",
            "download_progress": "Postęp Pobierania",
            "install_progress": "Postęp Instalacji",
            
            # Uninstall
            "uninstall_title": "Odinstaluj OptiScaler",
            "uninstall_confirm": "Czy na pewno chcesz odinstalować OptiScaler z tej gry?\n\nTo przywróci oryginalne pliki gry.",
            "uninstalling": "Odinstalowywanie...",
            "uninstall_success": "OptiScaler został pomyślnie odinstalowany!",
            "uninstall_failed": "Nie udało się odinstalować OptiScaler:",
            
            # Global Settings
            "global_settings_title": "Ustawienia Aplikacji",
            "language_settings": "Ustawienia Języka",
            "interface_language": "Język Interfejsu:",
            "language_restart_note": "Niektóre zmiany mogą wymagać ponownego uruchomienia aplikacji",
            "app_settings": "Ustawienia Aplikacji",
            "debug_mode": "Tryb Debug:",
            "enable_debug": "Włącz wyjście debug",
            "auto_update_check": "Aktualizacje:",
            "check_for_updates": "Sprawdzaj aktualizacje automatycznie",
            "appearance_theme": "Motyw:",
            "cache_settings": "Zarządzanie Cache",
            "clear_cache": "Wyczyść Cache",
            "open_cache_folder": "Otwórz Folder Cache",
            "confirm_clear_cache": "Wyczyść Cache",
            "clear_cache_warning": "To usunie wszystkie buforowane obrazy gier i pobrane pliki.\n\nCzy jesteś pewny?",
            "clear": "Wyczyść",
            "cancel": "Anuluj",
            "success": "Sukces",
            "cache_cleared": "Cache wyczyszczony pomyślnie!",
            "failed_to_clear_cache": "Nie udało się wyczyścić cache",
            "cache_folder_not_found": "Folder cache nie został znaleziony",
            "about": "O programie",
            "app_description": "Przyjazny użytkownikowi interfejs graficzny do zarządzania instalacjami OptiScaler w bibliotece gier.",
            "view_on_github": "Zobacz na GitHub",
            "optiscaler_project": "Projekt OptiScaler",
            
            # Setting Descriptions
            "fsr_enabled_desc": "Włącz AMD FidelityFX Super Resolution - poprawia wydajność przy dobrej jakości wizualnej",
            "fsr_quality_desc": "Tryb Jakości FSR - wyższe liczby = lepsza wydajność, niższa jakość wizualna",
            "dlss_enabled_desc": "Włącz NVIDIA DLSS - skalowanie oparte na AI dla kart graficznych RTX",
            "dlss_quality_desc": "Tryb Jakości DLSS - równoważy wydajność i wierność wizualną",
            "xess_enabled_desc": "Włącz Intel XeSS - działa na większości kart graficznych, najlepiej na Intel Arc",  
            "xess_quality_desc": "Tryb Jakości XeSS - wyższe liczby = lepsza wydajność",
            
            # Common OptiScaler Settings
            "upscaler_enabled_desc": "Włącz/wyłącz technologię skalowania",
            "upscaler_quality_desc": "Poziom jakości skalowania - wpływa na wydajność vs jakość wizualną",
            "upscaler_sharpness_desc": "Intensywność wyostrzania obrazu - wyższe wartości = ostrzejszy obraz",
            "renderscale_desc": "Skala rozdzielczości wewnętrznego renderowania - niższa = lepsza wydajność",
            "motioncutoff_desc": "Próg wektora ruchu dla skalowania - wpływa na klarowność ruchu",
            "debug_desc": "Włącz wyjście debug i nakładki do rozwiązywania problemów",
            "logtofile_desc": "Zapisuj informacje debug do plików dziennika",
            "autoexposure_desc": "Automatyczna regulacja ekspozycji dla lepszej jakości obrazu",
            "hdr_desc": "Obsługa High Dynamic Range dla kompatybilnych wyświetlaczy",
            "colorspace_desc": "Konfiguracja przestrzeni kolorów dla dokładnej reprodukcji kolorów",
        }
    
    def set_language(self, language_code):
        """Set the current language"""
        if language_code in self.translations:
            self.current_language = language_code
            return True
        return False
    
    def get_languages(self):
        """Get list of available languages"""
        return {
            "en": "English",
            "da": "Dansk", 
            "pl": "Polski"
        }
    
    def t(self, key, *args):
        """Translate a key to current language"""
        translation = self.translations.get(self.current_language, {}).get(key)
        
        # Fallback to English if translation not found
        if translation is None:
            translation = self.translations.get("en", {}).get(key, key)
        
        # Format with arguments if provided
        if args:
            try:
                return translation.format(*args)
            except:
                return translation
        
        return translation

# Global i18n instance
i18n = I18n()

def t(key, *args):
    """Shortcut function for translations"""
    return i18n.t(key, *args)

def set_language(language_code):
    """Shortcut function to set language"""
    return i18n.set_language(language_code)

def get_languages():
    """Shortcut function to get available languages"""
    return i18n.get_languages()
