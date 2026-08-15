---
title: "MVP Scope"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - mvp
  - engineering
  - canonical
---

# MVP scope

## Goal

Prove the category, not universal format support.

The MVP must demonstrate:
1. actual file inspection;
2. typed destination constraints;
3. compatibility diagnosis;
4. minimum-mutation planning;
5. safe execution;
6. post-validation;
7. understandable explanation;
8. one-click web experience.

## Recommended v0 families

### Images
Common:
- JPEG
- PNG
- WebP
- HEIC/HEIF if packaging is viable
- TIFF

Constraints:
- accepted formats;
- max bytes;
- min/max dimensions;
- aspect ratio;
- alpha allowed/required;
- animation allowed.

Transforms:
- orientation normalize;
- format encode;
- resize;
- crop with consent;
- size fitting;
- alpha flatten with explicit background.

### Media
Common input:
- MP4
- MOV
- WebM
- MKV where providers support.

Inspect:
- container;
- video/audio codec;
- dimensions;
- fps;
- duration;
- bytes.

Transforms:
- remux;
- selective video/audio transcode;
- size target;
- resize;
- fps cap.

## Defer

- Office documents;
- CAD/3D;
- ebooks;
- fonts;
- complex archives;
- OCR;
- complex PDF optimization;
- DRM;
- dozens of social platform profiles.

## MVP destination strategy

Use:
1. custom explicit requirements;
2. generic image target;
3. generic media target;
4. a few carefully verified real profiles only if current/sourced.

## Acceptance criteria

### Technical
- detect codec inside container;
- no-op when valid;
- preserve already-compatible streams;
- planner tests;
- output validation;
- resource limits;
- deterministic structured result schema.

### Product
A nontechnical tester can adapt a file without choosing an output format and understand what changed.

### Demo
Three reproducible fixtures:
1. MP4 + wrong codec;
2. remux-only case;
3. image wrong format/size.

## Anti-scope rule

A new format is not enough reason to expand MVP. It should demonstrate a new compatibility capability.

## Footprint acceptance criteria

The MVP should prove:
- web initial path does not eagerly load a heavyweight media runtime;
- CLI/check works without an execution provider for inspection/planning-only use;
- already-compatible fixture completes without encoder startup;
- image workflow does not initialize media provider;
- integrations contain no destination-specific compatibility logic.
