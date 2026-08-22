param(
    [switch]$RequireDependencyTools
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $failures = [System.Collections.Generic.List[string]]::new()
    $skips = [System.Collections.Generic.List[string]]::new()

    $secretPattern = '-----BEGIN ([A-Z ]+ )?PRIVATE KEY-----|ghp_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}|sk_live_[A-Za-z0-9]{20,}'
    $nativePref = $null
    if (Test-Path variable:/PSNativeCommandUseErrorActionPreference) {
        $nativePref = $PSNativeCommandUseErrorActionPreference
        $PSNativeCommandUseErrorActionPreference = $false
    }
    try {
        $secretFiles = @(git grep -Il -E -- "$secretPattern" .)
        $gitStatus = $LASTEXITCODE
    }
    finally {
        if ($null -ne $nativePref) {
            $PSNativeCommandUseErrorActionPreference = $nativePref
        }
    }
    if ($gitStatus -gt 1) {
        Write-Host "Tracked-secret git grep failed with exit code $gitStatus"
        $failures.Add("high-signal tracked-secret scan")
    }
    elseif ($secretFiles.Count -gt 0) {
        $secretFiles | ForEach-Object { Write-Error "Possible tracked secret in file: $_" }
        $failures.Add("high-signal tracked-secret scan")
    }
    else {
        Write-Host "Tracked-secret scan (filenames only): PASS"
    }

    $allowedBinaries = @(
        "fixtures/media/compatible-h264-aac.mp4",
        "fixtures/media/corrupt-truncated.mp4",
        "fixtures/media/mismatch-hevc-aac.mp4",
        "fixtures/media/refusal-hdr10-hevc-aac.mp4",
        "fixtures/media/remux-h264-aac.mov",
        "fixtures/media/unsupported-extra-video.mp4",
        "fixtures/image/compatible-jpeg.jpg",
        "fixtures/image/mismatch-png.png",
        "fixtures/image/transparent-png.png",
        "fixtures/image/crop-grid.png",
        "fixtures/image/malformed-image.jpg",
        "fixtures/image/oversized-pixels.png",
        "fixtures/image/synthetic-single.heic",
        "docs/04-Engineering/evidence/consumer-image-upload-mvp/before-desktop.png",
        "docs/04-Engineering/evidence/consumer-image-upload-mvp/before-mobile.png",
        "docs/04-Engineering/evidence/consumer-image-upload-mvp/after-desktop.png",
        "docs/04-Engineering/evidence/consumer-image-upload-mvp/after-mobile.png",
        "docs/brand/ft-mark-light.png",
        "docs/brand/ft-icon-dark.png",
        "docs/brand/ft-mark-simple.png"
    )
    $unexpectedBinaries = [System.Collections.Generic.List[string]]::new()
    foreach ($path in @(git ls-files)) {
        $full = Join-Path $root $path
        if (-not (Test-Path -LiteralPath $full) -or (Get-Item -LiteralPath $full -Force).Length -eq 0) {
            continue
        }
        $stream = [IO.File]::OpenRead($full)
        try {
            $buffer = New-Object byte[] ([Math]::Min(8192, $stream.Length))
            $read = $stream.Read($buffer, 0, $buffer.Length)
            $isBinary = $buffer[0..([Math]::Max(0, $read - 1))] -contains 0
        }
        finally {
            $stream.Dispose()
        }
        $normalized = $path.Replace('\', '/')
        if ($isBinary -and $normalized -notin $allowedBinaries) {
            $unexpectedBinaries.Add($normalized)
        }
    }
    if ($unexpectedBinaries.Count -gt 0) {
        $unexpectedBinaries | ForEach-Object { Write-Error "Unexpected tracked binary: $_" }
        $failures.Add("unexpected binary scan")
    }
    else {
        Write-Host "Tracked binary allow-list: PASS"
    }

    $licenseFiles = @(git ls-files | Where-Object { (Split-Path -Leaf $_) -match '^(?i:licen[sc]e|copying|notice)(\..*)?$' })
    if ($licenseFiles.Count -ne 1 -or $licenseFiles[0].Replace('\', '/') -ne "LICENSE") {
        $licenseFiles | ForEach-Object { Write-Error "Unexpected tracked license/notice file: $_" }
        $failures.Add("license-content inventory")
    }
    else {
        $licenseOk = $true
        $licenseText = [IO.File]::ReadAllText((Join-Path $root "LICENSE"))
        $requiredLicenseMarkers = @(
            "Apache License",
            "Version 2.0, January 2004",
            "TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION",
            "END OF TERMS AND CONDITIONS"
        )
        if ($licenseText.Length -lt 10000) {
            Write-Error "Root LICENSE is unexpectedly short for the Apache-2.0 terms"
            $licenseOk = $false
        }
        foreach ($marker in $requiredLicenseMarkers) {
            if (-not $licenseText.Contains($marker)) {
                Write-Error "Root LICENSE is missing a required Apache-2.0 legal marker"
                $licenseOk = $false
            }
        }

        $cargoManifest = [IO.File]::ReadAllText((Join-Path $root "Cargo.toml"))
        if ($cargoManifest -notmatch '(?m)^license\s*=\s*"Apache-2\.0"\s*$') {
            Write-Error "Cargo workspace metadata is missing the Apache-2.0 SPDX expression"
            $licenseOk = $false
        }
        $readme = [IO.File]::ReadAllText((Join-Path $root "README.md"))
        if (-not $readme.Contains("[Apache License 2.0](LICENSE)")) {
            Write-Error "README is missing the root Apache-2.0 license link"
            $licenseOk = $false
        }

        if ($licenseOk) {
            Write-Host "Tracked license content, Apache-2.0 SPDX metadata, and legal markers: PASS"
        }
        else {
            $failures.Add("license content/SPDX/legal-marker scan")
        }
    }

    $requiredReleaseInputs = @(
        ".github/dependabot.yml",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "Cargo.lock",
        "deny.toml",
        "dist-workspace.toml",
        "rust-toolchain.toml",
        "fixtures/media/SHA256SUMS",
        "fixtures/image/SHA256SUMS",
        "web/package.json",
        "web/package-lock.json",
        "web/.node-version"
    )
    foreach ($path in $requiredReleaseInputs) {
        git ls-files --error-unmatch -- $path 1>$null 2>$null
        if ($LASTEXITCODE -ne 0) {
            $failures.Add("untracked release input: $path")
        }
        git check-ignore -q -- $path
        if ($LASTEXITCODE -eq 0) {
            $failures.Add("ignored release input: $path")
        }
    }
    $dirty = @(git status --porcelain=v1 --untracked-files=all)
    if ($dirty.Count -gt 0) {
        $dirty | ForEach-Object {
            $path = if ($_.Length -gt 3) { $_.Substring(3) } else { "unknown" }
            Write-Error "Dirty or untracked path: $path"
        }
        $failures.Add("clean tracked release-input check")
    }
    else {
        Write-Host "Tracked/ignored/dirty release-input checks: PASS"
    }

    foreach ($tool in @("cargo-audit", "cargo-deny")) {
        if (Get-Command $tool -ErrorAction SilentlyContinue) {
            if ($tool -eq "cargo-audit") {
                cargo audit
            }
            else {
                cargo deny check
            }
            if ($LASTEXITCODE -ne 0) {
                $failures.Add("$tool scan")
            }
            else {
                Write-Host "$tool dependency/license scan: PASS"
            }
        }
        else {
            $skips.Add("$tool unavailable")
            if ($RequireDependencyTools) {
                $failures.Add("$tool required but unavailable")
            }
        }
    }

    foreach ($skip in $skips) {
        Write-Warning "SKIP: $skip"
    }
    if ($failures.Count -gt 0) {
        $failures | Sort-Object -Unique | ForEach-Object { Write-Error "FAIL: $_" }
        throw "Public-readiness scan failed"
    }
    Write-Host "Public-readiness scan: PASS"
}
finally {
    Pop-Location
}
