---
title: "CLI Specification"
type: spec
status: active
implementation: mixed
updated: 2026-08-15
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
fitifact adapt <file> <typed constraints> [-o <new-output>] [--json] [--dry-run]
fitifact doctor
```

Typed constraint flags are:

```text
--container <container>
--video-codec <codec>
--audio-codec <codec>
--max-size <bytes>
--max-width <pixels>
--max-height <pixels>
--constraints <file.yaml>
```

At least one constraint flag or `--constraints` file is required for `check`,
`plan`, and `adapt`. Human output is the default; `--json` is available for all
automatable commands. `adapt --dry-run` is equivalent to planning and creates no
output.

Container and HEVC-to-H.264 changes are executable within the D-020 matrix.
File-size and dimension constraints are check-only in v0.1. Unsupported
mutations return an unsatisfiable result rather than a misleading plan.

Default adaptation output is a new sibling file named with `.adapted` before
the target extension. `-o` selects a different new output path. Existing paths
are refused; there is no overwrite option.

## Exit codes — implemented

- 0: success or a valid plan;
- 2: incompatible check result;
- 3: no satisfiable plan;
- 4: invalid or unsupported input/inspection;
- 5: provider or execution failure;
- 6: validation failure;
- 7: security/policy block;
- 64: CLI usage or other unmapped error.

`doctor` checks only system `ffprobe` and `ffmpeg` availability and versions.

## Deferred grammar

Profiles and destination lookup (`--for`, `--profile`, `profiles`), provider
listing, natural-language requirements, preferences, `--explain`, `--replace`,
registry checks, cloud authentication, and remote execution are deferred. They
must not appear in v0.1 usage examples as if implemented.
