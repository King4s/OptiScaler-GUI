"""
Tests for the Heroic Launcher scanner (issue #3).

Builds a fake Heroic config tree covering all four backends
(legendary/Epic, GOG, nile/Amazon, sideloaded) and verifies
_scan_heroic_games finds the games with correct titles.
"""
import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scanner.game_scanner import GameScanner


SAFETY = {
    'engine': 'Unknown', 'anti_cheat_list': [],
    'community_verified': False, 'engine_supported': True,
}


def _make_game_dir(base: Path, name: str) -> Path:
    game_dir = base / name
    game_dir.mkdir(parents=True)
    (game_dir / "game.exe").touch()
    return game_dir


def _write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data), encoding="utf-8")


@pytest.fixture
def heroic_root(tmp_path):
    """A Heroic config root populated with one game per backend."""
    root = tmp_path / "heroic"
    games_dir = tmp_path / "Games"

    epic_dir = _make_game_dir(games_dir, "EpicGame")
    _write_json(root / "legendaryConfig" / "legendary" / "installed.json", {
        "epicapp1": {
            "app_name": "epicapp1",
            "title": "Epic Test Game",
            "install_path": str(epic_dir),
            "executable": "game.exe",
        },
    })

    gog_dir = _make_game_dir(games_dir, "GogGame")
    _write_json(root / "gog_store" / "installed.json", {
        "installed": [
            {"appName": "1207658930", "install_path": str(gog_dir), "platform": "windows"},
        ],
    })
    _write_json(root / "store_cache" / "gog_library.json", {
        "games": [
            {"app_name": "1207658930", "title": "GOG Test Game"},
        ],
    })

    amazon_dir = _make_game_dir(games_dir, "AmazonGame")
    _write_json(root / "nile_config" / "nile" / "installed.json", [
        {"id": "amzn1.adg.product.test", "path": str(amazon_dir), "version": "1.0"},
    ])
    _write_json(root / "nile_config" / "nile" / "library.json", [
        {"id": "amzn1.adg.product.test", "product": {"title": "Amazon Test Game"}},
    ])

    sideload_dir = _make_game_dir(games_dir, "SideloadGame")
    _write_json(root / "sideload_apps" / "library.json", {
        "games": [
            {
                "app_name": "sideload1",
                "title": "Sideload Test Game",
                "is_installed": True,
                "folder_name": str(sideload_dir),
                "install": {"executable": str(sideload_dir / "game.exe"), "platform": "Windows"},
            },
        ],
    })

    return root


def _scan(scanner: GameScanner, root: Path):
    with patch.object(scanner, '_find_heroic_config_roots', return_value=[root]), \
         patch.object(scanner, 'fetch_game_image', return_value=None), \
         patch.object(scanner, '_detect_optiscaler', return_value=False), \
         patch.object(scanner, '_is_game_folder', return_value=True), \
         patch.object(scanner, 'analyze_game_safety', return_value=dict(SAFETY)):
        return scanner._scan_heroic_games()


def test_scans_all_heroic_backends(heroic_root):
    games = _scan(GameScanner(), heroic_root)

    names = sorted(g.name for g in games)
    assert names == [
        "Amazon Test Game",
        "Epic Test Game",
        "GOG Test Game",
        "Sideload Test Game",
    ]
    assert all(g.platform == "Heroic" for g in games)
    assert all(Path(g.path).is_dir() for g in games)


def test_gog_title_falls_back_to_folder_name(tmp_path):
    root = tmp_path / "heroic"
    gog_dir = _make_game_dir(tmp_path / "Games", "Cyberpunk_2077")
    _write_json(root / "gog_store" / "installed.json", {
        "installed": [{"appName": "123", "install_path": str(gog_dir)}],
    })

    games = _scan(GameScanner(), root)

    assert len(games) == 1
    assert games[0].name == "Cyberpunk 2077"


def test_older_gog_library_location_supplies_title(tmp_path):
    root = tmp_path / "heroic"
    gog_dir = _make_game_dir(tmp_path / "Games", "GogGame")
    _write_json(root / "gog_store" / "installed.json", {
        "installed": [{"appName": "42", "install_path": str(gog_dir)}],
    })
    _write_json(root / "gog_store" / "library.json", {
        "games": [{"app_name": "42", "title": "Old Library Title"}],
    })

    games = _scan(GameScanner(), root)

    assert [g.name for g in games] == ["Old Library Title"]


def test_missing_install_dirs_are_skipped(tmp_path):
    root = tmp_path / "heroic"
    _write_json(root / "legendaryConfig" / "legendary" / "installed.json", {
        "gone": {"title": "Uninstalled Game", "install_path": str(tmp_path / "does_not_exist")},
    })

    assert _scan(GameScanner(), root) == []


def test_malformed_store_files_do_not_crash(tmp_path):
    root = tmp_path / "heroic"
    (root / "legendaryConfig" / "legendary").mkdir(parents=True)
    (root / "legendaryConfig" / "legendary" / "installed.json").write_text("{not json", encoding="utf-8")
    _write_json(root / "gog_store" / "installed.json", {"installed": "unexpected-type"})
    _write_json(root / "sideload_apps" / "library.json", ["unexpected-type"])

    assert _scan(GameScanner(), root) == []


def test_same_install_path_reported_once(tmp_path):
    root = tmp_path / "heroic"
    game_dir = _make_game_dir(tmp_path / "Games", "SharedGame")
    _write_json(root / "legendaryConfig" / "legendary" / "installed.json", {
        "app1": {"title": "Shared Game", "install_path": str(game_dir)},
    })
    _write_json(root / "gog_store" / "installed.json", {
        "installed": [{"appName": "9", "install_path": str(game_dir)}],
    })

    games = _scan(GameScanner(), root)

    assert len(games) == 1
    assert games[0].name == "Shared Game"


def test_no_heroic_install_returns_empty(tmp_path):
    games = _scan(GameScanner(), tmp_path / "nonexistent-heroic")
    assert games == []
