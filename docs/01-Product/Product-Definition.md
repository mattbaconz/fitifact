---
title: "Product Definition"
type: product
status: active
implementation: mixed
updated: 2026-08-22
canonical: true
tags:
  - product
  - canonical
---

# Product definition

This is the broader product definition. The current public candidate includes
the CLI/media engine and the D-026 static consumer image workflow. Destination
profile lookup, hosted/cloud execution, and broader file families are deferred.

## What Fitifact is

Fitifact is a **file compatibility adapter**.

It receives:
1. an input artifact;
2. a destination contract.

It returns:
1. compatible output if safely possible;
2. explanation of incompatibilities;
3. exact changes;
4. validation evidence;
5. alternatives if constraints cannot reasonably be satisfied.

## Core user story

> I have a file that does not work where I need it. I do not want to learn file-format internals. Make the smallest changes required so it works.

For the consumer image workflow:

> **Make your image pass the upload.** Drop the file, paste what the form told
> you, confirm the one-line target, adapt locally, then download an output
> validated against the requirements you confirmed.

## Product modes

### Destination mode
`Where should this work? [PowerPoint]`

### Requirements mode
`MP4/H.264, max 25 MB, max 1080p`

### Rejection mode
Paste: `Unsupported file. MP4/H.264 only. Max 25 MB.`

### Developer mode
`adapt(file, constraints)`

### Diagnostic mode
`Why won't this file work?`

## Success states

- `compatible` — input already satisfies target.
- `adapted` — transformed output validates.
- `cannot_satisfy` — no acceptable plan.
- `failed` — execution or validation failure.

There should not be a vague “conversion successful” state.

## Preserve defaults

Prefer preserving:
- semantics;
- resolution;
- frame rate;
- audio;
- metadata where appropriate;
- transparency;
- color fidelity;
- animation.

Metadata is stripped from changed image outputs in this MVP and disclosed; it
is not silently claimed as preserved. Transparency is preserved only through
PNG and never flattened implicitly. A crop is never executed without explicit
consent. Lossy quality reduction and upscaling are always warned.

## Current consumer boundary

- JPEG, PNG, still WebP, and single-image HEIC can be inputs; outputs stay
  JPEG or PNG. Public/default web builds include the D-028 lazy HEIC decoder;
  `FITIFACT_HEIC_APPROVED=false` remains the decoder-free proof.
- Files are processed in a dedicated browser worker. **Your image stays on this
  device.** There is no telemetry, upload, or cloud fallback.
- The encoded input limit is 32 MiB and the decoded limit is 24 megapixels.
- Animation and multi-image content are refused.
- Validation proves only the confirmed typed requirements, not undocumented
  destination rules or guaranteed server acceptance.

## Core promise

> **Fitifact changes files because it has to, not because it can.**
