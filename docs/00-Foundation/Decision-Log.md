---
title: "Decision Log"
type: decision-log
status: active
updated: 2026-08-21
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
**Status:** superseded in part by D-020 and D-025.
The first public engine slice was media-only (D-020). The first image
executable matrix is D-025. Broader image formats remain deferred.

## D-011 — Rust core
**Status:** accepted for v0 by D-018.

## D-012 — Apache-2.0 for Fitifact-owned core
**Status:** accepted.
Use Apache-2.0 for the public Fitifact repository. Third-party providers and
dependencies remain subject to their own licenses and legal review.

## D-013 — Thin integrations
**Status:** accepted.
The CLI is the reference integration. A local-only static drop page plus
`fitifact-wasm` covers the D-025 image matrix in-browser without a media
runtime. Deferred hosted web, browser extension, desktop, mobile, and SDK
integrations must stay thin and reuse the same compatibility core.

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
The v0 engine is a Rust workspace (`fitifact` library, `fitifact-cli`, and
`fitifact-wasm`). The product is the planner/constraint core; FFmpeg is a
subprocess for media. Images use an in-process provider so WASM can share it.

## D-019 — CLI-only first slice
**Status:** superseded in part by D-026.
v0.1 ships a thin CLI only. Web, extension, mobile, desktop shell, and cloud are
deferred. A one-click web app remains a later public-MVP surface, not a
prerequisite for proving `adapt(file, constraints)`.

## D-020 — Media-only first slice
**Status:** accepted.
v0.1 proves inspection, typed constraints, minimum-mutation planning, selective
video transcode, remux, no-op, and post-validation on media via FFmpeg/ffprobe.
Its executable matrix is MP4/H.264/AAC no-op, MOV/H.264/AAC remux to MP4, and
MP4/HEVC-to-H.264 video transcode with compatible AAC copied. It refuses every
other mutation, including WebM, Matroska, unknown containers, and MOV/HEVC.
File-size and video-dimension constraints are check-only in v0.1. Images wait
until this media milestone is reliable.

## D-021 — System FFmpeg/ffprobe, not bundled
**Status:** accepted.
v0 invokes system `ffmpeg` and `ffprobe` on PATH as subprocesses with argv arrays. Missing binaries are `PROVIDER_MISSING`. Bundling and license-redistribution work are deferred.

## D-022 — Lexicographic bounded planner, not Pareto
**Status:** accepted.
v0 searches a tiny capability catalog (BFS, max depth 2) and ranks by semantic loss, lossy steps, streams changed, then step count. Pareto scoring waits until there are enough real alternatives to justify it.

## D-023 — Fitifact public-name candidate and publication gate
**Status:** accepted; owner directed public GitHub 2026-08-18.
**Recorded:** 2026-08-15; owner sign-off 2026-08-18.

Replace the historical Shoehorn codename with **Fitifact** in the repository.
Automated exact-name checks across GitHub, crates.io (including hyphen and
underscore variants), npm, executable/command names, and ICANN/RDAP found no
material collision signal. This result is not legal clearance.

A 2026-08-16 search packet is recorded in [[01-Product/Naming-Brand]]; it is
not billed attorney clearance. WIPO Global Brand Database interactive search
was not completed. Owner directed public GitHub create, `main` push, and
annotated `v0.1.0-rc.1` on 2026-08-18. GitHub Release publication remains
gated by `FITIFACT_PUBLICATION_APPROVED` and Environment `public-release`.
Do not publish crates.io/npm packages or make trademark claims from this
packet.

## D-024 — Public core and private operations boundary
**Status:** accepted.
The Apache-2.0 public repository contains the core, schemas, local provider
framework, CLI, tests, fixtures, and documentation. Managed cloud execution,
infrastructure, credentials, metering, private profiles, continuous verification
operations, and enterprise control-plane code are deferred and belong in a
separate private checkout. Local Fitifact has no telemetry, network activity, or
implicit cloud fallback.

## D-025 — First image executable matrix
**Status:** accepted as the first image slice; executable image scope extended by D-026.
**Recorded:** 2026-08-16.

After the media `0.1.0-rc.1` freeze, the first image slice is JPEG already
matching JPEG → no-op, and PNG targeting JPEG → in-process encode. It refuses
WebP, HEIC/HEIF, TIFF, animation, claiming alpha-preserving JPEG, and
resize/byte-fitting. The image provider is in-process Rust so WASM can share it;
it must not construct or spawn FFmpeg. Media remains system FFmpeg (D-021). Do
not reopen the D-020 media matrix.

## D-026 — Local consumer image upload MVP
**Status:** accepted.
**Recorded:** 2026-08-21.

The next public candidate adds a static Vite/React product backed by
`fitifact-wasm` and a dedicated module worker. Its consumer promise is **“Make
your image pass the upload.”** The persistent privacy disclosure is **“Your
image stays on this device.”** A successful result is **“validated against the
requirements you confirmed”**; it is never described as guaranteed acceptance
by a destination server.

The browser accepts typed or parsed JPEG/PNG, byte, and integer image-dimension
requirements. It can no-op, preserve source format, crop only with explicit
consent, resize with quality/upscale warnings, fit JPEG bytes within bounded
attempts, preserve PNG losslessly where possible, strip metadata with
disclosure, refuse implicit transparency flattening, and re-inspect/validate
every output. Inputs are limited to 32 MiB encoded and 24 megapixels decoded.
Animation and multi-image inputs are refused.

JPEG and PNG use the in-process Rust engine. HEIC is a replaceable lazy decoder
path, disabled by default and compiled only when `FITIFACT_HEIC_APPROVED=true`.
Approval must cover the pinned decoder build and its LGPL-3.0 notices; HEIC is
decoded to owned pixels and then enters the same Rust plan/execute/validate
path. There is no upload, telemetry, CDN decoder, cloud fallback, hosted
service, destination profile, or server-acceptance guarantee.

The media matrix and provider rules in D-020/D-021 remain frozen. Broader file
families, hosted operation, automatic destination discovery, and the long-term
“Any file. Any destination.” vision remain deferred.
