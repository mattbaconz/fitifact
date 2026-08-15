$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$missing = [System.Collections.Generic.List[string]]::new()
$markdownFiles = @(rg --files $root -g "*.md" | ForEach-Object {
    if ([IO.Path]::IsPathRooted($_)) { $_ } else { Join-Path $root $_ }
})

function Test-Target([string]$Source, [string]$RawTarget) {
    $target = $RawTarget.Trim().Trim('<', '>')
    if (-not $target -or $target.StartsWith('#') -or $target -match '^[a-zA-Z][a-zA-Z0-9+.-]*:') {
        return
    }
    $target = ($target -split '#', 2)[0]
    try {
        $target = [Uri]::UnescapeDataString($target)
    }
    catch {
        $missing.Add("$Source -> malformed URI target")
        return
    }
    $candidate = Join-Path (Split-Path -Parent $Source) $target
    if (-not (Test-Path -LiteralPath $candidate)) {
        $missing.Add("$Source -> $RawTarget")
    }
}

foreach ($file in $markdownFiles) {
    $text = Get-Content -Raw -LiteralPath $file
    foreach ($match in [regex]::Matches($text, '(?m)!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)')) {
        Test-Target $file $match.Groups[1].Value
    }
    foreach ($match in [regex]::Matches($text, '\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\]')) {
        $target = $match.Groups[1].Value
        if (-not [IO.Path]::HasExtension($target)) {
            $target += ".md"
        }
        $candidate = Join-Path (Join-Path $root "docs") $target
        if (-not (Test-Path -LiteralPath $candidate)) {
            $missing.Add("$file -> $($match.Value)")
        }
    }
}

if ($missing.Count -gt 0) {
    $missing | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
    throw "$($missing.Count) local documentation link(s) are invalid"
}

Write-Host "Local Markdown and wiki-link targets: PASS ($($markdownFiles.Count) files)"
