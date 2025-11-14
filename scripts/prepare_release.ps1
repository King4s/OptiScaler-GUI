# PowerShell release helper script
# Usage: Open a PowerShell prompt at repository root and run: ./scripts/prepare_release.ps1 -Version 0.4.0
param(
    [string]$Version = "0.4.0"
)

Write-Host "Running tests..."
py -3 -m pytest -q
if ($LASTEXITCODE -ne 0) { Write-Host "Tests failed - aborting" -ForegroundColor Red; exit 1 }

Write-Host "Building portable executable (PyInstaller)"
py -3 build.py
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed - aborting" -ForegroundColor Red; exit 1 }

Write-Host "Preparing Git tag v$Version (will not push automatically)"

git add -A
git commit -m "chore(release): v$Version" -a
if ($LASTEXITCODE -ne 0) { Write-Host "Git commit failed - aborting" -ForegroundColor Red; exit 1 }

git tag -a "v$Version" -m "Release v$Version"
Write-Host "Created tag v$Version - please push and create a GitHub release as needed."

Write-Host "Done. Artifacts are in the 'build' or 'dist' folder. Upload to GitHub releases via the web UI."