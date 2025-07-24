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
            
            # Setting Descriptions
            "fsr_enabled_desc": "Enable AMD FidelityFX Super Resolution - improves performance with good visual quality",
            "fsr_quality_desc": "FSR Quality Mode - higher numbers = better performance, lower visual quality",
            "dlss_enabled_desc": "Enable NVIDIA DLSS - AI-powered upscaling for RTX graphics cards",
            "dlss_quality_desc": "DLSS Quality Mode - balances performance and visual fidelity", 
            "xess_enabled_desc": "Enable Intel XeSS - works on most graphics cards, best on Intel Arc",
            "xess_quality_desc": "XeSS Quality Mode - higher numbers = better performance",
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
            
            # Setting Descriptions
            "fsr_enabled_desc": "Aktiver AMD FidelityFX Super Resolution - forbedrer ydeevne med god visuel kvalitet",
            "fsr_quality_desc": "FSR Kvalitets Mode - højere tal = bedre ydeevne, lavere visuel kvalitet",
            "dlss_enabled_desc": "Aktiver NVIDIA DLSS - AI-drevet opskalering til RTX grafikkort",
            "dlss_quality_desc": "DLSS Kvalitets Mode - balancerer ydeevne og visuel kvalitet",
            "xess_enabled_desc": "Aktiver Intel XeSS - virker på de fleste grafikkort, bedst på Intel Arc",
            "xess_quality_desc": "XeSS Kvalitets Mode - højere tal = bedre ydeevne",
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
            
            # Setting Descriptions
            "fsr_enabled_desc": "Włącz AMD FidelityFX Super Resolution - poprawia wydajność przy dobrej jakości wizualnej",
            "fsr_quality_desc": "Tryb Jakości FSR - wyższe liczby = lepsza wydajność, niższa jakość wizualna",
            "dlss_enabled_desc": "Włącz NVIDIA DLSS - skalowanie oparte na AI dla kart graficznych RTX",
            "dlss_quality_desc": "Tryb Jakości DLSS - równoważy wydajność i wierność wizualną",
            "xess_enabled_desc": "Włącz Intel XeSS - działa na większości kart graficznych, najlepiej na Intel Arc",  
            "xess_quality_desc": "Tryb Jakości XeSS - wyższe liczby = lepsza wydajność",
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
