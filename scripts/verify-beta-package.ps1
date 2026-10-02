param(
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceTree,
    [Parameter(Mandatory)][string]$SourceArchive,
    [string]$Version = '2026.9.0',
    [string]$SevenZip = (Join-Path $env:ProgramFiles '7-Zip/7z.exe')
)

$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$sourceArchiveHash = (Get-FileHash -LiteralPath $SourceArchive -Algorithm SHA256).Hash.ToLowerInvariant()
if (!(Test-Path -LiteralPath $SevenZip -PathType Leaf)) { throw '7-Zip is missing' }
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Choose a new output directory; existing artifacts are preserved' }
if ((Get-Item -LiteralPath $binary).Length -gt 12MB) { throw 'Binary exceeds the 12 MiB CI budget' }
$output = (New-Item -ItemType Directory -Path $OutputDirectory).FullName
$packagedExe = Join-Path $output 'OptiScaler-GUI.exe'
Copy-Item -LiteralPath $binary -Destination $packagedExe

Push-Location $output
try {
    & $SevenZip a -t7z -mx=9 'OptiScaler-GUI.7z' 'OptiScaler-GUI.exe'
    if ($LASTEXITCODE -ne 0) { throw '7z creation failed' }
    & $SevenZip t 'OptiScaler-GUI.7z'
    if ($LASTEXITCODE -ne 0) { throw '7z integrity check failed' }
} finally { Pop-Location }

$hashes = @{}
$checksumLines = foreach ($name in @('OptiScaler-GUI.exe', 'OptiScaler-GUI.7z')) {
    $hashes[$name] = (Get-FileHash -LiteralPath (Join-Path $output $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    "$($hashes[$name])  $name"
}
$checksumLines | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii

$smokeResults = foreach ($language in @('da', 'en', 'pl')) {
    $smoke = (New-Item -ItemType Directory -Path (Join-Path $output "smoke-$language")).FullName
    & $SevenZip x (Join-Path $output 'OptiScaler-GUI.7z') "-o$smoke" -y | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Archive extraction failed for $language" }
    $smokeExe = Join-Path $smoke 'OptiScaler-GUI.exe'
    $extractedHash = (Get-FileHash -LiteralPath $smokeExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($extractedHash -ne $hashes['OptiScaler-GUI.exe']) { throw 'Extracted EXE differs from the original' }
    $cache = (New-Item -ItemType Directory -Path (Join-Path $smoke 'cache')).FullName
    @{language=$language; check_updates=$false; auto_update_optiscaler=$false; effects_enabled=$false} |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $cache 'config.json') -Encoding utf8
    $process = Start-Process -FilePath $smokeExe -WorkingDirectory $smoke -WindowStyle Hidden -PassThru
    try {
        Start-Sleep -Seconds 8
        $process.Refresh()
        if ($process.HasExited) { throw "GUI exited during $language startup with code $($process.ExitCode)" }
        $log = Join-Path $smoke 'logs/optiscaler-gui.log'
        $expected = "OptiScaler GUI $Version started"
        if (!(Test-Path -LiteralPath $log) -or !(Select-String -LiteralPath $log -SimpleMatch $expected -Quiet)) {
            throw "Expected startup version was not logged for $language"
        }
        if (Test-Path -LiteralPath (Join-Path $smoke 'logs/crash.log')) { throw "Crash log found for $language" }
        [pscustomobject]@{language=$language; startup_seconds=8; startup_passed=$true; extracted_sha256=$extractedHash}
    } finally {
        $process.Refresh()
        if (!$process.HasExited) { Stop-Process -Id $process.Id -Force }
        $process.Dispose()
    }
}

[ordered]@{
    version=$Version
    source_tree=$SourceTree
    source_archive_sha256=$sourceArchiveHash
    verified_utc=[DateTime]::UtcNow.ToString('o')
    executable_bytes=(Get-Item -LiteralPath $packagedExe).Length
    sha256=$hashes
    archive_integrity_passed=$true
    startup=$smokeResults
    limitations=@('Startup only; visual translation and gameplay not verified', 'Real-game install/update/uninstall and Fatekeeper overlay not exercised')
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'verification.json') -Encoding utf8
Write-Output "Verified candidate: $output"
