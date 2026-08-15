---
title: "Product Definition"
type: product
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - product
  - canonical
---

# Product definition

This is the broader product definition. v0.1 implements developer and
diagnostic CLI behavior for media only. Destination/profile lookup,
rejection-text parsing, images, web UI, and cloud execution are deferred.

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

## Core promise

> **Fitifact changes files because it has to, not because it can.**
