---
title: "Plan Specification"
type: spec
status: active
implementation: implemented-v0.1
updated: 2026-08-16
canonical: true
tags:
  - plan
  - spec
---

# Plan specification

The public plan envelope is `fitifact.plan/v1`. Every outcome is tagged as
`compatible`, `planned`, or `cannot_satisfy` and records planner version
`0.1.0`.

```yaml
schema: fitifact.plan/v1
kind: planned
planner_version: 0.1.0
plan:
  schema: fitifact.plan/v1
  planner_version: 0.1.0
  steps:
    - id: step-1
      operation: media.transcode_video
      target:
        container: mp4
        video_codec: h264
      reasons:
        - constraint_id: video-codec
          message: The target requires H.264 video.
      expected:
        - field: media.video.codec
          value: h264
        - field: media.container
          value: mp4
      preservation:
        - video_dimensions
        - audio_stream_copied
      warnings: []
  preserved:
    - video_dimensions
    - audio_stream_copied
  warnings: []
```

Plans contain typed logical operations and targets only. They never serialize a
provider name, executable, shell command, or argv. Reasons link steps to hard
constraints; expected facts state only proven post-step facts; preservation
claims state only guarantees of the logical operation.

## v0.1 catalog and search

The catalog contains two operations: lossless remux to MP4, and HEVC-to-H.264
video transcode to MP4 while copying already-valid AAC audio. Breadth-first
search is bounded to depth 2 and candidates rank lexicographically by semantic
loss, lossy steps, streams changed, then step count.

The planner refuses unsafe stream topology, non-MP4 targets, audio transcode,
resize, size fitting, unsupported codecs/containers, HDR or greater-than-8-bit
conversion, unknown facts needed for safety, and any transform after which a
size constraint would be uncertain. `cannot_satisfy.blocking[]` carries stable
machine-readable codes, related constraint IDs, and a readable message.

Task 03 makes `adapt` replan from inspected input and constraints rather than
trusting plan JSON. Input/constraint hashes and provider-capability snapshots
remain future reproducibility work and are not claimed by v0.1.
