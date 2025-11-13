# Test Environment Quick Reference

## Setup
```bash
# One-time initialization
.\setup_test_env.ps1
# or
setup_test_env.bat
```

## Directory Map
| Path | Purpose |
|------|---------|
| `test_env/fixtures/` | Sample test data & configs |
| `test_env/mock_games/` | Mock game directories |
| `test_env/cache/` | Downloaded/extracted files (auto-generated) |
| `test_env/outputs/` | Test logs & reports |

## Create Mock Game
```bash
# Unreal Engine game
mkdir test_env\mock_games\GameName\Engine\Binaries\Win64

# Standard game
mkdir test_env\mock_games\GameName
```

## Run Tests
```bash
# Individual test
python test_archive_extractor.py

# All tests
.\run_progress_tests.bat

# With test env variable
$env:TEST_ENV_PATH = ".\test_env"
python test_archive_extractor.py
```

## Clean Cache
```bash
# Remove downloads (keep fixtures)
rm test_env\cache\optiscaler_downloads -r
rm test_env\cache\extracted -r

# Reset cache
mkdir test_env\cache\optiscaler_downloads
mkdir test_env\cache\extracted
```

## Git Safety
✅ Automatically excluded from git  
✅ Safe for test files and binary data  
✅ Each developer has independent test setup  
✅ No risk of committing test files

## Documentation
- `test_env/README.md` - Full test environment guide
- `TEST_ENV_SETUP.md` - Setup and usage guide
- This file - Quick reference

## Key Files
| File | Purpose |
|------|---------|
| `.gitignore` | Excludes test_env/ from git |
| `setup_test_env.ps1` | PowerShell setup script |
| `setup_test_env.bat` | Batch file setup script |
| `test_env/README.md` | Detailed documentation |
| `test_env/fixtures/OptiScaler.ini.sample` | Sample config |

---
Created: November 13, 2025
For more info: See `TEST_ENV_SETUP.md` or `test_env/README.md`
