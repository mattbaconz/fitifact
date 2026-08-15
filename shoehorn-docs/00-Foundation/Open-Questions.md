---
title: "Open Questions"
type: open-questions
status: active
updated: 2026-08-15
canonical: true
tags:
  - open-questions
---

# Open questions

## Brand
- Replace Shoehorn before public launch?
- Which domains/package names are available?
- Trademark clearance?

## MVP
- Images after the media engine slice, not in v0 (D-020). PDF still later.
- Does paste-rejection ship in v0.1? No for this slice.
- How many verified destination profiles at launch? None in v0; local YAML/flags only.

## Architecture
- Rust core vs. TypeScript + native provider process for v0? **Rust** (D-018).
- Scalar vs. Pareto/lexicographic planner? **Lexicographic bounded search** (D-022).
- How should hardware encoders be modeled? Still open; v0 uses libx264 software encode only.

## Browser
- WASM cutoff before native/cloud?
- How much page constraint discovery is reliable?
- How should explicit consent work?

## Registry
- What exactly earns “verified”?
- Separate registry repo?
- Re-verification cadence?

## Cloud
- Direct upload vs. presigned storage?
- Retention default?
- Region strategy?
- Compute billing primitive?

## Legal
- FFmpeg build/license obligations?
- Codec patents by hosted region?
- Ghostscript/office tooling licensing?
- Profile data redistribution?
- Brand clearance?

## Quality
- Cross-family quality loss model?
- Numeric score or descriptive tier?

## AI
- Is a model needed at all?
- Local/hosted?
- Deterministic fallback and evidence policy?

## Business
- Consumer Pro vs. mostly API/enterprise?
- How much free cloud compute?
