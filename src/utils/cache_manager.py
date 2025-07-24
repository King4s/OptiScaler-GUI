"""
Cache management utilities for OptiScaler-GUI
"""
import os
import time
import shutil
from pathlib import Path
from utils.config import config

class CacheManager:
    """Manages cache directories and cleanup operations"""
    
    def __init__(self):
        self.cache_dir = Path(config.cache_dir)
        self.game_cache_dir = Path(config.game_cache_dir)
    
    def get_cache_size(self):
        """Get total cache size in MB"""
        total_size = 0
        try:
            for path in self.cache_dir.rglob('*'):
                if path.is_file():
                    total_size += path.stat().st_size
        except (OSError, PermissionError):
            pass
        return total_size / (1024 * 1024)  # Convert to MB
    
    def get_cache_stats(self):
        """Get detailed cache statistics"""
        stats = {
            'total_size_mb': 0,
            'image_count': 0,
            'image_size_mb': 0,
            'log_size_mb': 0,
            'other_size_mb': 0
        }
        
        try:
            # Count images
            for image_file in self.game_cache_dir.glob('*'):
                if image_file.is_file() and image_file.suffix.lower() in ['.jpg', '.jpeg', '.png', '.webp']:
                    size = image_file.stat().st_size
                    stats['image_count'] += 1
                    stats['image_size_mb'] += size / (1024 * 1024)
            
            # Check log file
            log_file = Path(config.log_file_path)
            if log_file.exists():
                stats['log_size_mb'] = log_file.stat().st_size / (1024 * 1024)
            
            # Calculate other files
            stats['total_size_mb'] = self.get_cache_size()
            stats['other_size_mb'] = stats['total_size_mb'] - stats['image_size_mb'] - stats['log_size_mb']
            
        except (OSError, PermissionError) as e:
            print(f"Error getting cache stats: {e}")
        
        return stats
    
    def cleanup_old_images(self, max_age_days=30):
        """Remove old cached images"""
        removed_count = 0
        removed_size = 0
        cutoff_time = time.time() - (max_age_days * 24 * 60 * 60)
        
        try:
            for image_file in self.game_cache_dir.glob('*'):
                if image_file.is_file() and image_file.suffix.lower() in ['.jpg', '.jpeg', '.png', '.webp']:
                    if image_file.stat().st_mtime < cutoff_time:
                        size = image_file.stat().st_size
                        image_file.unlink()
                        removed_count += 1
                        removed_size += size
        except (OSError, PermissionError) as e:
            print(f"Error cleaning up old images: {e}")
        
        return removed_count, removed_size / (1024 * 1024)
    
    def cleanup_large_cache(self):
        """Clean up cache if it exceeds size limit"""
        current_size = self.get_cache_size()
        
        if current_size > config.max_cache_size_mb:
            print(f"Cache size ({current_size:.1f} MB) exceeds limit ({config.max_cache_size_mb} MB)")
            
            # First, try removing old images
            removed_count, removed_size = self.cleanup_old_images(max_age_days=7)
            if removed_count > 0:
                print(f"Removed {removed_count} old images ({removed_size:.1f} MB)")
            
            # If still too large, remove oldest images
            current_size = self.get_cache_size()
            if current_size > config.max_cache_size_mb:
                self._cleanup_by_size()
    
    def _cleanup_by_size(self):
        """Remove oldest files until cache is under size limit"""
        try:
            # Get all image files with their modification times
            image_files = []
            for image_file in self.game_cache_dir.glob('*'):
                if image_file.is_file() and image_file.suffix.lower() in ['.jpg', '.jpeg', '.png', '.webp']:
                    image_files.append((image_file.stat().st_mtime, image_file))
            
            # Sort by modification time (oldest first)
            image_files.sort()
            
            removed_count = 0
            removed_size = 0
            
            for mtime, image_file in image_files:
                current_size = self.get_cache_size()
                if current_size <= config.max_cache_size_mb * 0.8:  # Leave some margin
                    break
                
                size = image_file.stat().st_size
                image_file.unlink()
                removed_count += 1
                removed_size += size
            
            if removed_count > 0:
                print(f"Removed {removed_count} oldest images ({removed_size / (1024 * 1024):.1f} MB)")
                
        except (OSError, PermissionError) as e:
            print(f"Error cleaning up cache by size: {e}")
    
    def clear_all_cache(self):
        """Clear all cache files (use with caution)"""
        try:
            if self.cache_dir.exists():
                shutil.rmtree(self.cache_dir)
                self.cache_dir.mkdir(exist_ok=True)
                self.game_cache_dir.mkdir(exist_ok=True)
                print("All cache cleared")
                return True
        except (OSError, PermissionError) as e:
            print(f"Error clearing cache: {e}")
            return False

# Global cache manager instance
cache_manager = CacheManager()
