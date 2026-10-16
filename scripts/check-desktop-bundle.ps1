$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$release = Join-Path $root "apps\desktop\src-tauri\target\release"
$ffmpeg = Join-Path $release "ffmpeg.exe"
$ffprobe = Join-Path $release "ffprobe.exe"
$app = @(
    (Join-Path $release "Fitifact.exe"),
    (Join-Path $release "fitifact-desktop.exe")
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $app) {
    throw "Desktop release executable was not produced in $release"
}
if (-not (Test-Path -LiteralPath $ffmpeg)) {
    throw "Packaged desktop is missing ffmpeg.exe next to the app"
}
if (-not (Test-Path -LiteralPath $ffprobe)) {
    throw "Packaged desktop is missing ffprobe.exe next to the app"
}

$conf = Get-Content -Raw -LiteralPath (Join-Path $root "apps\desktop\src-tauri\tauri.conf.json")
if ($conf -notmatch '"version"\s*:\s*"([^"]+)"') {
    throw "Could not read desktop version from tauri.conf.json"
}
$version = $Matches[1]
$installerName = "Fitifact_${version}_x64-setup.exe"
$installer = Join-Path $release "bundle\nsis\$installerName"
if (-not (Test-Path -LiteralPath $installer)) {
    throw "NSIS installer missing: $installerName"
}

Write-Host "Desktop bundle sidecars: PASS"
Write-Host "App: $app"
Write-Host "Installer: $installer"
