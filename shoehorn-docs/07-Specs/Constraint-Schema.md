---
title: "Constraint Schema"
type: spec
status: active
updated: 2026-08-15
canonical: true
tags:
  - constraints
  - spec
---

# Constraint schema

Status: conceptual.

## Constraint

```json
{
  "id": "max-size",
  "field": "file.bytes",
  "op": "lte",
  "value": 5000000,
  "severity": "hard",
  "source": "src-1"
}
```

## Operators

Initial:
- eq;
- neq;
- in;
- not_in;
- lt/lte/gt/gte;
- exists;
- absent;
- ratio_eq.

Later:
- conditional;
- any_of groups.

## Fields

### File
`file.bytes`, `file.family`, `file.extension`, `file.mime`

### Image
`image.format`, `width`, `height`, `aspect_ratio`, `alpha`, `frame_count`, `bit_depth`, `colorspace`

### Media
`media.container`, `duration_ms`, `video.codec`, `video.width`, `video.height`, `video.fps`, `video.pixel_format`, `video.hdr`, `audio.codec`, `audio.channels`, `audio.sample_rate`

### PDF later
`pdf.version`, `pdf.pages`, `pdf.encrypted`, active features, etc.

## Preferences

```json
{
  "preserve": {
    "resolution": "high",
    "frameRate": "high",
    "audio": "high",
    "metadata": "medium"
  },
  "execution": {
    "prefer": "local"
  }
}
```

Do not expose internal numeric weights as ordinary user API.

## Units

Canonical:
- bytes;
- pixels;
- milliseconds;
- rational fps;
- Hz.

Human parsers normalize.

## Unknown

Unknown inspection fact does not satisfy a hard constraint.

## Extension namespace

Third parties can define:
`x.vendor.field`

Core ignores unknown extension fields unless registered.
