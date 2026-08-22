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
$ciWorkflow = [IO.File]::ReadAllText((Join-Path $root ".github\workflows\ci.yml"))
$ciRequirements = @(
    @{ Name = "quality job"; Pattern = '(?m)^  quality:\s*$' },
    @{ Name = "platform matrix job"; Pattern = '(?m)^  platform:\s*$' },
    @{ Name = "MSRV job"; Pattern = '(?m)^  msrv:\s*$' },
    @{ Name = "supply-chain job"; Pattern = '(?m)^  supply-chain:\s*$' },
    @{ Name = "fixture gate"; Pattern = '\./scripts/check-fixtures\.ps1' },
    @{ Name = "documentation gate"; Pattern = '\./scripts/check-doc-links\.ps1' },
    @{ Name = "pinned Node action"; Pattern = 'actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4\.4\.0' },
    @{ Name = "pinned Node file"; Pattern = 'node-version-file: web/\.node-version' },
    @{ Name = "pinned npm"; Pattern = 'npm install --global npm@11\.5\.1' },
    @{ Name = "WASM target"; Pattern = 'rustup target add wasm32-unknown-unknown' },
    @{ Name = "locked web install"; Pattern = '(?m)^\s+run: npm ci\s*$' },
    @{ Name = "web lint"; Pattern = '(?m)^\s+run: npm run lint\s*$' },
    @{ Name = "web unit tests"; Pattern = '(?m)^\s+run: npm test\s*$' },
    @{ Name = "default web build"; Pattern = '(?m)^\s+run: npm run build\s*$' },
    @{ Name = "decoder-free HEIC gate"; Pattern = 'FITIFACT_HEIC_APPROVED: "false"' },
    @{ Name = "decoder-free HEIC browser check"; Pattern = 'npm run test:e2e:heic-off' },
    @{ Name = "all-browser workflow"; Pattern = '(?m)^\s+run: npm run test:e2e\s*$' },
    @{ Name = "Pages base path"; Pattern = 'FITIFACT_BASE_PATH: /fitifact/' },
    @{ Name = "public HEIC Pages assertion"; Pattern = 'Public Pages artifact is missing the lazy HEIC decoder' },
    @{ Name = "default HEIC artifact assertion"; Pattern = 'Default artifact is missing the lazy HEIC decoder' },
    @{ Name = "Pages asset-path assertion"; Pattern = 'Pages artifact does not use the /fitifact/ base path' },
    @{ Name = "pinned Pages artifact upload"; Pattern = 'actions/upload-pages-artifact@7b1f4a764d45c48632c6b24a0339c27f5614fb0b # v4' },
    @{ Name = "post-gate Pages deploy job"; Pattern = '(?ms)^  deploy-pages:\r?\n.*?^    needs:\r?\n      - quality\r?\n      - web\r?\n      - platform\r?\n      - msrv\r?\n      - supply-chain\s*$' },
    @{ Name = "pinned Pages deployment"; Pattern = 'actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e # v4' }
)
foreach ($requirement in $ciRequirements) {
    if ($ciWorkflow -notmatch $requirement.Pattern) {
        throw "CI workflow is missing invariant: $($requirement.Name)"
    }
}
if ($ciWorkflow -match '(?m)^\s*continue-on-error:\s*true\s*$') {
    throw "CI workflow must not weaken a gate with continue-on-error"
}
$releaseRequirements = @(
    @{ Name = "read-only plan job"; Pattern = '(?ms)^  plan:\r?\n.*?^    permissions:\r?\n      "contents": "read"\s*$' },
    @{ Name = "repository publication approval"; Pattern = "vars\.FITIFACT_PUBLICATION_APPROVED == 'true'" },
    @{ Name = "protected release environment"; Pattern = '(?m)^      name: public-release\s*$' },
    @{ Name = "tag planning without release creation"; Pattern = '(?m)^\s+dist plan .*--output-format=json' },
    @{ Name = "cargo-dist 0.32 GitHub host source evidence"; Pattern = 'cargo-dist v0\.32\.0.*implemented in CI backend' },
    @{ Name = "non-publication host status"; Pattern = 'dist manifest prepared successfully; GitHub publication has not started' },
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

$githubReleaseCreates = [regex]::Matches($releaseWorkflow, '(?m)^\s*gh release create\s+')
if ($githubReleaseCreates.Count -ne 1) {
    throw "Release workflow must contain exactly one GitHub release creation command; found $($githubReleaseCreates.Count)"
}
if ($releaseWorkflow -match '(?m)^\s*dist host .*--steps=create' -or
    $releaseWorkflow -match '(?m)^\s*gh release upload\s+' -or
    $releaseWorkflow -match '(?m)^\s*gh api .*\/releases' -or
    $releaseWorkflow -match '(?m)^\s*(?:-\s*)?uses:\s*(?:actions\/create-release|softprops\/action-gh-release)') {
    throw "Release workflow contains a second GitHub release creation/upload primitive"
}
$attestationIndex = $releaseWorkflow.IndexOf('uses: actions/attest@', [System.StringComparison]::Ordinal)
$releaseCreateIndex = $releaseWorkflow.IndexOf('gh release create ', [System.StringComparison]::Ordinal)
if ($attestationIndex -lt 0 -or $releaseCreateIndex -lt 0 -or $attestationIndex -gt $releaseCreateIndex) {
    throw "GitHub artifact attestation must complete before the sole GitHub release creation command"
}

Write-Host "GitHub YAML parse, action pins, publication gate, permissions, attestation order, single release creation, and SBOM scope: PASS ($($yamlFiles.Count) files)"
