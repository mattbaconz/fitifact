$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$missing = [System.Collections.Generic.List[string]]::new()
$markdownFiles = @(git -C $root ls-files -- '*.md' | ForEach-Object {
    Join-Path $root $_
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

$copyPaths = @(
    "docs/00-Foundation/Decision-Log.md",
    "docs/01-Product/Positioning-Messaging.md",
    "docs/01-Product/Product-Definition.md",
    "docs/03-Surfaces/Web-App.md",
    "docs/04-Engineering/MVP-Scope.md",
    "docs/06-Research/Competitors.md",
    "docs/06-Research/Threats.md",
    "web/src/App.tsx"
)
$copyText = ($copyPaths | ForEach-Object {
    [IO.File]::ReadAllText((Join-Path $root $_))
}) -join "`n"
foreach ($requiredCopy in @(
    "Make your image pass the upload",
    "Your image stays on this device",
    "validated against the requirements you confirmed"
)) {
    if (-not $copyText.Contains($requiredCopy)) {
        throw "Product copy boundary is missing: $requiredCopy"
    }
}
if ($copyText -match '(?i)validated acceptance') {
    throw "Product copy must not describe confirmed-constraint validation as validated acceptance"
}
Write-Host "Product copy boundaries and acceptance caveat: PASS"
