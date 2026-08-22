---
title: "Fitifact agent instructions"
type: agent-instructions
status: active
updated: 2026-08-18
canonical: true
tags:
  - agents
  - canonical
---

# AGENTS.md

This repository-root file is the canonical instruction document for coding
agents, research agents, documentation agents, and future maintainers. The file
at `docs/AGENTS.md` is only a pointer to this one.

## Mission

Build Fitifact as a **destination-first file compatibility engine**, not as a generic converter UI.

The canonical contract is:

```text
adapt(file, constraints) -> compatible artifact + machine-readable report
```

## Source-of-truth hierarchy

When documents disagree, use this precedence:

1. [`docs/00-Foundation/Decision-Log.md`](docs/00-Foundation/Decision-Log.md)
2. [`docs/00-Foundation/Product-Principles.md`](docs/00-Foundation/Product-Principles.md)
3. [`docs/01-Product/Product-Definition.md`](docs/01-Product/Product-Definition.md)
4. [`docs/02-Architecture/System-Architecture.md`](docs/02-Architecture/System-Architecture.md)
5. Relevant spec in `docs/07-Specs`
6. Relevant engineering / product document
7. Research notes
8. Examples

Never silently resolve a contradiction. Add it to
[`docs/00-Foundation/Open-Questions.md`](docs/00-Foundation/Open-Questions.md) or
update the decision log.

## v0.1 release boundary (D-018 through D-022)

The current public candidate is `0.1.0-rc.4`: a Rust workspace containing the
`fitifact` library, `fitifact-cli` binary, and `fitifact-wasm` bindings plus the
D-026 static consumer image product. The `0.1.0-rc.1` freeze SHA remains the
media-only D-020 boundary. Media still uses system `ffmpeg`/`ffprobe` from
`PATH`; they are never bundled. Images use an in-process Rust provider and must
not construct `FfmpegProvider`.

The executable adaptation catalog is deliberately small:

- MP4/H.264/AAC that already satisfies the constraints is a no-op;
- MOV/H.264/AAC can be remuxed to MP4 without re-encoding;
- MP4/HEVC/AAC can be transcoded to MP4/H.264/AAC while AAC is copied;
- JPEG/PNG that already satisfies the confirmed image target is a no-op;
- JPEG/PNG can be adapted in-process under D-026 format, byte, dimension, and
  explicit crop constraints;
- every mutation outside the frozen media matrix and D-026 image contract is
  refused explicitly.

File-size and video-dimension constraints can be inspected and checked, but the
media provider does not execute size fitting, resizing, or frame-rate changes.
The image provider executes only the bounded D-026 fitting pipeline. A provider
returning success is not proof of compatibility: every produced output must be
re-inspected and validated against the same hard constraints.

The bounded planner uses breadth-first search to depth 2 and lexicographic
ranking by semantic loss, lossy steps, streams changed, then step count. Do not
introduce Pareto scoring until a later decision supersedes D-022.

Images beyond the D-026 JPEG/PNG contract, browser extensions, desktop/mobile
shells, profiles, managed APIs/cloud execution, bundled FFmpeg, ffmpeg.wasm,
registry publishing, OS signing, package-manager formulae, and
telemetry/network activity are deferred. The public web app is static hosting
only; it adds no server or upload path. Design documents for deferred areas are
not evidence of implementation.

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
upload or make cloud execution a fallback.

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
FFmpeg/ffprobe as the media provider. TypeScript bindings, WASM, web surfaces,
and other providers are deferred design directions.

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
10. Update docs and decision log if architecture changed.

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
publication stays gated by `FITIFACT_PUBLICATION_APPROVED`. See
[`docs/01-Product/Naming-Brand.md`](docs/01-Product/Naming-Brand.md).

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
- Do not eagerly load codecs/providers.
- No-op path must not initialize encoders.
- Browser initial bundle must not include heavyweight media WASM by default.
- Prefer streaming/file handles over reading entire large artifacts into RAM.
- Avoid full-file hashing unless cryptographic identity/idempotency actually requires it.
- Do not introduce a persistent background daemon without an explicit decision.
- Prefer OS-native providers when they satisfy exact target semantics and validation.
- Heavy providers must remain replaceable.
- Avoid Electron by default; choose a thin native/system-webview shell unless measurements justify it.
