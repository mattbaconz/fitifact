$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$media = Join-Path $root "fixtures\media"
$manifest = Join-Path $media "SHA256SUMS"
$expectedNames = @(
    "compatible-h264-aac.mp4",
    "corrupt-truncated.mp4",
    "mismatch-hevc-aac.mp4",
    "refusal-hdr10-hevc-aac.mp4",
    "remux-h264-aac.mov",
    "unsupported-extra-video.mp4"
)

if (-not (Test-Path -LiteralPath $manifest)) {
    throw "Missing fixtures/media/SHA256SUMS"
}

$listed = @{}
foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "Malformed SHA256SUMS line"
    }
    $listed[$Matches[2]] = $Matches[1]
}

$actualNames = @(Get-ChildItem -LiteralPath $media -File | Where-Object {
    $_.Extension -in @(".mp4", ".mov")
} | ForEach-Object Name | Sort-Object)
if (Compare-Object ($expectedNames | Sort-Object) $actualNames) {
    throw "Fixture directory does not contain exactly the six canonical media binaries"
}

foreach ($name in $expectedNames) {
    $path = Join-Path $media $name
    $length = (Get-Item -LiteralPath $path).Length
    if ($length -gt 100KB) {
        throw "Fixture exceeds the 100 KiB limit: $name"
    }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if (-not $listed.ContainsKey($name) -or $listed[$name] -ne $hash) {
        throw "Fixture checksum mismatch: $name"
    }
    Write-Host "$hash  $name ($length bytes)"
}

$readme = Get-Content -Raw -LiteralPath (Join-Path $media "README.md")
foreach ($required in @("2026-08-16", "ffmpeg version", "ffprobe version", "lavfi")) {
    if (-not $readme.Contains($required)) {
        throw "Fixture provenance is missing: $required"
    }
}

Write-Host "Fixture hashes, size bounds, inventory, and provenance: PASS"

$image = Join-Path $root "fixtures\image"
$imageManifest = Join-Path $image "SHA256SUMS"
$imageNames = @(
    "compatible-jpeg.jpg",
    "crop-grid.png",
    "malformed-image.jpg",
    "mismatch-png.png",
    "oversized-pixels.png",
    "still-webp.webp",
    "synthetic-single.heic",
    "transparent-png.png"
)
if (-not (Test-Path -LiteralPath $imageManifest)) {
    throw "Missing fixtures/image/SHA256SUMS"
}
$imageListed = @{}
foreach ($line in Get-Content -LiteralPath $imageManifest) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "Malformed image SHA256SUMS line"
    }
    $imageListed[$Matches[2]] = $Matches[1]
}
$imageActual = @(Get-ChildItem -LiteralPath $image -File | Where-Object {
    $_.Extension -in @(".jpg", ".jpeg", ".png", ".heic", ".webp")
} | ForEach-Object Name | Sort-Object)
if (Compare-Object ($imageNames | Sort-Object) $imageActual) {
    throw "Image fixture directory does not contain exactly the eight canonical image binaries"
}
foreach ($name in $imageNames) {
    $path = Join-Path $image $name
    $length = (Get-Item -LiteralPath $path).Length
    $limit = if ($name -eq "oversized-pixels.png") { 512KB } else { 100KB }
    if ($length -gt $limit) {
        throw "Image fixture exceeds its $limit-byte repository limit: $name"
    }
    if ($name -eq "oversized-pixels.png" -and $length -le 100KB) {
        throw "Oversized image fixture must remain above the ordinary 100 KiB fixture bound"
    }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if (-not $imageListed.ContainsKey($name) -or $imageListed[$name] -ne $hash) {
        throw "Image fixture checksum mismatch: $name"
    }
    Write-Host "$hash  $name ($length bytes)"
}
$imageReadme = Get-Content -Raw -LiteralPath (Join-Path $image "README.md")
foreach ($required in @("2026-08-21", "owned synthetic pixels", "Windows HEIF encoder", "JPEG", "PNG", "WebP", "HEIC", "malformed", "24-megapixel")) {
    if (-not $imageReadme.Contains($required)) {
        throw "Image fixture provenance is missing: $required"
    }
}
Write-Host "Image fixture hashes, size bounds, inventory, and provenance: PASS"
