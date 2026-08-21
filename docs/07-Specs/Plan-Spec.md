---
title: "Plan Specification"
type: spec
status: active
implementation: implemented-v0.1-and-d026
updated: 2026-08-21
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
        - video_pixel_format
        - video_color_metadata
        - audio_stream_copied
      warnings: []
  preserved:
    - video_dimensions
    - video_pixel_format
    - video_color_metadata
    - audio_stream_copied
  warnings: []
```

Plans contain typed logical operations and targets only. They never serialize a
provider name, executable, shell command, or argv. Reasons link steps to hard
constraints; expected facts state only proven post-step facts; preservation
claims state only guarantees of the logical operation.

## Native media catalog and search

The bounded media catalog contains lossless MOV/H.264-to-MP4 remux and
MP4/HEVC-to-H.264 video transcode while copying already-valid AAC audio. WebM,
Matroska, unknown containers, and MOV/HEVC remain outside the executable media
matrix and return `cannot_satisfy`. Breadth-first search is bounded
to depth 2 and candidates rank lexicographically by semantic loss, lossy steps,
streams changed, then step count.

The planner intersects every same-field `in` constraint before deciding the
effective target, so constraint order cannot change feasibility. It refuses
unsafe stream topology, non-MP4 targets, audio transcode, resize, size fitting,
unsupported codecs/containers, HDR or bit-depth conversion, pixel-format or
color conversion, unknown facts needed for safety, and any transform after
which a size constraint would be uncertain. Selective transcode is restricted
to known 8-bit `yuv420p`, SDR, limited-range BT.709 input; other or unknown
pixel/color facts return stable blocking reasons rather than allowing FFmpeg to
force a semantic conversion. `cannot_satisfy.blocking[]` carries stable
machine-readable codes, related constraint IDs, and a readable message.

Task 03 makes `adapt` replan from inspected input and constraints rather than
trusting plan JSON. Input/constraint hashes and provider-capability snapshots
remain future reproducibility work and are not claimed by v0.1.

## D-026 typed image plan

Image execution uses the distinct `fitifact.image-adapt-plan/v1` envelope. Its
required `plan` member is the same deserializable `fitifact.plan/v1` shape used
by media planning, with an additive typed `image.adapt` step. The distinct
outer discriminator holds execution-specific source, crop, fitting, and no-op
fields without assigning an incompatible shape to `fitifact.plan/v1`. This
abridged excerpt shows the discriminator and embedded canonical plan:

```yaml
schema: fitifact.image-adapt-plan/v1
plan:
  schema: fitifact.plan/v1
  planner_version: 0.1.0
  steps:
    - id: step-1
      operation: image.adapt
      target:
        image:
          noop: false
          source_format: png
          source_width: 1200
          source_height: 630
          output:
            format: jpeg
            width: 1200
            height: 630
      reasons:
        - constraint_id: image-format
          message: The image output must satisfy image.format.
      expected:
        - field: image.format
          value: jpeg
        - field: image.width
          value: 1200
        - field: image.height
          value: 630
      preservation: [image_dimensions]
      warnings: []
  preserved: [image_dimensions]
  warnings: []
```

The envelope returns one typed `image.adapt` target with source/target format
and dimensions, optional `max_bytes`, preservation claims, metadata behavior,
quality/upscale warnings, proportional-reduction permission, and a crop object
that states whether explicit consent is required. It is not provider argv and
cannot carry a shell command.

JPEG/PNG plans may be no-op, source-format preserving, format-changing,
aspect-preserving resize, consented crop, or bounded byte fitting. The engine
refuses implicit transparency flattening and a required crop without a valid
normalized crop rectangle plus `crop_consent: true`. Changed output uses
`normalize_orientation_and_strip`; unchanged output uses
`preserve_unchanged`.

Execution is bounded to seven JPEG encodes (quality 95 through 50), three
proportional dimension reductions, 32 MiB encoded input, and 24 megapixels
decoded. The produced bytes are re-inspected and checked against the same
constraints. Provider success cannot substitute for validation.

HEIC decoding is an approval-gated pre-inspection adapter, not a new plan
operation. Exactly one HEIC image is decoded to RGBA/PNG and then receives the
same typed image plan. Zero/multiple images are refused.
