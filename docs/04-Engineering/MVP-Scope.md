---
title: "MVP Scope"
type: engineering
status: active
implementation: implemented
updated: 2026-08-21
canonical: true
tags:
  - mvp
  - engineering
  - canonical
---

# MVP scope

The `0.1.0-rc.4` public candidate combines the frozen native media slice with
the D-026 local consumer image upload MVP. It produces static
web assets only; no deployment or publication is implied.

## Native CLI/media matrix

1. MP4/H.264/AAC already satisfying the target: no-op.
2. MOV/H.264/AAC targeting MP4/H.264/AAC: remux.
3. MP4/HEVC/AAC targeting MP4/H.264/AAC: transcode video, copy AAC.
4. Every other media mutation: explicit refusal.

Media dimension and byte constraints remain check-only. System FFmpeg/ffprobe
remain external providers under D-020/D-021.

## Consumer image matrix

The product promise is **“Make your image pass the upload.”** Users paste
requirements, review normalized format/size/dimensions, choose an image, review
the minimum plan and any crop, adapt locally, review validation, and download.

- Parse JPEG/JPG/PNG, decimal/binary byte ceilings, exact dimensions, and
  minimum/maximum width/height language into deterministic typed constraints.
- Inspect JPEG/PNG, no-op compatible inputs, preserve JPEG/PNG source format
  where valid, encode PNG/JPEG, crop only with explicit consent, resize with an
  upscale warning, fit JPEG quality from 95 down to 50 in at most seven
  encodes, and perform at most three proportional dimension-reduction rounds.
- Normalize EXIF orientation, disclose that changed image outputs strip
  metadata, preserve alpha only through PNG, and refuse implicit transparency
  flattening.
- Re-inspect and validate every output against the same hard constraints.
  Successful copy must say the result was **“validated against the requirements
  you confirmed”** and must not guarantee destination-server acceptance.
- Enforce 32 MiB encoded input and 24-megapixel decoded limits. Refuse
  animation/multi-image content and surface cancellation/resource errors.
- Keep **“Your image stays on this device.”** visible. The static product has no
  telemetry, payload upload, CDN decoder, account, or cloud fallback.

HEIC detection is present, but decoding is disabled by default. An approved
build may set `FITIFACT_HEIC_APPROVED=true` to include pinned `libheif-js`
1.19.8 as an isolated lazy decoder. Approval includes LGPL-3.0 notices and
build review. Decoded pixels enter the same Rust validation path; multiple
images are refused. HEIC is not a default-format-support claim.

## Public/private boundary

The public Apache-2.0 repository owns the engine, schemas, planner, local
providers, CLI, WASM bridge, static UI, tests, legal synthetic fixtures, and
documentation. Managed APIs/cloud execution, credentials, metering, private
profiles, registry operations, and enterprise control-plane code remain
deferred outside this repository.

## Human continuation gate

Engineering verification is necessary but not the viability result. The
post-build ten-task moderated protocol in
[[04-Engineering/Consumer-Image-Moderated-Test]] must be executed with real
people and live form/application photo requirements. Do not fabricate results.

## Deferred

Hosted processing, destination profiles, automatic requirement capture,
WebP/TIFF/animation adaptation, media in the browser, accounts, billing,
browser extensions, PWA/share targets, desktop/mobile shells, and the broad
“Any file. Any destination.” vision are outside this MVP.
