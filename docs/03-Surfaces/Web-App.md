---
title: "Web App"
type: surface
status: active
implementation: partial
updated: 2026-08-16
canonical: true
tags:
  - web
  - surface
---

# Web app

## Role

Zero-install trial, YouTube destination, consumer discovery and gateway to FOSS/native/cloud.

## Implemented local drop page

`web/index.html` plus `web/app.js` is a static, local-only drop flow for the
D-025 image matrix. It is not a Next.js app, not hosted, and not a cloud
upload form.

```text
Drop -> inspect -> check -> plan -> adapt -> validate -> save
```

The page loads `./pkg/fitifact_wasm.js` from a same-origin wasm-pack build of
`crates/fitifact-wasm`. It never fetches a media provider and never ships
ffmpeg.wasm. Video files return `INSPECTION_UNSUPPORTED` and tell the user to
use the CLI. JPEG/PNG previews use object URLs; untrusted SVG/HTML is not
rendered in origin.

The page states `Uploads to Fitifact: 0 bytes` because the file never leaves
the tab: no network requests, no analytics, no CDN fonts.

Optional local build:

```text
wasm-pack build crates/fitifact-wasm --target web --out-dir ../../web/pkg
```

On `wasm32-unknown-unknown`, `getrandom` may need
`RUSTFLAGS=--cfg getrandom_backend="wasm_js"`. Native `cargo test -p fitifact-wasm`
covers the byte API without a wasm target. Generated `web/pkg` output is gitignored.

## Deferred processing

- media: ffmpeg.wasm is still not in this repository;
- heavy jobs: native companion or explicit cloud, elsewhere.

## Trust

Only say `Uploads to Fitifact: 0 bytes` when payload truly stays local.

## Errors

Handle:
- unsupported inspection;
- ambiguous requirements;
- no plan;
- local WASM module missing;
- execution failure;
- validation failure.

## Landing demos

- PNG that needs JPEG;
- JPEG that already fits;
- a video file that must use the CLI.

## Safety

Use object URLs for raster previews only. Never render hostile HTML/SVG with
active privileges in app origin.

## Lazy-loading strategy

The initial page contains UI plus the image WASM module when built. It does
not eagerly ship a media transcoder.

```text
image selected -> in-process JPEG/PNG path
video selected -> inspect-not-available; use the CLI
heavy job -> native companion or explicit cloud
```
