"""
Tests for the v0.5.2 scan performance refactor:
- FolderFacts single-pass collection feeding all detectors
- Steam manifest map (O(N) name/appid lookup)
- Steam library dedup across overlapping scan sources
"""
import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scanner.game_scanner import GameScanner, FolderFacts


def _make_unity_game(base: Path, name: str) -> Path:
    d = base / name
    (d / "Data").mkdir(parents=True)
    (d / f"{name}.exe").touch()
    (d / "UnityPlayer.dll").touch()
    for i in range(6):
        (d / "Data" / f"data{i}.bin").touch()
    (d / "EasyAntiCheat").mkdir()
    (d / "EasyAntiCheat" / "EasyAntiCheat.exe").touch()
    return d


def test_folder_facts_feed_all_detectors(tmp_path):
    scanner = GameScanner()
    game_dir = _make_unity_game(tmp_path, "FactsGame")

    facts = scanner._collect_folder_facts(game_dir)

    assert isinstance(facts, FolderFacts)
    assert facts.found_exe
    assert facts.found_game_content
    assert facts.file_count > 5
    assert scanner._is_game_folder(game_dir, facts) is True
    assert scanner._detect_engine_type(game_dir, facts) == "Unity"
    assert "EasyAntiCheat" in scanner._detect_anti_cheat(game_dir, facts)
    # dxgi.dll in root should flip OptiScaler detection using the same facts
    assert scanner._detect_optiscaler(game_dir, facts) is False
    (game_dir / "dxgi.dll").touch()
    facts2 = scanner._collect_folder_facts(game_dir)
    assert scanner._detect_optiscaler(game_dir, facts2) is True


def test_detectors_work_without_facts(tmp_path):
    """Standalone calls (no shared facts) must still work — API compat."""
    scanner = GameScanner()
    game_dir = _make_unity_game(tmp_path, "NoFactsGame")
    assert scanner._is_game_folder(game_dir) is True
    assert scanner._detect_engine_type(game_dir) == "Unity"
    assert "EasyAntiCheat" in scanner._detect_anti_cheat(game_dir)


def test_excluded_folder_names_rejected(tmp_path):
    scanner = GameScanner()
    d = _make_unity_game(tmp_path, "Redist")
    assert scanner._is_game_folder(d) is False


ACF_TEMPLATE = '''"AppState"
{
"appid" "%s"
"name" "%s"
"installdir" "%s"
}
'''


def _make_steam_library(base: Path, games: dict) -> Path:
    """games: installdir -> (appid, name). Returns library root."""
    steamapps = base / "steamapps"
    common = steamapps / "common"
    common.mkdir(parents=True)
    for installdir, (appid, name) in games.items():
        (steamapps / f"appmanifest_{appid}.acf").write_text(
            ACF_TEMPLATE % (appid, name, installdir))
        gd = common / installdir
        gd.mkdir()
        (gd / "game.exe").touch()
        for i in range(6):
            (gd / f"asset{i}.pak").touch()
    return base


def test_steam_manifest_map(tmp_path):
    scanner = GameScanner()
    lib = _make_steam_library(tmp_path / "Steam", {
        "TestGame": ("111", "Test Game"),
        "OtherGame": ("222", "Other Game"),
    })
    mapping = scanner._build_steam_manifest_map(lib / "steamapps")
    assert mapping["testgame"] == ("Test Game", "111")
    assert mapping["othergame"] == ("Other Game", "222")
    # _find_steam_game_info stays API-compatible on top of the map
    assert scanner._find_steam_game_info("TestGame", lib / "steamapps") == ("Test Game", "111")


def test_steam_library_scanned_once(tmp_path):
    scanner = GameScanner()
    lib = _make_steam_library(tmp_path / "Steam", {
        "DedupGame": ("333", "Dedup Game"),
    })
    scanner._scanned_steam_roots = set()

    with patch.object(scanner, 'analyze_game_safety', return_value={
        'engine': 'Unknown', 'anti_cheat_list': [],
        'community_verified': False, 'engine_supported': True,
    }):
        first = scanner._scan_steam_library(lib)
        second = scanner._scan_steam_library(lib)

    assert [g.name for g in first] == ["Dedup Game"]
    assert first[0].appid == "333"
    assert first[0].platform == "Steam"
    assert second == []  # same library must not be scanned twice per pass


def test_scan_does_not_fetch_images(tmp_path):
    """Images are fetched by the GUI after render — never during the scan."""
    scanner = GameScanner()
    lib = _make_steam_library(tmp_path / "Steam", {
        "NoImgGame": ("444", "No Img Game"),
    })
    scanner._scanned_steam_roots = set()

    with patch.object(scanner, 'analyze_game_safety', return_value={
        'engine': 'Unknown', 'anti_cheat_list': [],
        'community_verified': False, 'engine_supported': True,
    }), patch.object(scanner, 'fetch_game_image') as fetch_mock:
        games = scanner._scan_steam_library(lib)

    assert len(games) == 1
    assert games[0].image_path is None
    fetch_mock.assert_not_called()
