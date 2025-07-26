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
    def __init__(self):
        self.github_release_url = "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest"
        self.download_dir = "C:\\OptiScaler-GUI\\cache\\optiscaler_downloads"
        os.makedirs(self.download_dir, exist_ok=True)

    def _download_latest_release(self):
        try:
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
            with requests.get(download_url, stream=True) as r:
                r.raise_for_status()
                with open(archive_filename, 'wb') as f:
                    for chunk in r.iter_content(chunk_size=8192):
                        f.write(chunk)
            return archive_filename
        except Exception as e:
            debug_log(f"Exception in _download_latest_release: {e}")
            return None

    def _extract_release(self, archive_path, game_path=None):
        extract_path = os.path.join(self.download_dir, "extracted_optiscaler")
        if os.path.exists(extract_path):
            shutil.rmtree(extract_path)
        os.makedirs(extract_path)

        if archive_path.endswith('.7z'):
            # Always use the full path to 7z.exe
            try:
                result = subprocess.run(
                    [r'C:\Program Files\7-Zip\7z.exe', 'x', archive_path, f'-o{extract_path}', '-y'],
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE
                )
            except Exception as e:
                print(f"ERROR: Failed to extract .7z archive with 7z.exe: {e}")
                return None
        else:
            print("ERROR: Only .7z extraction is supported in this version.")
            return None
        return extract_path

    def install_optiscaler(self, game_path):
        debug_log("install_optiscaler start", game_path)
        zip_path = self._download_latest_release()
        debug_log("zip_path", zip_path)
        if not zip_path:
            debug_log("Download failed")
            return False

        extracted_path = self._extract_release(zip_path, game_path=game_path)
        debug_log("extracted_path", extracted_path)
        if not extracted_path:
            debug_log("Extract failed")
            return False

        # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
        unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
        if os.path.isdir(unreal_dir):
            dest_dir = unreal_dir
            debug_log(f"Detected Unreal Engine, using {dest_dir} as destination.")
        else:
            dest_dir = game_path
            debug_log(f"Using game root as destination: {dest_dir}")

        os.makedirs(dest_dir, exist_ok=True)

        optiscaler_files = [
            "dxgi.dll",
            "OptiScaler.ini",
            # Add other files if necessary
        ]

        success = True
        for root, _, files in os.walk(extracted_path):
            for file in files:
                if file in optiscaler_files:
                    src_path = os.path.join(root, file)
                    dest_path = os.path.join(dest_dir, file)
                    try:
                        shutil.copy2(src_path, dest_path)
                    except Exception as e:
                        print(f"ERROR: Failed to copy {file}: {e}")
                        success = False

        debug_log("install_optiscaler success?", success)
        return success

    def _infer_type(self, value, comment):
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
                status_callback("Downloading OptiScaler...")
            zip_path = self._download_latest_release()
            if not zip_path:
                if status_callback:
                    status_callback("Download failed.")
                if done_callback:
                    done_callback(False)
                return
            if status_callback:
                status_callback("Extracting files...")
            extracted_path = self._extract_release(zip_path)
            if not extracted_path:
                if status_callback:
                    status_callback("Extraction failed.")
                if done_callback:
                    done_callback(False)
                return
            if status_callback:
                status_callback("Copying files...")
            result = self.install_optiscaler(game_path)
            if done_callback:
                done_callback(result)
            if status_callback:
                status_callback("Done!" if result else "Install failed.")
        t = threading.Thread(target=worker)
        t.start()

    def detect_gpu_type(self):
        """
        Detects the GPU type (Nvidia, AMD/Intel, Unknown) based on system files and wmic output.
        Returns: 'nvidia', 'amd', 'intel', or 'unknown'
        """
        if os.path.exists(os.path.join(os.environ.get('WINDIR', 'C:\\Windows'), 'System32', 'nvapi64.dll')):
            return 'nvidia'
        try:
            import subprocess
            result = subprocess.run(['wmic', 'path', 'win32_VideoController', 'get', 'name'], capture_output=True, text=True)
            gpu_names = result.stdout.lower()
            if 'nvidia' in gpu_names:
                return 'nvidia'
            elif 'amd' in gpu_names or 'radeon' in gpu_names:
                return 'amd'
            elif 'intel' in gpu_names:
                return 'intel'
        except Exception:
            pass
        return 'unknown'

    def list_available_target_filenames(self, extracted_path):
        """
        Dynamically list possible target filenames for OptiScaler (dll/asi) in the extracted folder.
        Returns a list of filenames (e.g., ['dxgi.dll', ...])
        """
        candidates = []
        for fname in os.listdir(extracted_path):
            if fname.lower().endswith(('.dll', '.asi')) and 'optiscaler' not in fname.lower():
                candidates.append(fname)
        defaults = [
            'dxgi.dll', 'winmm.dll', 'version.dll', 'dbghelp.dll',
            'd3d12.dll', 'wininet.dll', 'winhttp.dll', 'OptiScaler.asi'
        ]
        for d in defaults:
            if d not in candidates:
                candidates.append(d)
        return candidates

    def check_dlss_file(self, extracted_path):
        """
        Checks if nvngx_dlss.dll is present in the extracted folder or subfolders.
        Returns the path if found, else None.
        """
        for root, _, files in os.walk(extracted_path):
            for file in files:
                if file.lower() == 'nvngx_dlss.dll':
                    return os.path.join(root, file)
        return None

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
        dlss_path = self.check_dlss_file(extracted_path)
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
        Check if OptiScaler is installed in the given game directory
        """
        # Auto-detect Unreal Engine (Engine/Binaries/Win64 exists)
        unreal_dir = os.path.join(game_path, "Engine", "Binaries", "Win64")
        if os.path.isdir(unreal_dir):
            check_dir = unreal_dir
        else:
            check_dir = game_path
            
        # Check for key OptiScaler files
        key_files = ["OptiScaler.dll", "OptiScaler.ini"]
        
        for filename in key_files:
            if os.path.exists(os.path.join(check_dir, filename)):
                return True
        return False
