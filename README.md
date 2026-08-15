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
| Supported video/audio streams in the wrong container | Remux without re-encoding |
| HEVC video with compatible AAC audio, target MP4/H.264/AAC | Transcode video to H.264 and copy audio |
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
`libx264`, MP4 muxing, and destination/temp write access. FFmpeg 6.1 or newer is
the tested baseline; older majors warn but are not rejected solely for age.

Fitifact has no telemetry and performs no network activity. FFmpeg and ffprobe
are system dependencies and are not bundled.

## Build and use

```console
cargo build --release --locked
cargo run -p fitifact-cli -- doctor
cargo run -p fitifact-cli -- inspect video.mp4
cargo run -p fitifact-cli -- check video.mp4 --container mp4 --video-codec h264 --audio-codec aac
cargo run -p fitifact-cli -- plan video.mov --container mp4 --video-codec h264 --audio-codec aac
cargo run -p fitifact-cli -- adapt video.mov --container mp4 --video-codec h264 --audio-codec aac
```

Use `--json` with every command for structured output; engine failures use
`fitifact.error/v1`.
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

## Project status and distribution

The intended public home is
[`mattbaconz/fitifact`](https://github.com/mattbaconz/fitifact). v0.1
distribution is GitHub-only; the Cargo packages are not published. Public
publication remains blocked until owner/legal sign-off on the Fitifact name.

See [`docs/README.md`](docs/README.md) for the documentation status model and
[`AGENTS.md`](AGENTS.md) for canonical project constraints.

## Contributing and security

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before proposing changes. Report
security issues privately as described in [`SECURITY.md`](SECURITY.md).

Licensed under the [Apache License 2.0](LICENSE).
