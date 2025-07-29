#!/usr/bin/env python3
"""
System Requirements Checker for OptiScaler GUI
Checks for required system components and suggests solutions
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

import shutil
from utils.translation_manager import t
from utils.debug import debug_log

class SystemRequirementsChecker:
    """Checks system requirements and provides user-friendly recommendations"""
    
    def __init__(self):
        self.requirements = {
            "python": {"required": True, "status": False, "message": ""},
            "7zip": {"required": False, "status": False, "message": ""},
            "py7zr": {"required": False, "status": False, "message": ""},
            "dependencies": {"required": True, "status": False, "message": ""}
        }
        
    def check_all_requirements(self):
        """Check all system requirements"""
        self._check_python()
        self._check_7zip()
        self._check_py7zr()
        self._check_dependencies()
        
        return self.requirements
    
    def _check_python(self):
        """Check Python version"""
        try:
            version = sys.version_info
            if version.major >= 3 and version.minor >= 8:
                self.requirements["python"]["status"] = True
                self.requirements["python"]["message"] = f"Python {version.major}.{version.minor}.{version.micro} ✅"
            else:
                self.requirements["python"]["message"] = f"Python {version.major}.{version.minor} - requires 3.8+ ❌"
        except Exception as e:
            self.requirements["python"]["message"] = f"Python check failed: {e} ❌"
    
    def _check_7zip(self):
        """Check for system 7-Zip installation"""
        seven_zip_paths = [
            r'C:\Program Files\7-Zip\7z.exe',
            r'C:\Program Files (x86)\7-Zip\7z.exe',
            '7z'  # Try system PATH
        ]
        
        for path in seven_zip_paths:
            if shutil.which(path) or (Path(path).exists() if os.path.isabs(path) else False):
                self.requirements["7zip"]["status"] = True
                self.requirements["7zip"]["message"] = f"7-Zip found at {path} ✅"
                return
        
        self.requirements["7zip"]["message"] = "7-Zip not found ⚠️"
    
    def _check_py7zr(self):
        """Check for py7zr Python library"""
        try:
            import py7zr
            self.requirements["py7zr"]["status"] = True
            self.requirements["py7zr"]["message"] = "py7zr library available ✅"
        except ImportError:
            self.requirements["py7zr"]["message"] = "py7zr library not installed ⚠️"
    
    def _check_dependencies(self):
        """Check for required Python dependencies"""
        required_modules = [
            "customtkinter",
            "requests", 
            "PIL",
            "psutil"
        ]
        
        missing_modules = []
        for module in required_modules:
            try:
                __import__(module)
            except ImportError:
                missing_modules.append(module)
        
        if not missing_modules:
            self.requirements["dependencies"]["status"] = True
            self.requirements["dependencies"]["message"] = "All dependencies available ✅"
        else:
            self.requirements["dependencies"]["message"] = f"Missing: {', '.join(missing_modules)} ❌"
    
    def get_archive_extraction_status(self):
        """Get status of archive extraction capabilities"""
        seven_zip_ok = self.requirements["7zip"]["status"]
        py7zr_ok = self.requirements["py7zr"]["status"]
        
        if seven_zip_ok and py7zr_ok:
            return "optimal", "Both 7-Zip and py7zr available - optimal performance ✅"
        elif seven_zip_ok:
            return "good", "7-Zip available - good performance ✅"
        elif py7zr_ok:
            return "functional", "py7zr available - functional but slower ⚠️"
        else:
            return "broken", "No 7z extraction method available - .7z files cannot be extracted ❌"
    
    def get_user_recommendations(self):
        """Get user-friendly recommendations for missing requirements"""
        recommendations = []
        
        # Check Python
        if not self.requirements["python"]["status"]:
            recommendations.append({
                "priority": "critical",
                "title": "Update Python",
                "description": "Python 3.8 or newer is required",
                "action": "Download Python from https://python.org"
            })
        
        # Check dependencies
        if not self.requirements["dependencies"]["status"]:
            recommendations.append({
                "priority": "critical", 
                "title": "Install Python Dependencies",
                "description": "Required Python packages are missing",
                "action": "Run: pip install -r requirements.txt"
            })
        
        # Check archive extraction
        status, message = self.get_archive_extraction_status()
        if status == "broken":
            recommendations.append({
                "priority": "critical",
                "title": "Install Archive Extraction Tool",
                "description": "Cannot extract OptiScaler .7z files",
                "action": "Install 7-Zip from https://www.7-zip.org/ OR run: pip install py7zr"
            })
        elif status == "functional":
            recommendations.append({
                "priority": "optional",
                "title": "Install 7-Zip for Better Performance", 
                "description": "7-Zip provides faster extraction than py7zr",
                "action": "Download from https://www.7-zip.org/"
            })
        
        return recommendations
    
    def generate_user_report(self):
        """Generate user-friendly system requirements report"""
        self.check_all_requirements()
        
        report = {
            "overall_status": "ready",
            "requirements": self.requirements,
            "archive_status": self.get_archive_extraction_status(),
            "recommendations": self.get_user_recommendations()
        }
        
        # Determine overall status
        critical_missing = any(
            not req["status"] for req in self.requirements.values() 
            if req["required"]
        )
        
        archive_status, _ = self.get_archive_extraction_status()
        if critical_missing or archive_status == "broken":
            report["overall_status"] = "not_ready"
        elif archive_status == "functional":
            report["overall_status"] = "functional"
        
        return report

def show_requirements_dialog():
    """Show system requirements in a user-friendly dialog"""
    try:
        import customtkinter as ctk
        from CTkMessagebox import CTkMessagebox
        
        checker = SystemRequirementsChecker()
        report = checker.generate_user_report()
        
        # Build message
        message_parts = ["System Requirements Check:\n"]
        
        for name, req in report["requirements"].items():
            message_parts.append(f"• {req['message']}")
        
        message_parts.append(f"\nArchive Extraction: {report['archive_status'][1]}")
        
        if report["recommendations"]:
            message_parts.append("\nRecommendations:")
            for rec in report["recommendations"]:
                priority_icon = "🚨" if rec["priority"] == "critical" else "💡"
                message_parts.append(f"{priority_icon} {rec['title']}: {rec['action']}")
        
        # Show dialog
        icon = "info" if report["overall_status"] == "ready" else "warning"
        title = "System Ready" if report["overall_status"] == "ready" else "System Requirements"
        
        CTkMessagebox(
            title=title,
            message="\n".join(message_parts),
            icon=icon
        )
        
    except ImportError:
        # Fallback to console output
        print_requirements_report()

def print_requirements_report():
    """Print requirements report to console"""
    checker = SystemRequirementsChecker()
    report = checker.generate_user_report()
    
    print("🔧 OptiScaler GUI - System Requirements Check")
    print("=" * 60)
    
    for name, req in report["requirements"].items():
        print(f"  {req['message']}")
    
    print(f"\n📦 Archive Extraction: {report['archive_status'][1]}")
    
    if report["recommendations"]:
        print(f"\n💡 Recommendations:")
        for rec in report["recommendations"]:
            priority_icon = "🚨" if rec["priority"] == "critical" else "💡"
            print(f"  {priority_icon} {rec['title']}")
            print(f"     {rec['description']}")
            print(f"     Action: {rec['action']}\n")
    
    status_icons = {
        "ready": "✅",
        "functional": "⚠️", 
        "not_ready": "❌"
    }
    
    status_messages = {
        "ready": "OptiScaler GUI is ready to use!",
        "functional": "OptiScaler GUI will work but with reduced performance",
        "not_ready": "OptiScaler GUI cannot run until requirements are met"
    }
    
    print(f"{status_icons[report['overall_status']]} {status_messages[report['overall_status']]}")

# Global instance
requirements_checker = SystemRequirementsChecker()

if __name__ == "__main__":
    print_requirements_report()
