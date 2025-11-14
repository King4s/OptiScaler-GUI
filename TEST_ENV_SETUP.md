# Test Environment Setup Guide

## Quick Start

The test environment has been initialized. Here's how to use it:

### 1. Using the Test Environment

The `test_env/` directory is now ready for testing. It contains:
- **fixtures/** - Sample files and configurations for testing
- **mock_games/** - Create mock game directories here for testing
- **cache/** - Cache files used during testing (7z downloads, extracted files)
- **outputs/** - Test results, logs, and reports

### 2. Directory Structure

```
test_env/
├── README.md                          # Test environment documentation
├── fixtures/                          # Test data
│   ├── archives/                      # Place 7z/zip test files here
│   ├── ini_configs/                   # Sample INI configurations
│   └── OptiScaler.ini.sample         # Example OptiScaler.ini
├── mock_games/                        # Mock game installations
│   └── [create game directories here]
├── cache/                             # Test cache (auto-populated)
│   ├── optiscaler_downloads/         # Downloaded test archives
│   └── extracted/                    # Extracted test archives
└── outputs/                          # Test results
    ├── logs/                         # Test logs
    └── reports/                      # Test reports
```

### 3. Git Exclusion

The `test_env/` directory is automatically excluded from git via `.gitignore`:
```
# Test environment (local testing only, excluded from git)
test_env/
```

This means:
- ✅ Safe to store local test files
- ✅ Large binary files won't pollute the repository
- ✅ Test cache won't be committed
- ✅ Each developer can have different test setups

### 4. Running Tests

#### Setup Test Environment (First Time)
```bash
# PowerShell
.\setup_test_env.ps1

# Or batch script
setup_test_env.bat
```

#### Run Tests with Test Environment
```bash
# Set environment variable (optional)
$env:TEST_ENV_PATH = ".\test_env"

# Run individual test
python test_archive_extractor.py

# Run all tests
.\run_progress_tests.bat
```

### 5. Creating Mock Game Directories

Example: Create a mock Unreal Engine game for testing

```bash
# Create directory structure
mkdir test_env\mock_games\TestGameUE\Engine\Binaries\Win64
mkdir test_env\mock_games\TestGameUE\Content

# Create mock executable
echo "Mock UE Game" > test_env\mock_games\TestGameUE\TestGame.exe
```

### PowerShell & Test Behavior
When running tests that involve library discovery on Windows, note:
- The `use_powershell_discovery` is optional and configurable. If PowerShell is installed and the setting is enabled, the PoC may run during scanning; the test harness frequently mocks or sets `use_powershell_discovery` to `False` or patches `self.after` to a synchronous stub to avoid background-thread races.
- Tests may replace `self.after` (Tk main-loop scheduling) with a synchronous lambda during UI tests to avoid race conditions; this is intentional and kept in tests to make UI assertions deterministic.

Example: Create a standard game directory

```bash
mkdir test_env\mock_games\TestGameStandard
echo "Mock Game" > test_env\mock_games\TestGameStandard\game.exe
```

### 6. Sample Test Configurations

Use the provided sample configurations:

```bash
# Copy sample config
cp test_env\fixtures\OptiScaler.ini.sample test_env\mock_games\TestGame\OptiScaler.ini

# Edit as needed for your test
# Add test-specific settings
```

### 7. Common Testing Patterns

#### Testing Archive Extraction
```python
# test_archive_extraction_local.py
from src.utils.archive_extractor import archive_extractor
from pathlib import Path

test_env = Path("test_env")
test_archive = test_env / "fixtures" / "archives" / "TestArchive.7z"
extract_path = test_env / "cache" / "extracted"

success, msg, result_path = archive_extractor.extract_archive(
    test_archive, 
    extract_path
)
assert success
```

#### Testing Game Installation
```python
# test_installation_local.py
from src.optiscaler.manager import OptiScalerManager
from pathlib import Path

test_env = Path("test_env")
game_path = test_env / "mock_games" / "TestGame"
game_path.mkdir(parents=True, exist_ok=True)

manager = OptiScalerManager(
    download_dir=str(test_env / "cache")
)
success, msg = manager.install_optiscaler(
    str(game_path),
    target_filename="dxgi.dll"
)

# Check results
assert (game_path / "dxgi.dll").exists()
```

### 8. Cleaning Up

Clean test cache while keeping test structures:
```bash
# Remove cache files
rm test_env\cache\optiscaler_downloads -r
rm test_env\cache\extracted -r

# Keep fixtures and mock_games
```

Clean all test outputs:
```bash
# Remove test results
rm test_env\outputs -r
mkdir test_env\outputs
mkdir test_env\outputs\logs
mkdir test_env\outputs\reports
```

Reset test environment completely:
```bash
# WARNING: This removes everything including fixtures
rm test_env -r

# Then re-run setup
.\setup_test_env.ps1
```

### 9. Integration with CI/CD

For CI/CD systems:
- Test environment is not committed to git
- CI systems create fresh test_env on each run
- No cache carries over between builds
- Results stored in test_env/outputs for reporting

Example CI setup:
```yaml
# .github/workflows/tests.yml
- name: Setup Test Environment
  run: .\setup_test_env.ps1

- name: Run Tests
  run: python test_archive_extractor.py

- name: Upload Test Results
  if: always()
  uses: actions/upload-artifact@v2
  with:
    name: test-results
    path: test_env/outputs/
```

### 10. Troubleshooting

#### "test_env not found" error
```bash
# Re-run setup script
.\setup_test_env.ps1
```

#### "Permission denied" when cleaning
```powershell
# On Windows, close any open editors/tools using test_env
# Then use Force flag
rm test_env -r -Force
```

#### Large test_env directory
```bash
# Check size
du -sh test_env/

# Clean cache
rm test_env/cache -r
mkdir test_env/cache
```

#### Git accidentally tracks test_env
```bash
# Remove from git (doesn't delete local files)
git rm --cached -r test_env/

# Verify it's excluded
git status
```

## Next Steps

1. ✅ Test environment initialized
2. 📝 Create mock games for your test scenarios
3. 🧪 Write unit tests using test_env fixtures
4. 📊 Review test outputs in test_env/outputs/
5. 🔄 Clean up cache between test runs

For more detailed information, see `test_env/README.md`
