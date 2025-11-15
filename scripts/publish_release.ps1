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
$portableCandidates = @("dist/OptiScaler-GUI-Portable", "dist/OptiScaler-GUI")
$archivePath = "dist/OptiScaler-GUI-Portable.zip"
# Detect portable build folder
$portableFolder = $null
foreach ($candidate in $portableCandidates) { if (Test-Path $candidate) { $portableFolder = $candidate; break } }
if ($portableFolder -and (Test-Path $portableFolder)) {
    if (Test-Path $archivePath) { Remove-Item $archivePath -Force }
    Write-Host "Zipping portable build to $archivePath..."
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $absPortableFolder = (Resolve-Path $portableFolder).Path
    $absArchivePath = Join-Path (Get-Location).Path $archivePath
    [System.IO.Compression.ZipFile]::CreateFromDirectory($absPortableFolder, $absArchivePath)
}

# Compose release body
if (Test-Path $BodyFile) { $body = Get-Content $BodyFile -Raw } else { $body = "Release $Version" }

Write-Host "Creating GitHub release (tag v$Version)..."
# Find the executable: prefer single-file build, otherwise portable folder exe
$singleExeCandidates = @("dist/OptiScaler-GUI.exe", "dist/OptiScaler-GUI/OptiScaler-GUI.exe")
$exePath = $null
foreach ($candidate in $singleExeCandidates) { if (Test-Path $candidate) { $exePath = $candidate; break } }
if (-not $exePath) { Write-Host "Unable to locate any built executable in dist/. Please run build.py and ensure artifacts exist before publishing." -ForegroundColor Red; exit 1 }
gh release create "v$Version" "$exePath" -t "$ReleaseName" -n "$body"
if ($LASTEXITCODE -ne 0) { Write-Host "Failed to create GitHub release" -ForegroundColor Red; exit 1 }

if (Test-Path $archivePath) {
    gh release upload "v$Version" $archivePath -R (git config --get remote.origin.url)
} elseif ($portableFolder -and (Test-Path $portableFolder)) {
    # If we couldn't zip (maybe not needed), upload the folder EXE instead
    $portableExeCandidate = "$portableFolder/OptiScaler-GUI.exe"
    if (Test-Path $portableExeCandidate) {
        gh release upload "v$Version" $portableExeCandidate -R (git config --get remote.origin.url)
    }
}

Write-Host "Release v$Version published successfully!" -ForegroundColor Green
