import json
import subprocess
import shutil
import time
import os
from pathlib import Path
from typing import List, Dict
from utils.debug import debug_log
from utils.config import config
from utils.config import get_config_value
import psutil
import re
try:
    import winreg
except Exception:
    winreg = None
try:
    import vdf
except Exception:
    vdf = None


PWSH_CMD = (
    "Get-InstalledPrograms | Get-DetectedLaunchers | Get-ActiveDrives | Find-GameLibraries | ConvertTo-Json -Depth 4"
)

_CACHE_FILE = Path(config.cache_dir) / 'library_discovery.json'
def _get_cache_ttl():
    try:
        return int(getattr(config, 'library_discovery_cache_ttl', 86400))
    except Exception:
        return 86400


def _find_powershell_exe():
    # Prefer pwsh if available (PowerShell Core), otherwise fallback
    pwsh = shutil.which('pwsh') or shutil.which('powershell')
    return pwsh


def get_game_libraries_from_powershell(timeout: int = 30, force_refresh: bool = False) -> List[Dict]:
    """Run the PowerShell PoC to detect game libraries.

    Returns: list of {Launcher, Drive, Path, Source}
    """
    pwsh = _find_powershell_exe()
    if not pwsh:
        debug_log('PowerShell not found; cannot run GameLibraries PoC')
        return []

    # The PoC pipeline should be embedded here; for maintainability we call an inline PS script
    ps_script = r"Get-InstalledPrograms | Get-DetectedLaunchers | Get-ActiveDrives | Find-GameLibraries | ConvertTo-Json -Depth 4"
    # Try to use cached values if present and valid
    try:
        if not force_refresh and _CACHE_FILE.exists():
            try:
                with open(_CACHE_FILE, 'r', encoding='utf-8') as f:
                    cached = json.load(f)
                ts = cached.get('_timestamp', 0)
                if time.time() - ts < _get_cache_ttl():
                    debug_log('Using cached library discovery results')
                    return cached.get('data', [])
            except Exception as e:
                # Read error - ignore cache
                debug_log(f'Failed to read library discovery cache: {e}')

    except Exception:
        pass

    try:
        proc = subprocess.run([
            pwsh, '-NoProfile', '-NonInteractive', '-Command', ps_script
        ], capture_output=True, text=True, timeout=timeout)
        if proc.returncode != 0 or not proc.stdout:
            debug_log(f'PowerShell PoC returned non-zero or empty stdout: rc={proc.returncode} err={proc.stderr}')
            return []
        try:
            data = json.loads(proc.stdout)
            if isinstance(data, dict):
                data = [data]
            # Normalize keys
            normalized = []
            for entry in data:
                normalized.append({
                    'Launcher': entry.get('Launcher'),
                    'Drive': entry.get('Drive'),
                    'Path': entry.get('Path'),
                    'Source': entry.get('Source', 'Powershell')
                })
            # Normalize and deduplicate before returning
            normalized = [normalize_library_entry(x) for x in normalized]
            normalized = deduplicate_libraries(normalized)
            # Write cache (unless in force_refresh mode)
            if not force_refresh:
                try:
                    _CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
                    with open(_CACHE_FILE, 'w', encoding='utf-8') as f:
                        json.dump({'_timestamp': int(time.time()), 'data': normalized}, f, indent=2)
                except Exception as e:
                    debug_log(f'Failed to write library discovery cache: {e}')
            return normalized
        except Exception as e:
            debug_log(f'Failed to parse JSON from PowerShell PoC: {e}')
            return []
    except Exception as e:
        debug_log(f'Failed to run PowerShell PoC: {e}')
        return []


def get_game_libraries(use_powershell=True, timeout=30, force_refresh: bool = False) -> List[Dict]:
    # If not on Windows, return empty results
    if os.name != 'nt':
        debug_log('Not running on Windows; library discovery fallback only supports Windows at the moment')
        return []

    # If configured to use powershell discovery and available, try it first
    use_ps = bool(get_config_value('use_powershell_discovery', use_powershell)) and use_powershell
    if use_ps and _find_powershell_exe():
        res = get_game_libraries_from_powershell(timeout=timeout, force_refresh=force_refresh)
        if res:
            return res

    # Fallback to pure-Python discovery (registry + drive heuristics)
    debug_log('PowerShell not available or returned no results; running fallback discovery')
    try:
        return get_game_libraries_from_fallback(force_refresh=force_refresh)
    except Exception as e:
        debug_log(f'Fallback discovery failed: {e}')
        return []


