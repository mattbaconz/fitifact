---
title: "Build vs Buy"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - build-vs-buy
---

# Build vs buy

## Build — differentiation
- artifact schema;
- constraints;
- compiler;
- checker;
- planner;
- profile format;
- registry semantics;
- validation orchestration;
- explanation model;
- SDK contract.

## Integrate — commodity mechanics
- codecs;
- image encode/decode;
- PDF internals;
- office rendering/conversion;
- archive mechanics;
- hardware encoders.

## Candidate tools

### FFmpeg/ffprobe
Media inspection/transforms. Review build/license/codec matrix.

### ImageMagick/libvips
Image transforms. Sandbox broad parser surface.

### qpdf/Ghostscript/MuPDF
Potential PDF providers; license and fidelity review required.

### LibreOffice
Office conversion later; verify fidelity and deployment.

### Pandoc
Semantic document transforms, not universal visual fidelity.

## Do not build
H.264 encoder, JPEG encoder, PDF renderer, DOCX layout engine.

## Rule

> Build what decides **what should happen**. Integrate what already knows **how to encode it**.

## Use the operating system as a provider

Where a platform exposes a capable native codec/image framework, Shoehorn should be able to use it through the same provider interface.

Potential:
- Windows Media Foundation;
- Apple VideoToolbox/platform image APIs;
- Android MediaCodec.

FFmpeg remains a powerful portable fallback, not a conceptual dependency of the compatibility planner.
