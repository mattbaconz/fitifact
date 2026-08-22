$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo run --locked -p fitifact --example generate_image_fixtures
    if ($LASTEXITCODE -ne 0) {
        throw "Rust image fixture generator failed (exit $LASTEXITCODE)."
    }

    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "generate-heic-fixture.ps1")
        if ($LASTEXITCODE -ne 0) {
            throw "Windows HEIC fixture generator failed (exit $LASTEXITCODE)."
        }
    }
    else {
        Write-Warning "HEIC generation requires Windows PowerShell 5.1 plus the Microsoft HEIF Image Extension. Existing fixture remains checksum-verified."
    }
}
finally {
    Pop-Location
}
