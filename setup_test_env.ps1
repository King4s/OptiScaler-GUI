#!/usr/bin/env pwsh
<#
.SYNOPSIS
Test Environment Initialization Script for OptiScaler-GUI

.DESCRIPTION
Sets up test_env directory with all necessary subdirectories and sample files.
Run this script after cloning the repository to initialize the test environment.

.EXAMPLE
.\setup_test_env.ps1

.NOTES
This script must be run from the OptiScaler-GUI root directory
#>

param()

# Configuration
$testEnvPath = "test_env"
$gitKeepFiles = @(
    "$testEnvPath\outputs\logs\.gitkeep",
    "$testEnvPath\outputs\reports\.gitkeep"
)

# Directory structure to create
$directories = @(
    $testEnvPath,
    "$testEnvPath\fixtures",
    "$testEnvPath\fixtures\archives",
    "$testEnvPath\fixtures\ini_configs",
    "$testEnvPath\mock_games",
    "$testEnvPath\cache",
    "$testEnvPath\cache\optiscaler_downloads",
    "$testEnvPath\cache\extracted",
    "$testEnvPath\outputs",
    "$testEnvPath\outputs\logs",
    "$testEnvPath\outputs\reports"
)

function Write-Step {
    param([string]$Message)
    Write-Host "[*] $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[+] $Message" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "    $Message" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "OptiScaler-GUI Test Environment Setup" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host ""

# Check if we're in the right directory
if (-not (Test-Path ".github")) {
    Write-Host "ERROR: Please run this script from the OptiScaler-GUI root directory" -ForegroundColor Red
    Write-Host ""
    exit 1
}

Write-Step "Creating test_env directory structure..."
Write-Host ""

# Create all directories
foreach ($dir in $directories) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Success "Created $dir/"
    } else {
        Write-Info "$dir/ already exists"
    }
}

# Create .gitkeep files
Write-Host ""
Write-Step "Preserving directory structure..."
foreach ($file in $gitKeepFiles) {
    $dir = Split-Path $file
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    if (-not (Test-Path $file)) {
        New-Item -ItemType File -Path $file -Force | Out-Null
        Write-Success "Created $file"
    }
}

# Display summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Setup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

Write-Host "Test Environment Structure:" -ForegroundColor Yellow
Write-Info "test_env/"
Write-Info " - fixtures/           (Test data and samples)"
Write-Info "   - archives/          (Test archives)"
Write-Info "   - ini_configs/       (Sample INI files)"
Write-Info "   - OptiScaler.ini.sample"
Write-Info " - mock_games/         (Mock game directories)"
Write-Info " - cache/              (Cached downloads)"
Write-Info "   - optiscaler_downloads/"
Write-Info "   - extracted/"
Write-Info " - outputs/            (Test results)"
Write-Info "   - logs/"
Write-Info "   - reports/"
Write-Host ""

Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Info "1. Copy test archives to test_env\fixtures\archives\"
Write-Info "2. Create mock game directories as needed"
Write-Info "3. Run tests: python test_archive_extractor.py"
Write-Info "4. Check results in test_env\outputs\"
Write-Host ""

Write-Host "For more information, see test_env\README.md" -ForegroundColor Cyan
Write-Host ""
