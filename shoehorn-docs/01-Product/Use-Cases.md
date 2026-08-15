---
title: "Use Cases"
type: product
status: active
updated: 2026-08-15
canonical: true
tags:
  - use-cases
---

# Use cases

## Signature use cases

### Rejected media upload
`video.mov` is HEVC/AAC, 41.8 MB. Target needs MP4/H.264 <=25 MB. Shoehorn selectively transcodes video, preserves valid audio, fits size, validates.

### “It is already MP4. Why does it fail?”
MP4 container contains unsupported HEVC. Shoehorn diagnoses container vs. codec and changes only the incompatible stream.

### Assignment file too large
Target `< 2 MB`. Shoehorn tries lossless/low-impact reductions first, then controlled quality reduction, stopping below a safe margin. PDF is likely phase 2.

### Profile image requirements
HEIC/TIFF input; target needs JPEG/PNG, square, min dimensions, max bytes. Crop requires consent.

### Presentation media compatibility
Resolve exact app/platform profile, diagnose unsupported container/codec/audio, adapt and validate.

## Strong extensions

- email attachment fit;
- browser-safe media;
- uploader auto-repair;
- CMS normalization;
- messaging/share-sheet compatibility;
- archive repack;
- audio codec/channel/sample-rate adaptation;
- transparency flattening with explicit background;
- animated image/video bridge.

## Developer use cases

### Auto-heal upload widget
`validate -> plan -> consent -> adapt -> validate -> upload`

### Backend normalization
Store deterministic delivery-safe variants.

### Preflight API
Inspect/check/plan without processing.

### Policy enforcement
Company declares accepted formats, sizes, dimensions and metadata policies.

## Anti-use cases

Not the first choice for:
- video/photo editing;
- professional encoding ladders;
- authoring;
- archival preservation;
- malware certification;
- DRM bypass;
- arbitrary novelty conversion.
