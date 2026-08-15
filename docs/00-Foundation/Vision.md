---
title: "Vision"
type: foundation
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - vision
  - foundation
---

# Vision

## North star

Fitifact becomes the compatibility layer between **arbitrary files** and **arbitrary destinations**.

Not a website where you convert files, but infrastructure that makes file incompatibility disappear.

## Today

```text
user has file
   ↓
destination rejects it
   ↓
user researches requirements
   ↓
user finds converter
   ↓
user guesses output settings
   ↓
user retries
   ↓
maybe accepted
```

## Fitifact future

```text
user has file
   ↓
destination understands its own constraints
   ↓
Fitifact adapts locally or via managed worker
   ↓
validated output
   ↓
accepted
```

Eventually, users should rarely need to know Fitifact exists.

A file uploader should be able to say:

```text
This file doesn't meet our requirements.
[ Fix automatically ]
```

An OS should be able to say:

```text
Open with target app
↓
adapt only if required
```

A developer should be able to write:

```text
adapt(file, constraints)
```

instead of a custom web of validation, format checks, transcoding presets and user-facing error messages.

## Category thesis

Hardware has adapters because interfaces disagree.

Software files have the same problem:
- containers;
- codecs;
- dimensions;
- size ceilings;
- profiles;
- colorspaces;
- alpha;
- metadata expectations;
- document feature support;
- archive methods;
- platform capabilities.

But software usually exposes the disagreement directly to the user.

Fitifact defines **file adaptation** as a first-class primitive:

> Transform a file only as much as necessary to satisfy a destination contract.

## Success layers

### 1. Loved FOSS utility
People discover it, drop a rejected file, and get a working result.

### 2. Developer primitive
Products integrate Fitifact so users stop seeing preventable upload errors.

### 3. Compatibility infrastructure
Fitifact operates a continuously tested compatibility registry and managed compute fabric.

## Anti-vision

Fitifact fails if it becomes:
- a “2000 formats supported” SEO converter;
- a wrapper around one FFmpeg command;
- a giant list of unvalidated presets;
- an AI chat box around file conversion;
- a cloud-only service that uploads everything by default;
- an OSS teaser whose useful engine is proprietary;
- a brittle browser extension that claims universal magic.

## Design philosophy

The internal architecture can be sophisticated. The user contract should remain primitive:

```text
This file doesn't work there.
Fitifact can make it fit.
```

That asymmetry—deep engine, simple mental model—is part of the product.
