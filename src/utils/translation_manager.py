import json
import os
import sys
from pathlib import Path
from typing import Dict, Any, Optional

class TranslationManager:
    def __init__(self):
        self.translations: Dict[str, Dict[str, Any]] = {}
        self.current_language = "en"
        
        # PyInstaller-compatible path handling
        if getattr(sys, 'frozen', False):
            # PyInstaller environment
            bundle_dir = Path(sys._MEIPASS)
            self.translations_dir = bundle_dir / 'src' / 'translations'
        else:
            # Development environment
            self.translations_dir = Path(__file__).parent.parent / "translations"
        
        self.load_all_translations()
    
    def load_all_translations(self):
        """Load all translation files from the translations directory"""
        translation_files = {
            "en": "en.json",
            "da": "da.json", 
            "pl": "pl.json"
        }
        
        for lang_code, filename in translation_files.items():
            filepath = self.translations_dir / filename
            if filepath.exists():
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        self.translations[lang_code] = json.load(f)
                    print(f"Loaded translation file: {filepath}")
                except Exception as e:
                    print(f"Error loading translation file {filename}: {e}")
                    # Fallback to empty dict if file can't be loaded
                    self.translations[lang_code] = {}
            else:
                print(f"Translation file not found: {filepath}")
                self.translations[lang_code] = {}

    def reload_translations(self):
        """Reload all translation files from disk"""
        self.translations.clear()
        self.load_all_translations()
    
    def set_language(self, language_code: str):
        """Set the current language"""
        if language_code in self.translations:
            self.current_language = language_code
        else:
            print(f"Language {language_code} not available, using English")
            self.current_language = "en"
    
    def get_text(self, key_path: str, default: Optional[str] = None) -> str:
        """
        Get translated text using dot notation path (e.g., 'ui.app_title')
        Falls back to English if key not found in current language
        Falls back to default if key not found in any language
        """
        # Try current language first
        text = self._get_nested_value(self.translations.get(self.current_language, {}), key_path)
        
        # Fallback to English if not found
        if text is None and self.current_language != "en":
            text = self._get_nested_value(self.translations.get("en", {}), key_path)
        
        # Fallback to default or key path if still not found
        if text is None:
            if default is not None:
                return default
            return f"[{key_path}]"  # Show key path when translation missing
        
        return text
    
    def _get_nested_value(self, data: Dict[str, Any], key_path: str) -> Optional[str]:
        """Get value from nested dictionary using dot notation"""
        keys = key_path.split('.')
        current = data
        
        try:
            for key in keys:
                current = current[key]
            return str(current) if current is not None else None
        except (KeyError, TypeError):
            return None
    
    def get_setting_description(self, section: str, key: str) -> Optional[str]:
        """Get description for an OptiScaler setting"""
        desc_key = f"optiscaler_settings.{section.lower()}.{key.lower()}"
        desc = self.get_text(desc_key)
        
        # If we get back the key path (translation not found), return None
        if desc.startswith("[") and desc.endswith("]"):
            return None
        
        return desc
    
    def get_section_title(self, section: str) -> str:
        """Get user-friendly title for an OptiScaler section"""
        section_key = f"sections.{section}"
        title = self.get_text(section_key)
        
        # If we get back the key path (translation not found), return original section name
        if title == f"[{section_key}]" or title == section_key:
            return section
        
        return title
    
    def get_setting_value_label(self, value: str) -> str:
        """Get user-friendly label for setting values like true/false/auto"""
        value_lower = value.lower()
        
        if value_lower == "true":
            return self.get_text("setting_values.enabled", "Enabled")
        elif value_lower == "false":
            return self.get_text("setting_values.disabled", "Disabled")
        elif value_lower == "auto":
            return self.get_text("setting_values.auto", "Auto")
        else:
            return value  # Return original value if no mapping
    
    def get_raw_value_from_label(self, label: str) -> str:
        """Convert user-friendly label back to raw value for saving"""
        # Get the translated values for comparison
        enabled_label = self.get_text("setting_values.enabled", "Enabled")
        disabled_label = self.get_text("setting_values.disabled", "Disabled")
        auto_label = self.get_text("setting_values.auto", "Auto")
        
        if label == enabled_label:
            return "true"
        elif label == disabled_label:
            return "false"
        elif label == auto_label:
            return "auto"
        else:
            return label  # Return as-is if no mapping found
    
    def get_languages(self) -> Dict[str, str]:
        """Get available languages with their display names"""
        return {
            "en": "English",
            "da": "Dansk", 
            "pl": "Polski"
        }

# Global instance
_translation_manager = None

def get_translation_manager() -> TranslationManager:
    """Get the global translation manager instance"""
    global _translation_manager
    if _translation_manager is None:
        _translation_manager = TranslationManager()
    return _translation_manager

def t(key_path: str, default: Optional[str] = None) -> str:
    """Shorthand function for getting translations"""
    return get_translation_manager().get_text(key_path, default)

def set_language(language_code: str):
    """Set the current language"""
    get_translation_manager().set_language(language_code)

def reload_translations():
    """Reload all translation files from disk"""
    get_translation_manager().reload_translations()

def get_languages() -> Dict[str, str]:
    """Get available languages with their display names"""
    return get_translation_manager().get_languages()
