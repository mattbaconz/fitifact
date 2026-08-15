---
title: "Transformation Graph"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - planner
  - graph
---

# Transformation graph

## Model

Nodes are artifact states. Edges are transform capabilities.

```text
MOV/HEVC/AAC/41MB
   │ transcode video
   ▼
MOV/H264/AAC/~32MB
   │ remux
   ▼
MP4/H264/AAC/~32MB
   │ fit size
   ▼
MP4/H264/AAC/24.5MB
```

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

File size is often an estimate. Edge can expose range or bounded iterative controller.

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
