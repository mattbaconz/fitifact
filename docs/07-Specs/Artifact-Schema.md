---
title: "Artifact Schema"
type: spec
status: active
implementation: implemented-v0.1
updated: 2026-08-16
canonical: true
tags:
  - artifact
  - inspection
  - spec
---

# Artifact schema

Inspections serialize as `fitifact.artifact/v1`. The artifact records raw byte
length, normalized container and duration facts, inspection provider metadata,
and every ffprobe stream in input order.

Each stream carries an optional probe index and a tagged `type`: `video`,
`audio`, `subtitle`, `data`, `attachment`, or `unknown`. A missing probe index is
serialized as unknown; Fitifact does not invent one.

Video facts include codec, width, height, rational frame rate, pixel format,
bit depth, color range/space/transfer/primaries, and explicit `sdr`, `hdr`, or
`unknown` HDR status. Audio facts include codec, channels, and sample rate.
Other stream types retain their codec and original unknown type where present.
Missing probe facts remain optional/unknown and can never produce a passing hard
constraint.

Inspection metadata records provider name, provider version when ffprobe emits
it, completeness, and warnings. Inspection may represent files broader than the
v0.1 execution topology; the planner separately requires exactly one video,
zero or one audio, and no other streams before it proposes a mutation.
