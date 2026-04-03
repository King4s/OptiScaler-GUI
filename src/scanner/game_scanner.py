import os
import threading
import vdf
import requests
from PIL import Image
from io import BytesIO
import re
import json
import time
from pathlib import Path
from utils.config import config
from scanner.library_discovery import get_game_libraries, compute_library_summary
from utils.cache_manager import cache_manager
from utils.performance import timed
from utils.debug import debug_log
from utils.compatibility_checker import compatibility_checker

class Game:
    def __init__(self, name, path, appid=None, image_path=None, optiscaler_installed=False, engine=None, anti_cheat_list=None, community_verified=False, engine_supported=True, platform=None):
        self.name = name
        self.path = str(path)  # Ensure string for compatibility
        self.appid = appid
        self.image_path = image_path
        self.optiscaler_installed = optiscaler_installed  # Track OptiScaler installation status
        self.engine = engine
        self.engine_supported = engine_supported
        # Default to 'Local' when result not tied to a known launcher to ensure the UI shows a tag
        self.platform = platform if platform is not None else 'Local'
        self.anti_cheat_list = anti_cheat_list or []
        self.community_verified = community_verified

class GameScanner:
    def __init__(self):
        self.steam_paths = self._find_steam_paths()
        self.epic_games_paths = self._find_epic_games_paths()
        self.gog_paths = self._find_gog_paths()
        self.xbox_paths = self._find_xbox_paths()
        self.game_cache_dir = Path(config.game_cache_dir)
        # Load community verified games from data file if present
        self.community_verified = set()
        try:
            community_file = Path(__file__).parent.parent / 'data' / 'community_verified_games.json'
            if community_file.exists():
                with open(community_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    for g in data.get('games', []):
                        # Use normalized name and optional appid for lookups
                        name_key = (g.get('name') or '').lower().strip()
                        appid_key = str(g.get('appid') or '')
                        self.community_verified.add((name_key, appid_key))
        except Exception as e:
            debug_log(f"Failed to load community-verified game list: {e}")
        self.no_image_path = config.no_image_path
        # session for requests to enable keep-alive and connection pooling
        self._requests_session = requests.Session()
        # Callback invoked after the background Steam app list load completes.
        # Set by game_list_frame to schedule a thumbnail retry pass.
        self.on_app_list_ready = None
        # Build app list from local manifests immediately (fast, no network needed)
        self.steam_app_list = self._build_local_steam_app_list()
        # Kick off a background download of the full SteamSpy catalogue
        threading.Thread(target=self._load_steamspy_app_list_async, daemon=True).start()
        # Summary and timing info for last library discovery
        self.last_library_summary = None
        self.last_library_scan_seconds = None
        # Cached results from the last scan (to avoid unnecessary rescans)
        self._cached_games = None

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
        # Attempt registry detection (fallback)
        try:
            from scanner.library_discovery import _find_steam_install_from_registry
            reg_path = _find_steam_install_from_registry()
            if reg_path:
                potential_paths.insert(0, Path(reg_path))
        except Exception:
            pass
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

    def _find_heroic_paths(self):
        """Find Heroic Launcher metadata directories."""
        candidates = [
            Path.home() / "AppData" / "Roaming" / "heroic",
            Path.home() / "AppData" / "Local" / "heroic",
        ]
        found_paths = []
        for base in candidates:
            if not base.exists() or not base.is_dir():
                continue
            for rel in ["GamesConfig", "legendaryConfig/legendary", "gog_store"]:
                p = base / rel
                if p.exists() and p.is_dir():
                    found_paths.append(str(p))
                    debug_log(f"Found Heroic path: {p}")
        return found_paths

    def _scan_heroic_games(self):
        """Scan Heroic Launcher JSON configs for installed Epic/GOG games."""
        games = []
        try:
            for root in self._find_heroic_paths():
                root_path = Path(root)
                for cfg in root_path.glob("*.json"):
                    try:
                        with open(cfg, "r", encoding="utf-8") as f:
                            data = json.load(f)
                        if not isinstance(data, dict):
                            continue
                        install = data.get("install") or {}
                        install_path = install.get("install_path") or data.get("install_path")
                        if not install_path:
                            continue
                        game_name = data.get("title") or data.get("name") or Path(install_path).name
                        if not game_name:
                            continue
                        p = Path(install_path)
                        if not p.exists() or not self._is_game_folder(p):
                            continue
                        image_path = self.fetch_game_image(game_name)
                        optiscaler_installed = self._detect_optiscaler(p)
                        safety = self.analyze_game_safety(Game(game_name, str(p)))
                        games.append(Game(
                            name=game_name,
                            path=str(p),
                            image_path=image_path,
                            optiscaler_installed=optiscaler_installed,
                            engine=safety["engine"],
                            anti_cheat_list=safety["anti_cheat_list"],
                            community_verified=safety["community_verified"],
                            engine_supported=safety.get("engine_supported", True),
                            platform="Heroic"
                        ))
                    except Exception:
                        continue
        except Exception as e:
            debug_log(f"Heroic scan failed: {e}")
        return games

    def _detect_optiscaler(self, game_path):
        """Detect if OptiScaler is installed in a game directory"""
        try:
            game_path = Path(game_path)
            
            # OptiScaler indicator files
            optiscaler_files = [
                'nvngx_dlss.dll',     # Common OptiScaler DLL
                'nvngx_dlssg.dll',    # DLSS-G version
                'OptiScaler.dll',     # Direct OptiScaler
                'nvngx.dll',          # NVIDIA proxy
                'dxgi.dll',           # DirectX proxy
                'winmm.dll',          # Windows multimedia proxy
            ]
            
            # Check directories to scan for OptiScaler files
            check_directories = [game_path]  # Start with main directory
            
            # Add Unreal Engine directory if it exists (critical for UE games!)
            unreal_engine_dir = game_path / "Engine" / "Binaries" / "Win64"
            if unreal_engine_dir.exists() and unreal_engine_dir.is_dir():
                check_directories.append(unreal_engine_dir)
                debug_log(f"Added Unreal Engine directory to OptiScaler detection: {unreal_engine_dir}")
            
            # Check for indicator files in all relevant directories
            for check_dir in check_directories:
                for file_name in optiscaler_files:
                    if (check_dir / file_name).exists():
                        debug_log(f"Found OptiScaler indicator file: {file_name} in {check_dir}")
                        return True
                
                # Check in common OptiScaler subdirectories within each directory
                optiscaler_subdirs = ["D3D12_Optiscaler", "OptiScaler", "mods", "plugins"]
                for subdir in optiscaler_subdirs:
                    subdir_path = check_dir / subdir
                    if subdir_path.exists() and subdir_path.is_dir():
                        # Check for OptiScaler files in subdirectory
                        for file_name in optiscaler_files:
                            if (subdir_path / file_name).exists():
                                debug_log(f"Found OptiScaler in subdirectory: {check_dir}/{subdir}/{file_name}")
                                return True
            
            return False
            
        except Exception as e:
            debug_log(f"Error detecting OptiScaler in {game_path}: {e}")
            return False

    def _detect_engine_type(self, game_path: Path) -> str:
        """Detect common engine types for a game folder (Unreal, Unity, Custom)."""
        try:
            game_path = Path(game_path)
            # Unreal Engine detection
            if (game_path / 'Engine' / 'Binaries' / 'Win64').exists():
                return 'Unreal'
            # Unity detection: presence of UnityPlayer.dll or Assets folder
            if (game_path / 'UnityPlayer.dll').exists() or (game_path / 'Assets').exists():
                return 'Unity'
            # Godot detection: .import/gui or project file
            if any(game_path.glob('*.godot')) or (game_path / '.import').exists():
                return 'Godot'
            # Prism3D detection: check top-level exes, dlls and known SCS/EU/ATS patterns
            try:
                # Check .exe names
                for p in game_path.glob('*.exe'):
                    name = p.name.lower()
                    if 'prism' in name or ('euro' in name and 'truck' in name) or 'ats' in name or 'eurotruck' in name.replace(' ', ''):
                        return 'Prism3D'
                # Check top-level dlls
                for d in game_path.glob('*.dll'):
                    dname = d.name.lower()
                    if 'prism' in dname or 'prism3d' in dname or 'scs' in dname:
                        return 'Prism3D'
                # Check for common SCS files or engine config names in top-level
                for cfg in ('prismengine.ini', 'engine.ini', 'scs_game.ini', 'scs_game.txt'):
                    if (game_path / cfg).exists():
                        return 'Prism3D'
            except Exception:
                pass
            return 'Unknown'
        except Exception as e:
            debug_log(f"Engine detection failed for {game_path}: {e}")
            return 'Unknown'

    def _detect_anti_cheat(self, game_path: Path) -> list:
        """Detect presence of common anti-cheat software in the game folder.
        Returns a list of detected anti-cheat names.
        """
        ac_list = []
        try:
            indicators = {
                'EasyAntiCheat': ['EasyAntiCheat.sys', 'EasyAntiCheat.exe', 'EasyAntiCheat'],
                'BattlEye': ['beclient.dll', 'BEService.exe', 'BattleEye'],
                'Vanguard': ['vgc.sys', 'vgtray.exe', 'Vanguard'],
                'Easy Anti-Cheat': ['EasyAntiCheat'],
                'BattleEye (BE)':[ 'BEService.exe', 'BattleEye']
            }
            for name, patterns in indicators.items():
                for pattern in patterns:
                    # Look for files or folders containing the pattern
                    # Prefers top-level check and limited depth to avoid full recursive traversal
                    try:
                        # Check top-level
                        if any(p.name.lower() == pattern.lower() or pattern.lower() in p.name.lower() for p in game_path.iterdir() if p.is_file()):
                            ac_list.append(name)
                            break
                        # Check a shallow depth: direct children directories' files
                        for sub in (game_path.iterdir() if game_path.exists() else []):
                            if sub.is_dir():
                                for p in sub.iterdir():
                                    if p.is_file() and (p.name.lower() == pattern.lower() or pattern.lower() in p.name.lower()):
                                        ac_list.append(name)
                                        raise StopIteration
                        # As a final fallback, check a limited set of files via glob patterns
                    except StopIteration:
                        break
                    except Exception:
                        pass
                        ac_list.append(name)
                        break
            # Filter duplicates
            ac_list = list(dict.fromkeys(ac_list))
            return ac_list
        except Exception as e:
            debug_log(f"Anti-cheat detection failed for {game_path}: {e}")
            return ac_list

    def _detect_anti_cheat_shallow(self, game_path: Path) -> list:
        """Shallow anti-cheat detection for registry/appx scans to avoid expensive recursive scans."""
        ac_list = []
        try:
                indicators = {
                    'EasyAntiCheat': ['EasyAntiCheat.sys', 'EasyAntiCheat.exe', 'EasyAntiCheat'],
                    'BattlEye': ['beclient.dll', 'BEService.exe', 'BattleEye'],
                    'Vanguard': ['vgc.sys', 'vgtray.exe', 'Vanguard']
                }
                # List only top-level files and folders (avoid recursion)
                for p in Path(game_path).iterdir():
                    name = p.name.lower()
                    for ac_name, patterns in indicators.items():
                        for pattern in patterns:
                            if pattern.lower() in name:
                                ac_list.append(ac_name)
                                break
                return list(dict.fromkeys(ac_list))
        except Exception as e:
            debug_log(f"Shallow anti-cheat detection failed for {game_path}: {e}")
            return []

    def _is_community_verified(self, name: str, appid: str) -> bool:
        name_key = (name or '').lower().strip()
        appid_key = str(appid or '').strip()
        # Match by (name, appid) pair if available
        if (name_key, appid_key) in self.community_verified:
            return True
        # Match by name-only if present
        # If any entry matches the name regardless of appid, consider it verified
        if any(entry[0] == name_key for entry in self.community_verified):
            return True
        # Match by appid-only if present
        if any(entry[1] == appid_key and appid_key for entry in self.community_verified):
            return True
        return False

    def analyze_game_safety(self, game: Game) -> dict:
        """Return safety analysis info for a game (engine, anti_cheat_list, community_verified)
        This does lightweight checks without doing a deep scan.
        """
        try:
            gp = Path(game.path)
            engine = self._detect_engine_type(gp)
            anti_cheat_list = self._detect_anti_cheat(gp)
            community_verified = self._is_community_verified(game.name, str(game.appid or ''))
            engine_supported = compatibility_checker.is_engine_supported(engine)
            return {
                'engine': engine,
                'anti_cheat_list': anti_cheat_list,
                'community_verified': community_verified
                , 'engine_supported': engine_supported
            }
        except Exception as e:
            debug_log(f"Failed to analyze safety for {game}: {e}")
            return {'engine': 'Unknown', 'anti_cheat_list': [], 'community_verified': False}

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

    def _scan_installed_programs(self):
        """Scan Windows registry for installed programs and return Game objects.
        This covers classic Win32 installed programs under the Uninstall registry key.
        """
        games = []
        try:
            import winreg
            # Check both HKLM and HKCU uninstall keys (32/64-bit aware)
            uninstall_keys = [
                (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
                (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
                (winreg.HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
            ]

            for root, subkey in uninstall_keys:
                try:
                    with winreg.OpenKey(root, subkey) as key:
                        for i in range(0, winreg.QueryInfoKey(key)[0]):
                            try:
                                sk_name = winreg.EnumKey(key, i)
                                with winreg.OpenKey(key, sk_name) as sk:
                                    try:
                                        display_name = winreg.QueryValueEx(sk, 'DisplayName')[0]
                                    except Exception:
                                        display_name = None
                                    try:
                                        install_loc = winreg.QueryValueEx(sk, 'InstallLocation')[0]
                                    except Exception:
                                        install_loc = None
                                    try:
                                        display_icon = winreg.QueryValueEx(sk, 'DisplayIcon')[0]
                                    except Exception:
                                        display_icon = None

                                    # Normalize name & pick a path to check
                                    if not display_name:
                                        continue
                                    path_to_check = None
                                    if install_loc and Path(install_loc).exists():
                                        path_to_check = Path(install_loc)
                                    elif display_icon:
                                        # DisplayIcon may be like C:\Program Files\App\app.exe, strip args
                                        path_candidate = Path(display_icon.split(',')[0].strip('"'))
                                        if path_candidate.exists():
                                            path_to_check = path_candidate.parent
                                    # Guard against scanning system root folders or top-level Program Files
                                    root_blacklist = {Path("C:/Program Files"), Path("C:/Program Files (x86)"), Path("C:/Windows"), Path("C:/"), Path("C:/Program Files/Common Files")}
                                    # Skip scanning if path is too shallow (root or Program Files root)
                                    if not path_to_check:
                                        continue
                                    try:
                                        resolved = Path(path_to_check).resolve()
                                        if len(resolved.parts) <= 2:
                                            continue
                                    except Exception:
                                        # If we can't resolve, skip unsafe scanning
                                        continue
                                    if Path(path_to_check) and Path(path_to_check) not in root_blacklist and self._is_game_folder(path_to_check):
                                        optiscaler_installed = self._detect_optiscaler(path_to_check)
                                        # Use shallow safety check to avoid deep scans when enumerating installed programs
                                        engine = self._detect_engine_type(Path(path_to_check))
                                        anti_cheat_list = self._detect_anti_cheat_shallow(Path(path_to_check))
                                        community_verified = self._is_community_verified(display_name, '')
                                        engine_supported = compatibility_checker.is_engine_supported(engine)
                                        safety = {
                                            'engine': engine,
                                            'anti_cheat_list': anti_cheat_list,
                                            'community_verified': community_verified,
                                            'engine_supported': engine_supported
                                        }
                                        games.append(Game(name=display_name, path=str(path_to_check), image_path=None, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Registry'))
                            except Exception:
                                continue
                except (FileNotFoundError, PermissionError):
                    continue
        except Exception as e:
            debug_log(f"Registry scan failed: {e}")
        return games

    def _scan_appx_packages(self):
        """Scan Appx/UWP packages using PowerShell Get-AppxPackage command and return Game objects.
        This is a pragmatic approach that uses PowerShell's Get-AppxPackage -> ConvertTo-Json output.
        """
        games = []
        try:
            import subprocess, json
            # Use PowerShell to list appx packages and get Name, PackageFullName, InstallLocation
            ps_cmd = "Get-AppxPackage | Select Name, PackageFullName, InstallLocation | ConvertTo-Json"
            proc = subprocess.run(["powershell", "-NoProfile", "-Command", ps_cmd], capture_output=True, text=True, timeout=30)
            if proc.returncode != 0 or not proc.stdout:
                return games
            # Parse JSON; can be an array or single object
            data = json.loads(proc.stdout)
            if isinstance(data, dict):
                data = [data]
            for p in data:
                name = p.get('Name')
                install_loc = p.get('InstallLocation')
                if not name or not install_loc:
                    continue
                if Path(install_loc).exists() and self._is_game_folder(install_loc):
                    optiscaler_installed = self._detect_optiscaler(install_loc)
                    # Use shallow detections for Appx packages
                    engine = self._detect_engine_type(Path(install_loc))
                    anti_cheat_list = self._detect_anti_cheat_shallow(Path(install_loc))
                    community_verified = self._is_community_verified(name, '')
                    engine_supported = compatibility_checker.is_engine_supported(engine)
                    safety = {
                        'engine': engine,
                        'anti_cheat_list': anti_cheat_list,
                        'community_verified': community_verified,
                        'engine_supported': engine_supported
                    }
                    games.append(Game(name=name, path=install_loc, image_path=None, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Appx'))
        except Exception as e:
            debug_log(f"Appx scan failed: {e}")
        return games

    @timed("game_scan")
    def scan_games(self, force_refresh: bool = False):
        # Return cached games if available and a forced refresh was not requested
        try:
            if not force_refresh and self._cached_games is not None:
                debug_log(f"Using cached game list ({len(self._cached_games)} games)")
                return list(self._cached_games)
        except Exception:
            pass
        # Perform cache cleanup before scanning
        cache_manager.cleanup_large_cache()
        
        all_games = []
        all_games.extend(self._scan_steam_games())
        all_games.extend(self._scan_epic_games())
        all_games.extend(self._scan_gog_games())
        all_games.extend(self._scan_xbox_games())
        all_games.extend(self._scan_heroic_games())
        # Add a fast discovery step on Windows that uses PowerShell to find library roots
        try:
            start_lib = time.time()
            libraries = get_game_libraries(use_powershell=True, force_refresh=force_refresh)
            self.last_library_scan_seconds = time.time() - start_lib
            debug_log(f"Library discovery found {len(libraries)} roots in {self.last_library_scan_seconds:.2f}s")
            self.last_library_summary = compute_library_summary(libraries)
            from utils.config import get_config_value
            excluded = get_config_value('excluded_drives', '') or ''
            excluded_list = [e.strip().upper() for e in str(excluded).split(',') if e.strip()]
            for lib in libraries:
                try:
                    launcher = lib.get('Launcher')
                    lib_path = lib.get('Path')
                    if not lib_path:
                        continue
                    drive_letter = (lib.get('Drive') or lib_path[0]).upper().strip() if lib.get('Drive') or lib_path else None
                    if drive_letter and drive_letter.replace(':','').upper() in excluded_list:
                        debug_log(f"Skipping library root on excluded drive: {lib_path}")
                        continue
                    p = Path(lib_path)
                    if launcher == 'Steam' and (p.exists() and p.is_dir()):
                        # Steam library - normally steamapps/common
                        common = p
                        library_path = common.parent
                        try:
                            st_games = self._scan_steam_common_folder(common, library_path)
                            all_games.extend(st_games)
                        except Exception as e:
                            debug_log(f"Failed scanning Steam library {common}: {e}")
                    elif launcher == 'Epic' and p.exists() and p.is_dir():
                        # Adapt the Epic scanning: process directories under the root
                        try:
                            for gf in p.iterdir():
                                if not gf.is_dir():
                                    continue
                                if self._is_game_folder(gf):
                                    game_name = self._read_epic_game_name(gf)
                                    if not game_name:
                                        raw = gf.name.replace("_", " ").replace("-", " ")
                                        game_name = self._split_camel_case(raw).title()
                                    image_path = self.fetch_game_image(game_name)
                                    optiscaler_installed = self._detect_optiscaler(gf)
                                    safety = self.analyze_game_safety(Game(game_name, str(gf)))
                                    all_games.append(Game(name=game_name, path=str(gf), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Epic'))
                        except Exception as e:
                            debug_log(f"Failed scanning Epic root {p}: {e}")
                    elif launcher == 'GOG' and p.exists() and p.is_dir():
                        try:
                            for gf in p.iterdir():
                                if not gf.is_dir():
                                    continue
                                if self._is_game_folder(gf):
                                    # Use GOG metadata for proper name
                                    game_name = gf.name.replace("_", " ").replace("-", " ").title()
                                    info_files = list(gf.glob("goggame-*.info"))
                                    if info_files:
                                        try:
                                            with open(info_files[0], 'r', encoding='utf-8') as _f:
                                                _gi = json.load(_f)
                                            game_name = _gi.get("gameTitle", game_name)
                                        except Exception:
                                            pass
                                    image_path = self.fetch_game_image(game_name)
                                    optiscaler_installed = self._detect_optiscaler(gf)
                                    safety = self.analyze_game_safety(Game(game_name, str(gf)))
                                    all_games.append(Game(name=game_name, path=str(gf), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='GOG'))
                        except Exception as e:
                            debug_log(f"Failed scanning GOG root {p}: {e}")
                    elif launcher == 'Xbox' and p.exists() and p.is_dir():
                        try:
                            is_xbx = p.name.lower() == 'xboxgames'
                            is_wapps = p.name.lower() == 'windowsapps'
                            for gf in p.iterdir():
                                if not gf.is_dir():
                                    continue
                                if is_xbx:
                                    is_game = self._is_game_folder(gf) or self._is_xbox_game_folder(gf)
                                    if not is_game:
                                        continue
                                    game_name = gf.name.replace("_", " ").replace("-", " ").title()
                                elif is_wapps:
                                    if not self._is_appx_game_candidate(gf.name):
                                        continue
                                    if not self._is_game_folder(gf):
                                        continue
                                    game_name = self._parse_appx_package_name(gf.name).title()
                                    if not game_name or len(game_name) < 2:
                                        continue
                                else:
                                    if not self._is_game_folder(gf):
                                        continue
                                    game_name = gf.name.replace("_", " ").replace("-", " ").title()
                                image_path = self.fetch_game_image(game_name)
                                optiscaler_installed = self._detect_optiscaler(gf)
                                safety = self.analyze_game_safety(Game(game_name, str(gf)))
                                all_games.append(Game(name=game_name, path=str(gf), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Xbox'))
                        except Exception as e:
                            debug_log(f"Failed scanning Xbox root {p}: {e}")
                except Exception as e:
                    debug_log(f"Failed processing library entry {lib}: {e}")
        except Exception as e:
            debug_log(f"Library root discovery failed: {e}")

        # Filter out launchers and pure demo entries
        _launcher_keywords = ('launcher', 'redistributable', 'directx', 'vcredist',
                               'dotnet', 'steamworks common')
        all_games = [
            g for g in all_games
            if not any(kw in g.name.lower() for kw in _launcher_keywords)
        ]

        # Deduplicate: prefer the entry that already has an image.
        # Primary key: (name, path). Secondary: (name, platform) so the same
        # game detected from both C:\XboxGames and WindowsApps isn't shown twice.
        unique_games = {}
        name_platform_seen = {}
        for game in all_games:
            normalized_name = game.name.lower().strip()
            normalized_path = os.path.normpath(game.path).lower()
            path_key = (normalized_name, normalized_path)
            plat_key = (normalized_name, (game.platform or '').lower())

            if path_key in unique_games:
                # Keep whichever entry has a real image
                existing = unique_games[path_key]
                no_img = str(self.no_image_path)
                if (not existing.image_path or existing.image_path == no_img) and game.image_path and game.image_path != no_img:
                    unique_games[path_key] = game
                continue

            if plat_key in name_platform_seen:
                # Same name+platform from a different path — skip the duplicate
                continue

            unique_games[path_key] = game
            name_platform_seen[plat_key] = True
        
        debug_log(f"Scan complete: Found {len(unique_games)} unique games")
        result = list(unique_games.values())
        # Cache scan result for subsequent calls
        try:
            self._cached_games = list(result)
        except Exception:
            self._cached_games = None
        return result

    def clear_cached_games(self):
        """Clear cached game scan results (used when forcing UI refresh or cache invalidation)."""
        try:
            self._cached_games = None
        except Exception:
            pass

    def get_cached_games(self):
        """Return the cached games list or None if not cached."""
        return list(self._cached_games) if self._cached_games is not None else None

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
                    optiscaler_installed = self._detect_optiscaler(game_path)
                    safety = self.analyze_game_safety(Game(name, str(game_path), appid=appid))
                    # Ensure platform is set so UI shows the correct launcher tag
                    return Game(name=name, path=str(game_path), appid=appid, image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Steam')
            
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

    @timed("scan_steam_common_folder")
    def _scan_steam_common_folder(self, common_path, library_path):
        """Scan a Steam common folder for games"""
        games = []
        steamapps_path = library_path / "steamapps"
        # Process game folders concurrently to utilize multiple cores for I/O-bound operations
        # Determine reasonable worker count
        import concurrent.futures, os
        max_workers = getattr(config, 'max_workers', min(8, (os.cpu_count() or 1) * 4))

        def process_game_folder(game_folder):
            try:
                if not game_folder.is_dir() or not self._is_game_folder(game_folder):
                    return None

                game_info = self._find_steam_game_info(game_folder.name, steamapps_path)
                if game_info:
                    name, appid = game_info
                    image_path = self.fetch_game_image(name, appid)
                    optiscaler_installed = self._detect_optiscaler(game_folder)
                    safety = self.analyze_game_safety(Game(name, str(game_folder), appid=appid))
                    return Game(name=name, path=str(game_folder), appid=appid, image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Steam')
                else:
                    image_path = self.fetch_game_image(game_folder.name)
                    optiscaler_installed = self._detect_optiscaler(game_folder)
                    safety = self.analyze_game_safety(Game(game_folder.name, str(game_folder)))
                    return Game(name=game_folder.name, path=str(game_folder), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Steam')
            except Exception as e:
                debug_log(f"Error processing game folder {game_folder}: {e}")
                return None

        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [executor.submit(process_game_folder, gf) for gf in common_path.iterdir()]
            for future in concurrent.futures.as_completed(futures):
                game = future.result()
                if game:
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

    @timed("scan_epic_games")
    def _scan_epic_games(self):
        """Scan Epic Games installations with Path objects"""
        epic_games = []
        for epic_path_str in self.epic_games_paths:
            try:
                epic_path = Path(epic_path_str)
                if not epic_path.exists():
                    continue

                import concurrent.futures, os
                max_workers = getattr(config, 'max_workers', min(8, (os.cpu_count() or 1) * 4))

                def process_epic_folder(game_folder):
                    try:
                        if not game_folder.is_dir() or not self._is_game_folder(str(game_folder)):
                            return None
                        has_egstore = (game_folder / ".egstore").exists()
                        has_manifest = any(f.suffix == ".mancfg" for f in game_folder.glob("*.mancfg"))
                        if has_egstore or has_manifest:
                            # First try to read the actual game title from Epic metadata
                            game_name = self._read_epic_game_name(game_folder)
                            if not game_name:
                                # Fallback: split CamelCase folder name then title-case it
                                raw = game_folder.name.replace("_", " ").replace("-", " ")
                                game_name = self._split_camel_case(raw).title()
                            image_path = self.fetch_game_image(game_name)
                            optiscaler_installed = self._detect_optiscaler(game_folder)
                            safety = self.analyze_game_safety(Game(game_name, str(game_folder)))
                            return Game(name=game_name, path=str(game_folder), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Epic')
                        return None
                    except Exception as e:
                        debug_log(f"Error processing Epic folder {game_folder}: {e}")
                        return None

                with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
                    futures = [executor.submit(process_epic_folder, gf) for gf in epic_path.iterdir()]
                for future in concurrent.futures.as_completed(futures):
                    game = future.result()
                    if game:
                        epic_games.append(game)
                        
            except (OSError, PermissionError) as e:
                debug_log(f"Error scanning Epic Games path {epic_path}: {e}")
                continue
                
        return epic_games

    @timed("scan_gog_games")
    def _scan_gog_games(self):
        """Scan GOG installations with Path objects"""
        gog_games = []
        for gog_path_str in self.gog_paths:
            try:
                gog_path = Path(gog_path_str)
                if not gog_path.exists():
                    continue
                    
                # Parallelize GOG folder scanning
                import concurrent.futures, os
                max_workers = getattr(config, 'max_workers', min(8, (os.cpu_count() or 1) * 4))

                def process_gog_folder(game_folder):
                    try:
                        if not game_folder.is_dir() or not self._is_game_folder(str(game_folder)):
                            return None

                        # Look for goggame-*.info files
                        info_files = list(game_folder.glob("goggame-*.info"))
                        if info_files:
                            try:
                                with open(info_files[0], 'r', encoding='utf-8') as f:
                                    game_info = json.load(f)
                                    game_name = game_info.get("gameTitle", game_folder.name.replace("_", " ").replace("-", " ").title())
                                    image_path = self.fetch_game_image(game_name)
                                    optiscaler_installed = self._detect_optiscaler(game_folder)
                                    safety = self.analyze_game_safety(Game(game_name, str(game_folder)))
                                    return Game(name=game_name, path=str(game_folder), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='GOG')
                            except (json.JSONDecodeError, UnicodeDecodeError, KeyError) as e:
                                debug_log(f"Error parsing GOG info file {info_files[0].name}: {e}")

                        # Fallback: use folder name
                        game_name = game_folder.name.replace("_", " ").replace("-", " ").title()
                        optiscaler_installed = self._detect_optiscaler(game_folder)
                        safety = self.analyze_game_safety(Game(game_name, str(game_folder)))
                        return Game(name=game_name, path=str(game_folder), optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Epic')
                    except Exception as e:
                        debug_log(f"Error processing GOG folder {game_folder}: {e}")
                        return None

                with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
                    futures = [executor.submit(process_gog_folder, gf) for gf in gog_path.iterdir()]
                    for future in concurrent.futures.as_completed(futures):
                        game = future.result()
                        if game:
                            gog_games.append(game)
                    
            except (OSError, PermissionError) as e:
                debug_log(f"Error scanning GOG path {gog_path}: {e}")
                continue
                
        return gog_games

    # Xbox streaming/packaging file extensions that identify a Game Pass title in C:\XboxGames
    _XBOX_GAME_EXTENSIONS = {'.xsp', '.smd', '.xct', '.xvi'}

    def _is_xbox_game_folder(self, game_folder: Path) -> bool:
        """Return True if the folder looks like an Xbox Game Pass title.
        Xbox games in C:\\XboxGames store packaging files (.xsp, .smd, .xct, .xvi)
        and/or a gamelaunchhelper.exe at the top level, not traditional game binaries."""
        try:
            for item in game_folder.iterdir():
                if item.suffix.lower() in self._XBOX_GAME_EXTENSIONS:
                    return True
                if item.name.lower() == 'gamelaunchhelper.exe':
                    return True
                # Some Xbox games have a Content subfolder with actual game data
                if item.name.lower() == 'content' and item.is_dir():
                    return True
        except (PermissionError, OSError):
            pass
        return False

    @timed("scan_xbox_games")
    def _scan_xbox_games(self):
        """Scan Xbox Games installations with Path objects"""
        xbox_games = []
        for xbox_path_str in self.xbox_paths:
            try:
                xbox_path = Path(xbox_path_str)
                if not xbox_path.exists():
                    continue

                import concurrent.futures, os
                max_workers = getattr(config, 'max_workers', min(8, (os.cpu_count() or 1) * 4))
                is_xboxgames_root = xbox_path.name.lower() == 'xboxgames'
                is_windowsapps = xbox_path.name.lower() == 'windowsapps'

                def process_xbox_folder(game_folder, _is_xbx=is_xboxgames_root, _is_wapps=is_windowsapps):
                    try:
                        if not game_folder.is_dir():
                            return None

                        if _is_xbx:
                            # C:\XboxGames — accept folders that pass standard check OR Xbox packaging check
                            is_game = self._is_game_folder(str(game_folder)) or self._is_xbox_game_folder(game_folder)
                            if not is_game:
                                return None
                            game_name = game_folder.name.replace("_", " ").replace("-", " ").title()
                        elif _is_wapps:
                            # C:\Program Files\WindowsApps — Appx package folder names
                            folder_name = game_folder.name
                            # Skip known non-game packages quickly
                            if not self._is_appx_game_candidate(folder_name):
                                return None
                            # Require at least a standard game folder check
                            if not self._is_game_folder(str(game_folder)):
                                return None
                            # Parse readable name from Publisher.AppName_Version_Arch_Hash
                            game_name = self._parse_appx_package_name(folder_name).title()
                            if not game_name or len(game_name) < 2:
                                return None
                        else:
                            if not self._is_game_folder(str(game_folder)):
                                return None
                            game_name = game_folder.name.replace("_", " ").replace("-", " ").title()

                        image_path = self.fetch_game_image(game_name)
                        optiscaler_installed = self._detect_optiscaler(game_folder)
                        safety = self.analyze_game_safety(Game(game_name, str(game_folder)))
                        return Game(name=game_name, path=str(game_folder), image_path=image_path, optiscaler_installed=optiscaler_installed, engine=safety['engine'], anti_cheat_list=safety['anti_cheat_list'], community_verified=safety['community_verified'], engine_supported=safety.get('engine_supported', True), platform='Xbox')
                    except Exception as e:
                        debug_log(f"Error processing Xbox folder {game_folder}: {e}")
                        return None

                with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
                    futures = [executor.submit(process_xbox_folder, gf) for gf in xbox_path.iterdir()]
                    for future in concurrent.futures.as_completed(futures):
                        game = future.result()
                        if game:
                            xbox_games.append(game)

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
                # If still no appid, return placeholder and log for diagnostics
                debug_log(f"No Steam AppID for '{game_name}'; using placeholder image")
                return str(self.no_image_path)

        # Check if image already exists in cache.
        # Prefer appid-keyed file (accurate) then fall back to name-keyed file (legacy).
        cache_dir = Path(self.game_cache_dir)
        cache_stems = [f"appid_{appid}"] if appid else []
        safe_name = re.sub(r'[<>:"/\\|?*]', '_', game_name)
        cache_stems.append(safe_name)

        for stem in cache_stems:
            for ext in ['jpg', 'png', 'jpeg', 'webp']:
                cached_path = cache_dir / f"{stem}.{ext}"
                if cached_path.exists():
                    return str(cached_path)

        def _download_and_cache_image(url, label):
            """Download image from url, resize, save to cache. Returns path string or None."""
            try:
                resp = self._requests_session.get(url, stream=True, timeout=config.image_download_timeout)
                resp.raise_for_status()
                image = Image.open(BytesIO(resp.content))
                image.thumbnail(config.max_image_size, Image.Resampling.LANCZOS)
                if image.mode in ('RGBA', 'LA', 'P'):
                    rgb_image = Image.new('RGB', image.size, (255, 255, 255))
                    rgb_image.paste(image, mask=image.split()[-1] if image.mode == 'RGBA' else None)
                    image = rgb_image
                out_path = cache_dir / f"appid_{appid}.jpg"
                image.save(out_path, 'JPEG', quality=config.image_quality, optimize=True)
                debug_log(f"Fetched Steam image for {game_name} (AppID: {appid}) via {label} -> {out_path}")
                return str(out_path)
            except Exception as e:
                debug_log(f"Image download failed ({label}) for {game_name} (AppID: {appid}): {e}")
                return None

        try:
            # Primary: Steam CDN Akamai header.jpg
            steam_image_url = f"https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"
            response = self._requests_session.get(steam_image_url, stream=True, timeout=config.image_download_timeout)
            if response.status_code == 200:
                image = Image.open(BytesIO(response.content))
                image.thumbnail(config.max_image_size, Image.Resampling.LANCZOS)
                if image.mode in ('RGBA', 'LA', 'P'):
                    rgb_image = Image.new('RGB', image.size, (255, 255, 255))
                    rgb_image.paste(image, mask=image.split()[-1] if image.mode == 'RGBA' else None)
                    image = rgb_image
                image_path = cache_dir / f"appid_{appid}.jpg"
                image.save(image_path, 'JPEG', quality=config.image_quality, optimize=True)
                debug_log(f"Fetched Steam image for {game_name} (AppID: {appid}) via CDN -> {image_path}")
                return str(image_path)

            # Fallback: Steam Store API — gets the actual hosted image URL for demos/DLC/edge cases
            debug_log(f"CDN returned {response.status_code} for {game_name} (AppID: {appid}), trying Store API")
            store_url = f"https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic"
            store_resp = self._requests_session.get(store_url, timeout=config.image_download_timeout)
            if store_resp.status_code == 200:
                store_data = store_resp.json()
                app_info = store_data.get(str(appid), {})
                if app_info.get('success'):
                    header_image = app_info.get('data', {}).get('header_image')
                    if header_image:
                        result = _download_and_cache_image(header_image, "Store API header_image")
                        if result:
                            return result
                    # Also try capsule_image if header_image not found/failed
                    capsule_image = app_info.get('data', {}).get('capsule_image')
                    if capsule_image:
                        result = _download_and_cache_image(capsule_image, "Store API capsule_image")
                        if result:
                            return result

        except (requests.RequestException, OSError, Image.UnidentifiedImageError) as e:
            debug_log(f"Error fetching Steam image for {game_name} (AppID: {appid}): {e}")

        # Return placeholder if no image found
        return str(self.no_image_path)

    def _build_local_steam_app_list(self):
        """Build a name→AppID map from locally installed Steam manifest files.
        This is instant (no network), covers all games the user has installed on Steam,
        and is used immediately on startup while the full SteamSpy list downloads."""
        # Check disk cache first (populated by the SteamSpy background loader)
        app_list_cache_path = Path(config.steam_app_list_cache_path)
        if app_list_cache_path.exists():
            cache_age_days = (time.time() - app_list_cache_path.stat().st_mtime) / (24 * 60 * 60)
            if cache_age_days < config.steam_app_list_cache_days:
                try:
                    with open(app_list_cache_path, 'r', encoding='utf-8') as f:
                        apps = json.load(f)
                    self._normalized_steam_app_map = {
                        re.sub(r'\s+', ' ', re.sub(r'[^a-z0-9\s]', ' ', name)).strip(): appid
                        for name, appid in apps.items()
                    }
                    debug_log(f"Loaded {len(apps)} Steam apps from disk cache")
                    return apps
                except Exception as e:
                    debug_log(f"Cache read error, falling back to local manifests: {e}")

        apps = {}
        normalized_apps = {}
        steam_dirs = []

        # Collect all Steam library paths from libraryfolders.vdf
        for steam_path in self.steam_paths:
            lf = Path(steam_path) / 'steamapps' / 'libraryfolders.vdf'
            if lf.exists():
                try:
                    data = vdf.load(open(lf, encoding='utf-8', errors='replace'))
                    for v in data.get('libraryfolders', {}).values():
                        if isinstance(v, dict) and 'path' in v:
                            steam_dirs.append(str(Path(v['path']) / 'steamapps'))
                except Exception:
                    pass
            steam_dirs.append(str(Path(steam_path) / 'steamapps'))

        for sdir in steam_dirs:
            sd = Path(sdir)
            if not sd.exists():
                continue
            for mf in sd.glob('appmanifest_*.acf'):
                try:
                    data = vdf.load(open(mf, encoding='utf-8', errors='replace'))
                    state = data.get('AppState', {})
                    name = (state.get('name') or '').strip()
                    appid = str(state.get('appid') or '')
                    if name and appid:
                        key = name.lower()
                        apps[key] = appid
                        norm = re.sub(r'\s+', ' ', re.sub(r'[^a-z0-9\s]', ' ', key)).strip()
                        normalized_apps[norm] = appid
                except Exception:
                    pass

        self._normalized_steam_app_map = normalized_apps
        debug_log(f"Built local Steam app list from manifests: {len(apps)} games")
        return apps

    def _load_steamspy_app_list_async(self):
        """Background thread: download the SteamSpy catalogue page by page and
        merge it into steam_app_list.  Persists to disk so subsequent launches
        are instant.  Calls self.on_app_list_ready() when done so the UI can
        schedule a thumbnail retry for games that had no image."""
        app_list_cache_path = Path(config.steam_app_list_cache_path)
        # Skip download if cache is fresh
        if app_list_cache_path.exists():
            cache_age_days = (time.time() - app_list_cache_path.stat().st_mtime) / (24 * 60 * 60)
            if cache_age_days < config.steam_app_list_cache_days:
                debug_log("SteamSpy cache is fresh, skipping download")
                return

        debug_log("Downloading SteamSpy app catalogue in background...")
        apps = dict(self.steam_app_list)  # start from the local manifests
        normalized_apps = dict(getattr(self, '_normalized_steam_app_map', {}))

        try:
            page = 0
            while True:
                url = f"https://steamspy.com/api.php?request=all&page={page}"
                try:
                    resp = requests.get(url, timeout=15)
                    resp.raise_for_status()
                    data = resp.json()
                except Exception as e:
                    debug_log(f"SteamSpy page {page} failed: {e}")
                    break
                if not data:
                    break
                for appid_str, info in data.items():
                    name = (info.get('name') or '').strip()
                    if not name or len(name) < 2:
                        continue
                    if re.match(r'^(dlc|demo|beta|test)\b', name.lower()):
                        continue
                    key = name.lower()
                    apps[key] = appid_str
                    norm = re.sub(r'\s+', ' ', re.sub(r'[^a-z0-9\s]', ' ', key)).strip()
                    normalized_apps[norm] = appid_str
                debug_log(f"SteamSpy page {page}: {len(data)} entries, total {len(apps)}")
                page += 1
                if page > 50:  # safety cap — 50 000 games is ample
                    break
                time.sleep(0.5)  # be polite to the API

        except Exception as e:
            debug_log(f"SteamSpy download aborted: {e}")

        # Persist to disk
        try:
            app_list_cache_path.parent.mkdir(parents=True, exist_ok=True)
            with open(app_list_cache_path, 'w', encoding='utf-8') as f:
                json.dump(apps, f, separators=(',', ':'))
            debug_log(f"SteamSpy catalogue saved: {len(apps)} games")
        except Exception as e:
            debug_log(f"Failed to save SteamSpy cache: {e}")

        # Update in-memory maps atomically
        self.steam_app_list = apps
        self._normalized_steam_app_map = normalized_apps
        debug_log("Steam app list ready — notifying UI for thumbnail retry")

        # Notify the UI so it can re-fetch images for games that were missing them
        if callable(self.on_app_list_ready):
            try:
                self.on_app_list_ready()
            except Exception as e:
                debug_log(f"on_app_list_ready callback error: {e}")

    def _read_epic_game_name(self, game_folder: Path) -> str:
        """Try to read the real game title from Epic Games metadata files.
        Returns the game name string, or empty string if not found."""
        try:
            # Check .egstore folder for .mancfg manifest files
            egstore = game_folder / '.egstore'
            if egstore.exists():
                for mf in egstore.glob('*.mancfg'):
                    try:
                        with open(mf, 'r', encoding='utf-8', errors='ignore') as f:
                            data = json.load(f)
                        title = data.get('DisplayName') or data.get('AppName') or ''
                        if title and len(title) > 1:
                            return title.strip()
                    except Exception:
                        pass
            # Also check top-level .mancfg files
            for mf in game_folder.glob('*.mancfg'):
                try:
                    with open(mf, 'r', encoding='utf-8', errors='ignore') as f:
                        data = json.load(f)
                    title = data.get('DisplayName') or data.get('AppName') or ''
                    if title and len(title) > 1:
                        return title.strip()
                except Exception:
                    pass
        except Exception:
            pass
        return ''

    @staticmethod
    def _split_camel_case(name: str) -> str:
        """Split a CamelCase or PascalCase identifier into space-separated words.
        E.g. 'Reddeadredemption2' -> 'Red Dead Redemption 2'
             'HaloInfinite' -> 'Halo Infinite'
        """
        # Insert space before uppercase letters that follow lowercase letters
        s = re.sub(r'([a-z])([A-Z])', r'\1 \2', name)
        # Insert space before a digit sequence that follows a letter
        s = re.sub(r'([a-zA-Z])(\d)', r'\1 \2', s)
        # Insert space before a letter that follows a digit
        s = re.sub(r'(\d)([a-zA-Z])', r'\1 \2', s)
        return s

    @staticmethod
    def _parse_appx_package_name(folder_name: str) -> str:
        """Extract a human-readable game name from a WindowsApps folder name.
        Folder names follow the pattern: Publisher.AppName_Version_Arch_Hash
        or Publisher.AppName_Version_Arch__Hash (double underscore).
        Returns the AppName portion with CamelCase split.
        """
        # Strip version/arch/hash suffix: everything after the first underscore that looks like a version
        # Pattern: Name_1.2.3.4_arch__hash or Name_1.2.3.4_arch_hash
        base = re.sub(r'_[\d]+\.\d+.*$', '', folder_name)
        # Remove Publisher. prefix if present (e.g. 'Hoodedhorse.Manorlords' -> 'Manorlords')
        if '.' in base:
            # Take the part after the last dot as the app name
            base = base.split('.')[-1]
        # Split CamelCase
        readable = GameScanner._split_camel_case(base)
        return readable.strip()

    # Known non-game keyword fragments present in Appx package names (Publisher.AppName part only)
    # These match runtimes, codecs, system utilities, and other non-game packages.
    _APPX_NON_GAME_KEYWORDS = (
        'vclibs', 'runtime', 'framework', 'xaml', 'webpimage', 'hevcvideo',
        'heifimage', 'mpeg2video', 'vp9video', 'webmedia', 'codec', 'videoextension',
        'imageextension', 'storepurchase', 'storeengagement', 'store.engagement',
        'directxruntime',
        'net.native', 'appruntime', 'winappruntime', 'windowsappruntime',
        'gamingservices', 'xboxgamingoverlay', 'gamingapp', 'xboxidentityprovider',
        'xboxdevices', 'xbox.tcui', 'bingwallpaper', 'getstarted', 'gethelp',
        'startexperiencesapp', 'commandpalette', 'webexperience', 'crossdevice',
        'sechealthui', 'screensketch', 'widgetsplatform', 'foundrylocal',
        'applicationcompatibility', 'avcencoder', 'windowsstore', 'storepurchaseapp',
        'yourphone', 'windowsterminal', 'windowsnotepad', 'windowscalculator',
        'windowscamera', 'windowsphotos', 'windowsappruntime', 'zunemusic',
        'microsoftmahjong', 'candycrush', 'linkedinfor', 'whatsappdesktop',
        'clipchamp', 'dolbyaccess', 'dtssound', 'armourycrate', 'lgmonitor',
        'realtekaudio', 'tobiieyetracking', 'speedtest', 'hdrcalibration',
        'camostudio', 'icloud', 'itunes', 'applemusic', 'appletv',
        'primevideo', 'vidstok', 'gameassist', 'paint', 'photos',
        'minecraftuwp', 'minecraftlauncher',
        'facebook', 'netflix', 'desktopappinstaller', 'languageexperience',
        'solitairecollection', 'onedrivesync', 'powertoys', 'microsoftedge',
        'preordercontent', 'cloudspremium', 'anthropic', 'claude_',
    )

    def _is_appx_game_candidate(self, folder_name: str) -> bool:
        """Return True if the WindowsApps folder looks like it could be a game.
        Filters out known system runtimes, utilities, and non-game apps."""
        lower = folder_name.lower()
        # WindowsApps packages should have a version number (x.y.z.w); skip folders that don't
        # This filters out 'Deleted', 'Merged', 'DeletedAllUserPackages' etc.
        if '_' not in folder_name:
            return False
        # Skip packages whose Publisher.Name portion starts with only digits
        # (hex/numeric app IDs like '1Ed5Aea5.4160926B82Db', '549981C3F5F10' — clearly not games)
        name_part = lower.split('_')[0]  # strip version suffix
        app_part = name_part.split('.')[-1]  # take the AppName after last dot
        if app_part and re.match(r'^[0-9a-f]{6,}$', app_part.replace(' ', '')):
            return False  # looks like a hex identifier, not a readable name
        # Skip known non-game keywords anywhere in the package name
        for kw in self._APPX_NON_GAME_KEYWORDS:
            if kw in lower:
                return False
        return True

    # Platform/edition qualifiers appended to game names by stores/launchers.
    # We strip these progressively to find the base Steam title.
    _NAME_STRIP_SUFFIXES = re.compile(
        r'\s+(for\s+windows|for\s+pc|for\s+xbox|game\s+of\s+the\s+year\s+edition|'
        r'goty\s+edition|complete\s+edition|definitive\s+edition|enhanced\s+edition|'
        r'remastered|standard\s+edition|gold\s+edition|deluxe\s+edition|'
        r'ultimate\s+edition|windows\s+edition|pc\s+edition|microsoft\s+store)$',
        re.IGNORECASE,
    )

    def _get_appid_from_name(self, game_name):
        """Get Steam AppID from game name with fallback handling"""
        normalized_game_name = (game_name or '').lower().strip()
        if not normalized_game_name:
            return None

        # Build the list of name variants to try (strip platform/edition suffixes progressively)
        variants = [normalized_game_name]
        stripped = normalized_game_name
        while True:
            new = self._NAME_STRIP_SUFFIXES.sub('', stripped).strip()
            if new == stripped or not new:
                break
            variants.append(new)
            stripped = new

        for candidate in variants:
            # 1. Exact match
            if candidate in self.steam_app_list:
                return self.steam_app_list[candidate]

            # 2. Normalized (punctuation-stripped) match
            norm = re.sub(r'\s+', ' ', re.sub(r'[^a-z0-9\s]', ' ', candidate)).strip()
            if hasattr(self, '_normalized_steam_app_map') and norm in self._normalized_steam_app_map:
                return self._normalized_steam_app_map[norm]

            # 3. Token-subset match: all query tokens must appear in the Steam name
            tokens = set(norm.split())
            if not tokens:
                continue
            best_match = None
            best_score = 0
            for name_key, appid in (self.steam_app_list.items() if self.steam_app_list else []):
                key_norm = re.sub(r'[^a-z0-9\s]', ' ', name_key).strip()
                key_tokens = set(key_norm.split())
                if not key_tokens:
                    continue
                if not tokens.issubset(key_tokens):
                    continue
                score = len(key_tokens)
                if best_match is None or score < best_score:
                    best_score = score
                    best_match = appid
            if best_match:
                return best_match

        # 4. Last resort: difflib fuzzy match on the original normalized name
        norm_orig = re.sub(r'\s+', ' ', re.sub(r'[^a-z0-9\s]', ' ', normalized_game_name)).strip()
        if len(norm_orig) >= 4 and hasattr(self, '_normalized_steam_app_map') and self._normalized_steam_app_map:
            try:
                import difflib
                candidates = list(self._normalized_steam_app_map.keys())
                matches = difflib.get_close_matches(norm_orig, candidates, n=1, cutoff=0.82)
                if matches:
                    appid = self._normalized_steam_app_map[matches[0]]
                    debug_log(f"Found appid via fuzzy match for '{game_name}' -> '{matches[0]}' -> {appid}")
                    return appid
            except Exception:
                pass

        debug_log(f"No Steam AppID found for '{game_name}'")
        return None