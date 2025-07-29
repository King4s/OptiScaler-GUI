import requests
import zipfile
import os
import shutil
import configparser
import re
import threading
import subprocess
import requests
import zipfile
from datetime import datetime
from pathlib import Path
from utils.debug import debug_log
from utils.archive_extractor import archive_extractor

# Configuration constants - moved to top for easier maintenance
class OptiScalerConfig:
    """Configuration constants for OptiScaler operations"""
    GITHUB_API_URL = "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest"
    SEVEN_ZIP_PATHS = [
        r'C:\Program Files\7-Zip\7z.exe',
        r'C:\Program Files (x86)\7-Zip\7z.exe',
        '7z'  # Try system PATH
    ]
    ARCHIVE_EXTENSIONS = ['.7z', '.zip']
    DOWNLOAD_CHUNK_SIZE = 8192
    SUBPROCESS_TIMEOUT = 30
    
    # File lists
    ADDITIONAL_FILES = [
        "OptiScaler.ini",
        "amd_fidelityfx_dx12.dll",
        "amd_fidelityfx_vk.dll", 
        "libxess_dx11.dll",
        "libxess.dll",
        "nvngx_dlss.dll",
    ]
    
    PROXY_FILENAMES = [
        'dxgi.dll',           # Default DirectX Graphics Infrastructure
        'winmm.dll',          # Windows Multimedia API
        'version.dll',        # Version Information API
        'dbghelp.dll',        # Debug Help Library
        'd3d12.dll',          # Direct3D 12 API
        'wininet.dll',        # Windows Internet API
        'winhttp.dll',        # Windows HTTP Services
        'OptiScaler.asi',     # ASI plugin format
        'nvngx.dll',          # NVIDIA NGX (for advanced scenarios)
    ]
    
    # Install indicator files
    INSTALL_INDICATORS = [
        "OptiScaler.log",
        "amd_fidelityfx_dx12.dll",
        "amd_fidelityfx_vk.dll",
        "libxess.dll",
        "libxess_dx11.dll",
        "Remove OptiScaler.bat"
    ]

