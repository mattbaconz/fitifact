# Fitifact

Fitifact is a destination-first media compatibility CLI. It inspects what a
file actually contains, checks typed destination constraints, chooses the
smallest supported change, executes it with system FFmpeg, and validates the
result.

```text
inspect -> constraints -> check -> plan -> execute -> validate
```

## v0.1 scope

This release is intentionally narrow:

| Input and requested result | v0.1 behavior |
| --- | --- |
| MP4 with H.264 video and AAC audio already satisfying the target | No-op; no encoder starts |
| MOV with H.264 video and AAC audio, target MP4/H.264/AAC | Remux without re-encoding |
| MP4 with HEVC video and AAC audio, target MP4/H.264/AAC | Transcode video to H.264 and copy audio |
| Any other mutation | Refuse explicitly |

File-size and video-dimension constraints are supported by `inspect` and
`check`, but v0.1 cannot execute size fitting or resizing. It never overwrites
the input or an existing output, never silently drops streams, and never treats
an FFmpeg success code as proof of compatibility.

Images, profiles, natural-language parsing, web/browser/desktop/mobile
interfaces, cloud execution, managed APIs, bundled FFmpeg, package registries,
OS signing, and package-manager formulae are deferred.

## Requirements

- Rust toolchain with edition 2024 support (when building from source)
- `ffprobe` on `PATH` for inspection, check, and plan
- `ffmpeg` on `PATH` for adaptations that require a change

Run `fitifact doctor` (or `fitifact doctor --json`) to verify versions,
`libx264`, MP4 muxing, and destination/temp write access. Doctor warns when the
FFmpeg major version is older than 6; CI currently tests FFmpeg 7.x. FFmpeg 6.x
is accepted without an age warning and is not rejected solely for age.

Fitifact has no telemetry and performs no network activity. FFmpeg and ffprobe
are system dependencies and are not bundled.

Install FFmpeg from its official project or an operating-system package source:
[FFmpeg's download page](https://ffmpeg.org/download.html) links official source
code and compiled-package providers.

```console
# Ubuntu/Debian
sudo apt update && sudo apt install ffmpeg

# macOS (Homebrew)
brew install ffmpeg

# Windows (WinGet)
winget install --id Gyan.FFmpeg -e
```

Confirm both `ffmpeg -version` and `ffprobe -version`, then run
`fitifact doctor`. FFmpeg remains external software under the license terms of
the build you install.

## Build and use

```console
cargo build --release --locked
cargo run -p fitifact-cli -- doctor
cargo run -p fitifact-cli -- inspect video.mp4
cargo run -p fitifact-cli -- check video.mp4 --container mp4 --video-codec h264 --audio-codec aac
cargo run -p fitifact-cli -- plan video.mov --container mp4 --video-codec h264 --audio-codec aac
cargo run -p fitifact-cli -- adapt video.mov --container mp4 --video-codec h264 --audio-codec aac
cargo run -p fitifact-cli -- bench
```

Use `--json` with every command for structured output; engine failures use
`fitifact.error/v1`. `fitifact bench` (and `fitifact bench --json`) is the
canonical demo: it times no-op, remux, and HEVC transcode on the tracked
fixtures and prints spawn/provider proofs. Run it from the repository root so
`fixtures/media` resolves.
Use `adapt --dry-run` to plan without writing a file. By default, adaptation
writes a unique sibling such as `video.fitifact.mp4` or
`video.fitifact.2.mp4`; `-o` chooses another new path and existing paths are
refused. The transform timeout defaults to 1800 seconds and can be set from 1
through 86400 seconds with `--timeout-seconds`.

`--max-size` accepts exact whole bytes, decimal `MB`, or binary `MiB` (for
example `25000000`, `25 MB`, or `25 MiB`). Adaptation plans are always recreated
from fresh inspection and typed constraints; saved JSON plans are never
executable input.

Typed constraints can also be loaded from YAML:

```console
cargo run -p fitifact-cli -- check video.mp4 --constraints fixtures/constraints/mp4-h264-aac.yaml
```

## Prepared release installation and verification

No release assets exist yet. After owner/legal approval and publication, the
prepared GitHub release workflow is configured to produce ZIP or tar.gz
archives, `fitifact-cli-installer.sh`, `fitifact-cli-installer.ps1`, a unified
`sha256.sum`, per-archive SHA-256 files, and a CycloneDX
`fitifact-cli.cdx.xml` SBOM.

The checked-in package and binary version is the unpublished
`0.1.0-rc.1` candidate. Stable `0.1.0` requires a later reviewed version-bump
commit after RC acceptance; this commit must not receive the stable tag.

Download `sha256.sum` and the one archive for your target into the same
directory. Verify only that exact downloaded asset before extraction:

```console
# Linux x64
line="$(grep -E '^[0-9a-f]{64}  fitifact-cli-x86_64-unknown-linux-gnu\.tar\.gz$' sha256.sum)" && printf '%s\n' "$line" | sha256sum --check --strict -

# macOS Intel
line="$(grep -E '^[0-9a-f]{64}  fitifact-cli-x86_64-apple-darwin\.tar\.gz$' sha256.sum)" && printf '%s\n' "$line" | shasum --algorithm 256 --check -

# macOS Apple Silicon
line="$(grep -E '^[0-9a-f]{64}  fitifact-cli-aarch64-apple-darwin\.tar\.gz$' sha256.sum)" && printf '%s\n' "$line" | shasum --algorithm 256 --check -
```

Each `grep` pattern is anchored to one filename; a missing manifest entry makes
the pipeline fail. On Windows, select and compare the exact x64 ZIP entry:

```powershell
$asset = "fitifact-cli-x86_64-pc-windows-msvc.zip"
$entries = @(Select-String -Path .\sha256.sum -Pattern "^[0-9a-fA-F]{64}  $([regex]::Escape($asset))$")
if ($entries.Count -ne 1) { throw "Expected exactly one checksum entry for $asset" }
$expected = ($entries[0].Line -split '\s+', 2)[0]
$actual = (Get-FileHash ".\$asset" -Algorithm SHA256).Hash
if ($actual -ne $expected) { throw "SHA-256 mismatch for $asset" }
"$asset`: SHA-256 verified"
```

GitHub artifact attestations can be verified with GitHub CLI after replacing
`<archive>` with the downloaded release filename:

```console
gh attestation verify <archive> --repo mattbaconz/fitifact
```

The CycloneDX XML describes the resolved Cargo dependency graph and is released
alongside the archives. Windows binaries are not Authenticode-signed, and macOS
binaries are not code-signed or notarized in v0.1; users should expect platform
warnings and rely on the checksum, SBOM, attestation, and source/tag provenance.

If no prebuilt archive is suitable, the exact prepared source fallback is:

```console
cargo install --git https://github.com/mattbaconz/fitifact --locked fitifact-cli
```

This command becomes usable only after the public repository exists.

## Project status and distribution

The intended public home is
[`mattbaconz/fitifact`](https://github.com/mattbaconz/fitifact). v0.1
distribution is GitHub-only; the Cargo packages are not published. Public
publication remains blocked until owner/legal sign-off on the Fitifact name.

The release procedure and clean-machine acceptance gates are documented in
[`docs/04-Engineering/Release-Checklist.md`](docs/04-Engineering/Release-Checklist.md).

See [`docs/README.md`](docs/README.md) for the documentation status model and
[`AGENTS.md`](AGENTS.md) for canonical project constraints.

## Contributing and security

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before proposing changes. Report
security issues privately as described in [`SECURITY.md`](SECURITY.md).

Licensed under the [Apache License 2.0](LICENSE).
