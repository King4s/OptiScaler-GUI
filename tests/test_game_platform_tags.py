"""
Test that all games have appropriate platform tags.
"""
import pytest
from pathlib import Path
from unittest.mock import patch
import sys

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from scanner.game_scanner import Game, GameScanner


def test_game_has_platform_attribute():
    game = Game(name="Test Game", path="/fake/path", platform="Steam")
    assert game.platform == "Steam"


def test_game_platform_defaults_to_local():
    """Game defaults platform to 'Local' when not specified."""
    game = Game(name="Test Game", path="/fake/path")
    assert game.platform == "Local"


def test_parse_steam_acf_sets_platform():
    """_parse_steam_acf must return a Game with platform='Steam'."""
    import tempfile

    acf_content = '''"AppState"\n{\n"appid" "12345"\n"name" "Test Game"\n"installdir" "TestGame"\n}\n'''

    scanner = GameScanner()

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        steamapps_path = tmpdir / "steamapps"
        (steamapps_path / "common" / "TestGame").mkdir(parents=True)
        (steamapps_path / "common" / "TestGame" / "game.exe").touch()
        acf_file = steamapps_path / "appmanifest_12345.acf"
        acf_file.write_text(acf_content)

        with patch.object(scanner, 'fetch_game_image', return_value=None), \
             patch.object(scanner, '_detect_optiscaler', return_value=False), \
             patch.object(scanner, 'analyze_game_safety', return_value={
                 'engine': 'Unknown', 'anti_cheat_list': [],
                 'community_verified': False, 'engine_supported': True
             }), \
             patch.object(scanner, '_is_game_folder', return_value=True):

            game = scanner._parse_steam_acf(acf_file, steamapps_path)

        assert game is not None
        assert game.platform == 'Steam'
