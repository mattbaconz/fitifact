---
title: "Roadmap"
type: roadmap
status: active
updated: 2026-08-15
canonical: true
tags:
  - roadmap
---

# Roadmap

## Phase 0 — architecture spike
- media/image inspection;
- constraint schema;
- fake transform graph;
- planner;
- FFmpeg provider;
- image provider;
- CLI;
- fixtures.

Exit: no-op/remux/selective transcode chosen correctly.

## Phase 1 — public FOSS MVP
- web app;
- local processing where practical;
- CLI;
- custom constraints;
- small sourced profile set;
- explanation UI;
- approved license;
- security process;
- docs.

## Phase 2 — rejection compiler
- paste error/requirements;
- deterministic parser;
- evidence spans;
- conflict UI;
- optional model-assisted leftovers.

## Phase 3 — browser extension
- page hints with consent;
- selected file;
- local/native delegation;
- save/retry workflow.

## Phase 4 — registry
- profile repo;
- provenance;
- fixtures;
- freshness tooling;
- verification criteria.

## Phase 5 — cloud API
- presigned uploads;
- job queue/workers;
- webhooks;
- retention;
- metering;
- abuse control.

## Phase 6 — desktop/mobile
- context menus;
- share sheet;
- native companion;
- signing/update.

## Phase 7 — developer platform
- uploader SDK;
- framework integrations;
- private profiles;
- policy/dashboard.

## Phase 8 — enterprise
- private workers;
- SSO;
- audit;
- data residency;
- SLA.

## Phase 9 — file-family expansion
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

## Phase 0.5 — footprint spike

Before polishing desktop/browser packaging:
- measure core cold start;
- measure initial web bundle;
- prove lazy provider loading;
- implement capability/provider discovery;
- prototype native IPC;
- prove no-op avoids codec startup.

This prevents “lightweight” from becoming a late optimization project.
