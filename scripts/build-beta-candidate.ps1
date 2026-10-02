param(
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][string]$TargetDirectory,
    [string]$Version = '2026.9.0-beta.2'
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$previousTarget = $env:CARGO_TARGET_DIR
Push-Location $repoRoot
try {
    # Snapshot all staged source before compiling, and refuse a mixed working tree.
    & git diff --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Stage the reviewed working tree before building' }
    $untracked = @(& git ls-files --others --exclude-standard)
    if ($LASTEXITCODE -ne 0 -or $untracked.Count -ne 0) { throw 'Untracked files must be staged or kept outside the repository' }
    $sourceTree = & git write-tree
    if ($LASTEXITCODE -ne 0 -or $sourceTree -notmatch '^[0-9a-f]{40}$') { throw 'Could not snapshot the Git index' }
    $target = [System.IO.Path]::GetFullPath($TargetDirectory)
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    $snapshot = Join-Path $target "source-$sourceTree.tar"
    & git archive --format=tar "--output=$snapshot" $sourceTree
    if ($LASTEXITCODE -ne 0) { throw 'Source archive creation failed' }
    $env:CARGO_TARGET_DIR = $target
    & cargo build --manifest-path rust/Cargo.toml --release --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & git diff --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Source changed during the build; do not distribute it' }
    $afterTree = & git write-tree
    if ($LASTEXITCODE -ne 0 -or $afterTree -ne $sourceTree) { throw 'Staged source changed during the build' }
    $untracked = @(& git ls-files --others --exclude-standard)
    if ($LASTEXITCODE -ne 0 -or $untracked.Count -ne 0) { throw 'New untracked files appeared during the build' }
    & (Join-Path $PSScriptRoot 'verify-beta-package.ps1') `
        -BinaryPath (Join-Path $target 'release/OptiScaler-GUI.exe') `
        -OutputDirectory $OutputDirectory -Version $Version `
        -SourceTree $sourceTree -SourceArchive $snapshot
    Write-Output "Build source tree: $sourceTree"
    Write-Output "Source archive: $snapshot"
} finally {
    $env:CARGO_TARGET_DIR = $previousTarget
    Pop-Location
}
