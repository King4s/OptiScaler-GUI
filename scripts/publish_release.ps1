param(
    [string]$Version = "0.3.6",
    [string]$ReleaseName = "OptiScaler-GUI v$Version",
    [string]$BodyFile = "CHANGELOG.md"
)

Write-Host "Publishing release v$Version..."

# Ensure gh is available
$gh = (Get-Command gh -ErrorAction SilentlyContinue)
if (-not $gh) {
    Write-Host "gh CLI not found. Install GitHub CLI (https://cli.github.com/) and log in using 'gh auth login'" -ForegroundColor Yellow
    exit 1
}

Write-Host "Pushing commits and tags to origin..."
git push origin HEAD --follow-tags
if ($LASTEXITCODE -ne 0) { Write-Host "Failed to push commits/tags" -ForegroundColor Red; exit 1 }

# Package portable build into .zip if exists
$portableFolder = "dist/OptiScaler-GUI-Portable"
$archivePath = "dist/OptiScaler-GUI-Portable.zip"
if (Test-Path $portableFolder) {
    if (Test-Path $archivePath) { Remove-Item $archivePath -Force }
    Write-Host "Zipping portable build to $archivePath..."
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path $portableFolder), (Resolve-Path $archivePath))
}

# Compose release body
if (Test-Path $BodyFile) { $body = Get-Content $BodyFile -Raw } else { $body = "Release $Version" }

Write-Host "Creating GitHub release (tag v$Version)..."
gh release create "v$Version" "dist/OptiScaler-GUI.exe" -t "$ReleaseName" -n "$body"
if ($LASTEXITCODE -ne 0) { Write-Host "Failed to create GitHub release" -ForegroundColor Red; exit 1 }

if (Test-Path $archivePath) {
    gh release upload "v$Version" $archivePath -R (git config --get remote.origin.url)
}

Write-Host "Release v$Version published successfully!" -ForegroundColor Green
