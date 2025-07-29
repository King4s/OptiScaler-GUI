#!/usr/bin/env python3
"""
OptiScaler Compatibility Checker
Handles version compatibility and API changes detection
"""

import sys
import os
from pathlib import Path

# PyInstaller-compatible path handling
def setup_paths():
    """Setup paths for both development and PyInstaller environments"""
    if getattr(sys, 'frozen', False):
        # PyInstaller environment
        bundle_dir = Path(sys._MEIPASS)
        src_dir = bundle_dir / 'src'
    else:
        # Development environment
        current_dir = Path(__file__).parent
        src_dir = current_dir.parent
    
    if str(src_dir) not in sys.path:
        sys.path.insert(0, str(src_dir))
    
    return src_dir

setup_paths()

import json
import requests
from datetime import datetime
from utils.debug import debug_log

class OptiScalerCompatibilityChecker:
    """Handles compatibility checking and API change detection"""
    
    def __init__(self):
        self.cache_dir = Path("cache")
        self.compatibility_cache = self.cache_dir / "optiscaler_compatibility.json"
        self.known_issues = {}
        self.version_compatibility = {}
        
        # Initialize cache directory
        self.cache_dir.mkdir(exist_ok=True)
        
        # Load known compatibility data
        self._load_compatibility_data()
    
    def _load_compatibility_data(self):
        """Load compatibility data from cache"""
        try:
            if self.compatibility_cache.exists():
                with open(self.compatibility_cache, 'r') as f:
                    data = json.load(f)
                    self.known_issues = data.get("known_issues", {})
                    self.version_compatibility = data.get("version_compatibility", {})
        except Exception as e:
            debug_log(f"Failed to load compatibility data: {e}")
            self.known_issues = {}
            self.version_compatibility = {}
    
    def _save_compatibility_data(self):
        """Save compatibility data to cache"""
        try:
            data = {
                "known_issues": self.known_issues,
                "version_compatibility": self.version_compatibility,
                "last_updated": datetime.now().isoformat()
            }
            with open(self.compatibility_cache, 'w') as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            debug_log(f"Failed to save compatibility data: {e}")
    
    def check_version_compatibility(self, version: str) -> dict:
        """Check if a specific version has known issues"""
        debug_log(f"Checking compatibility for version: {version}")
        
        # Default compatibility
        result = {
            "compatible": True,
            "issues": [],
            "warnings": [],
            "recommendations": []
        }
        
        # Check known issues
        if version in self.known_issues:
            issue_data = self.known_issues[version]
            result["compatible"] = issue_data.get("compatible", True)
            result["issues"] = issue_data.get("issues", [])
            result["warnings"] = issue_data.get("warnings", [])
            result["recommendations"] = issue_data.get("recommendations", [])
        
        # Add version-specific checks
        if "pre" in version.lower():
            result["warnings"].append("Pre-release version - may contain bugs")
            result["recommendations"].append("Consider waiting for stable release")
        
        if "nightly" in version.lower():
            result["warnings"].append("Nightly build - experimental features")
            result["recommendations"].append("Use with caution - frequent updates")
        
        return result
    
    def detect_api_changes(self, release_info: dict) -> dict:
        """Detect potential API changes in new releases"""
        debug_log("Detecting potential API changes...")
        
        changes = {
            "file_structure_changes": [],
            "config_changes": [],
            "installation_changes": [],
            "breaking_changes": []
        }
        
        # Check release notes for breaking change keywords
        body = release_info.get("body", "").lower()
        changelog = release_info.get("name", "").lower()
        
        # File structure change indicators
        file_indicators = [
            "file structure", "new files", "removed files", 
            "folder changes", "directory", "moved files"
        ]
        
        # Config change indicators
        config_indicators = [
            "config", "settings", "ini file", "configuration",
            "new option", "removed option", "parameter"
        ]
        
        # Installation change indicators
        install_indicators = [
            "installation", "install method", "proxy", "dll",
            "new installer", "install process"
        ]
        
        # Breaking change indicators
        breaking_indicators = [
            "breaking", "incompatible", "requires update",
            "no longer", "deprecated", "removed support"
        ]
        
        # Analyze release notes
        for indicator in file_indicators:
            if indicator in body or indicator in changelog:
                changes["file_structure_changes"].append(
                    f"Potential file structure change detected: '{indicator}'"
                )
        
        for indicator in config_indicators:
            if indicator in body or indicator in changelog:
                changes["config_changes"].append(
                    f"Potential configuration change detected: '{indicator}'"
                )
        
        for indicator in install_indicators:
            if indicator in body or indicator in changelog:
                changes["installation_changes"].append(
                    f"Potential installation change detected: '{indicator}'"
                )
        
        for indicator in breaking_indicators:
            if indicator in body or indicator in changelog:
                changes["breaking_changes"].append(
                    f"Potential breaking change detected: '{indicator}'"
                )
        
        # Check assets for new file types
        assets = release_info.get("assets", [])
        known_asset_patterns = [".7z", ".zip", "OptiScaler", "Daria"]
        
        for asset in assets:
            asset_name = asset.get("name", "").lower()
            if not any(pattern.lower() in asset_name for pattern in known_asset_patterns):
                changes["file_structure_changes"].append(
                    f"New asset type detected: {asset.get('name', 'Unknown')}"
                )
        
        return changes
    
    def get_safe_update_recommendation(self, current_version: str, target_version: str) -> dict:
        """Get recommendation for safe updating"""
        debug_log(f"Getting update recommendation: {current_version} -> {target_version}")
        
        recommendation = {
            "safe_to_update": True,
            "risk_level": "low",  # low, medium, high
            "precautions": [],
            "backup_recommended": False,
            "manual_verification": False
        }
        
        # Version jump analysis
        if self._is_major_version_jump(current_version, target_version):
            recommendation["risk_level"] = "medium"
            recommendation["backup_recommended"] = True
            recommendation["precautions"].append("Major version change detected")
        
        # Pre-release handling
        if "pre" in target_version.lower() or "nightly" in target_version.lower():
            recommendation["risk_level"] = "medium"
            recommendation["precautions"].append("Target is pre-release version")
        
        # Check compatibility
        compatibility = self.check_version_compatibility(target_version)
        if not compatibility["compatible"]:
            recommendation["safe_to_update"] = False
            recommendation["risk_level"] = "high"
            recommendation["precautions"].extend(compatibility["issues"])
        
        if compatibility["warnings"]:
            recommendation["risk_level"] = "medium"
            recommendation["precautions"].extend(compatibility["warnings"])
        
        return recommendation
    
    def _is_major_version_jump(self, current: str, target: str) -> bool:
        """Check if this is a major version jump"""
        try:
            # Extract version numbers
            current_parts = self._extract_version_parts(current)
            target_parts = self._extract_version_parts(target)
            
            if current_parts and target_parts:
                # Check major version difference
                return abs(target_parts[0] - current_parts[0]) >= 1
        except Exception as e:
            debug_log(f"Version comparison failed: {e}")
        
        return False
    
    def _extract_version_parts(self, version: str) -> list:
        """Extract numeric version parts"""
        import re
        
        # Remove 'v' prefix and extract numbers
        version_clean = version.lstrip('v')
        numbers = re.findall(r'\d+', version_clean)
        
        try:
            return [int(n) for n in numbers[:3]]  # Major.Minor.Patch
        except:
            return []
    
    def update_compatibility_info(self, version: str, compatibility_data: dict):
        """Update compatibility information for a version"""
        self.version_compatibility[version] = compatibility_data
        self._save_compatibility_data()
        debug_log(f"Updated compatibility info for {version}")
    
    def report_installation_issue(self, version: str, error: str, context: dict = None):
        """Report an installation issue for tracking"""
        if version not in self.known_issues:
            self.known_issues[version] = {
                "compatible": True,
                "issues": [],
                "warnings": [],
                "recommendations": []
            }
        
        # Add the issue
        issue_entry = {
            "error": error,
            "timestamp": datetime.now().isoformat(),
            "context": context or {}
        }
        
        self.known_issues[version]["issues"].append(issue_entry)
        
        # Mark as potentially incompatible if multiple issues
        if len(self.known_issues[version]["issues"]) >= 3:
            self.known_issues[version]["compatible"] = False
            self.known_issues[version]["recommendations"].append(
                "Multiple installation failures reported"
            )
        
        self._save_compatibility_data()
        debug_log(f"Reported issue for {version}: {error}")

# Global instance
compatibility_checker = OptiScalerCompatibilityChecker()
