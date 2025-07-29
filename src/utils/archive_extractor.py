#!/usr/bin/env python3
"""
Archive Extractor for OptiScaler GUI
Provides robust archive extraction with multiple fallback methods
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
import subprocess
import zipfile
from utils.debug import debug_log
from utils.translation_manager import t

# Try to import py7zr with fallback
try:
    import py7zr
    PY7ZR_AVAILABLE = True
    debug_log("py7zr library available for 7z extraction")
except ImportError:
    PY7ZR_AVAILABLE = False
    debug_log("py7zr library not available - 7z extraction will use system 7z.exe only")

class ArchiveExtractor:
    """
    Robust archive extractor with multiple fallback methods
    
    Extraction priority:
    1. System 7z.exe (fastest, most reliable for 7z)
    2. py7zr Python library (cross-platform fallback for 7z)
    3. Python zipfile (for ZIP archives)
    """
    
    def __init__(self):
        # Try bundled 7z.exe first (for portable version)
        bundled_7z = self._get_bundled_7z_path()
        
        self.seven_zip_paths = [
            bundled_7z,  # Bundled 7z.exe (portable version)
            r'C:\Program Files\7-Zip\7z.exe',
            r'C:\Program Files (x86)\7-Zip\7z.exe',
            '7z'  # Try system PATH
        ]
        # Filter out None values
        self.seven_zip_paths = [path for path in self.seven_zip_paths if path is not None]
        self.system_7z_path = self._find_system_7z()
        
    def _get_bundled_7z_path(self):
        """Get path to bundled 7z.exe in portable version"""
        if getattr(sys, 'frozen', False):
            # PyInstaller environment - check next to executable
            exe_dir = Path(sys.executable).parent
            bundled_7z = exe_dir / '7z.exe'
            if bundled_7z.exists():
                return str(bundled_7z)
        else:
            # Development environment - check in project root
            project_root = Path(__file__).parents[2]  # Go up from src/utils/
            bundled_7z = project_root / '7z.exe'
            if bundled_7z.exists():
                return str(bundled_7z)
        return None
        
    def _find_system_7z(self):
        """Find available system 7-Zip executable"""
        for path in self.seven_zip_paths:
            if shutil.which(path) or (Path(path).exists() if os.path.isabs(path) else False):
                debug_log(f"Found system 7-Zip at: {path}")
                return path
        
        debug_log("System 7-Zip not found in standard locations")
        return None
    
    def extract_archive(self, archive_path, extract_path, progress_callback=None):
        """
        Extract archive with automatic format detection and fallback methods
        
        Args:
            archive_path: Path to archive file
            extract_path: Directory to extract to
            progress_callback: Optional callback for progress updates
            
        Returns:
            tuple: (success: bool, message: str, extracted_path: str or None)
        """
        archive_path = Path(archive_path)
        extract_path = Path(extract_path)
        
        if not archive_path.exists():
            return False, f"Archive file not found: {archive_path}", None
        
        # Create extraction directory
        try:
            if extract_path.exists():
                shutil.rmtree(extract_path)
            extract_path.mkdir(parents=True, exist_ok=True)
        except Exception as e:
            return False, f"Failed to create extraction directory: {e}", None
        
        # Determine archive type and extract
        suffix = archive_path.suffix.lower()
        
        if suffix == '.7z':
            return self._extract_7z(archive_path, extract_path, progress_callback)
        elif suffix == '.zip':
            return self._extract_zip(archive_path, extract_path, progress_callback)
        else:
            return False, f"Unsupported archive format: {suffix}", None
    
    def _extract_7z(self, archive_path, extract_path, progress_callback=None):
        """Extract 7z archive with multiple fallback methods"""
        
        # Method 1: Try system 7z.exe first (fastest and most reliable)
        if self.system_7z_path:
            success, message, path = self._extract_7z_system(archive_path, extract_path, progress_callback)
            if success:
                return success, message, path
            
            debug_log(f"System 7z failed: {message}, trying Python fallback...")
            if progress_callback:
                progress_callback("System 7z failed, trying Python fallback...")
        
        # Method 2: Try py7zr Python library as fallback
        if PY7ZR_AVAILABLE:
            return self._extract_7z_python(archive_path, extract_path, progress_callback)
        
        # No extraction methods available
        error_msg = "Cannot extract 7z archive: Neither system 7z.exe nor py7zr library available"
        debug_log(error_msg)
        return False, error_msg, None
    
    def _extract_7z_system(self, archive_path, extract_path, progress_callback=None):
        """Extract 7z using system 7z.exe"""
        try:
            if progress_callback:
                progress_callback("Extracting with system 7z.exe...")
            
            debug_log(f"Extracting {archive_path} with system 7z: {self.system_7z_path}")
            
            result = subprocess.run(
                [self.system_7z_path, 'x', str(archive_path), f'-o{extract_path}', '-y'],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=300  # 5 minute timeout
            )
            
            if progress_callback:
                progress_callback("System 7z extraction completed")
            
            debug_log("System 7z extraction successful")
            return True, "Extracted successfully with system 7z.exe", str(extract_path)
            
        except subprocess.TimeoutExpired:
            error_msg = "System 7z extraction timed out (5 minutes)"
            debug_log(error_msg)
            return False, error_msg, None
            
        except subprocess.CalledProcessError as e:
            error_msg = f"System 7z extraction failed: {e.stderr.decode() if e.stderr else 'Unknown error'}"
            debug_log(error_msg)
            return False, error_msg, None
            
        except Exception as e:
            error_msg = f"Unexpected error during system 7z extraction: {e}"
            debug_log(error_msg)
            return False, error_msg, None
    
    def _extract_7z_python(self, archive_path, extract_path, progress_callback=None):
        """Extract 7z using py7zr Python library"""
        try:
            if progress_callback:
                progress_callback("Extracting with Python py7zr library...")
            
            debug_log(f"Extracting {archive_path} with py7zr Python library")
            
            with py7zr.SevenZipFile(archive_path, mode='r') as archive:
                archive.extractall(path=extract_path)
            
            if progress_callback:
                progress_callback("Python py7zr extraction completed")
            
            debug_log("Python py7zr extraction successful")
            return True, "Extracted successfully with Python py7zr library", str(extract_path)
            
        except py7zr.exceptions.Bad7zFile:
            error_msg = "Invalid or corrupted 7z file"
            debug_log(error_msg)
            return False, error_msg, None
            
        except Exception as e:
            error_msg = f"Python py7zr extraction failed: {e}"
            debug_log(error_msg)
            return False, error_msg, None
    
    def _extract_zip(self, archive_path, extract_path, progress_callback=None):
        """Extract ZIP archive using Python's zipfile module"""
        try:
            if progress_callback:
                progress_callback("Extracting ZIP archive...")
            
            debug_log(f"Extracting {archive_path} with Python zipfile")
            
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                zip_ref.extractall(extract_path)
            
            if progress_callback:
                progress_callback("ZIP extraction completed")
            
            debug_log("ZIP extraction successful")
            return True, "Extracted successfully with Python zipfile", str(extract_path)
            
        except zipfile.BadZipFile:
            error_msg = "Invalid or corrupted ZIP file"
            debug_log(error_msg)
            return False, error_msg, None
            
        except Exception as e:
            error_msg = f"ZIP extraction failed: {e}"
            debug_log(error_msg)
            return False, error_msg, None
    
    def get_extraction_capabilities(self):
        """Get information about available extraction methods"""
        capabilities = {
            "system_7z_available": self.system_7z_path is not None,
            "system_7z_path": self.system_7z_path,
            "py7zr_available": PY7ZR_AVAILABLE,
            "zip_available": True,  # Always available with Python
            "supported_formats": []
        }
        
        if capabilities["system_7z_available"] or capabilities["py7zr_available"]:
            capabilities["supported_formats"].append(".7z")
        
        if capabilities["zip_available"]:
            capabilities["supported_formats"].append(".zip")
        
        return capabilities
    
    def validate_archive(self, archive_path):
        """
        Validate archive integrity
        
        Returns:
            tuple: (is_valid: bool, message: str)
        """
        archive_path = Path(archive_path)
        
        if not archive_path.exists():
            return False, "Archive file does not exist"
        
        suffix = archive_path.suffix.lower()
        
        if suffix == '.7z':
            return self._validate_7z(archive_path)
        elif suffix == '.zip':
            return self._validate_zip(archive_path)
        else:
            return False, f"Unsupported archive format: {suffix}"
    
    def _validate_7z(self, archive_path):
        """Validate 7z archive"""
        # Try system 7z test first
        if self.system_7z_path:
            try:
                result = subprocess.run(
                    [self.system_7z_path, 't', str(archive_path)],
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=60
                )
                return True, "Archive is valid (verified with system 7z.exe)"
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                pass  # Try Python method
        
        # Try py7zr validation
        if PY7ZR_AVAILABLE:
            try:
                with py7zr.SevenZipFile(archive_path, mode='r') as archive:
                    archive.testzip()
                return True, "Archive is valid (verified with py7zr)"
            except Exception:
                return False, "Archive validation failed with py7zr"
        
        # No validation method available
        return True, "Cannot validate 7z archive (no validation method available)"
    
    def _validate_zip(self, archive_path):
        """Validate ZIP archive"""
        try:
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                bad_file = zip_ref.testzip()
                if bad_file:
                    return False, f"ZIP archive is corrupted (bad file: {bad_file})"
                return True, "ZIP archive is valid"
        except zipfile.BadZipFile:
            return False, "Invalid ZIP file format"
        except Exception as e:
            return False, f"ZIP validation failed: {e}"

# Global instance
archive_extractor = ArchiveExtractor()
