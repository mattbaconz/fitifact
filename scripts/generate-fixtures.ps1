param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "fixtures\media"
$fixtureNames = @(
    "compatible-h264-aac.mp4",
    "mismatch-hevc-aac.mp4",
    "remux-h264-aac.mov",
    "corrupt-truncated.mp4",
    "unsupported-extra-video.mp4",
    "refusal-hdr10-hevc-aac.mp4"
)

function Assert-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is not on PATH. Install system FFmpeg before generating fixtures."
    }
}

function Assert-Encoder([string]$Name, [string]$Purpose) {
    $encoders = & ffmpeg -hide_banner -encoders 2>$null | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg could not enumerate encoders (exit $LASTEXITCODE)."
    }
    if ($encoders -notmatch "(?m)^\s*[A-Z\.]{6}\s+$([regex]::Escape($Name))\s") {
        throw "ffmpeg encoder '$Name' is unavailable; it is required for $Purpose."
    }
}

function Invoke-Ffmpeg([string[]]$FfmpegArgs, [string]$Purpose) {
    Write-Host "ffmpeg $($FfmpegArgs -join ' ')"
    & ffmpeg @FfmpegArgs
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg failed while generating $Purpose (exit $LASTEXITCODE). Check encoder pixel-format support."
    }
}

Assert-Command "ffmpeg"
Assert-Command "ffprobe"
Assert-Encoder "libx264" "H.264 fixtures"
Assert-Encoder "libx265" "HEVC and HDR/10-bit fixtures"
Assert-Encoder "aac" "AAC fixture audio"

New-Item -ItemType Directory -Force -Path $out | Out-Null
$existing = @($fixtureNames | ForEach-Object { Join-Path $out $_ } | Where-Object { Test-Path -LiteralPath $_ })
if ($existing.Count -gt 0 -and -not $Force) {
    throw "Fixture outputs already exist. Re-run with -Force to replace only the six known fixture files."
}
if ($Force) {
    foreach ($path in $existing) {
        Remove-Item -LiteralPath $path
    }
}

$commonInput = @(
    "-nostdin", "-hide_banner", "-loglevel", "error", "-n",
    "-f", "lavfi", "-i", "testsrc2=size=160x120:rate=10:duration=0.6",
    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=0.6",
    "-map", "0:v:0", "-map", "1:a:0", "-map_metadata", "-1",
    "-metadata", "creation_time=1970-01-01T00:00:00Z",
    "-fflags", "+bitexact", "-flags:v", "+bitexact", "-flags:a", "+bitexact",
    "-threads", "1", "-c:a", "aac", "-b:a", "48k"
)
$h264 = @(
    "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-x264-params", "threads=1:lookahead_threads=1:sliced_threads=0",
    "-color_range", "tv", "-colorspace", "bt709", "-color_trc", "bt709",
    "-color_primaries", "bt709"
)
$hevcSdr = @(
    "-c:v", "libx265", "-pix_fmt", "yuv420p", "-tag:v", "hvc1",
    "-x265-params", "pools=none:frame-threads=1:wpp=0:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited",
    "-color_range", "tv", "-colorspace", "bt709", "-color_trc", "bt709",
    "-color_primaries", "bt709"
)

Invoke-Ffmpeg ($commonInput + $h264 + @(
    "-movflags", "+faststart", (Join-Path $out "compatible-h264-aac.mp4")
)) "compatible H.264/AAC MP4"

Invoke-Ffmpeg ($commonInput + $hevcSdr + @(
    "-movflags", "+faststart", (Join-Path $out "mismatch-hevc-aac.mp4")
)) "HEVC/AAC mismatch MP4"

Invoke-Ffmpeg ($commonInput + $h264 + @(
    "-movflags", "+faststart", (Join-Path $out "remux-h264-aac.mov")
)) "H.264/AAC MOV remux input"

$extraInput = @(
    "-nostdin", "-hide_banner", "-loglevel", "error", "-n",
    "-f", "lavfi", "-i", "testsrc2=size=160x120:rate=10:duration=0.6",
    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=0.6",
    "-f", "lavfi", "-i", "color=c=blue:size=32x32:rate=10:duration=0.6",
    "-map", "0:v:0", "-map", "1:a:0", "-map", "2:v:0",
    "-map_metadata", "-1", "-metadata", "creation_time=1970-01-01T00:00:00Z",
    "-fflags", "+bitexact", "-threads", "1",
    "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-x264-params", "threads=1:lookahead_threads=1:sliced_threads=0",
    "-c:a", "aac", "-b:a", "48k", "-movflags", "+faststart",
    (Join-Path $out "unsupported-extra-video.mp4")
)
Invoke-Ffmpeg $extraInput "unsupported extra-video topology"

$hdrInput = @(
    "-nostdin", "-hide_banner", "-loglevel", "error", "-n",
    "-f", "lavfi", "-i", "testsrc2=size=160x120:rate=10:duration=0.6",
    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=0.6",
    "-map", "0:v:0", "-map", "1:a:0", "-map_metadata", "-1",
    "-metadata", "creation_time=1970-01-01T00:00:00Z",
    "-fflags", "+bitexact", "-flags:v", "+bitexact", "-flags:a", "+bitexact",
    "-threads", "1", "-c:a", "aac", "-b:a", "48k",
    "-c:v", "libx265", "-pix_fmt", "yuv420p10le", "-tag:v", "hvc1",
    "-x265-params", "pools=none:frame-threads=1:wpp=0:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:range=limited",
    "-color_range", "tv", "-colorspace", "bt2020nc", "-color_trc", "smpte2084",
    "-color_primaries", "bt2020", "-movflags", "+faststart",
    (Join-Path $out "refusal-hdr10-hevc-aac.mp4")
)
Invoke-Ffmpeg $hdrInput "HDR/10-bit HEVC refusal input"

$source = Join-Path $out "compatible-h264-aac.mp4"
$truncated = Join-Path $out "corrupt-truncated.mp4"
$sourceStream = [System.IO.File]::OpenRead($source)
try {
    $buffer = New-Object byte[] 24
    $read = $sourceStream.Read($buffer, 0, $buffer.Length)
    $destinationStream = [System.IO.File]::Open($truncated, [System.IO.FileMode]::CreateNew)
    try {
        $destinationStream.Write($buffer, 0, $read)
    }
    finally {
        $destinationStream.Dispose()
    }
}
finally {
    $sourceStream.Dispose()
}

Write-Host "Generated with:"
& ffmpeg -version | Select-Object -First 1
& ffprobe -version | Select-Object -First 1
Write-Host "Fixtures written to $out"
