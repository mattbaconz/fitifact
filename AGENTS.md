---
title: "Fitifact agent instructions"
type: agent-instructions
status: active
updated: 2026-08-26
canonical: true
tags:
  - agents
  - canonical
---

# AGENTS.md

This repository-root file is the canonical instruction document for coding
agents, research agents, documentation agents, and future maintainers.

## Mission

Build Fitifact as a **destination-first file compatibility engine**, not as a generic converter UI.

The canonical contract is:

```text
adapt(file, constraints) -> compatible artifact + machine-readable report
```

## Source-of-truth hierarchy

When instructions disagree, this file wins, then `README.md` / `SECURITY.md` /
`CONTRIBUTING.md`, then code and tests. Never silently resolve a contradiction:
open an issue or record it here.

## v0.1 release boundary (D-018 through D-022)

The current public candidate is `1.0.0-rc.1`: a Rust workspace containing the
`fitifact` library, `fitifact-cli` binary, and `fitifact-wasm` bindings plus the
D-026/D-028 static consumer image product. The `0.1.0-rc.1` freeze SHA remains the
media-only D-020 boundary. The CLI uses `ffmpeg`/`ffprobe` from `PATH`. The
desktop app ships those binaries as sidecars (resolve env, then next to the
executable, then PATH). Images use an in-process Rust provider and must
not construct `FfmpegProvider`. ffmpeg.wasm is still forbidden.

The executable adaptation catalog is deliberately small:

- MP4/H.264/AAC that already satisfies the constraints is a no-op;
- MOV/MKV/WebM H.264+AAC can be remuxed to MP4 without re-encoding when bytes already pass;
- `matroska,webm` probes are WebM only when codecs are WebM-legal; H.264 in that probe is MKV;
- HEVC/VP8/VP9/AV1 can be transcoded to MP4/H.264 while AAC is copied when present;
- H.264 with a non-AAC soundtrack can encode audio to AAC and copy video;
- audio-only files (MP3/WAV/FLAC/OGG/M4A) on size-only targets are a no-op when under the ceiling; otherwise AAC in MP4 with an audio bitrate ladder;
- audio-only files aimed at a video target are refused;
- media `file.bytes` and dimension limits can be executed with scale-to-fit plus an H.264 bitrate ladder and quality floor;
- JPEG/PNG/still WebP that already satisfies the confirmed image target is a no-op;
- JPEG/PNG/still WebP/TIFF/BMP/GIF/single-image HEIC can be adapted in-process
  under D-026/D-029 format, byte, dimension, crop, and first-frame constraints
  to JPEG or PNG, plus WebP when the confirmed target names it;
- in the web app (D-037), video gets the no-op check, the MOV/MKV/WebM
  H.264+AAC remux to MP4, and the H.264 video transcode (scale-to-fit, the same
  bitrate ladder and quality floor, AAC copied or encoded) through the
  browser's WebCodecs encoders: Mediabunny, loaded in the worker after the page
  loads (D-045),
  probes, measures the encoders the engine asks about, executes, and re-probes;
  the Rust engine plans, judges the measurement, and validates. HDR, a stored
  display transform (rotation, mirroring, non-square pixels), an audio-only
  encode, and a missing or unsound encoder are refused with a pointer to the
  desktop app;
- every mutation outside that catalog is refused explicitly.

A provider returning success is not proof of
compatibility: every produced output must be re-inspected and validated against
the same hard constraints.

The bounded planner uses breadth-first search to depth 2 and lexicographic
ranking by semantic loss, lossy steps, streams changed, then step count. Do not
introduce Pareto scoring until a later decision supersedes D-022. Destination
policy is a YAML profile compiled to `ConstraintSet`; `plan.rs` must not match
on destination names.

Images beyond this still-image matrix, browser extensions, mobile
shells, managed APIs/cloud execution, ffmpeg.wasm, registry publishing, OS
signing, package-manager formulae, and telemetry/network activity are
deferred. The public web app is static hosting only; it adds no server or
upload path. A thin Tauri 2
host in `apps/desktop/` (D-031) reuses that UI plus the engine: still images
in-process/WASM, MP4/MOV/WebM/MKV and common audio via shipped FFmpeg sidecars.
The desktop crate is **not** a workspace member. Discord Nitro is declared in
Setup, never detected (D-032). Design documents for deferred areas are not
evidence of implementation.

Distribution for v0.1 is GitHub-only from
`https://github.com/mattbaconz/fitifact`. All Cargo packages remain
`publish = false`.

## Repository boundary

The public Apache-2.0 repository contains the core schemas, planner, local
execution framework, CLI, tests, fixtures, and documentation. Managed cloud,
operations, private profiles, credentials, infrastructure, metering, and
enterprise control-plane code belong in a separate private checkout if they are
ever approved. Do not create that checkout from this repository.

Local Fitifact performs no telemetry or network activity. Never add an implicit
upload or make cloud execution a fallback. Building the desktop package (not
the running app) may fetch FFmpeg once into `apps/desktop/src-tauri/binaries/`.

## Non-negotiable product invariants

- Do not expose format selection as the primary UX.
- Do not modify a file if it already satisfies the target.
- Do not re-encode a stream if a lossless operation is sufficient.
- Do not claim compatibility without validation.
- Do not infer a destination requirement without recording provenance and confidence.
- Do not allow a language model to directly emit executable transform commands.
- Never pass user-controlled strings into shell command construction.
- Every transform plan must be inspectable before execution.
- Every output must be validated against the same constraints used to plan it.
- The planner must distinguish **hard constraints** from **preferences**.
- Failure must be explicit. A wrong “success” is worse than a useful refusal.

