$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$binDir = Join-Path $root "apps\desktop\src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

# The one FFmpeg build the Windows app ships. apps/desktop/FFMPEG-NOTICE.md and
# web/public/THIRD_PARTY_NOTICES.md describe it; change all three together.
$pinnedVersion = "9.0.2"
$pinnedUrl = "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip"
$pinnedSha256 = "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba"

function Get-RustHostTriple {
    $line = rustc -vV | Select-String -Pattern '^host: ' | Select-Object -First 1
    if (-not $line) { throw "rustc did not report a host triple" }
    return ($line.Line -replace '^host:\s+', '').Trim()
}

$triple = Get-RustHostTriple
$exe = if ($triple -match 'windows') { ".exe" } else { "" }
$ffmpegDest = Join-Path $binDir "ffmpeg-$triple$exe"
$ffprobeDest = Join-Path $binDir "ffprobe-$triple$exe"
# Bundled beside the programs by tauri.conf.json.
$licenseDest = Join-Path $binDir "FFMPEG-LICENSE.txt"
$buildDest = Join-Path $binDir "FFMPEG-BUILD.txt"

if ($triple -ne "x86_64-pc-windows-msvc") {
    # Development only: no installer is built for this platform.
    foreach ($name in "ffmpeg", "ffprobe") {
        $command = Get-Command $name -ErrorAction SilentlyContinue
        if (-not $command) { throw "No $name on PATH for $triple. Install ffmpeg and ffprobe, then rerun." }
        Copy-Item -LiteralPath $command.Source -Destination (Join-Path $binDir "$name-$triple$exe") -Force
    }
    $note = "Copied from PATH for a local development build on $triple. This is not the build Fitifact ships; its own license applies."
    Set-Content -LiteralPath $licenseDest -Value $note -Encoding utf8
    Set-Content -LiteralPath $buildDest -Value $note -Encoding utf8
    Write-Host "Copied FFmpeg sidecars from PATH for $triple (development only)"
    exit 0
}

$present = (Test-Path -LiteralPath $ffmpegDest) -and (Test-Path -LiteralPath $ffprobeDest) -and
    (Test-Path -LiteralPath $licenseDest) -and (Test-Path -LiteralPath $buildDest) -and
    (Select-String -LiteralPath $buildDest -SimpleMatch "Version: $pinnedVersion-essentials_build-www.gyan.dev" -Quiet)
if ($present) {
    Write-Host "Pinned FFmpeg $pinnedVersion sidecars already present for $triple"
    exit 0
}

$work = Join-Path $env:TEMP "fitifact-ffmpeg-sidecar"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$zip = Join-Path $work (Split-Path -Leaf $pinnedUrl)
if (-not (Test-Path -LiteralPath $zip) -or (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash -ne $pinnedSha256) {
    Write-Host "Downloading FFmpeg $pinnedVersion essentials for $triple"
    Invoke-WebRequest -Uri $pinnedUrl -OutFile $zip
}
$actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash
if ($actual -ne $pinnedSha256) {
    Remove-Item -LiteralPath $zip -Force
    throw "FFmpeg archive SHA-256 mismatch: expected $pinnedSha256, got $actual"
}
$extract = Join-Path $work "extract"
if (Test-Path -LiteralPath $extract) { Remove-Item -LiteralPath $extract -Recurse -Force }
Expand-Archive -LiteralPath $zip -DestinationPath $extract
$build = Join-Path $extract ([IO.Path]::GetFileNameWithoutExtension($pinnedUrl))
Copy-Item -LiteralPath (Join-Path $build "bin\ffmpeg.exe") -Destination $ffmpegDest -Force
Copy-Item -LiteralPath (Join-Path $build "bin\ffprobe.exe") -Destination $ffprobeDest -Force
Copy-Item -LiteralPath (Join-Path $build "LICENSE") -Destination $licenseDest -Force
Copy-Item -LiteralPath (Join-Path $build "README.txt") -Destination $buildDest -Force
Write-Host "Wrote pinned FFmpeg $pinnedVersion sidecars and notices for $triple"
