---
title: "Core Engine"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - architecture
  - core
---

# Core engine

## Responsibilities

Own:
- normalized artifact state;
- typed constraints;
- compatibility evaluation;
- transform capability model;
- planning;
- explanation metadata.

Do not own:
- UI;
- cloud queue;
- arbitrary shell construction;
- platform scraping.

## Logical API

```text
inspect(file) -> Artifact
compile(sources) -> ConstraintSet
check(Artifact, ConstraintSet) -> CompatibilityReport
plan(Artifact, ConstraintSet, Preferences, Capabilities) -> PlanSet
execute(Plan, ExecutionContext) -> Artifact
validate(Artifact, ConstraintSet) -> ValidationReport
adapt(...) -> AdaptationResult
```

## Artifact state

Example:

```text
Artifact
- family: media
- container: mp4
- bytes: 41_800_000
- streams:
  - video
    codec: hevc
    width: 1920
    height: 1080
    fps: 60/1
    hdr: true
  - audio
    codec: aac
    channels: 2
```

## Constraint evaluation

Every hard constraint returns:
- pass;
- fail;
- unknown.

Unknown is never coerced to pass.

## Transform capability

```text
transcode-video:
  preconditions:
    family: media
    video_stream: true
  can_change:
    - video.codec
    - video.bitrate
    - video.pixel_format
  may_change:
    - file.bytes
    - metadata
  costs:
    quality_loss: variable
    compute: high
```

## Explanations as data

Violation:
```text
field: video.codec
actual: hevc
required: [h264]
simple: "Your video uses HEVC. This target needs H.264."
```

Do not generate user explanations from raw stderr.

## Purity

Planner should be unit-testable with fake artifacts and fake providers. Real files belong in integration tests.

## Extension process

A new file family needs:
1. normalized facts;
2. inspector;
3. constraints;
4. transforms;
5. validators;
6. fixtures;
7. security review.

Avoid destination-specific branches in core.