def get_game_libraries_from_fallback(steam_root: str | None = None, force_refresh: bool = False) -> List[Dict]:
    """Fallback discovery: use registry and known drive paths to find game libraries.
    Returns a list of dictionaries matching the keys from the PoC output.
    """
    results = []
    try:
        reg_entries = _get_installed_programs_from_registry()
        for entry in reg_entries:
            install = entry.get('InstallLocation') or entry.get('InstallPath') or ''
            if not install:
                continue
            # Only keep entries that look like a common game/launcher location (shallow heuristic)
            lower = install.lower()
            if any(x in lower for x in ('steam', 'epic games', 'gog', 'steamapps', 'program files', 'games')):
                path = str(Path(install).resolve())
                drive = Path(path).drive
                results.append({'Launcher': entry.get('DisplayName', 'Unknown'), 'Drive': drive, 'Path': path, 'Source': 'RegistryFallback'})
    except Exception as e:
        debug_log(f'Error enumerating registry: {e}')

    # Also scan drives for common library folders using psutil
    try:
        excluded = get_config_value('excluded_drives', '') or ''
        excluded_set = set(p.strip().upper() for p in excluded.split(',') if p.strip())
        partitions = psutil.disk_partitions(all=False)
        known_folders = [r"\Program Files\Steam\steamapps\common", r"\Program Files (x86)\Steam\steamapps\common", r"\Epic Games", r"\GOG Games", r"\Program Files\GOG Galaxy\Games"]
        for p in partitions:
            mount = Path(p.mountpoint)
            drive_letter = str(mount.drive or mount.anchor)
            if drive_letter.upper().rstrip(':') in excluded_set:
                continue
            # Shallow check for well-known folders
            for f in known_folders:
                candidate = mount / f.strip('\\')
                if candidate.exists():
                    results.append({'Launcher': 'DriveScan', 'Drive': drive_letter, 'Path': str(candidate), 'Source': 'DriveScan'})
    except Exception as e:
        debug_log(f'Error scanning drives for libraries: {e}')

    # Include WinRT Appx packages (if winrt available)
    results = _include_appx_winrt_into_results(results)

    # Try to detect Steam install path from registry and include its libraries
    try:
        steam_install = steam_root or _find_steam_install_from_registry()
        if steam_install:
            steamapps = Path(steam_install) / 'steamapps'
            if steamapps.exists():
                # Add steam_app steam library and parent path
                common = steamapps / 'common'
                if common.exists():
                    results.append({'Launcher': 'Steam', 'Drive': Path(steam_install).drive, 'Path': str(common), 'Source': 'Registry'})
    except Exception as e:
        debug_log(f'Failed to include Steam path from registry: {e}')

    # Include any additional Steam libraries from libraryfolders.vdf if found
    try:
        results = _include_steam_vdf_libraries(results, steam_root=steam_root)
    except Exception as e:
        debug_log(f'Failed to include Steam VDF libraries: {e}')

    # Normalize and deduplicate results
    try:
        normalized = [normalize_library_entry(x) for x in results]
        normalized = deduplicate_libraries(normalized)
        return normalized
    except Exception as e:
        debug_log(f'Failed to normalize/deduplicate libraries: {e}')
        # Fallback dedupe by simple key
        unique = {}
        for r in results:
            key = (r.get('Drive'), r.get('Path'))
            unique[key] = r
        return list(unique.values())


def compute_library_summary(libraries: List[Dict]) -> Dict[str, int]:
    """Compute a simple summary for a list of library entries.
    Returns a dict: {'total': n, 'Powershell': x, 'Steam': y, ...}
    """
    summary = {'total': 0}
    for lib in libraries:
        summary['total'] += 1
        src = lib.get('Source') or lib.get('Launcher') or 'Unknown'
        summary[src] = summary.get(src, 0) + 1
    return summary


