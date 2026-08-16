---
title: "CLI"
type: surface
status: active
implementation: v0.1
updated: 2026-08-16
canonical: true
tags:
  - cli
  - surface
---

# CLI

The Fitifact CLI is the only v0.1 user surface and the reference integration.
It is scriptable, human-readable by default, and supports structured JSON.

## Implemented commands

```text
fitifact inspect FILE [--json]
fitifact check FILE CONSTRAINTS [--json]
fitifact plan FILE CONSTRAINTS [--json]
fitifact adapt FILE CONSTRAINTS [-o OUTPUT] [--json] [--dry-run] [--timeout-seconds]
fitifact doctor [--json]
fitifact bench [--json] [--fixtures DIR] [--keep]
```

Constraints are typed flags (`--container`, `--video-codec`, `--audio-codec`,
`--max-size`, `--max-width`, `--max-height`) or `--constraints FILE.yaml`.
`adapt --dry-run` plans without writing.
`fitifact bench` is the canonical demo/benchmark: no-op, remux, and HEVC
transcode on tracked fixtures, with a human table or `fitifact.bench/v1` JSON.

## Executable and check-only constraints

MOV/H.264/AAC targeting MP4 remuxes without re-encoding. MP4/HEVC/AAC targeting
MP4/H.264 transcodes video and copies AAC. Already compatible MP4/H.264/AAC is
a no-op. File size and video dimensions can be inspected and checked but cannot
be changed by the v0.1 provider. Unsupported mutations are refused.

Every created output is re-inspected and validated. The default is a unique
sibling such as `video.fitifact.mp4`, then `video.fitifact.2.mp4`. `-o` may
select another new path; neither originals nor existing outputs are overwritten.

## Deferred commands and modes

Destination/profile lookup, `profiles`, `providers`, `--for`, natural-language
requirements, `--replace`, cloud selection/authentication, and richer
explanation flags are design ideas for later releases. They are not v0.1 CLI
syntax. Fitifact performs no network activity or implicit upload.
