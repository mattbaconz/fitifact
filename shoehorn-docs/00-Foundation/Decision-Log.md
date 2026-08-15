---
title: "Decision Log"
type: decision-log
status: active
updated: 2026-08-15
canonical: true
tags:
  - decision-log
  - canonical
---

# Decision log

## D-001 — Destination-first product contract
**Status:** accepted  
Canonical primitive: `adapt(file, constraints)`, not `convert(file, output_format)`.

## D-002 — Minimum mutation
**Status:** accepted  
Least-destructive valid path is a first-class objective.

## D-003 — Post-transform validation
**Status:** accepted  
Every output must be re-inspected and checked.

## D-004 — Genuine FOSS core
**Status:** accepted in principle  
Inspection model, constraints, planner, local framework, CLI and community profiles should be open.

## D-005 — Cloud monetizes operation
**Status:** accepted in principle  
Throughput, heavy compute, verified registry, APIs, teams, private workers, support and SLA.

## D-006 — Evidence-backed profiles
**Status:** accepted  
Profile constraints require provenance and freshness.

## D-007 — Natural language cannot directly produce commands
**Status:** accepted  
Any parser emits typed constraints only.

## D-008 — “Shoehorn” is a codename
**Status:** accepted  
Current software collisions require naming/trademark/domain review before launch.

## D-009 — Do not compete on format count
**Status:** accepted  
Use mature transformation engines underneath.

## D-010 — MVP focuses on media/image compatibility
**Status:** superseded in part by D-020.  
Images remain the next family after the media engine slice is proven.

## D-011 — Rust core
**Status:** accepted for v0 by D-018.

## D-012 — Apache-2.0 for Shoehorn-owned core
**Status:** proposed.  
Favored for broad embedding, subject to dependency/legal audit.

## D-013 — Thin integrations
**Status:** accepted.  
Browser, desktop, mobile, CLI, web and SDK integrations contain minimal orchestration/UI and reuse the same compatibility core.

## D-014 — Lazy provider loading
**Status:** accepted.  
Heavy transformation providers load only after a plan proves they are required.

## D-015 — No mandatory idle daemon
**Status:** accepted in principle.  
Shoehorn should not maintain an always-running background process unless a later feature clearly justifies the footprint/privacy cost.

## D-016 — Avoid Electron by default
**Status:** proposed, strongly favored.  
Prefer a thin native or system-webview desktop shell because the UI is simple and the product identity depends on low footprint.

## D-017 — Provider modularity
**Status:** accepted.  
OS-native, FFmpeg, image, PDF and other engines are replaceable capability providers. The planner is provider-independent.

## D-018 — Rust core for v0
**Status:** accepted.  
The v0 engine is a Rust workspace (`shoehorn` library + `shoehorn-cli`). The product is the planner/constraint core; FFmpeg is a subprocess. This avoids a TypeScript rewrite before native/WASM embeddings.

## D-019 — CLI-only first slice
**Status:** accepted.  
The engine slice ships a thin CLI only. Web, extension, mobile, desktop shell, and cloud are deferred. A one-click web app remains a later public-MVP surface, not a prerequisite for proving `adapt(file, constraints)`.

## D-020 — Media-only first slice
**Status:** accepted.  
v0 proves inspection, typed constraints, minimum-mutation planning, selective video transcode, remux, no-op, and post-validation on media via FFmpeg/ffprobe. Images wait until the HEVC-in-MP4 milestone is reliable.

## D-021 — System FFmpeg/ffprobe, not bundled
**Status:** accepted.  
v0 invokes system `ffmpeg` and `ffprobe` on PATH as subprocesses with argv arrays. Missing binaries are `PROVIDER_MISSING`. Bundling and license-redistribution work are deferred.

## D-022 — Lexicographic bounded planner, not Pareto
**Status:** accepted.  
v0 searches a tiny capability catalog (BFS, max depth 2) and ranks by semantic loss, lossy steps, streams changed, then step count. Pareto scoring waits until there are enough real alternatives to justify it.

