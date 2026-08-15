---
title: "File Inspection"
type: architecture
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - inspection
  - architecture
---

# File inspection

## Principle

**Never trust the extension.**

`video.mp4` can contain unsupported codecs. A file can be mislabeled, malformed, truncated or hostile.

## Layers

### Type/container sniffing
Use signatures and parser evidence, not filename alone.

### Structural metadata
Dimensions, streams, codec, profiles, duration, frame rate, alpha, color, pages, archive members, etc.

### Risk indicators
Macros, enormous dimensions, extreme frame/page counts, external references and decompression expansion.

These are indicators, not a malware verdict.

## Media facts

The `fitifact.artifact/v1` media inspection normalizes every stream, not only
the first video/audio stream:
- container;
- streams;
- stream index and tagged type, including subtitle, data, attachment, and unknown;
- video codec and dimensions;
- rational frame rate;
- bitrate;
- duration;
- pixel format and bit depth;
- explicit HDR status and color range/space/transfer/primaries;
- audio channels/sample rate;
- subtitle/attachment streams.

FFprobe is the v0.1 provider. Unknown or omitted facts remain optional and never
count as pass. Provider version is recorded when the probe output supplies it.

Inspection accepts broader topologies than execution. Before planning a v0.1
mutation, Fitifact requires exactly one video, zero or one audio, and no other
streams; it refuses unsafe topology rather than dropping streams.

## Image facts

- decoded format;
- dimensions;
- frame count;
- alpha;
- bit depth;
- colorspace/profile;
- orientation;
- animation;
- metadata size.

## PDF facts later

- page count;
- PDF version;
- encryption;
- embedded fonts/images;
- active features;
- page geometry.

## Result trust

Inspection records:
- provider;
- version;
- completeness;
- warnings.

## Resource limits

Inspection is an attack surface:
- max time;
- max memory;
- pixel limits;
- frame/page limits;
- archive recursion/entry limits;
- expansion ratio.

## Cache

Cache by content hash with inspector/schema version.

## Privacy

Inspection metadata can expose GPS, author names and other sensitive metadata. Keep local by default.
