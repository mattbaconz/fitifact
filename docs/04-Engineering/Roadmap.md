---
title: "Roadmap"
type: roadmap
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - roadmap
---

# Roadmap

Only the unpublished `0.1.0-rc.1` media freeze plus the later D-025 image and
local web commits are current. Every later phase below is deferred and is not
a commitment or claim of implementation.

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

## After the freeze — image and local web (this tree)
- JPEG no-op and PNG→JPEG in-process;
- `fixtures/image`;
- `fitifact-wasm` plus `web/` static drop page;
- no ffmpeg.wasm.

## Later public MVP — deferred
- hosted web app;
- WebP / HEIC / TIFF / animation;
- custom destination profiles;
- explanation UI;
- approved license;
- security process.

## Rejection compiler — deferred
- paste error/requirements;
- deterministic parser;
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
