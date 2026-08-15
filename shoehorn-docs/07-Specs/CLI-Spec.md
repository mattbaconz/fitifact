---
title: "CLI Specification"
type: spec
status: active
updated: 2026-08-15
canonical: true
tags:
  - cli
  - spec
---

# CLI specification

Status: design draft.

## Grammar

```text
shoehorn inspect <file>
shoehorn check <file> [target]
shoehorn plan <file> [target]
shoehorn adapt <file> [target]
shoehorn profiles ...
shoehorn providers ...
shoehorn doctor
```

Name changes with final brand.

## Target forms

Profile:
```text
--for vendor/app/platform
```

Profile file:
```text
--profile ./target.yaml
```

Inline:
```text
--max-size 25mb
--format mp4
--video-codec h264
--max-width 1920
```

Requirements text:
```text
--requirements requirements.txt
```

## Preferences

Potential:
```text
--preserve resolution
--preserve audio
--allow crop
--strip metadata
--prefer local
```

## Output

Human default.
Stable `--json` for automation.

## Dry-run

`adapt --dry-run` must not mutate or create output.

## Explain

`--explain` expands reasoning and provenance.

## Safety

Default output is a new sibling file.

`--replace` explicit and should still use safe backup policy unless advanced override.

## Exit codes

Draft:
- 0 success;
- 2 incompatible check result;
- 3 unsatisfiable;
- 4 unsupported;
- 5 execution failed;
- 6 validation failed;
- 7 security/policy blocked;
- 64 usage.

## Doctor

Checks:
- provider availability;
- versions;
- hardware encoders;
- registry;
- temp space;
- cloud auth if configured.
