---
title: "Web App"
type: surface
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - web
  - surface
---

# Web app

## Role

Zero-install trial, YouTube destination, consumer discovery and gateway to FOSS/native/cloud.

## Flow

`Drop -> inspect -> target -> check -> plan -> adapt -> validate -> save`

## Processing

- small image transforms: browser/local first;
- media: ffmpeg.wasm when practical;
- heavy jobs: native companion or explicit cloud.

## Trust

Only say `Uploads to Fitifact: 0 bytes` when payload truly stays local.

## Errors

Handle:
- unsupported inspection;
- ambiguous requirements;
- no plan;
- browser provider missing;
- out of memory;
- execution failure;
- validation failure.

## Landing demos

- correct MP4 container / wrong codec;
- file too large;
- already valid/no-op.

## SEO

Prefer intent:
- fix unsupported upload;
- MP4 not accepted;
- make video compatible;
- fit under upload limit.

Avoid thousands of low-value X-to-Y pages.

## Safety

Use workers/sandboxes. Never render hostile HTML/SVG with active privileges in app origin.

## Lazy-loading strategy

The initial page should contain only:
- UI;
- schemas/core;
- lightweight inspection.

Do not eagerly ship the media transcoder.

```text
image selected -> lazy image runtime if needed
video selected -> inspect first -> lazy media runtime only if transform needed
heavy job -> native companion or explicit cloud
```

The homepage should remain fast even as Fitifact gains more file families.
