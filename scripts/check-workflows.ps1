$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$yamlFiles = @(Get-ChildItem -LiteralPath (Join-Path $root ".github") -Recurse -File |
    Where-Object { $_.Extension -in @(".yml", ".yaml") })
if ($yamlFiles.Count -eq 0) {
    throw "No GitHub YAML files found"
}

$lintArguments = @("--yes", "yaml-lint@1.7.0") + @($yamlFiles.FullName)
& npx @lintArguments
if ($LASTEXITCODE -ne 0) {
    throw "GitHub YAML parse failed"
}

$badPins = [System.Collections.Generic.List[string]]::new()
foreach ($file in $yamlFiles) {
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $file.FullName) {
        $lineNumber += 1
        if ($line -match '^\s*(?:-\s*)?uses:\s*([^\s#]+)') {
            $reference = $Matches[1]
            if ($reference.StartsWith("./")) {
                continue
            }
            if ($reference -notmatch '@[0-9a-f]{40}$' -or $line -notmatch '#\s+v[0-9]') {
                $badPins.Add("$($file.FullName):$lineNumber")
            }
        }
    }
}
if ($badPins.Count -gt 0) {
    $badPins | ForEach-Object { Write-Error "Unpinned or unannotated action: $_" }
    throw "Every external action must use a full commit SHA and trailing version comment"
}

Write-Host "GitHub YAML parse and action-pin audit: PASS ($($yamlFiles.Count) files)"
