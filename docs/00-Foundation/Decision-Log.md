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
Inspection model, constraints, planner, local framework, and CLI are open.
Community profiles are deferred but should also be open when implemented.

## D-005 — Cloud monetizes operation
**Status:** accepted future direction; implementation deferred/private
A future private operations product may sell throughput, heavy compute, verified
registry operations, APIs, teams, private workers, support, and SLA. None is
implemented in v0.1 or this public repository.

## D-006 — Evidence-backed profiles
**Status:** accepted for deferred profile work
Profile constraints will require provenance and freshness when profiles enter
scope. v0.1 has no destination-profile registry or lookup.

## D-007 — Natural language cannot directly produce commands
**Status:** accepted
Any parser emits typed constraints only.

## D-008 — Retire the “Shoehorn” codename
**Status:** superseded by D-023.
The historical Shoehorn codename had material software/searchability collisions
and required replacement before launch.

## D-009 — Do not compete on format count
**Status:** accepted
Use mature transformation engines underneath.

## D-010 — MVP focuses on media/image compatibility
**Status:** superseded in part by D-020.
Images remain deferred to the later public MVP after the media engine slice is
proven.

## D-011 — Rust core
**Status:** accepted for v0 by D-018.

## D-012 — Apache-2.0 for Fitifact-owned core
**Status:** accepted.
Use Apache-2.0 for the public Fitifact repository. Third-party providers and
dependencies remain subject to their own licenses and legal review.

## D-013 — Thin integrations
**Status:** accepted.
The CLI is the only v0.1 integration. Deferred browser, desktop, mobile, web, and
SDK integrations must contain minimal orchestration/UI and reuse the same
compatibility core.

## D-014 — Lazy provider loading
**Status:** accepted.
Heavy transformation providers load only after a plan proves they are required.

## D-015 — No mandatory idle daemon
**Status:** accepted in principle.
Fitifact should not maintain an always-running background process unless a later feature clearly justifies the footprint/privacy cost.

## D-016 — Avoid Electron by default
**Status:** proposed, strongly favored.
Desktop work is deferred. If approved later, prefer a thin native or
system-webview shell because the UI is simple and the product identity depends
on low footprint.

## D-017 — Provider modularity
**Status:** accepted.
FFmpeg is the v0.1 provider. Deferred OS-native, image, PDF, and other engines
must remain replaceable capability providers; the planner is provider-independent.

## D-018 — Rust core for v0
**Status:** accepted.
The v0 engine is a Rust workspace (`fitifact` library + `fitifact-cli`). The product is the planner/constraint core; FFmpeg is a subprocess. This avoids a TypeScript rewrite before native/WASM embeddings.

## D-019 — CLI-only first slice
**Status:** accepted.
v0.1 ships a thin CLI only. Web, extension, mobile, desktop shell, and cloud are
deferred. A one-click web app remains a later public-MVP surface, not a
prerequisite for proving `adapt(file, constraints)`.

## D-020 — Media-only first slice
**Status:** accepted.
v0.1 proves inspection, typed constraints, minimum-mutation planning, selective
video transcode, remux, no-op, and post-validation on media via FFmpeg/ffprobe.
Its executable matrix is MP4/H.264/AAC no-op, acceptable streams in the wrong
container remux, and HEVC video to H.264 with compatible AAC audio copied. It
refuses every other mutation. File-size and video-dimension constraints are
check-only in v0.1. Images wait until this media milestone is reliable.

## D-021 — System FFmpeg/ffprobe, not bundled
**Status:** accepted.
v0 invokes system `ffmpeg` and `ffprobe` on PATH as subprocesses with argv arrays. Missing binaries are `PROVIDER_MISSING`. Bundling and license-redistribution work are deferred.

## D-022 — Lexicographic bounded planner, not Pareto
**Status:** accepted.
v0 searches a tiny capability catalog (BFS, max depth 2) and ranks by semantic loss, lossy steps, streams changed, then step count. Pareto scoring waits until there are enough real alternatives to justify it.

## D-023 — Fitifact public-name candidate and publication gate
**Status:** accepted pending legal sign-off.
**Recorded:** 2026-08-15.

Replace the historical Shoehorn codename with **Fitifact** in the repository.
Automated exact-name checks across GitHub, crates.io (including hyphen and
underscore variants), npm, executable/command names, and ICANN/RDAP found no
material collision signal. This result is not legal clearance.

Final human/legal review of USPTO, WIPO, and EUIPO records is still pending.
Public publication is blocked until explicit owner/legal sign-off. No public
repository, release, package publication, or naming claim may precede that
sign-off.

## D-024 — Public core and private operations boundary
**Status:** accepted.
The Apache-2.0 public repository contains the core, schemas, local provider
framework, CLI, tests, fixtures, and documentation. Managed cloud execution,
infrastructure, credentials, metering, private profiles, continuous verification
operations, and enterprise control-plane code are deferred and belong in a
separate private checkout. Local Fitifact has no telemetry, network activity, or
implicit cloud fallback.
