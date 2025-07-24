import os
import vdf
import requests
from PIL import Image
from io import BytesIO
import re
import json
import time
from utils.config import config
from utils.cache_manager import cache_manager
from utils.performance import timed

class Game:
    def __init__(self, name, path, appid=None, image_path=None):
        self.name = name
        self.path = path
        self.appid = appid
        self.image_path = image_path

class GameScanner:
    def __init__(self):
        self.steam_paths = self._find_steam_paths()
        self.epic_games_paths = self._find_epic_games_paths()
        self.gog_paths = self._find_gog_paths()
        self.xbox_paths = self._find_xbox_paths()
        self.game_cache_dir = str(config.game_cache_dir)
        self.no_image_path = config.no_image_path
        self.steam_app_list = self._get_steam_app_list() # Load Steam app list on init

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
        folder_name = os.path.basename(path).lower()
        if any(exclude_name.lower() in folder_name for exclude_name in self.exclude_folders):
            return False

        # Check for common game executables or significant game content
        found_exe = False
        found_game_content = False
        file_count = 0
        max_files_to_check = 1000  # Limit file checks for performance

        try:
            for root, dirs, files in os.walk(path):
                # Limit scan depth for performance
                current_depth = os.path.relpath(root, path).count(os.sep)
                if current_depth >= config.max_scan_depth:
                    dirs[:] = []  # Don't recurse further
                    continue
                
                for file in files:
                    file_count += 1
                    if file_count > max_files_to_check:
                        break  # Stop if we've checked too many files
                    
                    file_lower = file.lower()
                    if any(re.match(pattern, file_lower) for pattern in self.game_exe_patterns):
                        found_exe = True
                    if file_lower.endswith((".pak", ".uasset", ".dll", ".bin", ".unity3d")) or \
                       "unityplayer.dll" in file_lower or "unrealengine" in root.lower():
                        found_game_content = True
                
                if file_count > max_files_to_check:
                    break
                
        except (PermissionError, OSError) as e:
            print(f"Access denied or error scanning {path}: {e}")
            return False

        # A folder is considered a game if it has an executable or significant game content
        # and a reasonable number of files (to filter out empty or very small folders)
        return (found_exe or found_game_content) and file_count > 5

    @timed("game_scan")
    def scan_games(self):
        # Perform cache cleanup before scanning
        cache_manager.cleanup_large_cache()
        
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
        
        print(f"Scan complete: Found {len(unique_games)} unique games")
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
                        
                        # Robust parsing for .acf files using regex
                        try:
                            with open(acf_path, 'r', encoding='utf-8', errors='ignore') as f:
                                content = f.read()
                                # More flexible regex patterns with non-greedy matching
                                appid_match = re.search(r'"appid"\s+"([^"]+?)"', content)
                                name_match = re.search(r'"name"\s+"([^"]+?)"', content)
                                installdir_match = re.search(r'"installdir"\s+"([^"]+?)"', content)

                                appid = appid_match.group(1) if appid_match else None
                                name = name_match.group(1) if name_match else None
                                installdir = installdir_match.group(1) if installdir_match else None

                                if name and installdir: # We need at least name and installdir to proceed
                                    game_path = os.path.join(steamapps_path, "common", installdir)
                                    if os.path.exists(game_path) and self._is_game_folder(game_path):
                                        image_path = self.fetch_game_image(name, appid) # Pass name for lookup if appid is None
                                        game = Game(name=name, path=game_path, appid=appid, image_path=image_path)
                                        steam_games.append(game)
                                        print(f"DEBUG: Scanned Steam Game - Name: {game.name}, AppID: {game.appid}, Image Path: {game.image_path}")
                                else:
                                    missing_fields = []
                                    if not appid_match: missing_fields.append("appid")
                                    if not name_match: missing_fields.append("name")
                                    if not installdir_match: missing_fields.append("installdir")
                                    print(f"Warning: Could not extract required fields {', '.join(missing_fields)} from {file_name} using regex. AppID: {appid_match.group(1) if appid_match else 'N/A'}, Name: {name_match.group(1) if name_match else 'N/A'}, InstallDir: {installdir_match.group(1) if installdir_match else 'N/A'}")
                        except Exception as e:
                            print(f"Error reading or parsing Steam appmanifest {file_name}: {e}")

                # Also scan libraryfolders.vdf for other library locations
                library_folders_path = os.path.join(steam_path, "steamapps", "libraryfolders.vdf")
                if os.path.exists(library_folders_path):
                    try:
                        with open(library_folders_path, 'r', encoding='utf-8', errors='ignore') as f:
                            library_data = vdf.load(f)
                        
                        for key, lib_path_data in library_data['libraryfolders'].items():
                            if key.isdigit():
                                library_path = lib_path_data['path']
                                common_path = os.path.join(library_path, "steamapps", "common")
                                if os.path.exists(common_path):
                                    for game_folder in os.listdir(common_path):
                                        game_path = os.path.join(common_path, game_folder)
                                        if os.path.isdir(game_path) and self._is_game_folder(game_path):
                                            # Attempt to find appmanifest for this game folder
                                            found_app_info = False
                                            for file_name in os.listdir(os.path.join(library_path, "steamapps")):
                                                if file_name.startswith("appmanifest_") and file_name.endswith(".acf"):
                                                    acf_path = os.path.join(library_path, "steamapps", file_name)
                                                    try:
                                                        with open(acf_path, 'r', encoding='utf-8', errors='ignore') as f_acf:
                                                            content = f_acf.read()
                                                            appid_match = re.search(r'"appid"\s+"([^"]+?)"', content)
                                                            name_match = re.search(r'"name"\s+"([^"]+?)"', content)
                                                            installdir_match = re.search(r'"installdir"\s+"([^"]+?)"', content)

                                                            appid = appid_match.group(1) if appid_match else None
                                                            name = name_match.group(1) if name_match else None
                                                            installdir = installdir_match.group(1) if installdir_match else None

                                                            if name and installdir and installdir == game_folder:
                                                                image_path = self.fetch_game_image(name, appid)
                                                                game = Game(name=name, path=game_path, appid=appid, image_path=image_path)
                                                                steam_games.append(game)
                                                                print(f"DEBUG: Scanned Steam Game - Name: {game.name}, AppID: {game.appid}, Image Path: {game.image_path}")
                                                                found_app_info = True
                                                                break # Found appmanifest for this game folder
                                                    except Exception as e:
                                                        print(f"Error reading or parsing Steam appmanifest {file_name} in library folder: {e}")
                                            
                                            if not found_app_info: # Fallback if appmanifest not found or parsing fails
                                                game_name = game_folder.replace("_", " ").replace("-", " ").title()
                                                image_path = self.fetch_game_image(game_name) # Attempt lookup with just name
                                                steam_games.append(Game(name=game_name, path=game_path, image_path=image_path))
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
                            image_path = self.fetch_game_image(game_name)
                            epic_games.append(Game(name=game_name, path=game_path, image_path=image_path))
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
                                        image_path = self.fetch_game_image(game_name)
                                        gog_games.append(Game(name=game_name, path=game_path, image_path=image_path))
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
                        image_path = self.fetch_game_image(game_name)
                        xbox_games.append(Game(name=game_name, path=game_path, image_path=image_path))
        return xbox_games

    @timed("image_fetch")
    def fetch_game_image(self, game_name, appid=None):
        # If appid is not provided, try to get it from the cached list
        if not appid:
            appid = self._get_appid_from_name(game_name)
            if not appid:
                # If still no appid, return placeholder
                return self.no_image_path

        # Check if image already exists in cache
        safe_name = re.sub(r'[<>:"/\\|?*]', '_', game_name)  # Remove invalid filename chars
        for ext in ['png', 'jpg', 'jpeg', 'webp']:
            cached_path = os.path.join(self.game_cache_dir, f"{safe_name}.{ext}")
            if os.path.exists(cached_path):
                return cached_path

        # Try Steam CDN for Steam games
        steam_image_url = f"https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"
        try:
            response = requests.get(steam_image_url, stream=True, timeout=config.image_download_timeout)
            response.raise_for_status()
            
            # Load and optimize image
            image = Image.open(BytesIO(response.content))
            
            # Resize image to standard dimensions for consistency and memory savings
            image.thumbnail(config.max_image_size, Image.Resampling.LANCZOS)
            
            # Convert to RGB if necessary (for JPEG saving)
            if image.mode in ('RGBA', 'LA', 'P'):
                rgb_image = Image.new('RGB', image.size, (255, 255, 255))
                rgb_image.paste(image, mask=image.split()[-1] if image.mode == 'RGBA' else None)
                image = rgb_image
            
            filename = f"{safe_name}.jpg"
            image_path = os.path.join(self.game_cache_dir, filename)
            image.save(image_path, 'JPEG', quality=config.image_quality, optimize=True)
            return image_path
        except Exception as e:
            print(f"Error fetching Steam image for {game_name} (AppID: {appid}): {e}")

        # Return placeholder if no image found
        return self.no_image_path

    def _get_steam_app_list(self):
        app_list_cache_path = config.steam_app_list_cache_path
        
        # Check if cached list exists and is recent
        if os.path.exists(app_list_cache_path):
            last_modified = os.path.getmtime(app_list_cache_path)
            cache_age_days = (time.time() - last_modified) / (24 * 60 * 60)
            if cache_age_days < config.steam_app_list_cache_days:
                try:
                    with open(app_list_cache_path, 'r', encoding='utf-8') as f:
                        print(f"Using cached Steam app list (age: {cache_age_days:.1f} days)")
                        return json.load(f)
                except Exception as e:
                    print(f"Error reading cached Steam app list: {e}")

        # Fetch new list if cache is old or missing
        print("Fetching fresh Steam app list from API...")
        try:
            response = requests.get("https://api.steampowered.com/ISteamApps/GetAppList/v2/", 
                                  timeout=30, stream=True)
            response.raise_for_status()
            app_list_data = response.json()
            
            # Create a more memory-efficient lookup dictionary
            apps = {}
            for app in app_list_data['applist']['apps']:
                # Only store apps with meaningful names (filter out DLCs, tools, etc.)
                name = app['name'].strip().lower()
                if len(name) > 2 and not name.startswith(('dlc', 'demo', 'beta', 'test')):
                    apps[name] = str(app['appid'])
            
            # Save to cache
            with open(app_list_cache_path, 'w', encoding='utf-8') as f:
                json.dump(apps, f, separators=(',', ':'))  # Compact JSON
            
            print(f"Cached {len(apps)} Steam apps")
            return apps
        except Exception as e:
            print(f"Error fetching Steam app list: {e}")
            return {}

    def _get_appid_from_name(self, game_name):
        normalized_game_name = game_name.lower()
        return self.steam_app_list.get(normalized_game_name)