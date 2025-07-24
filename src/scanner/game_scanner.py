import os
import vdf
import requests
from PIL import Image
from io import BytesIO
import re

class Game:
    def __init__(self, name, path, image_url=None, image_path=None):
        self.name = name
        self.path = path
        self.image_url = image_url
        self.image_path = image_path

class GameScanner:
    def __init__(self):
        self.steam_paths = self._find_steam_paths()
        self.epic_games_paths = self._find_epic_games_paths()
        self.gog_paths = self._find_gog_paths()
        self.game_cache_dir = "C:\\OptiScaler-GUI\\cache\\game_images"
        os.makedirs(self.game_cache_dir, exist_ok=True)

    def _find_steam_paths(self):
        # Common Steam installation paths
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
        # Common Epic Games installation paths
        paths = [
            "C:\\Program Files\\Epic Games",
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def _find_gog_paths(self):
        # Common GOG installation paths
        paths = [
            "C:\\Program Files (x86)\\GOG Galaxy\\Games", # Default GOG Galaxy game installation path
            os.path.join(os.path.expanduser("~"), "GOG Games"), # User-specific GOG Games folder
        ]
        found_paths = []
        for path in paths:
            if os.path.exists(path):
                found_paths.append(path)
        return found_paths

    def scan_games(self):
        games = []
        games.extend(self._scan_steam_games())
        games.extend(self._scan_epic_games())
        games.extend(self._scan_gog_games())
        # Add other game launchers here (Xbox)
        return games

    def _scan_steam_games(self):
        steam_games = []
        for steam_path in self.steam_paths:
            library_folders_path = os.path.join(steam_path, "steamapps", "libraryfolders.vdf")
            if os.path.exists(library_folders_path):
                with open(library_folders_path, 'r') as f:
                    library_data = vdf.load(f)
                
                for key, lib_path_data in library_data['libraryfolders'].items():
                    if key.isdigit(): # Ensure it's a library folder entry
                        library_path = lib_path_data['path']
                        common_path = os.path.join(library_path, "steamapps", "common")
                        if os.path.exists(common_path):
                            for game_folder in os.listdir(common_path):
                                game_path = os.path.join(common_path, game_folder)
                                if os.path.isdir(game_path):
                                    # Simple heuristic: check for .exe or common game files
                                    is_game = False
                                    for root, _, files in os.walk(game_path):
                                        for file in files:
                                            if file.endswith((".exe", ".dll", ".pak", ".bin")):
                                                is_game = True
                                                break
                                        if is_game:
                                            break
                                    
                                    if is_game:
                                        game_name = game_folder.replace("_", " ").replace("-", " ").title()
                                        steam_games.append(Game(name=game_name, path=game_path))
        return steam_games

    def _scan_epic_games(self):
        epic_games = []
        for epic_path in self.epic_games_paths:
            if os.path.exists(epic_path):
                for game_folder in os.listdir(epic_path):
                    game_path = os.path.join(epic_path, game_folder)
                    if os.path.isdir(game_path):
                        # Simple heuristic: check for .exe or common game files
                        is_game = False
                        for root, _, files in os.walk(game_path):
                            for file in files:
                                if file.endswith((".exe", ".dll", ".pak", ".bin")):
                                    is_game = True
                                    break
                            if is_game:
                                break
                        
                        if is_game:
                            game_name = game_folder.replace("_", " ").replace("-", " ").title()
                            epic_games.append(Game(name=game_name, path=game_path))
        return epic_games

    def _scan_gog_games(self):
        gog_games = []
        for gog_path in self.gog_paths:
            if os.path.exists(gog_path):
                for game_folder in os.listdir(gog_path):
                    game_path = os.path.join(gog_path, game_folder)
                    if os.path.isdir(game_path):
                        # Simple heuristic: check for .exe or common game files
                        is_game = False
                        for root, _, files in os.walk(game_path):
                            for file in files:
                                if file.endswith((".exe", ".dll", ".pak", ".bin")):
                                    is_game = True
                                    break
                            if is_game:
                                break
                        
                        if is_game:
                            game_name = game_folder.replace("_", " ").replace("-", " ").title()
                            gog_games.append(Game(name=game_name, path=game_path))
        return gog_games

    def fetch_game_image(self, game_name):
        search_query = f"{game_name} game box art"
        try:
            search_results = default_api.google_web_search(query=search_query)
            if search_results and "output" in search_results:
                # Regex to find image URLs (jpg, png, webp) in the search results
                image_urls = re.findall(r'(https?://\S+\.(?:jpg|png|webp))(?=\s|$)', search_results["output"])
                for url in image_urls:
                    filename = f"{game_name.replace(' ', '_')}.{url.split('.')[-1]}"
                    image_path = self._download_image(url, filename)
                    if image_path:
                        print(f"Downloaded image for {game_name} to {image_path}")
                        return image_path
            print(f"No image found for: {game_name}")
            return None
        except Exception as e:
            print(f"Error fetching image for {game_name}: {e}")
            return None

    def _download_image(self, url, filename):
        try:
            response = requests.get(url)
            response.raise_for_status()
            image = Image.open(BytesIO(response.content))
            image_path = os.path.join(self.game_cache_dir, filename)
            image.save(image_path)
            return image_path
        except Exception as e:
            print(f"Error downloading image from {url}: {e}")
            return None