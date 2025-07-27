import os
import vdf
import requests
from PIL import Image
from io import BytesIO
import re
import json
import time
from pathlib import Path
from utils.config import config
from utils.cache_manager import cache_manager
from utils.performance import timed
from utils.debug import debug_log

class Game:
    def __init__(self, name, path, appid=None, image_path=None):
        self.name = name
        self.path = str(path)  # Ensure string for compatibility
        self.appid = appid
        self.image_path = image_path

class GameScanner:
    def __init__(self):
        self.steam_paths = self._find_steam_paths()
        self.epic_games_paths = self._find_epic_games_paths()
        self.gog_paths = self._find_gog_paths()
        self.xbox_paths = self._find_xbox_paths()
        self.game_cache_dir = Path(config.game_cache_dir)
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
        """Find Steam installation paths using Path objects"""
        potential_paths = [
            Path("C:/Program Files (x86)/Steam"),
            Path("C:/Program Files/Steam"),
            # Additional common locations
            Path.home() / "AppData/Local/Steam",
        ]
        found_paths = []
        for path in potential_paths:
            if path.exists() and path.is_dir():
                found_paths.append(str(path))
                debug_log(f"Found Steam path: {path}")
        return found_paths

    def _find_epic_games_paths(self):
        """Find Epic Games installation paths using Path objects"""
        potential_paths = [
            Path("C:/Program Files/Epic Games"),
            Path("C:/Program Files (x86)/Epic Games"),
        ]
        found_paths = []
        for path in potential_paths:
            if path.exists() and path.is_dir():
                found_paths.append(str(path))
                debug_log(f"Found Epic Games path: {path}")
        return found_paths

    def _find_gog_paths(self):
        """Find GOG installation paths using Path objects"""
        potential_paths = [
            Path("C:/Program Files (x86)/GOG Galaxy/Games"),
            Path.home() / "GOG Games",
        ]
        found_paths = []
        for path in potential_paths:
            if path.exists() and path.is_dir():
                found_paths.append(str(path))
                debug_log(f"Found GOG path: {path}")
        return found_paths

    def _find_xbox_paths(self):
        """Find Xbox Games installation paths using Path objects"""
        potential_paths = [
            Path("C:/XboxGames"),
            # Additional Xbox Game Pass locations
            Path("C:/Program Files/WindowsApps"),
        ]
        found_paths = []
        for path in potential_paths:
            if path.exists() and path.is_dir():
                found_paths.append(str(path))
                debug_log(f"Found Xbox path: {path}")
        return found_paths

    def _is_game_folder(self, path):
        """Enhanced game folder detection with improved performance"""
        path = Path(path)
        folder_name = path.name.lower()
        
        # Exclude common non-game folders
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
            debug_log(f"Access denied or error scanning {path}: {e}")
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
        
        debug_log(f"Scan complete: Found {len(unique_games)} unique games")
        return list(unique_games.values())

    def _scan_steam_games(self):
        """Enhanced Steam game scanning with Path objects and improved error handling"""
        steam_games = []
        for steam_path_str in self.steam_paths:
            steam_path = Path(steam_path_str)
            steamapps_path = steam_path / "steamapps"
            if not steamapps_path.exists():
                continue
                
            debug_log(f"Scanning Steam library: {steamapps_path}")
            
            try:
                # Parse .acf files for installed games
                for acf_file in steamapps_path.glob("appmanifest_*.acf"):
                    try:
                        game = self._parse_steam_acf(acf_file, steamapps_path)
                        if game:
                            steam_games.append(game)
                            debug_log(f"Found Steam game: {game.name} at {game.path}")
                    except Exception as e:
                        debug_log(f"Error parsing Steam ACF {acf_file.name}: {e}")

                # Scan additional library folders
                library_folders_file = steamapps_path / "libraryfolders.vdf"
                if library_folders_file.exists():
                    steam_games.extend(self._scan_steam_library_folders(library_folders_file))
                    
            except Exception as e:
                debug_log(f"Error scanning Steam path {steam_path}: {e}")
        
        return steam_games

    def _parse_steam_acf(self, acf_file, steamapps_path):
        """Parse a Steam ACF file to extract game information"""
        try:
            with open(acf_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
                
            # Extract fields using regex
            appid_match = re.search(r'"appid"\s+"([^"]+?)"', content)
            name_match = re.search(r'"name"\s+"([^"]+?)"', content)
            installdir_match = re.search(r'"installdir"\s+"([^"]+?)"', content)

            if not (name_match and installdir_match):
                return None

            appid = appid_match.group(1) if appid_match else None
            name = name_match.group(1)
            installdir = installdir_match.group(1)

            game_path = steamapps_path / "common" / installdir
            if game_path.exists() and self._is_game_folder(game_path):
                image_path = self.fetch_game_image(name, appid)
                return Game(name=name, path=str(game_path), appid=appid, image_path=image_path)
            
        except Exception as e:
            debug_log(f"Error parsing ACF file {acf_file}: {e}")
        
        return None

    def _scan_steam_library_folders(self, library_folders_file):
        """Scan additional Steam library folders"""
        games = []
        try:
            with open(library_folders_file, 'r', encoding='utf-8', errors='ignore') as f:
                library_data = vdf.load(f)
            
            for key, lib_data in library_data.get('libraryfolders', {}).items():
                if not key.isdigit():
                    continue
                    
                library_path = Path(lib_data.get('path', ''))
                if not library_path.exists():
                    continue
                    
                common_path = library_path / "steamapps" / "common"
                if common_path.exists():
                    games.extend(self._scan_steam_common_folder(common_path, library_path))
                    
        except Exception as e:
            debug_log(f"Error scanning Steam library folders: {e}")
        
        return games

    def _scan_steam_common_folder(self, common_path, library_path):
        """Scan a Steam common folder for games"""
        games = []
        steamapps_path = library_path / "steamapps"
        
        for game_folder in common_path.iterdir():
            if not game_folder.is_dir() or not self._is_game_folder(game_folder):
                continue
                
            # Try to find matching ACF file
            game_info = self._find_steam_game_info(game_folder.name, steamapps_path)
            if game_info:
                name, appid = game_info
                image_path = self.fetch_game_image(name, appid)
                game = Game(name=name, path=str(game_folder), appid=appid, image_path=image_path)
                games.append(game)
            else:
                # Fallback: use folder name
                image_path = self.fetch_game_image(game_folder.name)
                game = Game(name=game_folder.name, path=str(game_folder), image_path=image_path)
                games.append(game)
        
        return games

    def _find_steam_game_info(self, folder_name, steamapps_path):
        """Find Steam game info by matching install directory"""
        for acf_file in steamapps_path.glob("appmanifest_*.acf"):
            try:
                with open(acf_file, 'r', encoding='utf-8', errors='ignore') as f:
                    content = f.read()
                    
                installdir_match = re.search(r'"installdir"\s+"([^"]+?)"', content)
                if installdir_match and installdir_match.group(1) == folder_name:
                    appid_match = re.search(r'"appid"\s+"([^"]+?)"', content)
                    name_match = re.search(r'"name"\s+"([^"]+?)"', content)
                    
                    if name_match:
                        appid = appid_match.group(1) if appid_match else None
                        return name_match.group(1), appid
                        
            except Exception:
                continue
        
        return None

    def _scan_epic_games(self):
        """Scan Epic Games installations with Path objects"""
        epic_games = []
        for epic_path_str in self.epic_games_paths:
            try:
                epic_path = Path(epic_path_str)
                if not epic_path.exists():
                    continue
                    
                for game_folder in epic_path.iterdir():
                    if not game_folder.is_dir() or not self._is_game_folder(str(game_folder)):
                        continue
                        
                    # Check for .egstore folder or manifest files
                    has_egstore = (game_folder / ".egstore").exists()
                    has_manifest = any(f.suffix == ".mancfg" for f in game_folder.glob("*.mancfg"))
                    
                    if has_egstore or has_manifest:
                        game_name = game_folder.name.replace("_", " ").replace("-", " ").title()
                        image_path = self.fetch_game_image(game_name)
                        epic_games.append(Game(name=game_name, path=str(game_folder), image_path=image_path))
                        
            except (OSError, PermissionError) as e:
                debug_log(f"Error scanning Epic Games path {epic_path}: {e}")
                continue
                
        return epic_games

    def _scan_gog_games(self):
        """Scan GOG installations with Path objects"""
        gog_games = []
        for gog_path_str in self.gog_paths:
            try:
                gog_path = Path(gog_path_str)
                if not gog_path.exists():
                    continue
                    
                for game_folder in gog_path.iterdir():
                    if not game_folder.is_dir() or not self._is_game_folder(str(game_folder)):
                        continue
                        
                    # Look for goggame-*.info files
                    info_files = list(game_folder.glob("goggame-*.info"))
                    if info_files:
                        try:
                            with open(info_files[0], 'r', encoding='utf-8') as f:
                                game_info = json.load(f)
                                game_name = game_info.get("gameTitle", 
                                    game_folder.name.replace("_", " ").replace("-", " ").title())
                                image_path = self.fetch_game_image(game_name)
                                gog_games.append(Game(name=game_name, path=str(game_folder), image_path=image_path))
                                continue
                        except (json.JSONDecodeError, UnicodeDecodeError, KeyError) as e:
                            debug_log(f"Error parsing GOG info file {info_files[0].name}: {e}")
                    
                    # Fallback: use folder name
                    game_name = game_folder.name.replace("_", " ").replace("-", " ").title()
                    gog_games.append(Game(name=game_name, path=str(game_folder)))
                    
            except (OSError, PermissionError) as e:
                debug_log(f"Error scanning GOG path {gog_path}: {e}")
                continue
                
        return gog_games

    def _scan_xbox_games(self):
        """Scan Xbox Games installations with Path objects"""
        xbox_games = []
        for xbox_path_str in self.xbox_paths:
            try:
                xbox_path = Path(xbox_path_str)
                if not xbox_path.exists():
                    continue
                    
                for game_folder in xbox_path.iterdir():
                    if not game_folder.is_dir() or not self._is_game_folder(str(game_folder)):
                        continue
                        
                    game_name = game_folder.name.replace("_", " ").replace("-", " ").title()
                    image_path = self.fetch_game_image(game_name)
                    xbox_games.append(Game(name=game_name, path=str(game_folder), image_path=image_path))
                    
            except (OSError, PermissionError) as e:
                debug_log(f"Error scanning Xbox Games path {xbox_path}: {e}")
                continue
                
        return xbox_games

    @timed("image_fetch")
    def fetch_game_image(self, game_name, appid=None):
        """Fetch game image with Path objects and improved error handling"""
        # If appid is not provided, try to get it from the cached list
        if not appid:
            appid = self._get_appid_from_name(game_name)
            if not appid:
                # If still no appid, return placeholder
                return str(self.no_image_path)

        # Check if image already exists in cache
        safe_name = re.sub(r'[<>:"/\\|?*]', '_', game_name)  # Remove invalid filename chars
        cache_dir = Path(self.game_cache_dir)
        
        for ext in ['png', 'jpg', 'jpeg', 'webp']:
            cached_path = cache_dir / f"{safe_name}.{ext}"
            if cached_path.exists():
                return str(cached_path)

        try:
            # Try Steam CDN for Steam games
            steam_image_url = f"https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"
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
            
            # Save with Path object
            image_path = cache_dir / f"{safe_name}.jpg"
            image.save(image_path, 'JPEG', quality=config.image_quality, optimize=True)
            return str(image_path)
            
        except (requests.RequestException, OSError, Image.UnidentifiedImageError) as e:
            debug_log(f"Error fetching Steam image for {game_name} (AppID: {appid}): {e}")

        # Return placeholder if no image found
        return str(self.no_image_path)

    def _get_steam_app_list(self):
        """Get Steam app list with Path objects and improved caching"""
        app_list_cache_path = Path(config.steam_app_list_cache_path)
        
        # Check if cached list exists and is recent
        if app_list_cache_path.exists():
            cache_age_days = (time.time() - app_list_cache_path.stat().st_mtime) / (24 * 60 * 60)
            if cache_age_days < config.steam_app_list_cache_days:
                try:
                    with open(app_list_cache_path, 'r', encoding='utf-8') as f:
                        debug_log(f"Using cached Steam app list (age: {cache_age_days:.1f} days)")
                        return json.load(f)
                except (json.JSONDecodeError, UnicodeDecodeError) as e:
                    debug_log(f"Error reading cached Steam app list: {e}")

        # Fetch new list if cache is old or missing
        debug_log("Fetching fresh Steam app list from API...")
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
            
            # Save to cache with Path object
            app_list_cache_path.parent.mkdir(parents=True, exist_ok=True)
            with open(app_list_cache_path, 'w', encoding='utf-8') as f:
                json.dump(apps, f, separators=(',', ':'))  # Compact JSON
            
            debug_log(f"Cached {len(apps)} Steam apps")
            return apps
            
        except (requests.RequestException, json.JSONDecodeError, KeyError) as e:
            debug_log(f"Error fetching Steam app list: {e}")
            return {}

    def _get_appid_from_name(self, game_name):
        """Get Steam AppID from game name with fallback handling"""
        normalized_game_name = game_name.lower()
        return self.steam_app_list.get(normalized_game_name)