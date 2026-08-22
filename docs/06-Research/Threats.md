---
title: "Threats"
type: research
status: active
updated: 2026-08-21
canonical: true
tags:
  - threats
  - research
---

# Threats

## T1 — Converter collapse
**Severity: existential.**  
If UX is format-first, Fitifact is redundant.

## T2 — Incumbents add a constraint layer
Cloud conversion/upload companies already have infrastructure.

Mitigate with:
- FOSS standard;
- registry;
- integrations;
- planner quality;
- brand.

## T3 — Profile staleness
Old constraints create false confidence.

Mitigate provenance/freshness/testing.

## T4 — Hidden server rules
Some targets have undocumented validation.

Mitigate honest confidence and rejection refinement.

## T5 — Scope explosion
Every file family has unique semantics.

Mitigate family gates and provider abstraction.

## T6 — Security
Hostile files and complex parsers. See [[04-Engineering/Security-Privacy]].

## T7 — Licensing/patents
Codec/provider redistribution and hosted legal exposure.

Mitigate dependency/license ledger and modular providers.

## T8 — Cloud unit economics
Video can be expensive.

Mitigate local-first, cost previews, resource-aware pricing.

## T9 — Browser performance
WASM may be slow/large.

Mitigate native companion/cloud option.

## T10 — Mobile restrictions
Sandbox/background limits block “invisible everywhere.”

Mitigate honest share-extension integration.

## T11 — Naming collision
Fitifact already exists in software.

Mitigate rename/clearance.

## T12 — Minimum mutation not valued
Users may only care about output success.

Mitigate hiding complexity; turn it into speed/quality benefit.

## T13 — Wrong NLP constraints
Mitigate deterministic first, evidence spans and typed validation.

## T14 — Support burden
Obscure files can consume maintainers.

Mitigate scope/support tiers/community providers.

## T15 — Quality disputes
Mitigate previews, preferences, warnings, candidate plans.

## T16 — Destination terms restrict automated testing
Respect terms, use official docs/manual verification.

## T17 — SEO commodity trap
Avoid format-converter commodity marketing.

## T18 — Proprietary registry harms FOSS trust
Keep schema/community data open; monetize verified operations/SLA.

## T19 — Adapter metaphor implies losslessness
Messaging: smallest necessary change, not zero loss.

## T20 — AI feature creep
A model is a parser fallback, not the product.

## T21 — False destination confidence
Passing confirmed typed requirements does not prove undocumented server rules.
Use the exact boundary **“validated against the requirements you confirmed”**,
never guaranteed acceptance, and measure real destination acceptance in the
ten-task moderated gate.

## T22 — Browser image harm
Unapproved cropping, silent metadata behavior, transparency flattening, or
hidden quality/upscale loss can make a technically valid output harmful.
Mitigate with explicit crop consent, metadata disclosure, transparency refusal,
warnings, original preservation, requirement-by-requirement validation, and a
zero-harm continuation threshold.

## T23 — Local-processing claim drift
Third-party scripts, telemetry, CDN decoders, or a cloud fallback would make
“Your image stays on this device” false. Keep static assets same-origin, decoder
imports local and gated, CSP restrictive, payload network activity absent, and
browser flows verified.

## T24 — HEIC redistribution exposure
Decoder licensing, codec patents, multi-image semantics, and large decoded
allocations create separate risk. Pin/review notices, load only after HEIC
magic, refuse zero/multiple images, enforce the core pixel limit, ship notices
with the public build, and keep `FITIFACT_HEIC_APPROVED=false` as a
decoder-free proof.
