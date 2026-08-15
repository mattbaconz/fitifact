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

Only the v0.1 CLI/media slice is current. Every later phase below is deferred
and is not a commitment or claim of implementation.

## v0.1 — CLI/media slice (current)
- media inspection;
- constraint schema;
- bounded transform graph;
- planner;
- FFmpeg provider;
- CLI;
- fixtures.

Exit: no-op/remux/selective transcode chosen correctly.

## Later public MVP — deferred
- web app;
- image inspection and provider;
- local processing where practical;
- CLI;
- custom constraints;
- small sourced profile set;
- explanation UI;
- approved license;
- security process;
- docs.

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
