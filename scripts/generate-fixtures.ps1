$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "fixtures\media"
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Assert-Ffmpeg {
    if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
        throw "ffmpeg is not on PATH"
    }
}

function Test-Encoder([string]$Name) {
    $encoders = & ffmpeg -hide_banner -encoders 2>$null | Out-String
    return $encoders -match [regex]::Escape($Name)
}

function Invoke-Ffmpeg([string[]]$FfmpegArgs) {
    Write-Host "ffmpeg $($FfmpegArgs -join ' ')"
    & ffmpeg @FfmpegArgs
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg failed with exit $LASTEXITCODE"
    }
}

Assert-Ffmpeg

if (-not (Test-Encoder "libx264")) {
    throw "ffmpeg build is missing libx264"
}

$commonIn = @(
    "-nostdin", "-y",
    "-f", "lavfi", "-i", "testsrc=duration=0.4:size=160x120:rate=10",
    "-f", "lavfi", "-i", "sine=frequency=440:duration=0.4"
)

Invoke-Ffmpeg ($commonIn + @(
    "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-c:a", "aac", "-b:a", "64k",
    (Join-Path $out "h264-aac.mp4")
))

Invoke-Ffmpeg ($commonIn + @(
    "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-c:a", "aac", "-b:a", "64k",
    (Join-Path $out "h264-aac.mov")
))

if (Test-Encoder "libx265") {
    Invoke-Ffmpeg ($commonIn + @(
        "-c:v", "libx265", "-pix_fmt", "yuv420p", "-tag:v", "hvc1",
        "-c:a", "aac", "-b:a", "64k",
        (Join-Path $out "hevc-aac.mp4")
    ))
} else {
    Write-Warning "libx265 not available; skipped hevc-aac.mp4"
}

Write-Host "Fixtures written to $out"
