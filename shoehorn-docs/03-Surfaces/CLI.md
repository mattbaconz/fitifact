---
title: "CLI"
type: surface
status: active
updated: 2026-08-15
canonical: true
tags:
  - cli
  - surface
---

# CLI

## Goals
Scriptable, composable, human-friendly and JSON-stable.

## Commands

```text
shoehorn inspect FILE
shoehorn check FILE --for TARGET
shoehorn plan FILE --for TARGET
shoehorn adapt FILE --for TARGET
shoehorn adapt FILE --max-size 25mb --video-codec h264
shoehorn profile show ID
shoehorn providers
shoehorn doctor
```

Executable name changes if brand changes.

## Example

```text
INCOMPATIBLE

Video codec
 actual: HEVC
 required: H.264

Size
 actual: 41.8 MB
 max: 25 MB

Suggested plan:
 1. Transcode video only
 2. Fit under size

Audio preserved.
```

## JSON
All automatable commands support `--json`.

## Dry run
`adapt --dry-run` equals plan.

## Local/cloud
Explicit execution mode. No implicit upload.

## Safety
`--replace` explicit. Default creates new file.

## Reference-integration role

The CLI is the reference integration.

Given identical artifact facts, constraints, preferences and capability catalog, CLI/web/extension should choose equivalent plans.

If an integration has special compatibility behavior, move it into core/profile data rather than leaving logic in the integration.
