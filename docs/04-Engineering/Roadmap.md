---
title: "Roadmap"
type: roadmap
status: active
implementation: mixed
updated: 2026-08-21
canonical: true
tags:
  - roadmap
---

# Roadmap

The public `0.1.0-rc.4` candidate includes the frozen media matrix and the
D-026 local consumer image upload MVP. Every later phase below is deferred and
is not a commitment or implementation claim.

## v0.1 — CLI/media slice (frozen locally)
- media inspection;
- constraint schema;
- bounded transform graph;
- planner;
- FFmpeg provider;
- CLI;
- fixtures.

Exit: no-op/remux/selective transcode chosen correctly. Tag target:
`b033552cb2729e96ca97c649a7bb4a223f2ad900`.

## D-026 — consumer image upload MVP (implemented, human gate pending)
- deterministic image-requirement parsing and editable typed target;
- JPEG/PNG no-op, adaptation, crop consent, resize/byte fitting, warnings,
  metadata disclosure, transparency refusal, and post-validation;
- `fitifact-wasm`, dedicated module worker, and static Vite/React workflow;
- 32 MiB encoded/24 MP decoded bounds, cancellation, and explicit failures;
- default-off lazy approved HEIC decoder with notices and owned fixture;
- desktop/mobile browser verification and checksum-pinned synthetic fixtures;
- no ffmpeg.wasm, hosted service, telemetry, or cloud fallback.

Exit is not engineering tests alone. Run
[[04-Engineering/Consumer-Image-Moderated-Test]] with ten real tasks and meet
all 8/10, 8/10, 5/10, and zero-harm thresholds before continuing investment.

## Product expansion — deferred
- hosted web app;
- WebP / TIFF / animation adaptation and default HEIC enablement;
- custom destination profiles;
- automatic destination requirement capture.

## Broader rejection compiler — deferred
- broader non-image rejection text;
- evidence spans;
- conflict UI;
- optional model-assisted leftovers.

## Browser extension — deferred
- page hints with consent;
- selected file;
- local/native delegation;
- save/retry workflow.

## Registry — deferred
- profile repo;
- provenance;
- fixtures;
- freshness tooling;
- verification criteria.

## Private cloud API/operations — deferred
- presigned uploads;
- job queue/workers;
- webhooks;
- retention;
- metering;
- abuse control.

## Desktop/mobile — deferred
- context menus;
- share sheet;
- native companion;
- signing/update.

## Developer platform — deferred
- uploader SDK;
- framework integrations;
- private profiles;
- policy/dashboard.

## Enterprise — deferred
- private workers;
- SSO;
- audit;
- data residency;
- SLA.

## File-family expansion — deferred
PDFs/documents/archives/specialists only after planner and economics are proven.

## Kill criteria

Reconsider if:
- users strongly prefer format-first;
- minimum mutation rarely matters;
- profiles cannot be kept accurate;
- browser integration is too brittle;
- cloud economics are bad;
- users do not understand the adapter concept.

## Principle

Each phase deepens **compatibility automation**, not format count.

## Footprint spike — deferred

Before polishing desktop/browser packaging:
- measure core cold start;
- measure initial web bundle;
- prove lazy provider loading;
- implement capability/provider discovery;
- prototype native IPC;
- prove no-op avoids codec startup.

This prevents “lightweight” from becoming a late optimization project.