def normalize_library_path(path: str) -> str:
    """Normalize a library path: resolve symlinks, strip trailing slashes and normalize case for Windows.
    Returns a normalized string path for consistent deduplication across sources.
    """
    try:
        p = Path(path)
        # Use resolve(strict=False) so non-existing paths can be normalized too
        resolved = p.resolve(strict=False)
        # Convert backslashes and normalize case on Windows
        s = str(resolved)
        if os.name == 'nt':
            # Windows is case-insensitive; normalize to lower
            s = os.path.normcase(s)
        # Strip trailing slashes
        s = s.rstrip('\\/')
        return s
    except Exception:
        # Best effort fall back to returned normalized path using os.path.normpath
        try:
            s = os.path.normpath(path)
            if os.name == 'nt':
                s = os.path.normcase(s)
            return s.rstrip('\\/')
        except Exception:
            return path


def normalize_library_entry(entry: Dict) -> Dict:
    """Return a copy of a library entry with a normalized Path and Drive fields."""
    try:
        e = dict(entry)
        if e.get('Path'):
            # Keep the original user-facing path value (preserve case) but
            # store a normalized value for deduplication/lookup.
            orig_path = str(e['Path'])
            e['_NormalizedPath'] = normalize_library_path(orig_path)
            e['Path'] = orig_path
        if e.get('Drive'):
            # Normalize drive like 'C:' to uppercase and colon preserved
            d = str(e['Drive']).upper().replace(':', '')
            e['Drive'] = f"{d}:" if d else e['Drive']
        return e
    except Exception:
        return entry


def deduplicate_libraries(entries: List[Dict]) -> List[Dict]:
    """Deduplicate a list of library entries using normalized path and drive.
    The earliest occurrence is kept by default.
    """
    unique = {}
    for e in entries:
        try:
            ne = normalize_library_entry(e)
            # Use a normalized path for deduplication while returning the
            # user-facing original path value in the 'Path' field.
            key = (ne.get('Drive'), ne.get('_NormalizedPath') or normalize_library_path(ne.get('Path', '')))
            if key not in unique:
                unique[key] = ne
        except Exception:
            # Fallback: attempt to store raw entry
            key = (e.get('Drive'), e.get('Path'))
            if key not in unique:
                unique[key] = e
    return list(unique.values())


