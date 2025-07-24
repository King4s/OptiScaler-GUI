import os
import vdf
import requests
from PIL import Image
from io import BytesIO
import re
import json

class Game:
    def __init__(self, name, path, appid=None, image_url=None, image_path=None):
        self.name = name
        self.path = path
        self.appid = appid
        self.image_url = image_url
        self.image_path = image_path

class GameScanner:
    def __init__(self):
        self.steam_paths = self._find_steam_paths()
        self.epic_games_paths = self._find_epic_games_paths()
        self.gog_paths = self._find_gog_paths()
        self.xbox_paths = self._find_xbox_paths()
        self.game_cache_dir = "C:\\OptiScaler-GUI\\cache\\game_images"
        os.makedirs(self.game_cache_dir, exist_ok=True)
        self.no_image_path = os.path.join("C:\\OptiScaler-GUI\\assets\\icons", "no_image.png")

        # Common non-game folder names to exclude
        self.exclude_folders = [
            "_CommonRedist", "Redist", "SDK", "Tools", "Server", "Addons", "DLC",
            "Soundtrack", "ArtBook", "Manuals", "BonusContent", "Support", "Installer"
        ]
        # Common game executable patterns
        self.game_exe_patterns = [
            r".*\\.exe$",
            r".*game\\.exe$",
            r".*launcher\\.exe$",
            r".*\\d+\\.exe$", # e.g., game_1.exe
        ]

    def _find_steam_paths(self):
        paths = [
            "C:\\Program Files (x86)\\Steam",
            "C:\\Program Files\\Steam",
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def _find_epic_games_paths(self):
        paths = [
            "C:\\Program Files\\Epic Games",
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def _find_gog_paths(self):
        paths = [
            "C:\\Program Files (x86)\\GOG Galaxy\\Games",
            os.path.join(os.path.expanduser("~"), "GOG Games"),
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def _find_xbox_paths(self):
        paths = [
            "C:\\XboxGames",
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def _is_game_folder(self, path):
        # Exclude common non-game folders
        if any(exclude_name.lower() in os.path.basename(path).lower() for exclude_name in self.exclude_folders):
            return False

        # Check for common game executables or significant game content
        found_exe = False
        found_game_content = False
        file_count = 0

        for root, dirs, files in os.walk(path):
            for file in files:
                file_count += 1
                if any(re.match(pattern, file.lower()) for pattern in self.game_exe_patterns):
                    found_exe = True
                if file.lower().endswith((".pak", ".uasset", ".dll", ".bin")) or \
                   "unityplayer.dll" in file.lower() or "unrealengine" in root.lower():
                    found_game_content = True
            
            # Limit depth of walk to avoid scanning entire drives for performance
            if root != path and os.path.relpath(root, path).count(os.sep) > 2:
                del dirs[:] # Don't recurse further

        # A folder is considered a game if it has an executable or significant game content
        # and a reasonable number of files (to filter out empty or very small folders)
        return (found_exe or found_game_content) and file_count > 5 # Arbitrary threshold, can be tuned

    def scan_games(self):
        all_games = []
        all_games.extend(self._scan_steam_games())
        all_games.extend(self._scan_epic_games())
        all_games.extend(self._scan_gog_games())
        all_games.extend(self._scan_xbox_games())

        # Deduplicate games
        unique_games = {}
        for game in all_games:
            # Use a normalized name and path for deduplication
            normalized_name = game.name.lower().strip()
            normalized_path = os.path.normpath(game.path).lower()
            unique_id = (normalized_name, normalized_path)
            if unique_id not in unique_games:
                unique_games[unique_id] = game
        
        return list(unique_games.values())

    def _scan_steam_games(self):
        steam_games = []
        for steam_path in self.steam_paths:
            steamapps_path = os.path.join(steam_path, "steamapps")
            if os.path.exists(steamapps_path):
                # Scan for appmanifest_*.acf files
                for file_name in os.listdir(steamapps_path):
                    if file_name.startswith("appmanifest_") and file_name.endswith(".acf"):
                        acf_path = os.path.join(steamapps_path, file_name)
                        
                        # Try to parse using regex first for robustness
                        try:
                            with open(acf_path, 'r', encoding='utf-8', errors='ignore') as f:
                                content = f.read()
                                appid_match = re.search(r'"appid"\s+"(\d+)"', content)
                                name_match = re.search(r'"name"\s+"([^"]+)"', content)
                                installdir_match = re.search(r'"installdir"\s+"([^"]+)"', content)

                                if appid_match and name_match and installdir_match:
                                    appid = appid_match.group(1)
                                    name = name_match.group(1)
                                    installdir = installdir_match.group(1)
                                    game_path = os.path.join(steamapps_path, "common", installdir)
                                    if os.path.exists(game_path) and self._is_game_folder(game_path):
                                        steam_games.append(Game(name=name, path=game_path, appid=appid))
                                else:
                                    # Fallback to vdf.load if regex fails or doesn't find all fields
                                    f.seek(0) # Reset file pointer
                                    data = vdf.load(f)
                                    appid = data['AppState']['appid']
                                    name = data['AppState']['name']
                                    install_dir = data['AppState']['installdir']
                                    game_path = os.path.join(steamapps_path, "common", install_dir)
                                    if os.path.exists(game_path) and self._is_game_folder(game_path):
                                        steam_games.append(Game(name=name, path=game_path, appid=appid))
                        except Exception as e:
                            print(f"Error parsing Steam appmanifest {file_name}: {e}")

                # Also scan libraryfolders.vdf for other library locations
                library_folders_path = os.path.join(steam_path, "steamapps", "libraryfolders.vdf")
                if os.path.exists(library_folders_path):
                    try:
                        with open(library_folders_path, 'r') as f:
                            library_data = vdf.load(f)
                        
                        for key, lib_path_data in library_data['libraryfolders'].items():
                            if key.isdigit():
                                library_path = lib_path_data['path']
                                common_path = os.path.join(library_path, "steamapps", "common")
                                if os.path.exists(common_path):
                                    for game_folder in os.listdir(common_path):
                                        game_path = os.path.join(common_path, game_folder)
                                        if os.path.isdir(game_path) and self._is_game_folder(game_path):
                                            # Check for appmanifest in this library folder
                                            appmanifest_found = False
                                            for file_name in os.listdir(os.path.join(library_path, "steamapps")):
                                                if file_name.startswith("appmanifest_") and file_name.endswith(".acf"):
                                                    acf_path = os.path.join(library_path, "steamapps", file_name)
                                                    try:
                                                        with open(acf_path, 'r', encoding='utf-8', errors='ignore') as f_acf:
                                                            content = f_acf.read()
                                                            appid_match = re.search(r'"appid"\s+"(\d+)"', content)
                                                            name_match = re.search(r'"name"\s+"([^"]+)"', content)
                                                            installdir_match = re.search(r'"installdir"\s+"([^"]+)"', content)

                                                            if appid_match and name_match and installdir_match:
                                                                appid = appid_match.group(1)
                                                                name = name_match.group(1)
                                                                installdir = installdir_match.group(1)
                                                                if installdir == game_folder: # Ensure it matches the current game folder
                                                                    steam_games.append(Game(name=name, path=game_path, appid=appid))
                                                                    appmanifest_found = True
                                                            else:
                                                                # Fallback to vdf.load if regex fails
                                                                f_acf.seek(0)
                                                                data = vdf.load(f_acf)
                                                                if data['AppState']['installdir'] == game_folder:
                                                                    appid = data['AppState']['appid']
                                                                    name = data['AppState']['name']
                                                                    steam_games.append(Game(name=name, path=game_path, appid=appid))
                                                                    appmanifest_found = True
                                                    except Exception as e:
                                                        print(f"Error parsing Steam appmanifest {file_name} in library folder: {e}")
                                            
                                            if not appmanifest_found: # Fallback for games without manifest or if parsing fails
                                                game_name = game_folder.replace("_", " ").replace("-", " ").title()
                                                steam_games.append(Game(name=game_name, path=game_path))
                    except Exception as e:
                        print(f"Error parsing Steam libraryfolders.vdf: {e}")
        return steam_games

    def _scan_epic_games(self):
        epic_games = []
        for epic_path in self.epic_games_paths:
            if os.path.exists(epic_path):
                for game_folder in os.listdir(epic_path):
                    game_path = os.path.join(epic_path, game_folder)
                    if os.path.isdir(game_path) and self._is_game_folder(game_path):
                        # Check for .egstore folder or manifest files
                        if os.path.exists(os.path.join(game_path, ".egstore")) or \
                           any(f.endswith(".mancfg") for f in os.listdir(game_path)):
                            game_name = game_folder.replace("_", " ").replace("-", " ").title()
                            epic_games.append(Game(name=game_name, path=game_path))
        return epic_games

    def _scan_gog_games(self):
        gog_games = []
        for gog_path in self.gog_paths:
            if os.path.exists(gog_path):
                for game_folder in os.listdir(gog_path):
                    game_path = os.path.join(gog_path, game_folder)
                    if os.path.isdir(game_path) and self._is_game_folder(game_path):
                        # Look for goggame-*.info files
                        info_file_found = False
                        for file_name in os.listdir(game_path):
                            if file_name.startswith("goggame-") and file_name.endswith(".info"):
                                info_file_path = os.path.join(game_path, file_name)
                                try:
                                    with open(info_file_path, 'r', encoding='utf-8') as f:
                                        game_info = json.load(f)
                                        game_name = game_info.get("gameTitle", game_folder.replace("_", " ").replace("-", " ").title())
                                        gog_games.append(Game(name=game_name, path=game_path))
                                        info_file_found = True
                                        break
                                except Exception as e:
                                    print(f"Error parsing GOG info file {file_name}: {e}")
                        
                        if not info_file_found: # Fallback if info file not found or parsing fails
                            game_name = game_folder.replace("_", " ").replace("-", " ").title()
                            gog_games.append(Game(name=game_name, path=game_path))
        return gog_games

    def _scan_xbox_games(self):
        xbox_games = []
        for xbox_path in self.xbox_paths:
            if os.path.exists(xbox_path):
                for game_folder in os.listdir(xbox_path):
                    game_path = os.path.join(xbox_path, game_folder)
                    if os.path.isdir(game_path) and self._is_game_folder(game_path):
                        game_name = game_folder.replace("_", " ").replace("-", " ").title()
                        xbox_games.append(Game(name=game_name, path=game_path))
        return xbox_games

    def fetch_game_image(self, game_name, appid=None):
        # Check if image already exists in cache
        for ext in ['png', 'jpg', 'jpeg', 'webp']:
            cached_path = os.path.join(self.game_cache_dir, f"{game_name.replace(' ', '_')}.{ext}")
            if os.path.exists(cached_path):
                return cached_path

        # Try Steam CDN for Steam games
        if appid:
            steam_image_url = f"https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"
            try:
                response = requests.get(steam_image_url, stream=True, timeout=5)
                response.raise_for_status()
                image = Image.open(BytesIO(response.content))
                filename = f"{game_name.replace(' ', '_')}.jpg"
                image_path = os.path.join(self.game_cache_dir, filename)
                image.save(image_path)
                return image_path
            except Exception as e:
                print(f"Error fetching Steam image for {game_name} (AppID: {appid}): {e}")

        # Return placeholder if no image found
        return self.no_image_path