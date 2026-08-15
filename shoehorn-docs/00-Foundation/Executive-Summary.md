---
title: "Executive Summary"
type: foundation
status: active
updated: 2026-08-15
canonical: true
tags:
  - foundation
  - summary
---

# Executive summary

Shoehorn is a proposed **universal file compatibility layer**.

Existing file tools overwhelmingly ask users to choose a transformation:

```text
MOV -> MP4
PNG -> JPEG
DOCX -> PDF
```

Shoehorn starts from the user's actual intent:

```text
Make this file work in PowerPoint.
Make this image satisfy this upload form.
Make this PDF fit under 2 MB.
Make this video accepted by this service.
```

That changes the software contract from:

```text
convert(input_format, output_format)
```

to:

```text
adapt(file, destination_constraints)
```

## Product insight

People generally do not want a new file format. They want an outcome:

- upload accepted;
- media playable;
- attachment small enough;
- image dimensions valid;
- transparency preserved;
- document viewable;
- browser/device compatibility;
- app-specific compatibility.

Today's workflow pushes knowledge of codecs, containers, compression, DPI, MIME types, dimensions, and platform limits onto the user.

Shoehorn's thesis:

> **Compatibility should be the software's problem, not the user's.**

## Core engine

The engine has five conceptual stages:

1. **Inspect** the actual file, not just the extension.
2. **Compile constraints** from a profile, explicit structured input, page hints, or pasted requirements.
3. **Plan** the least-destructive valid transformation path.
4. **Execute** with trusted transform providers.
5. **Validate** the result against the original hard constraints.

```text
file
  ↓
INSPECT
  ↓
actual capabilities/state
  +
target constraints
  ↓
PLAN
  ↓
minimal transformation graph
  ↓
EXECUTE
  ↓
VALIDATE
  ↓
compatible output + explanation
```

## Strongest differentiation

Shoehorn must not compete on “number of formats.”

VERT, CloudConvert, Convert.to.it, File Converter, FFmpeg, ImageMagick and others already cover conversion extremely well.

Shoehorn's differentiation is:

- **destination-first** instead of output-format-first;
- **constraint-driven** rather than preset-only;
- **minimum mutation** rather than blind conversion;
- **compatibility verification** after processing;
- **explanation** of why the original failed;
- **portable compatibility profiles** with evidence and freshness;
- one core that can power web, browser, desktop, mobile, CLI, and developer API.

## FOSS strategy

The engine should be **genuinely open**, likely Apache-2.0 for Shoehorn-owned code after license review.

Open:
- inspection;
- constraint model;
- planner;
- local executor;
- CLI;
- SDK;
- community profiles;
- self-hosting.

Paid cloud:
- managed high-throughput adaptation;
- heavyweight CPU/GPU processing;
- queues/retries;
- verified compatibility registry;
- destination monitoring;
- team controls;
- webhooks;
- auditability;
- enterprise/private workers;
- SLAs/support.

## Competitive reality

The idea has substantial prior art in pieces:

- Android has compatible media transcoding.
- calibre auto-converts ebooks for destination devices.
- HandBrake has device presets.
- Smart Converter selectively converts only streams that need conversion.
- Clop automatically turns less compatible formats into broadly compatible ones.
- Cloudinary automatically chooses delivery formats.
- VERT and Convert.to.it are broad FOSS conversion experiences.
- CloudConvert, Filestack, Uploadcare, Transloadit, ConvertAPI provide hosted conversion infrastructure.
- Historical research such as Grace (2005) explored transparent conversion of browser-incompatible web formats.

This is validation but means Shoehorn should **not** claim invention of automatic compatibility or content adaptation.

The potentially under-owned horizontal abstraction is:

> **Any file + any destination constraints -> minimum-change compatible output.**

## Largest risks

1. Name collision: “Shoehorn” is already used in software.
2. Scope explosion across file types.
3. Incorrect/stale compatibility profiles.
4. “Success” outputs that still fail at the destination.
5. Security risk from hostile files and external parsers.
6. Codec/patent/license complexity.
7. Cloud processing costs.
8. Incumbents can add a destination-intent layer.
9. Browser/mobile platform restrictions.
10. Becoming a converter with extra steps instead of a new compatibility primitive.

## Recommended MVP

Start with **images + common video/audio constraints**:
- format/container;
- video/audio codec;
- max file size;
- dimensions;
- aspect ratio;
- frame rate ceiling;
- duration ceiling;
- transparency;
- basic color/alpha compatibility.

Add PDFs after the planner proves itself.

## Launch

Best current YouTube title:

> **I Made an Adapter for Files**

Opening premise:

> “When a cable doesn't fit, you use an adapter. When a file doesn't fit, computers expect you to understand codecs.”
