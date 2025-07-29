#!/usr/bin/env python3
"""
OptiScaler Robust Wrapper
Handles OptiScaler operations with defensive programming and fallback mechanisms
"""

import sys
import os
from pathlib import Path

# Add src to path
current_dir = Path(__file__).parent
src_dir = current_dir.parent
if str(src_dir) not in sys.path:
    sys.path.insert(0, str(src_dir))

import json
import shutil
import configparser
from typing import Tuple, Optional, Dict, List, Callable
from utils.debug import debug_log
from utils.compatibility_checker import compatibility_checker
from utils.translation_manager import t

class OptiScalerRobustWrapper:
    """Robust wrapper for OptiScaler operations with fallback mechanisms"""
    
    def __init__(self):
        self.optiscaler_manager = None
        self.fallback_methods = {}
        self.operation_history = []
        
        # Initialize manager with error handling
        self._initialize_manager()
        
        # Setup fallback methods
        self._setup_fallback_methods()
    
    def _initialize_manager(self):
        """Initialize OptiScaler manager with error handling"""
        try:
            from optiscaler.manager import OptiScalerManager
            self.optiscaler_manager = OptiScalerManager()
            debug_log("OptiScaler manager initialized successfully")
        except Exception as e:
            debug_log(f"Failed to initialize OptiScaler manager: {e}")
            self.optiscaler_manager = None
    
    def _setup_fallback_methods(self):
        """Setup fallback methods for common operations"""
        self.fallback_methods = {
            "is_installed": self._fallback_is_installed,
            "install": self._fallback_install,
            "uninstall": self._fallback_uninstall,
            "get_version": self._fallback_get_version
        }
    
    def safe_operation(self, operation_name: str, *args, **kwargs) -> Tuple[bool, str, Optional[any]]:
        """Safely execute an operation with fallback mechanisms"""
        debug_log(f"Executing safe operation: {operation_name}")
        
        # Record operation attempt
        operation_record = {
            "operation": operation_name,
            "args": str(args)[:200],  # Truncate for logging
            "timestamp": str(Path(__file__).stat().st_mtime)
        }
        
        try:
            # Try primary method
            if self.optiscaler_manager and hasattr(self.optiscaler_manager, operation_name):
                method = getattr(self.optiscaler_manager, operation_name)
                result = method(*args, **kwargs)
                
                operation_record["status"] = "success"
                operation_record["method"] = "primary"
                self.operation_history.append(operation_record)
                
                return True, "Operation successful", result
            
            # Try fallback method
            elif operation_name in self.fallback_methods:
                debug_log(f"Using fallback method for {operation_name}")
                fallback_method = self.fallback_methods[operation_name]
                result = fallback_method(*args, **kwargs)
                
                operation_record["status"] = "success"
                operation_record["method"] = "fallback"
                self.operation_history.append(operation_record)
                
                return True, "Operation successful (fallback)", result
            
            else:
                operation_record["status"] = "failed"
                operation_record["error"] = "No method available"
                self.operation_history.append(operation_record)
                
                return False, f"No method available for {operation_name}", None
        
        except Exception as e:
            error_msg = str(e)
            debug_log(f"Operation {operation_name} failed: {error_msg}")
            
            operation_record["status"] = "failed"
            operation_record["error"] = error_msg
            self.operation_history.append(operation_record)
            
            # Try fallback if primary failed
            if operation_name in self.fallback_methods and self.optiscaler_manager:
                try:
                    debug_log(f"Trying fallback for failed operation: {operation_name}")
                    fallback_method = self.fallback_methods[operation_name]
                    result = fallback_method(*args, **kwargs)
                    
                    operation_record["fallback_status"] = "success"
                    return True, f"Operation successful via fallback (primary failed: {error_msg})", result
                
                except Exception as fallback_error:
                    operation_record["fallback_status"] = "failed"
                    operation_record["fallback_error"] = str(fallback_error)
                    return False, f"Both primary and fallback failed: {error_msg}, {fallback_error}", None
            
            return False, error_msg, None
    
    def _fallback_is_installed(self, game_path: str) -> bool:
        """Fallback method to check if OptiScaler is installed"""
        try:
            game_path = Path(game_path)
            
            # Common OptiScaler files to check for
            indicator_files = [
                "OptiScaler.dll",
                "OptiScaler.ini",
                "dxgi.dll",  # Common proxy
                "winmm.dll", # Alternative proxy
                "nvngx.dll"  # NVIDIA proxy
            ]
            
            # Check for any indicator files
            for file_name in indicator_files:
                if (game_path / file_name).exists():
                    debug_log(f"Found OptiScaler indicator file: {file_name}")
                    return True
            
            # Check in subdirectories
            subdirs = ["D3D12_Optiscaler", "OptiScaler"]
            for subdir in subdirs:
                subdir_path = game_path / subdir
                if subdir_path.exists() and subdir_path.is_dir():
                    debug_log(f"Found OptiScaler subdirectory: {subdir}")
                    return True
            
            return False
            
        except Exception as e:
            debug_log(f"Fallback installation check failed: {e}")
            return False
    
    def _fallback_install(self, game_path: str, **kwargs) -> Tuple[bool, str]:
        """Fallback installation method"""
        try:
            debug_log(f"Attempting fallback installation to {game_path}")
            
            # Check if we have a cached OptiScaler download
            cache_dir = Path("cache/optiscaler_downloads")
            extracted_dir = cache_dir / "extracted_optiscaler"
            
            if not extracted_dir.exists():
                return False, "No OptiScaler cache available for fallback installation"
            
            # Find main OptiScaler.dll
            optiscaler_dll = extracted_dir / "OptiScaler.dll"
            if not optiscaler_dll.exists():
                return False, "OptiScaler.dll not found in cache"
            
            # Install basic files
            game_path = Path(game_path)
            
            # Copy main DLL as proxy (try common proxy names)
            proxy_names = ["dxgi.dll", "winmm.dll", "nvngx.dll"]
            proxy_installed = False
            
            for proxy_name in proxy_names:
                try:
                    target_path = game_path / proxy_name
                    if not target_path.exists():  # Don't overwrite existing files
                        shutil.copy2(optiscaler_dll, target_path)
                        debug_log(f"Installed OptiScaler as {proxy_name}")
                        proxy_installed = True
                        break
                except Exception as e:
                    debug_log(f"Failed to install as {proxy_name}: {e}")
                    continue
            
            if not proxy_installed:
                return False, "Could not install OptiScaler proxy DLL"
            
            # Copy additional files
            additional_files = [
                "OptiScaler.ini",
                "amd_fidelityfx_dx12.dll",
                "amd_fidelityfx_vk.dll",
                "libxess_dx11.dll",
                "libxess.dll"
            ]
            
            copied_files = []
            for file_name in additional_files:
                source_file = extracted_dir / file_name
                if source_file.exists():
                    try:
                        target_file = game_path / file_name
                        shutil.copy2(source_file, target_file)
                        copied_files.append(file_name)
                        debug_log(f"Copied {file_name}")
                    except Exception as e:
                        debug_log(f"Failed to copy {file_name}: {e}")
            
            success_msg = f"Fallback installation successful. Proxy installed, {len(copied_files)} additional files copied."
            return True, success_msg
            
        except Exception as e:
            error_msg = f"Fallback installation failed: {e}"
            debug_log(error_msg)
            return False, error_msg
    
    def _fallback_uninstall(self, game_path: str) -> Tuple[bool, str]:
        """Fallback uninstallation method"""
        try:
            debug_log(f"Attempting fallback uninstallation from {game_path}")
            
            game_path = Path(game_path)
            removed_files = []
            
            # Files to remove
            files_to_remove = [
                "OptiScaler.dll",
                "OptiScaler.ini", 
                "dxgi.dll",
                "winmm.dll",
                "nvngx.dll",
                "amd_fidelityfx_dx12.dll",
                "amd_fidelityfx_vk.dll",
                "libxess_dx11.dll",
                "libxess.dll",
                "nvngx_dlss.dll"
            ]
            
            # Remove files
            for file_name in files_to_remove:
                file_path = game_path / file_name
                if file_path.exists():
                    try:
                        file_path.unlink()
                        removed_files.append(file_name)
                        debug_log(f"Removed {file_name}")
                    except Exception as e:
                        debug_log(f"Failed to remove {file_name}: {e}")
            
            # Remove directories
            dirs_to_remove = ["D3D12_Optiscaler", "OptiScaler", "DlssOverrides"]
            removed_dirs = []
            
            for dir_name in dirs_to_remove:
                dir_path = game_path / dir_name
                if dir_path.exists() and dir_path.is_dir():
                    try:
                        shutil.rmtree(dir_path)
                        removed_dirs.append(dir_name)
                        debug_log(f"Removed directory {dir_name}")
                    except Exception as e:
                        debug_log(f"Failed to remove directory {dir_name}: {e}")
            
            if removed_files or removed_dirs:
                success_msg = f"Fallback uninstallation successful. Removed {len(removed_files)} files and {len(removed_dirs)} directories."
                return True, success_msg
            else:
                return False, "No OptiScaler files found to remove"
                
        except Exception as e:
            error_msg = f"Fallback uninstallation failed: {e}"
            debug_log(error_msg)
            return False, error_msg
    
    def _fallback_get_version(self, game_path: str) -> Optional[str]:
        """Fallback method to get OptiScaler version"""
        try:
            game_path = Path(game_path)
            
            # Try to read version from INI file
            ini_file = game_path / "OptiScaler.ini"
            if ini_file.exists():
                try:
                    config = configparser.ConfigParser()
                    config.read(ini_file)
                    
                    # Look for version in various sections
                    for section in config.sections():
                        if 'version' in config[section]:
                            return config[section]['version']
                        if 'Version' in config[section]:
                            return config[section]['Version']
                except Exception as e:
                    debug_log(f"Failed to read version from INI: {e}")
            
            # Try to extract from DLL (if possible)
            # This is complex and OS-specific, so we'll skip for now
            
            return "Unknown (fallback detection)"
            
        except Exception as e:
            debug_log(f"Fallback version detection failed: {e}")
            return None
    
    def validate_operation_environment(self, operation: str, game_path: str) -> Tuple[bool, List[str]]:
        """Validate environment before performing operation"""
        warnings = []
        
        try:
            game_path = Path(game_path)
            
            # Basic path validation
            if not game_path.exists():
                return False, ["Game path does not exist"]
            
            if not game_path.is_dir():
                return False, ["Game path is not a directory"]
            
            # Write permission check
            test_file = game_path / "optiscaler_test.tmp"
            try:
                test_file.touch()
                test_file.unlink()
            except Exception:
                warnings.append("May not have write permissions to game directory")
            
            # Space check for installation
            if operation == "install":
                available_space = shutil.disk_usage(game_path).free
                if available_space < 100 * 1024 * 1024:  # 100MB minimum
                    warnings.append("Low disk space available")
            
            # Existing game files check
            important_game_files = [
                "*.exe", "UnityPlayer.dll", "UE4Game.exe", 
                "d3d11.dll", "d3d12.dll"
            ]
            
            game_files_found = False
            for pattern in important_game_files:
                if list(game_path.glob(pattern)):
                    game_files_found = True
                    break
            
            if not game_files_found:
                warnings.append("No obvious game files detected in directory")
            
            return True, warnings
            
        except Exception as e:
            debug_log(f"Environment validation failed: {e}")
            return False, [f"Validation error: {e}"]
    
    def get_operation_history(self) -> List[Dict]:
        """Get history of operations for debugging"""
        return self.operation_history[-50:]  # Last 50 operations
    
    def health_check(self) -> Dict:
        """Perform health check of the wrapper"""
        health = {
            "manager_available": self.optiscaler_manager is not None,
            "fallback_methods": len(self.fallback_methods),
            "recent_operations": len([op for op in self.operation_history[-10:] if op.get("status") == "success"]),
            "recent_failures": len([op for op in self.operation_history[-10:] if op.get("status") == "failed"]),
            "status": "healthy"
        }
        
        # Determine overall health
        if not health["manager_available"] and health["recent_failures"] > 5:
            health["status"] = "critical"
        elif health["recent_failures"] > 3:
            health["status"] = "degraded"
        elif not health["manager_available"]:
            health["status"] = "limited"
        
        return health

# Global instance
robust_wrapper = OptiScalerRobustWrapper()
