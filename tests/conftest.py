import os
from pathlib import Path
import pytest


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
