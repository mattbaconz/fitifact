---
title: "Constraint Schema"
type: spec
status: active
implementation: implemented-v0.1
updated: 2026-08-16
canonical: true
tags:
  - constraints
  - spec
---

# Constraint schema

The public v0.1 contract is `fitifact.constraints/v1`. YAML and JSON use the
same hard-constraint shape. Callers must parse public input with
`compile_from_yaml` or `compile_from_json`; direct
`serde_json::from_str::<ConstraintSet>` only deserializes the Rust shape and is
not a semantic-validation entry point.

```yaml
schema: fitifact.constraints/v1
hard:
  - id: container
    field: media.container
    op: in
    value: [mp4]
  - id: video-codec
    field: media.video.codec
    op: in
    value: [h264]
  - id: audio-codec
    field: media.audio.codec
    op: in
    value: [aac]
preferences:
  preserve_audio: true
  preserve_resolution: true
```

## v0.1 fields and combinations

| Field | Operator | Value |
| --- | --- | --- |
| `file.family` | `eq` | known family string |
| `media.container` | `in` | non-empty known-container list |
| `media.video.codec` | `in` | non-empty known-video-codec list |
| `media.audio.codec` | `in` | non-empty known-audio-codec list |
| `file.bytes` | `lte` | positive integer bytes |
| `media.video.width` | `lte` | positive integer pixels |
| `media.video.height` | `lte` | positive integer pixels |
| `image.format` | `in` | non-empty known-image-format list |
| `image.width` | `lte` | positive integer pixels |
| `image.height` | `lte` | positive integer pixels |

Both validating compiler functions reject input over 1 MiB, a missing or wrong schema, an empty hard
target, blank or duplicate IDs, conflicting requirements, unknown keys or enum
values, empty lists, zero limits, and every unsupported field/operator/value
combination. Unknown extension fields are not ignored in v0.1.

Programmatic CLI inputs compile into this same validated model through
`compile`. Size text may be
whole unadorned bytes, decimal `MB`, or binary `MiB`; unit names are
case-insensitive. Unitless fractions, fractional-byte results, ambiguous units,
and overflow are rejected.

An unknown inspection fact never satisfies a hard constraint. File-size and
dimension constraints are check-only; the planner refuses fitting or
resizing. The first image target that can be produced is JPEG (D-025).
