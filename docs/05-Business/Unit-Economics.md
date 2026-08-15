---
title: "Unit Economics"
type: business
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - unit-economics
  - business
---

# Unit economics

## Why it matters

Fitifact Cloud can look cheap per request while becoming expensive due to:
- large ingress/egress;
- multi-pass video;
- GPU/CPU;
- temporary storage;
- retries.

## Cost model per job

Track:

```text
input transfer
+ storage time
+ CPU seconds
+ GPU seconds
+ intermediate disk
+ output transfer
+ registry/control-plane overhead
+ failed/retried compute
```

## Local-first economic advantage

Every successful local adaptation:
- costs cloud operator almost nothing;
- improves privacy;
- reduces gross-margin pressure.

This is strategically aligned rather than merely philosophical.

## Planning as cost control

Before execution:
- estimate resource class;
- reject abusive/impossible jobs;
- offer lower-cost plan;
- require cloud credit confirmation.

## Heavy transformations

High-risk:
- long 4K video;
- multi-pass exact-size jobs;
- document render farms.

Set:
- max duration/bytes;
- per-plan expected units;
- hard timeout.

## Gross margin

Measure by transform family, not one global average.

Example families:
- image;
- media-remux;
- media-transcode CPU;
- media-transcode GPU;
- document.

## Free tier

Free cloud should be capped by compute units, not only number of files.

## Egress

Avoid becoming a permanent storage/CDN by default. Outputs expire or export directly.

## Enterprise private workers

Can improve economics:
- customer pays compute;
- Fitifact charges control plane/support;
- reduced data-transfer burden.

## Pricing validation

Before public pricing, benchmark real provider cloud workloads and compare to current conversion APIs.