## Architecture guidance

Prefer a small pure core:

```text
inspect -> constraints -> plan -> execute -> validate
```

The planner should depend on abstract transform capabilities, not directly on FFmpeg/ImageMagick/etc.

External tools are **providers**. Fitifact is the orchestration and compatibility intelligence layer above them.

Current implementation: Rust domain core and native runtime with system
FFmpeg/ffprobe as the media provider, `fitifact-wasm` plus a static drop-first
web surface for the still-image matrix and browser video stream copy and
WebCodecs H.264 encode (Mediabunny in a worker chunk warmed after page load), and a thin Tauri 2 host in
`apps/desktop/` that reuses that UI. The desktop package stays out of the
workspace lockfile.

Picture format, bit depth, and HDR are profile constraints like any other; only
a video encode satisfies them, and HDR is tone-mapped to SDR only when the
FFmpeg build has `zscale` and `tonemap` (otherwise the plan is refused). Each
profile rule names a typed source (`official-doc`, `official-api`,
`fitifact-rule`, `user-config`, ...), trust is derived from those types, and
`web/public/registry.json` is generated from `profiles/` and checked by a
golden test (`FITIFACT_BLESS=1 cargo test -p fitifact --lib registry`
regenerates it).

The public documentation site is a separate Next.js/Fumadocs static export in
`apps/docs`. The Vite application remains at the root URL. `web`'s build merges
the docs export at `/docs/` (including the configured Pages base path). Shared
brand tokens and `ft-` components (see `DESIGN.md`), self-hosted fonts, and the
local `fitifact.theme` preference keep both surfaces coherent. Documentation
search is browser-local.

Native adaptation requests carry operation identifiers. Cancellation uses an
optional core token and cancellable process spawner; publication and cancellation
share an atomic boundary so completed output is never falsely called cancelled.

This is a recommendation, not an excuse to prematurely create 25 crates.

## Agent-friendly implementation cadence

For each feature:

1. Identify the user-visible compatibility problem.
2. Add or refine the relevant constraint schema.
3. Add fixtures representing valid and invalid inputs.
4. Add inspection support if needed.
5. Add transform capability metadata.
6. Add planner tests before wiring execution.
7. Add execution in a sandbox.
8. Validate output.
9. Add an explanation snapshot.
10. Update `AGENTS.md` if architecture changed.

## Definition of done

A compatibility feature is not done because “FFmpeg produced a file.”

It is done when:

- invalid input is correctly diagnosed;
- the planner chooses a defensible minimal plan;
- execution is safe;
- output satisfies all hard constraints;
- preservation preferences are honored where possible;
- failure modes are tested;
- user explanation is understandable;
- a reproducible fixture exists.

## Research rules

When adding compatibility profiles:

Profiles are deferred in v0.1. These rules apply only after a decision adds
them to scope.

- prefer official destination documentation;
- record source URL, scope, version, region if relevant, and `last_verified`;
- separate documented facts from empirical observations;
- do not copy requirements from random SEO converter sites unless explicitly marked unverified;
- if a platform's limits are account-, region-, or version-dependent, model that instead of flattening it.

## Naming and publication gate

Fitifact is the selected public name, but it is not legally cleared. The
2026-08-15 automated exact-name search found no material collision signal across
GitHub, crates.io hyphen/underscore variants, npm, command names, and ICANN/RDAP.
Owner directed public GitHub create/push/tag on 2026-08-18 against the
2026-08-16 packet; that is not billed attorney clearance. GitHub Release
publication stays gated by `FITIFACT_PUBLICATION_APPROVED`.

## Forbidden shortcuts

Do not:

- hardcode “MP4 = compatible” without inspecting codecs;
- equate file extension with actual format;
- use arbitrary shell strings;
- assume a conversion is lossless because the extension did not change;
- discard metadata silently;
- overwrite originals by default;
- pretend browser processing is always faster;
- claim “world's first” based on search absence;
- clone VERT/CloudConvert UX and call it Fitifact;
- overwrite original files or existing outputs;
- silently discard streams;
- treat a provider exit code as compatibility proof;
- add telemetry, network calls, or implicit cloud execution.

## Tone of user-facing copy

Preferred:

> Your video is MP4, but it contains HEVC video. This target needs H.264. I can change only the video stream.

Avoid:

> The codec/container matrix failed compatibility validation according to target capabilities.

Technical details can be expandable.

## Lightweight architecture invariants

- Core has no GUI dependency.
- Core has no mandatory network dependency.
- Integrations contain no destination-specific compatibility logic.
- Do not eagerly load codecs/providers. One exception (D-045): after the page's
  `load` event, the web app warms its worker, engine WASM, and video tools so a
  loaded page keeps working offline; it skips this under Save-Data and on desktop,
  and it never constructs an encoder.
- No-op path must not initialize encoders.
- Browser initial bundle must not include heavyweight media WASM by default.
- Prefer streaming/file handles over reading entire large artifacts into RAM.
- Avoid full-file hashing unless cryptographic identity/idempotency actually requires it.
- Do not introduce a persistent background daemon without an explicit decision.
- Prefer OS-native providers when they satisfy exact target semantics and validation.
- Heavy providers must remain replaceable.
- Avoid Electron by default; choose a thin native/system-webview shell unless measurements justify it.
