import requests
import zipfile
import os
import shutil
import configparser

class OptiScalerManager:
    def __init__(self):
        self.github_release_url = "https://api.github.com/repos/optiscaler/OptiScaler/releases/latest"
        self.download_dir = "C:\OptiScaler-GUI\cache\optiscaler_downloads"
        os.makedirs(self.download_dir, exist_ok=True)

    def _download_latest_release(self):
        try:
            response = requests.get(self.github_release_url)
            response.raise_for_status()
            release_info = response.json()
            assets = release_info.get("assets", [])
            
            zip_asset = None
            for asset in assets:
                if asset["name"].endswith(".zip"):
                    zip_asset = asset
                    break
            
            if not zip_asset:
                print("No zip asset found in the latest release.")
                return None

            download_url = zip_asset["browser_download_url"]
            zip_filename = os.path.join(self.download_dir, zip_asset["name"])

            print(f"Downloading OptiScaler from: {download_url}")
            with requests.get(download_url, stream=True) as r:
                r.raise_for_status()
                with open(zip_filename, 'wb') as f:
                    for chunk in r.iter_content(chunk_size=8192):
                        f.write(chunk)
            print(f"Downloaded to: {zip_filename}")
            return zip_filename
        except Exception as e:
            print(f"Error downloading OptiScaler release: {e}")
            return None

    def _extract_release(self, zip_path):
        extract_path = os.path.join(self.download_dir, "extracted_optiscaler")
        if os.path.exists(extract_path):
            shutil.rmtree(extract_path)
        os.makedirs(extract_path)

        print(f"Extracting {zip_path} to {extracted_path}")
        with zipfile.ZipFile(zip_path, 'r') as zip_ref:
            zip_ref.extractall(extract_path)
        print("Extraction complete.")
        return extract_path

    def install_optiscaler(self, game_path):
        print(f"Attempting to install OptiScaler to: {game_path}")
        zip_path = self._download_latest_release()
        if not zip_path:
            return False

        extracted_path = self._extract_release(zip_path)
        if not extracted_path:
            return False

        # Identify files to copy (dxgi.dll, OptiScaler.ini, etc.)
        # This might need to be more robust depending on OptiScaler's release structure
        optiscaler_files = [
            "dxgi.dll",
            "OptiScaler.ini",
            # Add other files if necessary, e.g., dxgi.pdb, etc.
        ]

        success = True
        for root, _, files in os.walk(extracted_path):
            for file in files:
                if file in optiscaler_files:
                    src_path = os.path.join(root, file)
                    dest_path = os.path.join(game_path, file)
                    try:
                        shutil.copy2(src_path, dest_path)
                        print(f"Copied {file} to {game_path}")
                    except Exception as e:
                        print(f"Error copying {file}: {e}")
                        success = False
        
        # Clean up extracted files (optional, but good practice)
        # shutil.rmtree(extracted_path)
        
        return success

    def read_optiscaler_ini(self, ini_path):
        settings = {}
        current_section = None
        current_comments = []

        with open(ini_path, 'r') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue

                if line.startswith('[') and line.endswith(']'):
                    current_section = line[1:-1]
                    settings[current_section] = {}
                    current_comments = []
                elif line.startswith(';'):
                    current_comments.append(line[1:].strip())
                elif '=' in line:
                    key, value = line.split('=', 1)
                    key = key.strip()
                    value = value.split(';', 1)[0].strip() # Remove inline comments
                    
                    if current_section:
                        settings[current_section][key] = {
                            "value": value,
                            "comment": "\n".join(current_comments)
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
                    f.write(f"{key}={data["value"]}\n")
                f.write("\n") # Add a newline after each section for readability