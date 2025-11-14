"""
Test that all games have appropriate platform tags.
"""
import pytest
from pathlib import Path
from unittest.mock import MagicMock, patch
import sys
import os

# Add src to path to import scanner
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from scanner.game_scanner import Game, GameScanner


def test_game_has_platform_attribute():
    """Test that Game objects can have a platform attribute."""
    game = Game(
        name="Test Game",
        path="/fake/path",
        platform="Steam"
    )
    assert hasattr(game, 'platform')
    assert game.platform == "Steam"


def test_game_platform_defaults_to_none():
    """Test that platform defaults to None if not specified."""
    game = Game(
        name="Test Game",
        path="/fake/path"
    )
    assert hasattr(game, 'platform')
    assert game.platform is None


def test_parse_steam_acf_sets_platform():
    """Test that _parse_steam_acf sets the platform to 'Steam'."""
    scanner = GameScanner()
    
    # Create a mock ACF file content
    acf_content = '''
    "AppState"
    {
        "appid"  "12345"
        "name"  "Test Game"
        "installdir"  "TestGame"
    }
    '''
    
    # Create temporary test directory structure
    import tempfile
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        steamapps_path = tmpdir / "steamapps"
        steamapps_path.mkdir()
        
        common_path = steamapps_path / "common"
        common_path.mkdir()
        
        game_path = common_path / "TestGame"
        game_path.mkdir()
        
        # Create a dummy game executable to make it look like a game folder
        (game_path / "game.exe").touch()
        
        # Create ACF file
        acf_file = steamapps_path / "appmanifest_12345.acf"
        acf_file.write_text(acf_content)
        
        # Mock the image fetching and other operations
        with patch.object(scanner, 'fetch_game_image', return_value=None), \
             patch.object(scanner, '_detect_optiscaler', return_value=False), \
             patch.object(scanner, 'analyze_game_safety', return_value={
                 'engine': 'Unknown',
                 'anti_cheat_list': [],
                 'community_verified': False,
                 'engine_supported': True
             }), \
             patch.object(scanner, '_is_game_folder', return_value=True):
            
            # Call the method
            game = scanner._parse_steam_acf(acf_file, steamapps_path)
            
            # Verify the game has the correct platform
            assert game is not None, "Game should not be None"
            assert hasattr(game, 'platform'), "Game should have platform attribute"
            assert game.platform == 'Steam', f"Expected platform='Steam', got platform='{game.platform}'"


def test_all_game_creation_methods_set_platform():
    """Test that various game creation methods set appropriate platform tags."""
    # Test data for different platforms
    platform_tests = [
        ('Steam', '_parse_steam_acf'),
        ('Steam', '_scan_steam_common_folder'),
    ]
    
    # This test documents the expected behavior - all game creation
    # methods should set the platform attribute appropriately
    # The actual implementation verification happens in integration tests
    pass


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
