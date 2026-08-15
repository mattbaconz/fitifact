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

$releaseWorkflow = [IO.File]::ReadAllText((Join-Path $root ".github\workflows\release.yml"))
$releaseRequirements = @(
    @{ Name = "read-only plan job"; Pattern = '(?ms)^  plan:\r?\n.*?^    permissions:\r?\n      "contents": "read"\s*$' },
    @{ Name = "repository publication approval"; Pattern = "vars\.FITIFACT_PUBLICATION_APPROVED == 'true'" },
    @{ Name = "protected release environment"; Pattern = '(?m)^      name: public-release\s*$' },
    @{ Name = "tag planning without release creation"; Pattern = '(?m)^\s+dist plan .*--output-format=json' },
    @{ Name = "single-binary-package CycloneDX invocation"; Pattern = 'cargo cyclonedx -v --format xml --describe binaries --manifest-path crates/fitifact-cli/Cargo\.toml' },
    @{ Name = "single-SBOM assertion"; Pattern = 'expected exactly one uploaded CycloneDX XML file' }
)
foreach ($requirement in $releaseRequirements) {
    if ($releaseWorkflow -notmatch $requirement.Pattern) {
        throw "Release workflow is missing invariant: $($requirement.Name)"
    }
}
foreach ($permission in @("contents", "attestations", "id-token")) {
    $writeCount = ([regex]::Matches($releaseWorkflow, "`"$permission`": `"write`"")).Count
    if ($writeCount -ne 1) {
        throw "Release workflow must grant $permission write exactly once (host only); found $writeCount"
    }
}
if ($releaseWorkflow -match '(?m)^  announce:\s*$') {
    throw "Release workflow contains the removed write-capable announce job"
}

Write-Host "GitHub YAML parse, action pins, publication gate, permissions, and SBOM scope: PASS ($($yamlFiles.Count) files)"
