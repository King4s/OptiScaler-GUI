import requests
import zipfile
import os
import shutil
import configparser
import re
import threading
import subprocess
from utils.debug import debug_log

class OptiScalerManager:
    """
    OptiScaler Manager for GUI
    
    This class handles downloading, installing, and managing OptiScaler.
    OptiScaler is the official project by the OptiScaler team: https://github.com/optiscaler/OptiScaler
    
    This manager automatically downloads the latest releases from the official repository
    and provides GUI-friendly installation methods.
    """
    def __init__(self):
        self.github_release_url = "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest"
        self.download_dir = "C:\\OptiScaler-GUI\\cache\\optiscaler_downloads"
        os.makedirs(self.download_dir, exist_ok=True)

    def _download_latest_release(self, progress_callback=None):
        try:
            if progress_callback:
                progress_callback("Fetching release information...")
                
            response = requests.get(self.github_release_url)
            response.raise_for_status()
            release_info = response.json()
            assets = release_info.get("assets", [])

            archive_asset = None
            for asset in assets:
                if asset["name"].endswith(".7z"):
                    archive_asset = asset
                    break

            if not archive_asset:
                debug_log("No .7z asset found in release assets.")
                return None

            download_url = archive_asset["browser_download_url"]
            archive_filename = os.path.join(self.download_dir, archive_asset["name"])

            # Check if file exists and is valid (try to list contents)
            if os.path.exists(archive_filename):
                try:
                    if progress_callback:
                        progress_callback("Validating existing archive...")
                    result = subprocess.run(
                        [r'C:\Program Files\7-Zip\7z.exe', 't', archive_filename],
                        check=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE
                    )
                    debug_log(f"Archive {archive_filename} exists and is valid, skipping download.")
                    return archive_filename
                except Exception as e:
                    debug_log(f"Existing archive is invalid or corrupt, will re-download: {e}")
                    os.remove(archive_filename)

            # Download if not present or invalid
            if progress_callback:
                progress_callback(f"Downloading {archive_asset['name']}...")
                
            with requests.get(download_url, stream=True) as r:
                r.raise_for_status()
                total_size = int(r.headers.get('content-length', 0))
                downloaded = 0
                
                with open(archive_filename, 'wb') as f:
                    for chunk in r.iter_content(chunk_size=8192):
                        f.write(chunk)
                        downloaded += len(chunk)
                        
                        # Update progress if we have total size
                        if total_size > 0 and progress_callback:
                            progress_percent = (downloaded / total_size) * 100
                            progress_callback(f"Downloading {archive_asset['name']} ({progress_percent:.1f}%)...")
                            
            return archive_filename
        except Exception as e:
            debug_log(f"Exception in _download_latest_release: {e}")
            return None

    def _extract_release(self, archive_path, game_path=None, progress_callback=None):
        if progress_callback:
            progress_callback("Preparing extraction...")
            
        extract_path = os.path.join(self.download_dir, "extracted_optiscaler")
        if os.path.exists(extract_path):
            shutil.rmtree(extract_path)
        os.makedirs(extract_path)

        if archive_path.endswith('.7z'):
            # Always use the full path to 7z.exe
            try:
                if progress_callback:
                    progress_callback("Extracting archive...")
                    
                result = subprocess.run(
                    [r'C:\Program Files\7-Zip\7z.exe', 'x', archive_path, f'-o{extract_path}', '-y'],
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE
                )
                
                if progress_callback:
                    progress_callback("Extraction completed")
                    
            except Exception as e:
                print(f"ERROR: Failed to extract .7z archive with 7z.exe: {e}")
                return None
        else:
            print("ERROR: Only .7z extraction is supported in this version.")
            return None
        return extract_path

    def install_optiscaler(self, game_path, target_filename='dxgi.dll', overwrite=False):
        """
        Enhanced OptiScaler installation based on official source code setup logic.
        """
        debug_log(f"Starting OptiScaler installation: {game_path}")
        debug_log(f"Target filename: {target_filename}")
        
        # Download latest release
        zip_path = self._download_latest_release()
        debug_log(f"Downloaded archive: {zip_path}")
        if not zip_path:
            debug_log("Download failed")
            return False

        # Extract files
        extracted_path = self._extract_release(zip_path, game_path=game_path)
        debug_log(f"Extracted to: {extracted_path}")
        if not extracted_path:
            debug_log("Extraction failed")
            return False

        # Remove setup marker file (like official setup does)
        marker_file = os.path.join(extracted_path, "!! EXTRACT ALL FILES TO GAME FOLDER !!")
        if os.path.exists(marker_file):
            os.remove(marker_file)
            debug_log("Removed setup marker file")

        # Detect installation directory (Unreal Engine vs regular game)
        unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
        if os.path.isdir(unreal_dir):
            dest_dir = unreal_dir
            debug_log(f"Detected Unreal Engine, installing to: {dest_dir}")
        else:
            dest_dir = game_path
            debug_log(f"Installing to game root: {dest_dir}")

        os.makedirs(dest_dir, exist_ok=True)

        # Check if target file already exists
        target_path = os.path.join(dest_dir, target_filename)
        if os.path.exists(target_path):
            if not overwrite:
                debug_log(f"Target file {target_filename} already exists and overwrite=False")
                return False
            else:
                os.remove(target_path)
                debug_log(f"Removed existing {target_filename}")

        # Find OptiScaler.dll in extracted files
        optiscaler_dll_path = None
        for root, _, files in os.walk(extracted_path):
            for file in files:
                if file.lower() == 'optiscaler.dll':
                    optiscaler_dll_path = os.path.join(root, file)
                    break
            if optiscaler_dll_path:
                break

        if not optiscaler_dll_path:
            debug_log("OptiScaler.dll not found in extracted files")
            return False

        success = True
        copied_files = []

        try:
            # Copy OptiScaler.dll as the target filename (main proxy DLL)
            shutil.copy2(optiscaler_dll_path, target_path)
            copied_files.append(target_filename)
            debug_log(f"Copied OptiScaler.dll as {target_filename}")

            # Copy additional OptiScaler files
            for root, _, files in os.walk(extracted_path):
                for file in files:
                    if file in self.get_additional_optiscaler_files():
                        src_path = os.path.join(root, file)
                        dest_file_path = os.path.join(dest_dir, file)
                        try:
                            shutil.copy2(src_path, dest_file_path)
                            copied_files.append(file)
                            debug_log(f"Copied additional file: {file}")
                        except Exception as e:
                            debug_log(f"Failed to copy {file}: {e}")
                            success = False

            # Create uninstaller script (like official setup)
            self.create_uninstaller_script(dest_dir, target_filename, copied_files)
            
            # Create default configuration if OptiScaler.ini wasn't copied
            config_path = os.path.join(dest_dir, 'OptiScaler.ini')
            if not os.path.exists(config_path):
                gpu_type = self.detect_gpu_type()
                self.create_default_config(dest_dir, gpu_type)
                debug_log("Created default OptiScaler configuration")

        except Exception as e:
            debug_log(f"Installation failed: {e}")
            success = False

        debug_log(f"Installation completed successfully: {success}")
        return success

    def get_additional_optiscaler_files(self):
        """
        Get list of additional OptiScaler files to copy (excluding main DLL).
        Based on official OptiScaler file structure.
        """
        return [
            "OptiScaler.ini",
            "amd_fidelityfx_dx12.dll",
            "amd_fidelityfx_vk.dll", 
            "libxess_dx11.dll",
            "libxess.dll",
            "nvngx_dlss.dll",  # DLSS library if present
        ]

    def create_uninstaller_script(self, install_dir, main_filename, copied_files):
        """
        Create an uninstaller script similar to official OptiScaler setup.
        Based on setup_windows.bat uninstaller creation logic.
        """
        uninstaller_path = os.path.join(install_dir, 'Remove OptiScaler.bat')
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
        Uses GPU detection to set appropriate defaults.
        """
        config_path = os.path.join(install_dir, 'OptiScaler.ini')
        
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
        settings = {}
        current_section = None
        current_comments = []

        with open(ini_path, 'r') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue

                if line.startswith('['):
                    current_section = line[1:-1]
                    settings[current_section] = {}
                    current_comments = []
                elif line.startswith(';'):
                    current_comments.append(line[1:].strip())
                elif '=' in line:
                    key, value = line.split('=', 1)
                    key = key.strip()
                    # Remove inline comments from value
                    value_without_inline_comment = value.split(';', 1)[0].strip()
                    
                    inferred_type, options = self._infer_type(value_without_inline_comment, "\n".join(current_comments))

                    if current_section:
                        settings[current_section][key] = {
                            "value": value_without_inline_comment,
                            "comment": "\n".join(current_comments),
                            "type": inferred_type,
                            "options": options
                        }
                    current_comments = []
        return settings

    def write_optiscaler_ini(self, ini_path, settings):
        with open(ini_path, 'w') as f:
            for section, keys in settings.items():
                f.write(f"[{section}]\n")
                for key, data in keys.items():
                    if data["comment"]:
                        for comment_line in data["comment"].split('\n'):
                            f.write(f"; {comment_line}\n")
                    f.write(f"{key}={data['value']}\n")
                f.write("\n") # Add a newline after each section for readability

    def install_optiscaler_threaded(self, game_path, status_callback=None, done_callback=None):
        def worker():
            if status_callback:
                status_callback("Starting OptiScaler installation...")
                
            zip_path = self._download_latest_release(progress_callback=status_callback)
            if not zip_path:
                if status_callback:
                    status_callback("Download failed.")
                if done_callback:
                    done_callback(False)
                return
                
            extracted_path = self._extract_release(zip_path, progress_callback=status_callback)
            if not extracted_path:
                if status_callback:
                    status_callback("Extraction failed.")
                if done_callback:
                    done_callback(False)
                return
                
            if status_callback:
                status_callback("Installing files...")
                
            result = self.install_optiscaler(game_path, overwrite=True)
            if done_callback:
                done_callback(result)
            if status_callback:
                status_callback("Installation completed!" if result else "Installation failed.")
        t = threading.Thread(target=worker)
        t.start()

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
        Enhanced GPU detection based on OptiScaler's source code methods.
        Detects GPU type using system files and WMI queries.
        Returns: 'nvidia', 'amd', 'intel', or 'unknown'
        """
        # Check for NVIDIA using nvapi64.dll in system32
        nvapi_path = os.path.join(os.environ.get('WINDIR', 'C:\\Windows'), 'System32', 'nvapi64.dll')
        if os.path.exists(nvapi_path):
            debug_log("NVIDIA GPU detected via nvapi64.dll")
            return 'nvidia'
        
        # Use WMI to query video controllers (similar to OptiScaler's wmic approach)
        try:
            import subprocess
            result = subprocess.run(
                ['wmic', 'path', 'win32_VideoController', 'get', 'name'], 
                capture_output=True, text=True, timeout=10
            )
            gpu_names = result.stdout.lower()
            
            if 'nvidia' in gpu_names:
                debug_log("NVIDIA GPU detected via WMI")
                return 'nvidia'
            elif 'amd' in gpu_names or 'radeon' in gpu_names:
                debug_log("AMD GPU detected via WMI")
                return 'amd'
            elif 'intel' in gpu_names:
                debug_log("Intel GPU detected via WMI") 
                return 'intel'
        except Exception as e:
            debug_log(f"GPU detection via WMI failed: {e}")
        
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
        Automates the OptiScaler setup steps dynamically, based on user/system choices.
        - extracted_path: path to extracted OptiScaler files
        - game_path: game folder
        - selected_filename: DLL/ASI filename to use (e.g. dxgi.dll)
        - gpu_type: 'nvidia', 'amd', 'intel', or 'unknown'
        - use_dlss: True/False (whether to copy nvngx_dlss.dll)
        - dlss_source_path: path to nvngx_dlss.dll (if needed)
        - overwrite: whether to overwrite existing target file
        Returns: (success, message)
        """
        try:
            # 1. Remove marker file if present
            marker = os.path.join(extracted_path, '!! EXTRACT ALL FILES TO GAME FOLDER !!')
            if os.path.exists(marker):
                os.remove(marker)

            # 2. Check for OptiScaler.dll
            optiscaler_dll = os.path.join(extracted_path, 'OptiScaler.dll')
            if not os.path.exists(optiscaler_dll):
                return False, 'OptiScaler.dll not found in extracted files.'

            # 3. Prepare destination
            dest_file = os.path.join(game_path, selected_filename)
            if os.path.exists(dest_file):
                if overwrite:
                    os.remove(dest_file)
                else:
                    return False, f"{selected_filename} already exists in game folder."

            # 4. Rename/copy OptiScaler.dll to selected filename
            shutil.copy2(optiscaler_dll, dest_file)

            # 5. DLSS handling (for AMD/Intel and if requested)
            nvngx_target = os.path.join(game_path, 'nvngx.dll')
            if use_dlss and gpu_type in ('amd', 'intel'):
                if not dlss_source_path or not os.path.exists(dlss_source_path):
                    return False, 'nvngx_dlss.dll not found for DLSS spoofing.'
                temp_copy = os.path.join(game_path, 'nvngx_dlss_copy.dll')
                shutil.copy2(dlss_source_path, temp_copy)
                if os.path.exists(nvngx_target):
                    os.remove(nvngx_target)
                os.rename(temp_copy, nvngx_target)

            # 6. Create uninstaller
            uninstaller_path = os.path.join(game_path, 'Remove OptiScaler.bat')
            with open(uninstaller_path, 'w', encoding='utf-8') as f:
                f.write('@echo off\n')
                f.write('cls\n')
                f.write('echo Removing OptiScaler...\n')
                if use_dlss and gpu_type in ('amd', 'intel'):
                    f.write('del nvngx.dll\n')
                f.write('del OptiScaler.log\n')
                f.write('del OptiScaler.ini\n')
                f.write(f'del {selected_filename}\n')
                f.write('del /Q D3D12_Optiscaler\\*\n')
                f.write('rd D3D12_Optiscaler\n')
                f.write('del /Q DlssOverrides\\*\n')
                f.write('rd DlssOverrides\n')
                f.write('del /Q Licenses\\*\n')
                f.write('rd Licenses\n')
                f.write('echo.\n')
                f.write('echo OptiScaler removed!\n')
                f.write('pause\n')
                f.write('del %0\n')

            return True, 'OptiScaler setup completed successfully.'
        except Exception as e:
            return False, f'Error during setup: {e}'

    def uninstall_optiscaler(self, game_path):
        """
        Uninstall OptiScaler from a game directory
        """
        debug_log(f"Starting OptiScaler uninstall from: {game_path}")
        
        try:
            # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
            unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
            if os.path.isdir(unreal_dir):
                install_dir = unreal_dir
                debug_log(f"Detected Unreal Engine, removing from: {install_dir}")
            else:
                install_dir = game_path
                debug_log(f"Using game root directory: {install_dir}")

            removed_files = []
            removed_dirs = []
            
            # List of OptiScaler files to remove
            optiscaler_files = [
                "OptiScaler.dll",
                "OptiScaler.ini", 
                "OptiScaler.log",
                "nvngx.dll",  # DLSS override file
                "amd_fidelityfx_dx12.dll",
                "amd_fidelityfx_vk.dll", 
                "libxess_dx11.dll",
                "libxess.dll",
                "Remove OptiScaler.bat"
            ]
            
            # Remove individual files
            for filename in optiscaler_files:
                file_path = os.path.join(install_dir, filename)
                if os.path.exists(file_path):
                    try:
                        os.remove(file_path)
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
                dir_path = os.path.join(install_dir, dirname)
                if os.path.exists(dir_path):
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
                    summary += f"Files removed: {', '.join(removed_files)}\n"
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
        Enhanced to check for any OptiScaler proxy DLL, not just OptiScaler.dll
        """
        # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
        unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
        if os.path.isdir(unreal_dir):
            check_dir = unreal_dir
        else:
            check_dir = game_path
            
        # Check for OptiScaler.ini first (most reliable indicator)
        ini_file = os.path.join(check_dir, "OptiScaler.ini")
        if os.path.exists(ini_file):
            debug_log(f"OptiScaler detected via OptiScaler.ini: {ini_file}")
            return True
            
        # Check for any of the possible proxy DLL names that could be OptiScaler
        possible_proxy_files = self.list_available_target_filenames()
        
        for proxy_filename in possible_proxy_files:
            proxy_path = os.path.join(check_dir, proxy_filename)
            if os.path.exists(proxy_path):
                # Additional check: see if there are other OptiScaler files nearby
                optiscaler_indicators = [
                    "OptiScaler.log",
                    "amd_fidelityfx_dx12.dll",
                    "amd_fidelityfx_vk.dll",
                    "libxess.dll",
                    "libxess_dx11.dll",
                    "Remove OptiScaler.bat"
                ]
                
                for indicator in optiscaler_indicators:
                    if os.path.exists(os.path.join(check_dir, indicator)):
                        debug_log(f"OptiScaler detected via {proxy_filename} + {indicator}")
                        return True
        
        debug_log(f"OptiScaler not detected in: {check_dir}")
        return False
