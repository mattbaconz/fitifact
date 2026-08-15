---
title: "Shoehorn Agent Instructions"
type: agent-instructions
status: active
updated: 2026-08-15
canonical: true
tags:
  - agents
  - canonical
---

# AGENTS.md

This is the canonical instruction document for coding agents, research agents, documentation agents, and future maintainers.

## Mission

Build Shoehorn as a **destination-first file compatibility engine**, not as a generic converter UI.

The canonical contract is:

```text
adapt(file, constraints) -> compatible artifact + machine-readable report
```

## Source-of-truth hierarchy

When documents disagree, use this precedence:

1. [[00-Foundation/Decision-Log]]
2. [[00-Foundation/Product-Principles]]
3. [[01-Product/Product-Definition]]
4. [[02-Architecture/System-Architecture]]
5. Relevant spec in `07-Specs`
6. Relevant engineering / product document
7. Research notes
8. Examples

Never silently resolve a contradiction. Add it to [[00-Foundation/Open-Questions]] or update the decision log.

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

External tools are **providers**. Shoehorn is the orchestration and compatibility intelligence layer above them.

Current recommendation:

- Rust domain core and native runtime.
- TypeScript SDK / web bindings.
- WASM for browser-safe parts of inspection/planning and selected transforms.
- External native providers for heavy conversion.

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

- prefer official destination documentation;
- record source URL, scope, version, region if relevant, and `last_verified`;
- separate documented facts from empirical observations;
- do not copy requirements from random SEO converter sites unless explicitly marked unverified;
- if a platform's limits are account-, region-, or version-dependent, model that instead of flattening it.

## Naming warning

“Shoehorn” is a working codename. As of 2026-08-15, `shoehorn.dev` is already used by an intelligent developer platform and `@total-typescript/shoehorn` exists as an npm package. Do not purchase assets, publish package names, or make trademark claims without a naming pass. See [[01-Product/Naming-Brand]].

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
- clone VERT/CloudConvert UX and call it Shoehorn.

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
