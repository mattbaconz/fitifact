---
title: "Quality and Loss Model"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - quality
  - architecture
---

# Quality and loss model

## Why this exists

“Quality” is not one universal number.

Shoehorn needs enough structure to prefer minimum-damage plans without pretending a JPEG quality value, video CRF and document editability are directly comparable.

## Loss dimensions

### Semantic loss
Did the meaning/capability change?
Examples:
- animation -> still;
- audio removed;
- editable vector -> raster;
- form fields flattened.

Highest priority.

### Structural loss
Features removed:
- metadata;
- subtitles;
- extra audio tracks;
- alpha;
- layers.

### Perceptual loss
Image/video/audio degradation.

### Resolution loss
Spatial dimensions reduced.

### Temporal loss
Frame rate/duration changes.

### Color loss
HDR->SDR, gamut/profile changes, bit-depth reduction.

## Loss classes

Use categorical classes for cross-family planning:

- `none`
- `structural_only`
- `low`
- `moderate`
- `high`
- `semantic`

Provider can include family-specific metrics internally.

## User preferences

Examples:
```text
preserve audio: hard
preserve resolution: high
preserve metadata: low
allow HDR->SDR: false
```

## Do not say “97% quality”

Unless a metric is rigorously defined, a fake precision number undermines trust.

Prefer:
> “Lossless remux”
> “Video re-encoded; resolution preserved”
> “HDR will be converted to SDR”
> “Image dimensions reduced from 4000×3000 to 2000×1500”

## Candidate ranking

Semantic loss dominates minor performance gains.

## Preview

For visible lossy transforms, the web app can offer before/after preview when feasible.

## Provider calibration

Over time, maintain benchmarks that map encoder settings to:
- perceptual metrics;
- size;
- speed.

Use those for planning within a media family, not as universal truth across families.
