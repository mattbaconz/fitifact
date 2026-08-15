---
title: "Plan Specification"
type: spec
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - plan
  - spec
---

# Plan specification

Status: conceptual.

## Example

```yaml
schema: fitifact.plan/v1
id: plan_123
input_hash: sha256:...
planner_version: 0.1.0
constraints_hash: sha256:...

steps:
  - id: step-1
    transform: media.transcode_video
    params:
      codec: h264
      preserve_resolution: true
    reason:
      constraint: video-codec

  - id: step-2
    transform: media.fit_size
    params:
      max_bytes: 25000000
      margin: 0.98
    reason:
      constraint: max-size

expected:
  media.video.codec: h264
  file.bytes:
    lte: 25000000

preserved:
  - media.audio.stream
  - media.video.width
  - media.video.height

cost:
  semantic_loss: none
  lossy_operations: 1
  compute: high

warnings: []
```

## Invariants

- exact input hash;
- exact constraints hash;
- typed steps;
- no shell;
- reasons link to requirements;
- expected state;
- validation implied by hard constraints.

## Expiry

Plan must be rejected/replanned if input, constraints or provider capability context materially changes.

## Alternatives

Can return:
- recommended;
- fastest;
- highest quality;
- smallest.

Every offered plan must satisfy hard constraints.

## Unsatisfiable

Return blocking constraints and minimal relaxations instead of fake plan.

## Provider binding

Logical transform can be provider-neutral until execution unless reproducibility pins a provider.
