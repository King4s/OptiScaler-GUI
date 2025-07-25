"""
Configuration management for OptiScaler-GUI
"""
import os
from pathlib import Path

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
        
        # Performance settings
        self.max_scan_depth = 3
        self.image_download_timeout = 10
        self.concurrent_downloads = 5
        
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
    """Get configuration value"""
    # This would be extended with a JSON config file in the future
    return default

def set_config_value(key, value):
    """Set configuration value"""
    # This would be extended with a JSON config file in the future
    return True