def _parse_steam_libraryfolders_vdf(vdf_file_path: str) -> List[str]:
    """Parse a Steam libraryfolders.vdf file and return a list of Steam library paths.
    Handles both older and newer VDF formats.
    """
    results = []
    try:
        p = Path(vdf_file_path)
        if not p.exists():
            return results
        # Read content to try different parsers
        with open(p, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        data = None
        # Try Valve VDF parsing if available
        if vdf is not None:
            try:
                # vdf.load expects a file-like object; vdf.loads parses a string
                vdf_data = vdf.loads(content)
                # If vdf_data does not include libraryfolders (common when JSON is passed), we discard it
                if isinstance(vdf_data, dict) and any(k.lower() == 'libraryfolders' for k in vdf_data.keys()):
                    data = vdf_data
                else:
                    data = None
            except Exception:
                # vdf failed to parse, we'll try JSON below
                data = None
        # If vdf parsing failed or didn't return libraryfolders, try JSON
        if data is None:
            try:
                import json as _json
                data = _json.loads(content)
            except Exception:
                # Could not parse the VDF or JSON content
                debug_log('Failed to parse libraryfolders.vdf as VDF or JSON')
                # Try a tiny ad-hoc parser for Valve KeyValue format if available
                try:
                    kv_res = _parse_steam_libraryfolders_vdf_kv(content)
                    return kv_res
                except Exception:
                    debug_log('Failed to parse libraryfolders.vdf as VDF, JSON, or KV')
                return results

        # Older format: { "libraryfolders": { "0": { "path" : "C:\\..." }, ... } }
        # Newer Steam format sometimes has path directly as keys or nested differently.
        lib_root = data.get('libraryfolders') or data.get('LibraryFolders') or data.get('libraryFolders') or {}
        if isinstance(lib_root, dict):
            for key, val in lib_root.items():
                if isinstance(val, dict):
                    path = val.get('path') or val.get('Path') or val.get('path')
                    if path:
                        results.append(str(Path(path)))
                elif isinstance(val, str):
                    # The new format sometimes stores just the path as string value
                    results.append(str(Path(val)))
    except Exception as e:
        debug_log(f'Failed to parse libraryfolders.vdf: {e}')
    return results


def _parse_steam_libraryfolders_vdf_kv(content: str) -> List[str]:
    """A tiny parser for the Valve KeyValue format specifically to extract library folder paths.
    This is a limited parser: it looks for 'path' entries under 'libraryfolders' and collects the values.
    It does not try to fully parse nested structures; it's only enough for a typical 'libraryfolders.vdf'.
    """
    results = []
    lines = [l.strip() for l in content.splitlines()]
    in_libraryfolders = False
    for i, line in enumerate(lines):
        if not in_libraryfolders:
            if line.lower().startswith('"libraryfolders"') or line.lower().startswith('libraryfolders'):
                in_libraryfolders = True
            continue
        # Break if we reach another top-level section
        if in_libraryfolders and (line.startswith('"') and '"' not in line.split()[0]):
            pass
        # Try to find 'path' entries e.g. "path"  "C:\\SteamLibrary"
        if '"path"' in line.lower() or line.lower().startswith('path'):
            # naive extraction: find the quoted value after path
            try:
                # find first quote after 'path'
                parts = line.split('"')
                if len(parts) >= 3:
                    val = parts[-2]
                else:
                    # maybe space-separated
                    val = line.split()[-1].strip('"')
                if val:
                    results.append(str(Path(val)))
            except Exception:
                continue
    return results



def _include_steam_vdf_libraries(results: list, steam_root: str | None = None):
    """Inspect any detected Steam install folder steam/steamapps/libraryfolders.vdf and add discovered library 'steamapps/common' entries."""
    try:
        # Look through results or detect common Steam paths
        paths_to_check = []
        for r in list(results):
            try:
                if r.get('Launcher') == 'Steam' and Path(r.get('Path', '')).exists():
                    # If the provided path is the 'steamapps/common' directory, get parent
                    p = Path(r.get('Path'))
                    if p.name.lower() == 'common':
                        paths_to_check.append(str(p))
                    else:
                        # Check if it contains 'steamapps' parent
                        if (p / 'steamapps').exists():
                            paths_to_check.append(str(p / 'steamapps' / 'common'))
            except Exception:
                continue

        # Also search for steam installs from registry (unless caller provided steam_root)
        try:
            steam_install = steam_root or _find_steam_install_from_registry()
            if steam_install:
                paths_to_check.append(str(Path(steam_install) / 'steamapps' / 'common'))
        except Exception:
            pass

        # For each potential steam path, check libraryfolders.vdf
        for p in paths_to_check:
            try:
                p_path = Path(p)
                steamapps = None
                if p_path.name.lower() == 'common':
                    # If input is '.../steamapps/common', set steamapps to parent 'steamapps'
                    steamapps = p_path.parent
                elif 'steamapps' in [part.lower() for part in p_path.parts]:
                    # find path up to 'steamapps'
                    parts = p_path.parts
                    for i, part in enumerate(parts):
                        if part.lower() == 'steamapps':
                            steamapps = Path(*parts[:i+1])
                            break
                else:
                    # p is likely steam root; try steamapps under it
                    cand = p_path / 'steamapps'
                    if cand.exists():
                        steamapps = cand
                # If we still don't have steamapps, try parent's steamapps
                if steamapps is None:
                    cand = p_path.parent / 'steamapps'
                    if cand.exists():
                        steamapps = cand
                # If we still don't have steamapps, skip
                if not steamapps:
                    continue
                steam_root = steamapps.parent
                vdf_file = steamapps / 'libraryfolders.vdf'
                if not vdf_file.exists():
                    # Try the file in steam root 'steamapps/libraryfolders.vdf'
                    vdf_file = steam_root / 'steamapps' / 'libraryfolders.vdf'
                if vdf_file.exists():
                    libs = _parse_steam_libraryfolders_vdf(str(vdf_file))
                    for lib in libs:
                        lib_p = Path(lib)
                        # If 'lib' already contains 'steamapps', avoid duplicating
                        if lib_p.name.lower() == 'steamapps':
                            steamapps_lib = lib_p
                        elif 'steamapps' in [p.lower() for p in lib_p.parts]:
                            # locate the steamapps part
                            parts = lib_p.parts
                            for i, part in enumerate(parts):
                                if part.lower() == 'steamapps':
                                    steamapps_lib = Path(*parts[:i+1])
                                    break
                            else:
                                steamapps_lib = lib_p / 'steamapps'
                        else:
                            steamapps_lib = lib_p / 'steamapps'
                        common = steamapps_lib / 'common'
                        if common.exists():
                            results.append({'Launcher': 'Steam', 'Drive': Path(lib_p).drive, 'Path': str(common), 'Source': 'SteamVDF'})
            except Exception as e:
                debug_log(f'Failed to include Steam VDF libraries from {p}: {e}')
    except Exception as e:
        debug_log(f'Failed to inspect Steam VDF libraries: {e}')

    return results


def _include_appx_winrt_into_results(results: list):
    try:
        pkgs = get_appx_packages_winrt()
        for p in pkgs:
            if p.get('InstallLocation') and Path(p.get('InstallLocation')).exists():
                install_loc = p.get('InstallLocation')
                drive = Path(install_loc).drive
                results.append({'Launcher': 'Appx', 'Drive': drive, 'Path': install_loc, 'Source': 'WinRT'})
    except Exception as e:
        debug_log(f'Failed to include WinRT Appx packages: {e}')
    return results



def _get_uninstall_key_paths():
    # Known uninstall registry hives/paths to check
    return [
        (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        (winreg.HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ]


def _get_installed_programs_from_registry():
    """Return a list of installed programs discovered via the classic Uninstall registry keys.
    Each item is a dict with keys: DisplayName, InstallLocation, InstallPath, UninstallString.
    This function is intentionally shallow and should not recurse into file system trees to avoid blocking.
    """
    results = []
    if winreg is None:
        return results
    try:
        for hive, subkey in _get_uninstall_key_paths():
            try:
                with winreg.OpenKey(hive, subkey) as key:
                    for i in range(0, winreg.QueryInfoKey(key)[0]):
                        try:
                            sk = winreg.EnumKey(key, i)
                            with winreg.OpenKey(key, sk) as sub:
                                try:
                                    name = winreg.QueryValueEx(sub, 'DisplayName')[0]
                                except Exception:
                                    name = None
                                try:
                                    loc = winreg.QueryValueEx(sub, 'InstallLocation')[0]
                                except Exception:
                                    loc = None
                                try:
                                    uninstall = winreg.QueryValueEx(sub, 'UninstallString')[0]
                                except Exception:
                                    uninstall = None
                                try:
                                    icon = winreg.QueryValueEx(sub, 'DisplayIcon')[0]
                                except Exception:
                                    icon = None
                                if name or loc:
                                    results.append({'DisplayName': name, 'InstallLocation': loc, 'UninstallString': uninstall, 'DisplayIcon': icon})
                        except Exception:
                            # Skip problematic subkey
                            continue
            except Exception:
                continue
    except Exception as e:
        debug_log(f'Error reading uninstall registry keys: {e}')
    return results


def get_appx_packages_winrt():
    """Attempt to enumerate Appx packages using the winrt PackageManager (if available)."""
    try:
        from winrt.windows.management.deployment import PackageManager
        pm = PackageManager()
        packages = pm.find_packages_for_user(None)
        result = []
        for pkg in packages:
            name = pkg.id.name
            install_loc = getattr(pkg, 'installed_location', None)
            install_path = getattr(install_loc, 'path', None) if install_loc else None
            result.append({'Name': name, 'InstallLocation': install_path})
        return result
    except Exception as e:
        debug_log(f'WinRT Appx enumeration not available: {e}')
        return []


def clear_library_cache():
    try:
        if _CACHE_FILE.exists():
            _CACHE_FILE.unlink()
            return True
    except Exception as e:
        debug_log(f'Failed to clear library discovery cache: {e}')
    return False


def _find_steam_install_from_registry():
    """Try to detect a Steam install path from the registry entries.
    Returns: path or None
    """
    if winreg is None:
        return None
    try:
        # Common Steam registry locations
        steam_keys = [
            (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Wow6432Node\Valve\Steam"),
            (winreg.HKEY_CURRENT_USER, r"SOFTWARE\Valve\Steam"),
        ]
        for hive, key in steam_keys:
            try:
                with winreg.OpenKey(hive, key) as k:
                    try:
                        install_path = winreg.QueryValueEx(k, 'InstallPath')[0]
                        if install_path and Path(install_path).exists():
                            return str(Path(install_path))
                    except Exception:
                        continue
            except Exception:
                continue
    except Exception as e:
        debug_log(f'Failed to read Steam path from registry: {e}')
    return None
