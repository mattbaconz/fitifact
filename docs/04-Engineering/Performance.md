---
title: "Performance"
type: engineering
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - performance
---

# Performance

## Main optimization

The fastest conversion is the one Fitifact avoids.

Minimum mutation creates wins:
- no-op;
- remux;
- one-stream transform;
- metadata-only fix.

## Budgets

Inspection should feel near-instant for common files.
Planning should be negligible vs transformation.
Browser work must run off main thread.
Native should stream where possible.

## WASM

ffmpeg.wasm makes browser-side media conversion possible and is strategically useful for privacy/zero-install.

But browser media processing can be slower and memory-constrained. Use selectively; native/cloud should exist for heavy jobs.

## Size-target adaptation

Use:
- bitrate estimate;
- safety margin;
- bounded two-pass/iterative attempts;
- early impossible detection.

## Cache

Safe:
- inspection by hash/version;
- profile resolution;
- provider capability graph.

Cross-user output deduplication is privacy-sensitive.

## Benchmark set

- small image;
- 50 MB 1080p;
- 500 MB 4K;
- no-op;
- remux;
- one-stream transcode;
- size target.

## Metric

Measure:
> time from incompatible input to **validated compatible output**.

## Optimization rules

Enforce:
1. no eager provider loading;
2. no unnecessary full-file reads;
3. stream large artifacts where possible;
4. avoid duplicate intermediates;
5. no-op loads almost nothing;
6. remux never initializes lossy encoders unnecessarily;
7. keep profiles compact/declarative;
8. AI/NLP is optional and lazy;
9. no permanent background service by default;
10. profile before micro-optimizing.

See [[04-Engineering/Performance-Budgets]].
