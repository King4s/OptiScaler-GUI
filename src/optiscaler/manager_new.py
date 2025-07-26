import requests
import zipfile
import os
import shutil
import configparser
import re
import threading
import subprocess
import time
from urllib.parse import urlparse
from utils.debug import debug_log

class OptiScalerManager:
    def __init__(self):
        self.github_release_url = "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest"
        self.download_dir = "C:\\OptiScaler-GUI\\cache\\optiscaler_downloads"
        self.request_timeout = 30  # seconds
        self.max_retries = 3
        self.retry_delay = 2  # seconds
        os.makedirs(self.download_dir, exist_ok=True)

    def _make_github_request(self, url, stream=False):
        """
        Make a robust HTTP request to GitHub with retry logic and proper error handling
        """
        headers = {
            'User-Agent': 'OptiScaler-GUI/1.0.0',
            'Accept': 'application/vnd.github.v3+json'
        }
        
        for attempt in range(self.max_retries):
            try:
                debug_log(f"GitHub request attempt {attempt + 1}/{self.max_retries}: {url}")
                
                response = requests.get(
                    url, 
                    headers=headers, 
                    timeout=self.request_timeout, 
                    stream=stream
                )
                
                # Handle rate limiting
                if response.status_code == 429:
                    reset_time = response.headers.get('X-RateLimit-Reset')
                    if reset_time:
                        wait_time = int(reset_time) - int(time.time())
                        debug_log(f"GitHub rate limit hit, waiting {wait_time} seconds")
                        if wait_time > 0 and wait_time < 300:  # Don't wait more than 5 minutes
                            time.sleep(wait_time)
                            continue
                
                response.raise_for_status()
                return response
                
            except requests.exceptions.Timeout:
                debug_log(f"GitHub request timeout on attempt {attempt + 1}")
            except requests.exceptions.ConnectionError:
                debug_log(f"GitHub connection error on attempt {attempt + 1}")
            except requests.exceptions.HTTPError as e:
                debug_log(f"GitHub HTTP error {e.response.status_code}: {e}")
                if e.response.status_code == 404:
                    debug_log("GitHub repository or release not found")
                    return None
                elif e.response.status_code in [500, 502, 503, 504]:
                    debug_log("GitHub server error, retrying...")
                else:
                    debug_log("Non-retryable HTTP error")
                    return None
            except Exception as e:
                debug_log(f"Unexpected error during GitHub request: {e}")
            
            if attempt < self.max_retries - 1:
                wait_time = self.retry_delay * (2 ** attempt)  # Exponential backoff
                debug_log(f"Retrying in {wait_time} seconds...")
                time.sleep(wait_time)
        
        debug_log("All GitHub request attempts failed")
        return None

    def get_release_info(self):
        """
        Get detailed information about the latest OptiScaler release
        Returns: dict with release information or None if failed
        """
        debug_log("Fetching latest release information from GitHub")
        
        response = self._make_github_request(self.github_release_url)
        if not response:
            return None
            
        try:
            release_info = response.json()
            
            # Validate required fields
            required_fields = ['tag_name', 'name', 'published_at', 'assets']
            for field in required_fields:
                if field not in release_info:
                    debug_log(f"Missing required field '{field}' in release info")
                    return None
            
            # Parse and validate assets
            assets = release_info.get("assets", [])
            valid_assets = []
            
            for asset in assets:
                if 'name' in asset and 'browser_download_url' in asset and 'size' in asset:
                    valid_assets.append({
                        'name': asset['name'],
                        'download_url': asset['browser_download_url'],
                        'size': asset['size'],
                        'content_type': asset.get('content_type', 'unknown')
                    })
            
            return {
                'tag_name': release_info['tag_name'],
                'name': release_info['name'],
                'published_at': release_info['published_at'],
                'body': release_info.get('body', ''),
                'prerelease': release_info.get('prerelease', False),
                'assets': valid_assets,
                'html_url': release_info.get('html_url', '')
            }
            
        except (ValueError, KeyError) as e:
            debug_log(f"Error parsing GitHub release info: {e}")
            return None

    def check_for_updates(self, current_version=None):
        """
        Check if a newer version of OptiScaler is available
        Returns: dict with update info or None if no update/error
        """
        debug_log("Checking for OptiScaler updates")
        
        release_info = self.get_release_info()
        if not release_info:
            debug_log("Could not fetch release information for update check")
            return None
        
        latest_version = release_info['tag_name']
        
        # If no current version provided, always suggest update
        if not current_version:
            debug_log(f"No current version specified, suggesting update to {latest_version}")
            return {
                'update_available': True,
                'latest_version': latest_version,
                'current_version': 'unknown',
                'release_name': release_info['name'],
                'release_notes': release_info['body'],
                'published_at': release_info['published_at'],
                'prerelease': release_info['prerelease'],
                'html_url': release_info['html_url']
            }
        
        # Compare versions
        if self._compare_versions(current_version, latest_version):
            debug_log(f"Update available: {current_version} -> {latest_version}")
            return {
                'update_available': True,
                'latest_version': latest_version,
                'current_version': current_version,
                'release_name': release_info['name'],
                'release_notes': release_info['body'],
                'published_at': release_info['published_at'],
                'prerelease': release_info['prerelease'],
                'html_url': release_info['html_url']
            }
        else:
            debug_log(f"No update needed: current {current_version} >= latest {latest_version}")
            return {
                'update_available': False,
                'latest_version': latest_version,
                'current_version': current_version
            }

    def _compare_versions(self, current, latest):
        """
        Compare version strings to determine if an update is available
        Returns: True if latest > current, False otherwise
        """
        try:
            # Remove 'v' prefix if present
            current = current.lstrip('v')
            latest = latest.lstrip('v')
            
            # Split version parts
            current_parts = [int(x) for x in current.split('.') if x.isdigit()]
            latest_parts = [int(x) for x in latest.split('.') if x.isdigit()]
            
            # Pad shorter version with zeros
            max_len = max(len(current_parts), len(latest_parts))
            current_parts.extend([0] * (max_len - len(current_parts)))
            latest_parts.extend([0] * (max_len - len(latest_parts)))
            
            # Compare parts
            for curr, lat in zip(current_parts, latest_parts):
                if lat > curr:
                    return True
                elif lat < curr:
                    return False
            
            return False
            
        except Exception as e:
            debug_log(f"Error comparing versions {current} vs {latest}: {e}")
            # If comparison fails, suggest update to be safe
            return True

    def test_github_connection(self):
        """
        Test GitHub API connectivity and return status information
        """
        debug_log("Testing GitHub API connection...")
        
        try:
            response = self._make_github_request("https://api.github.com/rate_limit")
            if response:
                rate_limit_info = response.json()
                debug_log("GitHub API connection successful")
                return {
                    'connected': True,
                    'rate_limit': rate_limit_info['rate']['limit'],
                    'remaining': rate_limit_info['rate']['remaining'],
                    'reset_time': rate_limit_info['rate']['reset']
                }
            else:
                debug_log("Failed to connect to GitHub API")
                return {'connected': False, 'error': 'Connection failed'}
                
        except Exception as e:
            debug_log(f"GitHub connection test error: {e}")
            return {'connected': False, 'error': str(e)}

    def _download_latest_release(self):
        """
        Download the latest OptiScaler release with enhanced GitHub handling
        Returns: path to downloaded archive or None if failed
        """
        try:
            # Get release information
            release_info = self.get_release_info()
            if not release_info:
                debug_log("Failed to get release information from GitHub")
                return None
            
            debug_log(f"Found release: {release_info['name']} ({release_info['tag_name']})")
            
            # Find suitable archive (.7z preferred)
            archive_asset = None
            for asset in release_info['assets']:
                if asset['name'].lower().endswith('.7z'):
                    archive_asset = asset
                    break
            
            if not archive_asset:
                debug_log("No .7z asset found in release assets")
                return None
            
            download_url = archive_asset['download_url']
            expected_size = archive_asset['size']
            archive_filename = os.path.join(self.download_dir, archive_asset['name'])
            
            debug_log(f"Target file: {archive_filename}")
            
            # Check if file exists and is valid
            if os.path.exists(archive_filename):
                try:
                    actual_size = os.path.getsize(archive_filename)
                    if actual_size == expected_size:
                        # Verify archive integrity
                        result = subprocess.run(
                            [r'C:\Program Files\7-Zip\7z.exe', 't', archive_filename],
                            check=True,
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            timeout=60
                        )
                        debug_log(f"Archive {archive_filename} exists and is valid, skipping download")
                        return archive_filename
                    else:
                        debug_log(f"Size mismatch: expected {expected_size}, got {actual_size}")
                    
                    os.remove(archive_filename)
                except Exception as e:
                    debug_log(f"Error checking existing archive: {e}")
                    try:
                        os.remove(archive_filename)
                    except:
                        pass
            
            # Download the archive
            debug_log(f"Downloading {archive_asset['name']} ({expected_size} bytes)")
            
            response = self._make_github_request(download_url, stream=True)
            if not response:
                debug_log("Failed to start download")
                return None
            
            # Download with progress tracking
            downloaded_size = 0
            with open(archive_filename, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    if chunk:
                        f.write(chunk)
                        downloaded_size += len(chunk)
                        
                        # Progress logging every 10MB
                        if downloaded_size % (10 * 1024 * 1024) == 0:
                            progress = (downloaded_size / expected_size) * 100 if expected_size > 0 else 0
                            debug_log(f"Download progress: {progress:.1f}% ({downloaded_size}/{expected_size} bytes)")
            
            # Verify download
            final_size = os.path.getsize(archive_filename)
            if final_size != expected_size:
                debug_log(f"Download size mismatch: expected {expected_size}, got {final_size}")
                os.remove(archive_filename)
                return None
            
            debug_log(f"Successfully downloaded and verified: {archive_filename}")
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

    # Include all the other original methods from the manager...
    # (This is a simplified version focusing on the GitHub enhancements)
    
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
