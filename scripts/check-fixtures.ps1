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
