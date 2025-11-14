import os
from pathlib import Path
import json
import psutil
import importlib

from src.scanner import library_discovery as ld


def test_parse_steam_libraryfolders_vdf(tmp_path, monkeypatch):
    # Create mock steam root and other library
    steam_root = tmp_path / 'Steam'
    other_library = tmp_path / 'OtherSteam'
    (steam_root / 'steamapps').mkdir(parents=True)
    (other_library / 'steamapps' / 'common').mkdir(parents=True)
    # Create libraryfolders.vdf pointing to the other library
    vdf_content = r'''"libraryfolders"
{
	"0"
	{
		"path"	"%s"
	}
}
''' % str(other_library).replace('\\', '\\\\')
    libs_file = steam_root / 'steamapps' / 'libraryfolders.vdf'
    libs_file.write_text(vdf_content)

    # Monkeypatch registry detection to point to steam_root
    monkeypatch.setattr(ld, '_find_steam_install_from_registry', lambda: str(steam_root))
    # Also ensure drive partitions scanning won't add unrelated entries
    monkeypatch.setattr(ld.psutil, 'disk_partitions', lambda all=False: [])

    res = ld.get_game_libraries_from_fallback()
    # Expect the other_library to be detected (common subfolder)
    assert any(str(other_library / 'steamapps' / 'common') == r.get('Path') for r in res)


def test_parse_steam_libraryfolders_vdf_new_format(tmp_path, monkeypatch):
    # New format: numeric keys with path string value
    steam_root = tmp_path / 'Steam'
    other_library = tmp_path / 'Library2'
    (steam_root / 'steamapps').mkdir(parents=True)
    (other_library / 'steamapps' / 'common').mkdir(parents=True)
    vdf_content = '{"libraryfolders": {"0": {"path": "%s"}}}' % str(other_library).replace('\\', '\\\\')
    libs_file = steam_root / 'steamapps' / 'libraryfolders.vdf'
    libs_file.write_text(vdf_content)

    monkeypatch.setattr(ld, '_find_steam_install_from_registry', lambda: str(steam_root))
    monkeypatch.setattr(psutil, 'disk_partitions', lambda all=False: [])

    res = ld.get_game_libraries_from_fallback()
    assert any(str(other_library / 'steamapps' / 'common') == r.get('Path') for r in res)


def test_kv_parser_fallback(tmp_path, monkeypatch):
    # If vdf package is not installed, our KV parser should still extract paths
    monkeypatch.setattr(ld, 'vdf', None)
    steam_root = tmp_path / 'SteamKV'
    other_library = tmp_path / 'OtherSteamKV'
    (steam_root / 'steamapps').mkdir(parents=True)
    (other_library / 'steamapps' / 'common').mkdir(parents=True)
    # Classic VDF string
    vdf_content = '"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"  "%s"\n\t}\n}\n' % str(other_library).replace('\\', '\\\\')
    libs_file = steam_root / 'steamapps' / 'libraryfolders.vdf'
    libs_file.write_text(vdf_content)
    monkeypatch.setattr(ld, '_find_steam_install_from_registry', lambda: str(steam_root))
    monkeypatch.setattr(ld.psutil, 'disk_partitions', lambda all=False: [])
    res = ld.get_game_libraries_from_fallback()
    assert any(str(other_library / 'steamapps' / 'common') == r.get('Path') for r in res)
