$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$binDir = Join-Path $root "apps\desktop\src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

function Get-RustHostTriple {
    $line = rustc -vV | Select-String -Pattern '^host: ' | Select-Object -First 1
    if (-not $line) { throw "rustc did not report a host triple" }
    return ($line.Line -replace '^host:\s+', '').Trim()
}

$triple = Get-RustHostTriple
$exe = if ($triple -match 'windows') { ".exe" } else { "" }
$ffmpegDest = Join-Path $binDir "ffmpeg-$triple$exe"
$ffprobeDest = Join-Path $binDir "ffprobe-$triple$exe"
$maxSidecarBytes = 120MB

function Sidecar-IsSmallEnough([string]$Path) {
    return (Test-Path -LiteralPath $Path) -and ((Get-Item -LiteralPath $Path).Length -le $maxSidecarBytes)
}

if ((Sidecar-IsSmallEnough $ffmpegDest) -and (Sidecar-IsSmallEnough $ffprobeDest)) {
    Write-Host "Desktop FFmpeg sidecars already present for $triple"
    exit 0
}

function Copy-FromPath([string]$Name, [string]$Dest) {
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) { return $false }
    $item = Get-Item -LiteralPath $command.Source
    if ($item.Length -gt $maxSidecarBytes) {
        Write-Host "Skipping PATH $Name ($([math]::Round($item.Length / 1MB)) MB); too large for a sidecar"
        return $false
    }
    Copy-Item -LiteralPath $item.FullName -Destination $Dest -Force
    return $true
}

$copiedFfmpeg = Copy-FromPath "ffmpeg" $ffmpegDest
$copiedProbe = Copy-FromPath "ffprobe" $ffprobeDest
if ($copiedFfmpeg -and $copiedProbe) {
    Write-Host "Copied FFmpeg sidecars from PATH for $triple"
    exit 0
}

if ($triple -ne "x86_64-pc-windows-msvc") {
    throw "No FFmpeg on PATH and no download recipe for $triple. Install ffmpeg and ffprobe, then rerun."
}

$work = Join-Path $env:TEMP "fitifact-ffmpeg-sidecar"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$zip = Join-Path $work "ffmpeg-essentials.zip"
$url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
Write-Host "Downloading FFmpeg essentials for $triple"
Invoke-WebRequest -Uri $url -OutFile $zip
$extract = Join-Path $work "extract"
if (Test-Path $extract) { Remove-Item -Recurse -Force $extract }
Expand-Archive -LiteralPath $zip -DestinationPath $extract
$ffmpegSrc = Get-ChildItem -Path $extract -Filter "ffmpeg.exe" -Recurse | Select-Object -First 1
$probeSrc = Get-ChildItem -Path $extract -Filter "ffprobe.exe" -Recurse | Select-Object -First 1
if (-not $ffmpegSrc -or -not $probeSrc) {
    throw "Downloaded FFmpeg archive did not contain ffmpeg.exe and ffprobe.exe"
}
Copy-Item -LiteralPath $ffmpegSrc.FullName -Destination $ffmpegDest -Force
Copy-Item -LiteralPath $probeSrc.FullName -Destination $ffprobeDest -Force
Write-Host "Wrote desktop FFmpeg sidecars for $triple"
