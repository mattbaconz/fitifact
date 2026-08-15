---
title: "CLI"
type: surface
status: active
implementation: v0.1
updated: 2026-08-15
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
fitifact adapt FILE CONSTRAINTS [-o OUTPUT] [--json] [--dry-run]
fitifact doctor
```

Constraints are typed flags (`--container`, `--video-codec`, `--audio-codec`,
`--max-size`, `--max-width`, `--max-height`) or `--constraints FILE.yaml`.
`adapt --dry-run` plans without writing.

## Executable and check-only constraints

Container changes can be executed by remux. HEVC video can be transcoded to
H.264 while compatible AAC audio is copied. Already compatible MP4/H.264/AAC is
a no-op. File size and video dimensions can be inspected and checked but cannot
be changed by the v0.1 provider. Unsupported mutations are refused.

Every created output is re-inspected and validated. The default is a new
`.adapted` sibling; `-o` may select another path, but neither originals nor
existing outputs are overwritten.

## Deferred commands and modes

Destination/profile lookup, `profiles`, `providers`, `--for`, natural-language
requirements, `--replace`, cloud selection/authentication, and richer
explanation flags are design ideas for later releases. They are not v0.1 CLI
syntax. Fitifact performs no network activity or implicit upload.
