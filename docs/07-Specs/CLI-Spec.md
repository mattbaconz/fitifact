---
title: "CLI Specification"
type: spec
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - cli
  - spec
---

# CLI specification

## v0.1 grammar — implemented

```text
fitifact inspect <file> [--json]
fitifact check <file> <typed constraints> [--json]
fitifact plan <file> <typed constraints> [--json]
fitifact adapt <file> <typed constraints> [-o <new-output>] [--json] [--dry-run] [--timeout-seconds <1..86400>]
fitifact doctor [--json]
fitifact bench [--json] [--fixtures DIR] [--keep]
fitifact --version
```

Typed constraint flags are:

```text
--container <container>
--video-codec <codec>
--audio-codec <codec>
--image-format <format>
--max-size <bytes|MB|MiB>
--max-width <pixels>
--max-height <pixels>
--constraints <file.yaml>
```

At least one constraint flag or `--constraints` file is required for `check`,
`plan`, and `adapt`. Human output is the default; `--json` is available for all
automatable commands, including `doctor`; engine failures requested as JSON use
the exact `fitifact.error/v1` envelope. `adapt --dry-run` always performs fresh
inspection/check/planning, is equivalent to `plan`, creates no output, and never
accepts a serialized plan for execution. Whole raw bytes, decimal `MB`, and
binary `MiB` use the strict shared size parser.
`--constraints` is mutually exclusive with every individual hard-target flag;
combining them is a structured usage error with exit 64. Constraint documents
are read through a 1 MiB bounded reader before UTF-8 and YAML validation; one
byte beyond the limit is rejected without reading or allocating the remainder.

Only MOV/H.264/AAC-to-MP4 remux and MP4/HEVC/AAC-to-MP4/H.264 transcode (video
re-encoded, AAC copied) are executable within the D-020 matrix. JPEG no-op and
PNG→JPEG encode are executable within D-025 and do not start FFmpeg. The remux
path requires a single compatible AAC audio stream and a safe stream topology.
File-size and dimension constraints are check-only. Unsupported source
containers, image formats, or mutations return an unsatisfiable result rather
than a misleading plan.

Default adaptation output is a unique sibling such as `video.fitifact.mp4` or
`photo.fitifact.jpg`, then a numeric suffix if needed. `-o` selects a different new output path. Existing
paths are refused before a provider starts; there is no overwrite option. Changed
output is written inside an atomically reserved hidden sibling workspace,
freshly hashed, inspected, and provenance-validated, then published with an
atomic create-if-absent hard link as the last fallible publication operation.
The final link is immediately identity-checked against the held validated stage.
After a failed or timed-out provider has been reaped, a regular partial is
identity-claimed and removed through the platform cleanup primitive. Ambiguous,
replaced, or unprovable objects are retained. Cleanup or identity-confirmation
problems emit a structured `cleanup_warning` containing a path and message. The
default transform timeout is 1800 seconds and `--timeout-seconds` is bounded to
1 through 86400 seconds.

## Exit codes — implemented

- 0: success or a valid plan;
- 2: incompatible check result;
- 3: no satisfiable plan;
- 4: invalid or unsupported input/inspection;
- 5: provider or execution failure;
- 6: validation failure;
- 7: security/policy block;
- 64: CLI usage, `REQUIREMENTS_AMBIGUOUS` / `REQUIREMENTS_CONFLICT`, or any
  other unmapped error.

`doctor` checks system `ffprobe` and `ffmpeg` versions, `libx264`, the MP4 muxer,
and destination/temp write health. Missing requirements fail with exit 5.
Doctor warns when the detected FFmpeg **major version is older than 6**. CI
currently installs FFmpeg 7.x. FFmpeg 6.x is accepted without an age warning
and is not rejected solely for age.

`bench` times the three canonical media fixtures (no-op, remux, transcode) and
the two canonical image fixtures (JPEG no-op, PNG encode), measures one CLI
inspect cold start, and prints a human table or `fitifact.bench/v1` JSON. It
constructs the FFmpeg transform provider only for remux/transcode. Image adapt
must prove zero ffmpeg spawns. Exit 0 if outcomes and proofs pass; exit 5 if
doctor is unhealthy; any other nonzero status means a proof or canonical
outcome failed. `--fixtures` selects the media directory (default
`fixtures/media` or `FITIFACT_FIXTURES`); image fixtures are the sibling
`image` directory. `--keep` retains the temporary adaptation workspace.

## Deferred grammar

Profiles and destination lookup (`--for`, `--profile`, `profiles`), provider
listing, natural-language requirements, preferences, `--explain`, `--replace`,
registry checks, cloud authentication, and remote execution are deferred. They
must not appear in v0.1 usage examples as if implemented.