class OptiScalerManager:
    """
    OptiScaler Manager for GUI
    
    This class handles downloading, installing, and managing OptiScaler.
    OptiScaler is the official project by the OptiScaler team: https://github.com/optiscaler/OptiScaler
    
    This manager automatically downloads the latest releases from the official repository
    and provides GUI-friendly installation methods.
    """
    def __init__(self, download_dir=None):
        self.github_release_url = OptiScalerConfig.GITHUB_API_URL
        
        # Use configurable download directory
        if download_dir:
            self.download_dir = Path(download_dir)
        else:
            # Default to cache in project directory
            project_root = Path(__file__).parent.parent.parent
            self.download_dir = project_root / "cache" / "optiscaler_downloads"
        
        self.download_dir.mkdir(parents=True, exist_ok=True)
        self._seven_zip_path = self._find_seven_zip()
        
        debug_log(f"OptiScalerManager initialized with download_dir: {self.download_dir}")
    
    def _find_seven_zip(self):
        """Find available 7-Zip executable"""
        for path in OptiScalerConfig.SEVEN_ZIP_PATHS:
            if shutil.which(path) or (Path(path).exists() if os.path.isabs(path) else False):
                debug_log(f"Found 7-Zip at: {path}")
                return path
        
        debug_log("7-Zip not found in standard locations")
        return None

    def _download_latest_release(self, progress_callback=None):
        """
        Download the latest OptiScaler release with improved error handling and validation
        
        Args:
            progress_callback: Optional callback for progress updates
            
        Returns:
            str: Path to downloaded archive, or None if failed
        """
        try:
            if progress_callback:
                progress_callback("Fetching release information...")
            
            debug_log("Fetching latest release information from GitHub API")
            
            # Fetch release information with timeout
            response = requests.get(
                self.github_release_url, 
                timeout=OptiScalerConfig.SUBPROCESS_TIMEOUT
            )
            response.raise_for_status()
            release_info = response.json()
            assets = release_info.get("assets", [])
            
            debug_log(f"Found {len(assets)} assets in release")

            # Find suitable archive asset
            archive_asset = None
            for asset in assets:
                if any(asset["name"].endswith(ext) for ext in OptiScalerConfig.ARCHIVE_EXTENSIONS):
                    archive_asset = asset
                    debug_log(f"Selected asset: {asset['name']}")
                    break

            if not archive_asset:
                debug_log(f"No suitable archive found. Available assets: {[a['name'] for a in assets]}")
                return None

            download_url = archive_asset["browser_download_url"]
            archive_filename = self.download_dir / archive_asset["name"]

            # Validate existing file
            if archive_filename.exists():
                if self._validate_archive(archive_filename, progress_callback):
                    debug_log(f"Valid archive exists, skipping download: {archive_filename}")
                    return str(archive_filename)
                else:
                    debug_log("Existing archive is invalid, removing and re-downloading")
                    archive_filename.unlink(missing_ok=True)

            # Download with progress tracking
            return self._download_file(download_url, archive_filename, archive_asset, progress_callback)
            
        except requests.RequestException as e:
            debug_log(f"Network error in _download_latest_release: {e}")
            return None
        except Exception as e:
            debug_log(f"Unexpected error in _download_latest_release: {e}")
            return None
    
    def _validate_archive(self, archive_path, progress_callback=None):
        """Validate archive integrity using robust validation methods"""
        try:
            if progress_callback:
                progress_callback("Validating existing archive...")
            
            # Use the robust archive extractor for validation
            is_valid, message = archive_extractor.validate_archive(archive_path)
            
            if is_valid:
                debug_log(f"Archive validation successful: {message}")
            else:
                debug_log(f"Archive validation failed: {message}")
            
            return is_valid
            
        except Exception as e:
            debug_log(f"Unexpected error during validation: {e}")
            return False
    
    def _download_file(self, url, filepath, asset_info, progress_callback=None):
        """Download file with progress tracking and error handling"""
        try:
            if progress_callback:
                progress_callback(f"Downloading {asset_info['name']}...")
            
            debug_log(f"Starting download: {url} -> {filepath}")
            
            with requests.get(url, stream=True, timeout=OptiScalerConfig.SUBPROCESS_TIMEOUT) as response:
                response.raise_for_status()
                total_size = int(response.headers.get('content-length', 0))
                downloaded = 0
                
                with open(filepath, 'wb') as f:
                    for chunk in response.iter_content(chunk_size=OptiScalerConfig.DOWNLOAD_CHUNK_SIZE):
                        if chunk:  # Filter out keep-alive chunks
                            f.write(chunk)
                            downloaded += len(chunk)
                            
                            # Update progress
                            if total_size > 0 and progress_callback and downloaded % (OptiScalerConfig.DOWNLOAD_CHUNK_SIZE * 10) == 0:
                                progress_percent = (downloaded / total_size) * 100
                                progress_callback(f"Downloading {asset_info['name']} ({progress_percent:.1f}%)...")
            
            # Verify download completed
            if total_size > 0 and filepath.stat().st_size != total_size:
                debug_log(f"Download size mismatch: expected {total_size}, got {filepath.stat().st_size}")
                filepath.unlink(missing_ok=True)
                return None
            
            debug_log(f"Download completed successfully: {filepath}")
            return str(filepath)
            
        except requests.RequestException as e:
            debug_log(f"Download failed: {e}")
            filepath.unlink(missing_ok=True)
            return None
        except Exception as e:
            debug_log(f"Unexpected download error: {e}")
            filepath.unlink(missing_ok=True)
            return None

    def _extract_release(self, archive_path, game_path=None, progress_callback=None):
        """
        Extract OptiScaler release archive with robust fallback methods
        
        Args:
            archive_path: Path to archive file
            game_path: Optional game path for context
            progress_callback: Optional progress callback
            
        Returns:
            str: Path to extracted files, or None if failed
        """
        if progress_callback:
            progress_callback("Preparing extraction...")
        
        archive_path = Path(archive_path)
        extract_path = self.download_dir / "extracted_optiscaler"
        
        debug_log(f"Extracting {archive_path} to {extract_path}")
        
        # Use the robust archive extractor with fallback methods
        success, message, extracted_path = archive_extractor.extract_archive(
            archive_path, 
            extract_path, 
            progress_callback
        )
        
        if success:
            debug_log(f"Extraction successful: {message}")
            return extracted_path
        else:
            debug_log(f"Extraction failed: {message}")
            if progress_callback:
                progress_callback(f"Extraction failed: {message}")
            return None
    
    def _extract_7z(self, archive_path, extract_path, progress_callback=None):
        """Extract 7z archive using 7-Zip"""
        if not self._seven_zip_path:
            debug_log("Cannot extract 7z: 7-Zip not found")
            return None
        
        try:
            if progress_callback:
                progress_callback("Extracting 7z archive...")
            
            result = subprocess.run(
                [self._seven_zip_path, 'x', str(archive_path), f'-o{extract_path}', '-y'],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=OptiScalerConfig.SUBPROCESS_TIMEOUT
            )
            
            if progress_callback:
                progress_callback("7z extraction completed")
            
            debug_log("7z extraction successful")
            return str(extract_path)
            
        except subprocess.TimeoutExpired:
            debug_log("7z extraction timed out")
            return None
        except subprocess.CalledProcessError as e:
            debug_log(f"7z extraction failed: {e.stderr.decode() if e.stderr else 'Unknown error'}")
            return None
        except Exception as e:
            debug_log(f"Unexpected error during 7z extraction: {e}")
            return None
    
    def _extract_zip(self, archive_path, extract_path, progress_callback=None):
        """Extract ZIP archive using Python's zipfile module"""
        try:
            if progress_callback:
                progress_callback("Extracting ZIP archive...")
            
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                zip_ref.extractall(extract_path)
            
            if progress_callback:
                progress_callback("ZIP extraction completed")
            
            debug_log("ZIP extraction successful")
            return str(extract_path)
            
        except zipfile.BadZipFile:
            debug_log("Invalid or corrupted ZIP file")
            return None
        except Exception as e:
            debug_log(f"ZIP extraction failed: {e}")
            return None

    def install_optiscaler(self, game_path, target_filename='dxgi.dll', overwrite=False, progress_callback=None):
        """
        Enhanced OptiScaler installation with improved error handling and performance.
        
        Args:
            game_path: Path to game directory
            target_filename: Target filename for OptiScaler proxy DLL
            overwrite: Whether to overwrite existing files
            progress_callback: Optional callback for progress updates
            
        Returns:
            tuple: (success: bool, message: str)
        """
        game_path = Path(game_path)
        debug_log(f"Starting OptiScaler installation: {game_path}")
        debug_log(f"Target filename: {target_filename}")
        
        try:
            if progress_callback:
                progress_callback("Starting installation...")
                
            # Download latest release
            zip_path = self._download_latest_release(progress_callback)
            if not zip_path:
                debug_log("Download failed")
                return False, "Download failed"
            debug_log(f"Downloaded archive: {zip_path}")

            # Extract files
            extracted_path = self._extract_release(zip_path, game_path=str(game_path), progress_callback=progress_callback)
            if not extracted_path:
                debug_log("Extraction failed")
                return False, "Extraction failed"
            debug_log(f"Extracted to: {extracted_path}")

            if progress_callback:
                progress_callback("Installing files...")

            # Remove setup marker file (like official setup does)
            marker_file = Path(extracted_path) / "!! EXTRACT ALL FILES TO GAME FOLDER !!"
            if marker_file.exists():
                marker_file.unlink()
                debug_log("Removed setup marker file")

            # Determine installation directory
            dest_dir = self._determine_install_directory(game_path)
            dest_dir.mkdir(parents=True, exist_ok=True)

            # Check target file conflicts
            target_path = dest_dir / target_filename
            if target_path.exists() and not overwrite:
                debug_log(f"Target file {target_filename} already exists and overwrite=False")
                return False, f"Target file {target_filename} already exists"
            elif target_path.exists():
                target_path.unlink()
                debug_log(f"Removed existing {target_filename}")

            # Find and copy main OptiScaler DLL
            optiscaler_dll_path = self._find_optiscaler_dll(extracted_path)
            if not optiscaler_dll_path:
                debug_log("OptiScaler.dll not found in extracted files")
                return False, "OptiScaler.dll not found in extracted files"

            # Install main DLL
            shutil.copy2(optiscaler_dll_path, target_path)
            copied_files = [target_filename]
            debug_log(f"Copied OptiScaler.dll as {target_filename}")

            # Copy additional files
            additional_files = self._copy_additional_files(extracted_path, dest_dir, progress_callback)
            copied_files.extend(additional_files)

            if progress_callback:
                progress_callback("Creating configuration...")

            # Create uninstaller script
            self.create_uninstaller_script(str(dest_dir), target_filename, copied_files)
            
            # Create default configuration if needed
            config_path = dest_dir / 'OptiScaler.ini'
            if not config_path.exists():
                gpu_type = self.detect_gpu_type()
                self.create_default_config(str(dest_dir), gpu_type)
                debug_log("Created default OptiScaler configuration")

            debug_log("Installation completed successfully")
            if progress_callback:
                progress_callback("Installation completed!")
            return True, "Installation completed successfully"

        except Exception as e:
            debug_log(f"Installation failed with exception: {e}")
            return False, f"Installation failed: {e}"
    
    def _determine_install_directory(self, game_path):
        """Determine correct installation directory (Unreal Engine vs regular game)"""
        game_path = Path(game_path)
        
        # Check for Unreal Engine structure
        unreal_dir = game_path / "Engine" / "Binaries" / "Win64"
        if unreal_dir.is_dir():
            debug_log(f"Detected Unreal Engine, installing to: {unreal_dir}")
            return unreal_dir
        else:
            debug_log(f"Installing to game root: {game_path}")
            return game_path

    def get_additional_optiscaler_files(self):
        """
        Get list of additional OptiScaler files to copy (excluding main DLL).
        Based on official OptiScaler file structure.
        """
        return OptiScalerConfig.ADDITIONAL_FILES.copy()

    def list_available_target_filenames(self, extracted_path=None):
        """
        List possible target filenames for OptiScaler based on official source code.
        Returns a list of supported DLL/ASI filenames from OptiScaler dllmain.cpp
        """
        return OptiScalerConfig.PROXY_FILENAMES.copy()

    def _find_optiscaler_dll(self, extracted_path):
        """Find OptiScaler.dll in extracted files with optimized search"""
        extracted_path = Path(extracted_path)
        
        # Direct check first (most common case)
        direct_path = extracted_path / 'OptiScaler.dll'
        if direct_path.exists():
            debug_log(f"Found OptiScaler.dll directly: {direct_path}")
            return str(direct_path)
        
        # Search in subdirectories (limited depth for performance)
        for dll_path in extracted_path.rglob('OptiScaler.dll'):
            debug_log(f"Found OptiScaler.dll in subdirectory: {dll_path}")
            return str(dll_path)
        
        debug_log("OptiScaler.dll not found in extracted files")
        return None

    def _copy_additional_files(self, extracted_path, dest_dir, progress_callback=None):
        """Copy additional OptiScaler files with improved error handling"""
        extracted_path = Path(extracted_path)
        dest_dir = Path(dest_dir)
        copied_files = []
        
        additional_files = self.get_additional_optiscaler_files()
        
        for file in additional_files:
            # Try direct path first
            src_path = extracted_path / file
            if not src_path.exists():
                # Search in subdirectories
                found_files = list(extracted_path.rglob(file))
                if found_files:
                    src_path = found_files[0]
                else:
                    debug_log(f"Additional file not found: {file}")
                    continue
            
            dest_file_path = dest_dir / file
            try:
                shutil.copy2(src_path, dest_file_path)
                copied_files.append(file)
                debug_log(f"Copied additional file: {file}")
                
                if progress_callback and len(copied_files) % 2 == 0:  # Update progress occasionally
                    progress_callback(f"Installing files... ({len(copied_files)}/{len(additional_files)})")
                    
            except Exception as e:
                debug_log(f"Failed to copy {file}: {e}")
        
        return copied_files

    def create_uninstaller_script(self, install_dir, main_filename, copied_files):
        """
        Create an uninstaller script similar to official OptiScaler setup.
        Enhanced with better error handling and Path usage.
        """
        install_dir = Path(install_dir)
        uninstaller_path = install_dir / 'Remove OptiScaler.bat'
        
        try:
            with open(uninstaller_path, 'w', encoding='utf-8') as f:
                f.write('@echo off\n')
                f.write('cls\n')
                f.write('echo OptiScaler Uninstaller\n')
                f.write('echo ======================\n')
                f.write('echo.\n')
                f.write('echo This will remove OptiScaler from this game.\n')
                f.write('echo.\n')
                f.write('set /p removeChoice="Do you want to remove OptiScaler? [y/n]: "\n')
                f.write('if "%removeChoice%"=="y" (\n')
                f.write('    echo Removing OptiScaler files...\n')
                
                # Remove main OptiScaler files
                for filename in copied_files:
                    f.write(f'    del "{filename}"\n')
                
                # Remove OptiScaler directories
                f.write('    del /Q D3D12_Optiscaler\\*\n')
                f.write('    rd D3D12_Optiscaler\n')
                f.write('    del /Q DlssOverrides\\*\n')
                f.write('    rd DlssOverrides\n')
                f.write('    del /Q Licenses\\*\n')
                f.write('    rd Licenses\n')
                f.write('    del OptiScaler.log\n')
                
                f.write('    echo.\n')
                f.write('    echo OptiScaler removed successfully!\n')
                f.write('    echo.\n')
                f.write(') else (\n')
                f.write('    echo.\n')
                f.write('    echo Operation cancelled.\n')
                f.write('    echo.\n')
                f.write(')\n')
                f.write('pause\n')
                f.write('if "%removeChoice%"=="y" (\n')
                f.write('    del "%0"\n')
                f.write(')\n')
            
            debug_log(f"Uninstaller script created: {uninstaller_path}")
            return True
        except Exception as e:
            debug_log(f"Failed to create uninstaller script: {e}")
            return False

    def create_default_config(self, install_dir, gpu_type='auto'):
        """
        Create a default OptiScaler.ini config based on official configuration structure.
        Enhanced with Path usage and better error handling.
        """
        install_dir = Path(install_dir)
        config_path = install_dir / 'OptiScaler.ini'
        
        # Default configuration based on OptiScaler source code Config.cpp
        default_config = f"""[OptiScaler]
; OptiScaler Configuration File
; Generated by OptiScaler-GUI

[GPU]
; GPU type detection - auto, nvidia, amd, intel
GPUType={gpu_type}

[DLSS]
; Enable DLSS support
Enabled=auto
; Path to DLSS library  
LibraryPath=auto
; DLSS feature path
FeaturePath=auto
; Path to NVNGX DLSS library
NVNGX_DLSS_Path=auto

[XeSS]
; Enable Intel XeSS support
Enabled=auto
; XeSS library path
LibraryPath=auto
; XeSS DirectX 11 library path
Dx11LibraryPath=auto

[FSR]
; Enable AMD FSR support
Enabled=auto

[Spoofing]
; Enable DXGI spoofing for AMD/Intel GPUs
Dxgi=auto
; Enable Streamline spoofing
Streamline=auto

[Log]
; Logging level (0=Error, 1=Warn, 2=Info, 3=Debug)
LogLevel=2
; Log to console
LogToConsole=false
; Log to file
LogToFile=true
; Log file name
LogFile=auto

[Overlay]
; Enable performance overlay
Enabled=true

[Hotkeys]
; Key to toggle overlay (Insert key)
ToggleOverlay=VK_INSERT
"""

        try:
            with open(config_path, 'w', encoding='utf-8') as f:
                f.write(default_config)
            debug_log(f"Created default config: {config_path}")
            return True
        except Exception as e:
            debug_log(f"Failed to create config: {e}")
            return False
    
    def _infer_type(self, value, comment):
        """Infer the type of a setting value based on its value and comment"""
        # Check for boolean options (true, false, auto)
        if "true or false" in comment.lower() or (value.lower() in ["true", "false", "auto"] and "auto" in comment.lower()):
            return "bool_options", ["true", "false", "auto"]

        # Check for options list in comments (e.g., "0 = Option A | 1 = Option B")
        options_match = re.search(r'(\d+\s*=\s*[^|]+(?:\|\s*\d+\s*=\s*[^|]+)*)', comment)
        if options_match:
            options_str = options_match.group(1)
            options = {}
            for item in options_str.split('|'):
                if '=' in item:
                    k, v = item.split('=', 1)
                    options[k.strip()] = v.strip()
            if options:
                # Check if 'auto' is also a valid option for this setting
                if "auto" in value.lower() or "default (auto)" in comment.lower():
                    if "auto" not in options.values(): # Ensure 'auto' is not already a display value
                        options["auto"] = "auto" # Use "auto" as both key and value for simplicity
                return "options", options

        # Check for integer
        try:
            int(value)
            return "int", None
        except ValueError:
            pass

        # Check for float
        try:
            float(value)
            return "float", None
        except ValueError:
            pass

        # Default to string
        return "string", None

    def read_optiscaler_ini(self, ini_path):
        """
        Read OptiScaler INI file with enhanced error handling and type inference
        
        Args:
            ini_path: Path to INI file
            
        Returns:
            dict: Parsed settings with metadata
        """
        ini_path = Path(ini_path)
        
        if not ini_path.exists():
            debug_log(f"INI file not found: {ini_path}")
            return {}
            
        settings = {}
        current_section = None
        current_comments = []

        try:
            with open(ini_path, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue

                    if line.startswith('['):
                        # Section header
                        if line.endswith(']'):
                            current_section = line[1:-1].strip()
                            settings[current_section] = {}
                            current_comments = []
                        else:
                            debug_log(f"Malformed section header at line {line_num}: {line}")
                            
                    elif line.startswith(';'):
                        # Comment line
                        current_comments.append(line[1:].strip())
                        
                    elif '=' in line:
                        # Key-value pair
                        if not current_section:
                            debug_log(f"Key-value pair outside section at line {line_num}: {line}")
                            continue
                            
                        key, value = line.split('=', 1)
                        key = key.strip()
                        
                        # Remove inline comments from value
                        value_without_inline_comment = value.split(';', 1)[0].strip()
                        
                        inferred_type, options = self._infer_type(
                            value_without_inline_comment, 
                            "\n".join(current_comments)
                        )

                        settings[current_section][key] = {
                            "value": value_without_inline_comment,
                            "comment": "\n".join(current_comments),
                            "type": inferred_type,
                            "options": options
                        }
                        current_comments = []
                        
        except UnicodeDecodeError as e:
            debug_log(f"Unicode decode error reading INI file: {e}")
            return {}
        except Exception as e:
            debug_log(f"Error reading INI file {ini_path}: {e}")
            return {}
            
        debug_log(f"Successfully read INI file with {len(settings)} sections")
        return settings

    def write_optiscaler_ini(self, ini_path, settings):
        """
        Write OptiScaler INI file with enhanced error handling and formatting
        
        Args:
            ini_path: Path to INI file
            settings: Settings dictionary to write
            
        Returns:
            bool: True if successful, False otherwise
        """
        ini_path = Path(ini_path)
        
        try:
            # Create parent directory if needed
            ini_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Create backup if file exists
            if ini_path.exists():
                backup_path = ini_path.with_suffix('.ini.backup')
                shutil.copy2(ini_path, backup_path)
                debug_log(f"Created backup: {backup_path}")
            
            with open(ini_path, 'w', encoding='utf-8') as f:
                for section_name, keys in settings.items():
                    f.write(f"[{section_name}]\n")
                    
                    for key, data in keys.items():
                        # Write comments if present
                        if data.get("comment"):
                            for comment_line in data["comment"].split('\n'):
                                if comment_line.strip():  # Skip empty comment lines
                                    f.write(f"; {comment_line}\n")
                        
                        # Write key-value pair
                        f.write(f"{key}={data['value']}\n")
                    
                    f.write("\n")  # Add spacing between sections
            
            debug_log(f"Successfully wrote INI file: {ini_path}")
            return True
            
        except Exception as e:
            debug_log(f"Error writing INI file {ini_path}: {e}")
            return False

    def load_settings(self, game_path):
        """
        Load OptiScaler settings from game directory with fallback to defaults
        
        Args:
            game_path: Path to game directory
            
        Returns:
            dict: Settings dictionary
        """
        game_path = Path(game_path)
        install_dir = self._determine_install_directory(game_path)
        ini_path = install_dir / "OptiScaler.ini"
        
        debug_log(f"Loading settings from: {ini_path}")
        
        if ini_path.exists():
            settings = self.read_optiscaler_ini(ini_path)
            if settings:
                return settings
        
        # Return default settings if file doesn't exist or is corrupted
        debug_log("Using default settings")
        return self._get_default_settings()

    def save_settings(self, game_path, settings):
        """
        Save OptiScaler settings to game directory
        
        Args:
            game_path: Path to game directory
            settings: Settings dictionary to save
            
        Returns:
            bool: True if successful, False otherwise
        """
        game_path = Path(game_path)
        install_dir = self._determine_install_directory(game_path)
        ini_path = install_dir / "OptiScaler.ini"
        
        debug_log(f"Saving settings to: {ini_path}")
        return self.write_optiscaler_ini(ini_path, settings)

    def _get_default_settings(self):
        """
        Get default OptiScaler settings
        
        Returns:
            dict: Default settings structure
        """
        return {
            "Engine": {
                "Mode": {
                    "value": "1",
                    "comment": "OptiScaler mode: 0=Off, 1=Performance, 2=Quality",
                    "type": "options",
                    "options": {"0": "Off", "1": "Performance", "2": "Quality"}
                },
                "Scale": {
                    "value": "1.5",
                    "comment": "Scaling factor",
                    "type": "float",
                    "options": None
                }
            }
        }

    def install_optiscaler_threaded(self, game_path, target_filename="nvngx.dll", progress_callback=None):
        """
        Install OptiScaler in a separate thread with comprehensive progress tracking
        
        Args:
            game_path: Path to game directory
            target_filename: Target proxy DLL filename
            progress_callback: Function to call with progress updates
            
        Returns:
            threading.Thread: Thread object for monitoring
        """
        
        def install_worker():
            """Worker function for installing OptiScaler"""
            try:
                debug_log(f"Starting threaded OptiScaler installation to: {game_path}")
                
                # Enhanced progress callback wrapper
                def enhanced_progress(stage, data=None):
                    if progress_callback:
                        progress_data = {
                            "stage": stage,
                            "game_path": str(game_path),
                            "target_filename": target_filename,
                            "timestamp": datetime.now().isoformat()
                        }
                        if data:
                            progress_data.update(data)
                        progress_callback(stage, progress_data)
                
                enhanced_progress("install_start")
                
                # Perform installation
                success, message = self.install_optiscaler(
                    game_path, 
                    target_filename, 
                    progress_callback=enhanced_progress
                )
                
                if success:
                    debug_log("Threaded installation completed successfully")
                    enhanced_progress("install_complete", {"status": "success", "message": message})
                else:
                    debug_log(f"Threaded installation failed: {message}")
                    enhanced_progress("install_error", {"status": "error", "message": message})
                    
            except Exception as e:
                error_msg = f"Installation thread error: {e}"
                debug_log(error_msg)
                if progress_callback:
                    progress_callback("install_error", {
                        "status": "error", 
                        "message": error_msg,
                        "game_path": str(game_path)
                    })
        
        # Create and start thread
        thread = threading.Thread(target=install_worker, daemon=True)
        thread.start()
        return thread

    def download_optiscaler_threaded(self, download_dir=None, progress_callback=None):
        """
        Download OptiScaler in a separate thread with improved error handling and progress tracking
        
        Args:
            download_dir: Override download directory
            progress_callback: Function to call with progress updates
            
        Returns:
            threading.Thread: Thread object for monitoring
        """
        
        def download_worker():
            """Worker function for downloading OptiScaler"""
            try:
                debug_log("Starting threaded OptiScaler download")
                
                # Download with progress tracking
                archive_path = self._download_latest_release(progress_callback=progress_callback)
                
                if archive_path:
                    debug_log("Threaded download completed successfully")
                    if progress_callback:
                        progress_callback("download_complete", {"status": "success", "archive_path": archive_path})
                else:
                    debug_log("Threaded download failed")
                    if progress_callback:
                        progress_callback("download_error", {"status": "error", "message": "Download failed"})
                        
            except Exception as e:
                error_msg = f"Download thread error: {e}"
                debug_log(error_msg)
                if progress_callback:
                    progress_callback("download_error", {"status": "error", "message": error_msg})
        
        # Create and start thread
        thread = threading.Thread(target=download_worker, daemon=True)
        thread.start()
        return thread

    def uninstall_optiscaler_threaded(self, game_path, progress_callback=None):
        """
        Uninstall OptiScaler in a separate thread with progress tracking
        
        Args:
            game_path: Path to game directory
            progress_callback: Function to call with progress updates
            
        Returns:
            threading.Thread: Thread object for monitoring
        """
        
        def uninstall_worker():
            """Worker function for uninstalling OptiScaler"""
            try:
                debug_log(f"Starting threaded OptiScaler uninstall from: {game_path}")
                
                if progress_callback:
                    progress_callback("uninstall_start", {
                        "game_path": str(game_path),
                        "timestamp": datetime.now().isoformat()
                    })
                
                # Perform uninstallation
                success, message = self.uninstall_optiscaler(game_path)
                
                if success:
                    debug_log("Threaded uninstall completed successfully")
                    if progress_callback:
                        progress_callback("uninstall_complete", {
                            "status": "success", 
                            "message": message,
                            "game_path": str(game_path)
                        })
                else:
                    debug_log(f"Threaded uninstall failed: {message}")
                    if progress_callback:
                        progress_callback("uninstall_error", {
                            "status": "error", 
                            "message": message,
                            "game_path": str(game_path)
                        })
                        
            except Exception as e:
                error_msg = f"Uninstall thread error: {e}"
                debug_log(error_msg)
                if progress_callback:
                    progress_callback("uninstall_error", {
                        "status": "error", 
                        "message": error_msg,
                        "game_path": str(game_path)
                    })
        
        # Create and start thread
        thread = threading.Thread(target=uninstall_worker, daemon=True)
        thread.start()
        return thread

    def list_available_target_filenames(self, extracted_path=None):
        """
        List possible target filenames for OptiScaler based on official source code.
        Returns a list of supported DLL/ASI filenames from OptiScaler dllmain.cpp
        """
        # Official OptiScaler supported proxy DLL names from source code
        supported_filenames = [
            'dxgi.dll',           # Default DirectX Graphics Infrastructure
            'winmm.dll',          # Windows Multimedia API
            'version.dll',        # Version Information API
            'dbghelp.dll',        # Debug Help Library
            'd3d12.dll',          # Direct3D 12 API
            'wininet.dll',        # Windows Internet API
            'winhttp.dll',        # Windows HTTP Services
            'OptiScaler.asi',     # ASI plugin format
            'nvngx.dll',          # NVIDIA NGX (for advanced scenarios)
        ]
        return supported_filenames

    def detect_gpu_type(self):
        """
        Enhanced GPU detection with improved error handling and multiple detection methods.
        Returns: 'nvidia', 'amd', 'intel', or 'unknown'
        """
        # Method 1: Check for NVIDIA using nvapi64.dll
        nvidia_paths = [
            Path(os.environ.get('WINDIR', 'C:\\Windows')) / 'System32' / 'nvapi64.dll',
            Path(os.environ.get('WINDIR', 'C:\\Windows')) / 'SysWOW64' / 'nvapi64.dll'
        ]
        
        for nvapi_path in nvidia_paths:
            if nvapi_path.exists():
                debug_log(f"NVIDIA GPU detected via {nvapi_path}")
                return 'nvidia'
        
        # Method 2: Use WMI to query video controllers
        try:
            result = subprocess.run(
                ['wmic', 'path', 'win32_VideoController', 'get', 'name'], 
                capture_output=True, 
                text=True, 
                timeout=10,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            )
            
            if result.returncode == 0:
                gpu_names = result.stdout.lower()
                
                # Check in order of preference
                if 'nvidia' in gpu_names:
                    debug_log("NVIDIA GPU detected via WMI")
                    return 'nvidia'
                elif 'amd' in gpu_names or 'radeon' in gpu_names:
                    debug_log("AMD GPU detected via WMI")
                    return 'amd'
                elif 'intel' in gpu_names:
                    debug_log("Intel GPU detected via WMI") 
                    return 'intel'
            else:
                debug_log(f"WMI query failed with return code: {result.returncode}")
                
        except subprocess.TimeoutExpired:
            debug_log("GPU detection via WMI timed out")
        except FileNotFoundError:
            debug_log("WMIC command not found")
        except Exception as e:
            debug_log(f"GPU detection via WMI failed: {e}")
        
        # Method 3: Check DirectX/OpenGL drivers
        try:
            driver_paths = [
                'nvoglv64.dll',  # NVIDIA OpenGL
                'atigktxx.dll',  # AMD
                'ig9icd64.dll',  # Intel
            ]
            
            system32 = Path(os.environ.get('WINDIR', 'C:\\Windows')) / 'System32'
            
            for driver in driver_paths:
                if (system32 / driver).exists():
                    if 'nvog' in driver:
                        debug_log(f"NVIDIA GPU detected via driver: {driver}")
                        return 'nvidia'
                    elif 'atig' in driver:
                        debug_log(f"AMD GPU detected via driver: {driver}")
                        return 'amd'
                    elif 'ig9' in driver:
                        debug_log(f"Intel GPU detected via driver: {driver}")
                        return 'intel'
        except Exception as e:
            debug_log(f"Driver detection failed: {e}")
        
        debug_log("GPU type could not be determined")
        return 'unknown'

    def check_dlss_availability(self, extracted_path):
        """
        Check if DLSS (nvngx_dlss.dll) is available in extracted files.
        Based on OptiScaler's CheckUpscalerFiles() function.
        """
        for root, _, files in os.walk(extracted_path):
            for file in files:
                if file.lower() == 'nvngx_dlss.dll':
                    dlss_path = os.path.join(root, file)
                    debug_log(f"DLSS library found: {dlss_path}")
                    return dlss_path
        debug_log("DLSS library (nvngx_dlss.dll) not found")
        return None

    def create_setup_script(self, game_path, selected_filename='dxgi.dll'):
        """
        Create an OptiScaler setup script similar to the official setup_windows.bat
        """
        script_path = os.path.join(game_path, 'Setup OptiScaler.bat')
        try:
            with open(script_path, 'w', encoding='utf-8') as f:
                f.write('@echo off\n')
                f.write('cls\n')
                f.write('echo OptiScaler Setup Script\n')
                f.write('echo ========================\n')
                f.write('echo.\n')
                f.write(f'echo Current OptiScaler filename: {selected_filename}\n')
                f.write('echo.\n')
                f.write('echo This script can be used to reconfigure OptiScaler\n')
                f.write('pause\n')
            debug_log(f"Setup script created: {script_path}")
            return True
        except Exception as e:
            debug_log(f"Failed to create setup script: {e}")
            return False

    def get_dynamic_setup_options(self, extracted_path):
        """
        Returns a dict with all dynamic options for the setup GUI:
        - gpu_type: detected GPU type
        - available_filenames: possible OptiScaler target filenames
        - dlss_present: True/False
        - dlss_path: path to nvngx_dlss.dll if present
        """
        gpu_type = self.detect_gpu_type()
        available_filenames = self.list_available_target_filenames(extracted_path)
        dlss_path = self.check_dlss_availability(extracted_path)
        return {
            'gpu_type': gpu_type,
            'available_filenames': available_filenames,
            'dlss_present': dlss_path is not None,
            'dlss_path': dlss_path
        }

    def run_dynamic_setup(self, extracted_path, game_path, selected_filename, gpu_type, use_dlss, dlss_source_path=None, overwrite=False):
        """
        Automates the OptiScaler setup steps dynamically with enhanced error handling
        
        Args:
            extracted_path: path to extracted OptiScaler files
            game_path: game folder
            selected_filename: DLL/ASI filename to use (e.g. dxgi.dll)
            gpu_type: 'nvidia', 'amd', 'intel', or 'unknown'
            use_dlss: True/False (whether to copy nvngx_dlss.dll)
            dlss_source_path: path to nvngx_dlss.dll (if needed)
            overwrite: whether to overwrite existing target file
            
        Returns:
            tuple: (success: bool, message: str)
        """
        try:
            extracted_path = Path(extracted_path)
            game_path = Path(game_path)
            
            debug_log(f"Running dynamic setup for {game_path} with filename {selected_filename}")
            
            # 1. Remove marker file if present
            marker = extracted_path / '!! EXTRACT ALL FILES TO GAME FOLDER !!'
            if marker.exists():
                marker.unlink()
                debug_log("Removed extraction marker file")

            # 2. Check for OptiScaler.dll
            optiscaler_dll = extracted_path / 'OptiScaler.dll'
            if not optiscaler_dll.exists():
                return False, 'OptiScaler.dll not found in extracted files.'

            # 3. Prepare destination
            dest_file = game_path / selected_filename
            if dest_file.exists():
                if overwrite:
                    dest_file.unlink()
                    debug_log(f"Removed existing {selected_filename}")
                else:
                    return False, f"{selected_filename} already exists in game folder."

            # 4. Copy OptiScaler.dll to selected filename
            shutil.copy2(optiscaler_dll, dest_file)
            debug_log(f"Copied OptiScaler.dll to {selected_filename}")

            # 5. DLSS handling (for AMD/Intel and if requested)
            nvngx_target = game_path / 'nvngx.dll'
            if use_dlss and gpu_type in ('amd', 'intel'):
                if not dlss_source_path or not Path(dlss_source_path).exists():
                    return False, 'nvngx_dlss.dll not found for DLSS spoofing.'
                
                temp_copy = game_path / 'nvngx_dlss_copy.dll'
                shutil.copy2(dlss_source_path, temp_copy)
                
                if nvngx_target.exists():
                    nvngx_target.unlink()
                    
                temp_copy.rename(nvngx_target)
                debug_log("DLSS spoofing configured")

            # 6. Create enhanced uninstaller
            self._create_uninstaller_batch(game_path, selected_filename, use_dlss, gpu_type)

            return True, 'OptiScaler setup completed successfully.'
            
        except Exception as e:
            debug_log(f"Error during dynamic setup: {e}")
            return False, f'Error during setup: {e}'

    def _create_uninstaller_batch(self, game_path, selected_filename, use_dlss, gpu_type):
        """
        Create an enhanced uninstaller batch file
        
        Args:
            game_path: Game directory path
            selected_filename: Proxy DLL filename used
            use_dlss: Whether DLSS spoofing was used
            gpu_type: GPU type for DLSS handling
        """
        uninstaller_path = game_path / 'Remove OptiScaler.bat'
        
        try:
            with open(uninstaller_path, 'w', encoding='utf-8') as f:
                f.write('@echo off\n')
                f.write('cls\n')
                f.write('echo Removing OptiScaler...\n')
                f.write('echo.\n')
                
                # Remove DLSS spoof if used
                if use_dlss and gpu_type in ('amd', 'intel'):
                    f.write('if exist nvngx.dll (\n')
                    f.write('    del nvngx.dll\n')
                    f.write('    echo Removed DLSS spoofing file\n')
                    f.write(')\n')
                
                # Remove OptiScaler files
                optiscaler_files = [
                    'OptiScaler.log',
                    'OptiScaler.ini',
                    selected_filename
                ]
                
                for filename in optiscaler_files:
                    f.write(f'if exist "{filename}" (\n')
                    f.write(f'    del "{filename}"\n')
                    f.write(f'    echo Removed {filename}\n')
                    f.write(')\n')
                
                # Remove directories
                directories = [
                    'D3D12_Optiscaler',
                    'DlssOverrides',
                    'Licenses'
                ]
                
                for dirname in directories:
                    f.write(f'if exist "{dirname}" (\n')
                    f.write(f'    rd /s /q "{dirname}"\n')
                    f.write(f'    echo Removed {dirname} directory\n')
                    f.write(')\n')
                
                f.write('echo.\n')
                f.write('echo OptiScaler removed successfully!\n')
                f.write('pause\n')
                f.write('del %0\n')  # Self-delete
                
            debug_log(f"Created uninstaller batch file: {uninstaller_path}")
            
        except Exception as e:
            debug_log(f"Failed to create uninstaller: {e}")

    def uninstall_optiscaler(self, game_path):
        """
        Uninstall OptiScaler from a game directory with improved error handling
        
        Args:
            game_path: Path to game directory
            
        Returns:
            tuple: (success: bool, message: str)
        """
        game_path = Path(game_path)
        debug_log(f"Starting OptiScaler uninstall from: {game_path}")
        
        try:
            # Determine installation directory
            install_dir = self._determine_install_directory(game_path)
            debug_log(f"Uninstalling from: {install_dir}")

            removed_files = []
            removed_dirs = []
            
            # List of OptiScaler files to remove (including proxy DLLs)
            optiscaler_files = [
                "OptiScaler.dll",
                "OptiScaler.ini", 
                "OptiScaler.log",
                "nvngx.dll",  # DLSS override file
                "Remove OptiScaler.bat",
                "Setup OptiScaler.bat",
            ]
            
            # Add additional files and proxy filenames
            optiscaler_files.extend(OptiScalerConfig.ADDITIONAL_FILES)
            optiscaler_files.extend(OptiScalerConfig.PROXY_FILENAMES)
            
            # Remove duplicate entries
            optiscaler_files = list(set(optiscaler_files))
            
            # Remove individual files
            for filename in optiscaler_files:
                file_path = install_dir / filename
                if file_path.exists():
                    try:
                        file_path.unlink()
                        removed_files.append(filename)
                        debug_log(f"Removed file: {filename}")
                    except Exception as e:
                        debug_log(f"Failed to remove {filename}: {e}")
            
            # List of OptiScaler directories to remove
            optiscaler_dirs = [
                "D3D12_Optiscaler",
                "DlssOverrides", 
                "Licenses"
            ]
            
            # Remove directories
            for dirname in optiscaler_dirs:
                dir_path = install_dir / dirname
                if dir_path.exists():
                    try:
                        shutil.rmtree(dir_path)
                        removed_dirs.append(dirname)
                        debug_log(f"Removed directory: {dirname}")
                    except Exception as e:
                        debug_log(f"Failed to remove directory {dirname}: {e}")
            
            # Create summary
            if removed_files or removed_dirs:
                summary = f"Successfully removed OptiScaler from {game_path}\n"
                if removed_files:
                    summary += f"Files removed: {', '.join(removed_files[:5])}" # Limit display
                    if len(removed_files) > 5:
                        summary += f" and {len(removed_files) - 5} more files"
                    summary += "\n"
                if removed_dirs:
                    summary += f"Directories removed: {', '.join(removed_dirs)}"
                debug_log("Uninstall completed successfully")
                return True, summary
            else:
                return False, "No OptiScaler files found to remove"
                
        except Exception as e:
            debug_log(f"Error during uninstall: {e}")
            return False, f"Error during uninstall: {e}"

    def is_optiscaler_installed(self, game_path):
        """
        Check if OptiScaler is installed in the given game directory.
        Enhanced with better detection logic and performance optimization.
        
        Args:
            game_path: Path to game directory
            
        Returns:
            bool: True if OptiScaler is detected, False otherwise
        """
        game_path = Path(game_path)
        
        # Determine check directory
        check_dir = self._determine_install_directory(game_path)
        
        debug_log(f"Checking for OptiScaler installation in: {check_dir}")
        
        # Primary check: OptiScaler.ini (most reliable indicator)
        ini_file = check_dir / "OptiScaler.ini"
        if ini_file.exists():
            debug_log(f"OptiScaler detected via OptiScaler.ini: {ini_file}")
            return True
        
        # Secondary check: Look for proxy DLLs with OptiScaler indicators
        for proxy_filename in OptiScalerConfig.PROXY_FILENAMES:
            proxy_path = check_dir / proxy_filename
            if proxy_path.exists():
                # Check for OptiScaler indicator files
                for indicator in OptiScalerConfig.INSTALL_INDICATORS:
                    indicator_path = check_dir / indicator
                    if indicator_path.exists():
                        debug_log(f"OptiScaler detected via {proxy_filename} + {indicator}")
                        return True
        
        debug_log(f"OptiScaler not detected in: {check_dir}")
        return False
