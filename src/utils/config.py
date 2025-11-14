"""
Configuration management for OptiScaler-GUI
"""
import os
from pathlib import Path
import json

class Config:
    """Configuration manager for OptiScaler-GUI"""
    
    def __init__(self):
        # Get the root directory of the project
        self.root_dir = Path(__file__).parent.parent.parent.absolute()
        
        # Cache directories
        self.cache_dir = self.root_dir / "cache"
        self.game_cache_dir = self.cache_dir / "game_images"
        self.optiscaler_downloads_dir = self.cache_dir / "optiscaler_downloads"
        
        # Assets directories
        self.assets_dir = self.root_dir / "assets"
        self.icons_dir = self.assets_dir / "icons"
        
        # Create directories if they don't exist
        self.cache_dir.mkdir(exist_ok=True)
        self.game_cache_dir.mkdir(exist_ok=True)
        self.optiscaler_downloads_dir.mkdir(exist_ok=True)
        self.assets_dir.mkdir(exist_ok=True)
        self.icons_dir.mkdir(exist_ok=True)
        
        # Image settings
        self.max_image_size = (300, 450)  # Standard game cover size
        self.image_quality = 85
        
        # Cache settings
        self.steam_app_list_cache_days = 7
        self.max_cache_size_mb = 500
        # Library discovery cache TTL in seconds (default 24 hours)
        # Set default; _settings will be updated after loading the config file
        self.library_discovery_cache_ttl = 86400
        
        # Performance settings
        self.max_scan_depth = 3
        self.image_download_timeout = 10
        self.concurrent_downloads = 5
        # Persistent GUI settings file
        self.config_file = self.cache_dir / 'config.json'
        # Load existing settings or create defaults
        self._settings = self._load_settings_file()

        # Threadpool defaults
        try:
            cpu_count = os.cpu_count() or 1
        except Exception:
            cpu_count = 1
        default_workers = min(8, cpu_count * 4)
        # allow persisted override from config.json
        self.max_workers = int(self._settings.get('max_workers', default_workers))
        # Apply persisted library discovery TTL if provided
        try:
            self.library_discovery_cache_ttl = int(self._settings.get('library_discovery_cache_ttl', self.library_discovery_cache_ttl))
        except Exception:
            pass

    def _load_settings_file(self):
        """Load settings from JSON file; return dict or empty dict on error"""
        try:
            if self.config_file.exists():
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    return json.load(f)
            else:
                return {}
        except Exception:
            return {}

    def _save_settings_file(self, settings: dict):
        """Save settings dict into JSON file"""
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(settings, f, indent=2)
            return True
        except Exception:
            return False
        
    @property
    def no_image_path(self):
        """Path to the no image placeholder"""
        return str(self.icons_dir / "no_image.png")
    
    @property
    def xbox_logo_path(self):
        """Path to the Xbox logo"""
        return str(self.icons_dir / "xbox_logo.png")
    
    @property
    def log_file_path(self):
        """Path to the debug log file"""
        return str(self.cache_dir / "optiscaler_gui_debug.log")
    
    @property
    def steam_app_list_cache_path(self):
        """Path to the Steam app list cache"""
        return str(self.game_cache_dir / "steam_app_list.json")

# Global configuration instance
config = Config()

# Simple config value functions for backwards compatibility
def get_config_value(key, default=None):
    """Get configuration value from persisted JSON settings (cache/config.json)"""
    return config._settings.get(key, default)

def set_config_value(key, value):
    """Set configuration value and persist to JSON settings file"""
    config._settings[key] = value
    try:
        # Update object attributes if config provides a known field
        if key == 'library_discovery_cache_ttl':
            try:
                config.library_discovery_cache_ttl = int(value)
            except Exception:
                pass
        config._save_settings_file(config._settings)
    except Exception:
        return False
    return True

# Compatibility function - left for testability in case other modules used it
def _read_json_file(file_path):
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return {}


