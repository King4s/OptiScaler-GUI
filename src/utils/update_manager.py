#!/usr/bin/env python3
"""
OptiScaler Update Manager
Handles checking for and applying OptiScaler updates
"""

import sys
import os
from pathlib import Path

# Add src to path for imports
current_dir = Path(__file__).parent
src_dir = current_dir.parent
if str(src_dir) not in sys.path:
    sys.path.insert(0, str(src_dir))

import requests
import json
from datetime import datetime
from utils.debug import debug_log
from optiscaler.manager import OptiScalerManager
from utils.translation_manager import t

class OptiScalerUpdateManager:
    """Manages OptiScaler updates and version tracking"""
    
    def __init__(self):
        self.github_api_url = "https://api.github.com/repos/optiscaler/OptiScaler/releases"
        self.cache_dir = Path("cache")
        self.version_cache_file = self.cache_dir / "optiscaler_version_cache.json"
        self.cache_dir.mkdir(exist_ok=True)
        
    def get_latest_release_info(self):
        """Get latest release information from GitHub API"""
        try:
            response = requests.get(f"{self.github_api_url}/latest", timeout=10)
            response.raise_for_status()
            return response.json()
        except Exception as e:
            debug_log(f"Failed to fetch latest release info: {e}")
            return None
    
    def get_all_releases(self, limit=10):
        """Get list of recent releases"""
        try:
            response = requests.get(f"{self.github_api_url}?per_page={limit}", timeout=10)
            response.raise_for_status()
            return response.json()
        except Exception as e:
            debug_log(f"Failed to fetch releases: {e}")
            return []
    
    def get_cached_version_info(self):
        """Get cached version information"""
        try:
            if self.version_cache_file.exists():
                with open(self.version_cache_file, 'r') as f:
                    return json.load(f)
        except Exception as e:
            debug_log(f"Failed to read version cache: {e}")
        return {}
    
    def save_version_cache(self, version_info):
        """Save version information to cache"""
        try:
            with open(self.version_cache_file, 'w') as f:
                json.dump(version_info, f, indent=2)
        except Exception as e:
            debug_log(f"Failed to save version cache: {e}")
    
    def check_for_updates(self):
        """
        Check if there are updates available
        Returns: dict with update information
        """
        latest_release = self.get_latest_release_info()
        if not latest_release:
            return {"available": False, "error": "Could not fetch release information"}
        
        cache = self.get_cached_version_info()
        latest_version = latest_release.get("tag_name", "")
        cached_version = cache.get("latest_known_version", "")
        last_check = cache.get("last_check", "")
        
        # Update cache
        cache.update({
            "latest_known_version": latest_version,
            "last_check": datetime.now().isoformat(),
            "release_info": {
                "tag_name": latest_release.get("tag_name"),
                "name": latest_release.get("name"),
                "published_at": latest_release.get("published_at"),
                "html_url": latest_release.get("html_url"),
                "body": latest_release.get("body", "")[:500]  # Limit changelog size
            }
        })
        self.save_version_cache(cache)
        
        # Check if this is a new version
        is_new_version = (latest_version != cached_version and 
                         cached_version != "" and 
                         latest_version != "")
        
        return {
            "available": is_new_version,
            "latest_version": latest_version,
            "cached_version": cached_version,
            "release_info": latest_release,
            "last_check": last_check
        }
    
    def get_installed_version_info(self, game_path):
        """Get version info of installed OptiScaler in a game directory"""
        game_path = Path(game_path)
        
        # Check for OptiScaler.ini which might contain version info
        ini_path = game_path / "OptiScaler.ini"
        version_info = {
            "installed": False,
            "version": "Unknown",
            "files": []
        }
        
        # Check if OptiScaler is installed
        manager = OptiScalerManager()
        if manager.is_optiscaler_installed(str(game_path)):
            version_info["installed"] = True
            
            # Try to find version information
            if ini_path.exists():
                try:
                    with open(ini_path, 'r') as f:
                        content = f.read()
                        # Look for version comments or info
                        for line in content.split('\n'):
                            if 'version' in line.lower() and ('#' in line or ';' in line):
                                version_info["version"] = line.strip()
                                break
                except Exception as e:
                    debug_log(f"Could not read version from {ini_path}: {e}")
            
            # List OptiScaler files
            optiscaler_files = []
            for pattern in ["OptiScaler*", "*nvngx*", "*dxgi*"]:
                optiscaler_files.extend(game_path.glob(pattern))
            
            version_info["files"] = [str(f.name) for f in optiscaler_files]
        
        return version_info
    
    def update_optiscaler_for_game(self, game_path, progress_callback=None):
        """Update OptiScaler for a specific game"""
        try:
            if progress_callback:
                progress_callback(t("status.checking_for_updates"))
            
            # Check if OptiScaler is installed
            manager = OptiScalerManager()
            if not manager.is_optiscaler_installed(game_path):
                return False, t("status.optiscaler_not_installed")
            
            if progress_callback:
                progress_callback(t("status.updating_optiscaler"))
            
            # Use the manager's update/reinstall functionality
            success, message = manager.install_optiscaler(
                game_path, 
                target_filename="nvngx.dll", 
                overwrite=True,
                progress_callback=progress_callback
            )
            
            if success:
                # Update our cache to mark this game as updated
                cache = self.get_cached_version_info()
                if "updated_games" not in cache:
                    cache["updated_games"] = {}
                
                cache["updated_games"][str(game_path)] = {
                    "updated_at": datetime.now().isoformat(),
                    "version": cache.get("latest_known_version", "Unknown")
                }
                self.save_version_cache(cache)
            
            return success, message
            
        except Exception as e:
            error_msg = f"Update failed: {e}"
            debug_log(error_msg)
            return False, error_msg
    
    def get_release_changelog(self, version_tag):
        """Get changelog for a specific version"""
        try:
            response = requests.get(f"{self.github_api_url}/tags/{version_tag}", timeout=10)
            response.raise_for_status()
            release_info = response.json()
            return release_info.get("body", "No changelog available")
        except Exception as e:
            debug_log(f"Failed to fetch changelog for {version_tag}: {e}")
            return "Could not fetch changelog"

# Global update manager instance
update_manager = OptiScalerUpdateManager()
