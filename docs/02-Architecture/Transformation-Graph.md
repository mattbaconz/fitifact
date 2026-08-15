---
title: "Transformation Graph"
type: architecture
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - planner
  - graph
---

# Transformation graph

## Model

Nodes are artifact states. Edges are transform capabilities.

For v0.1 the graph contains only MOV/H.264-to-MP4 lossless remux and selective
MP4/HEVC-to-H.264 video transcode with already-valid AAC copied. Other source
containers, size fitting, resizing, audio transcode, semantic/HDR conversion,
and stream dropping have no edges and therefore produce explicit
`cannot_satisfy` outcomes.

## Why graph search

Avoid:
```text
if source == MOV and target == X ...
```

Instead:
- destination -> constraints;
- providers -> edges;
- planner -> composition.

## Edge metadata

Each transform declares:
- ID/provider/version;
- preconditions;
- guaranteed effects;
- possible side effects;
- loss class;
- compute/memory estimate;
- execution modes;
- security profile.

## Edge classes

### Structural/lossless
- remux;
- archive repack;
- metadata operations;
- lossless optimize.

### Lossy media
- transcode;
- resize;
- fps reduction;
- image re-encode.

### Semantic
- flatten alpha;
- animation to still;
- editable doc to raster;
- drop tracks/features.

Semantic transforms carry high penalties and clearer consent.

## Output uncertainty

File size after remux or transcode is uncertain. If a mutation is required while
a size limit is present, v0.1 refuses the plan rather than predicting the limit
still passes. Bounded iterative fitting is deferred.

## Pruning

Use:
- family restrictions;
- destination-relevant fields;
- maximum depth;
- dominance pruning;
- semantic-loss ceiling;
- provider availability.

## Stable transform IDs

Human labels can change. IDs are API.
