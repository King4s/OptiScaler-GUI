import os
import sys
from pathlib import Path
import pytest

# Ensure 'src' is on sys.path so tests can import packages like 'scanner'.
ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT / 'src'
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))


@pytest.fixture(scope='session')
def test_env_path(tmp_path_factory):
    repo_root = Path(__file__).resolve().parents[1]
    test_env = repo_root / 'test_env'
    # Ensure test_env exists
    test_env.mkdir(parents=True, exist_ok=True)
    # Create basic directories used by tests
    (test_env / 'fixtures' / 'archives').mkdir(parents=True, exist_ok=True)
    (test_env / 'cache' / 'optiscaler_downloads').mkdir(parents=True, exist_ok=True)
    (test_env / 'cache' / 'extracted').mkdir(parents=True, exist_ok=True)
    (test_env / 'mock_games').mkdir(parents=True, exist_ok=True)
    (test_env / 'outputs' / 'logs').mkdir(parents=True, exist_ok=True)
    (test_env / 'outputs' / 'reports').mkdir(parents=True, exist_ok=True)
    return test_env


from src.scanner import library_discovery as ld


@pytest.fixture(autouse=False)
def patch_ld_psutil(monkeypatch):
    """Patch psutil in the module under test so disk partition scans don't hit real drives.

    This fixture wraps the typical monkeypatch usage so tests are clearer and consistent.
    """
    # Set disk_partitions to an empty list - avoids scanning the host system in tests
    monkeypatch.setattr(ld.psutil, 'disk_partitions', lambda all=False: [])
    return monkeypatch


@pytest.fixture
def steam_structure(tmp_path):
    """Create a standard steam structure and a secondary library for testing.

    - Returns (steam_root, other_library)
    """
    steam_root = tmp_path / 'Steam'
    other_library = tmp_path / 'OtherSteam'
    (steam_root / 'steamapps').mkdir(parents=True)
    (other_library / 'steamapps' / 'common').mkdir(parents=True)
    return steam_root, other_library
