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
    $secretFiles = @(git grep -Il -E $secretPattern -- . 2>$null)
    if ($secretFiles.Count -gt 0) {
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
        "fixtures/media/unsupported-extra-video.mp4"
    )
    $unexpectedBinaries = [System.Collections.Generic.List[string]]::new()
    foreach ($path in @(git ls-files)) {
        $full = Join-Path $root $path
        if (-not (Test-Path -LiteralPath $full) -or (Get-Item -LiteralPath $full).Length -eq 0) {
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
        Write-Host "Tracked license/notice inventory: PASS (Apache-2.0 root LICENSE only)"
    }

    $requiredReleaseInputs = @(
        ".github/dependabot.yml",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "Cargo.lock",
        "deny.toml",
        "dist-workspace.toml",
        "rust-toolchain.toml",
        "fixtures/media/SHA256SUMS"
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
