---
title: "Web App"
type: surface
status: active
implementation: implemented-static-local
updated: 2026-08-22
canonical: true
tags:
  - web
  - surface
---

# Web app

## Implemented surface

`web/` is a static Vite + React + TypeScript product backed by
`fitifact-wasm`. The default D-028 build (lazy HEIC decoder included) is
deployed from a fully green
`main` commit to [GitHub Pages](https://mattbaconz.github.io/fitifact/). The
deployment contains static files only and does not add a server, upload path,
telemetry endpoint, or cloud fallback.

```text
drop file -> inspect -> paste what the form said -> auto-summarize target
         -> plan minimum changes -> approve crop if needed -> Fix image
         -> re-inspect/validate -> download
```

The consumer headline is **“Make your image pass the upload.”** The drop zone
is visible on first paint. The paste field stays empty so people paste their
own rejection text. The persistent privacy disclosure is **“Your image stays on
this device.”** Successful results are described as **“validated against the
requirements you confirmed”**, never as guaranteed acceptance by the destination
server.

## Execution and trust boundary

The main thread transfers file bytes to a dedicated module worker. The worker
loads the Rust WASM bridge, performs typed parsing/planning/adaptation, and
returns transferable output bytes. Cancellation terminates the worker. Raster
previews use revocable object URLs only after inspection; SVG/HTML input is
never rendered. There are no telemetry calls, payload uploads, CDN fonts,
remote decoders, or implicit cloud fallback.

JPEG, PNG, and still WebP use the in-process Rust provider. Single-image HEIC
uses the lazy decoder, then the same provider. Changed outputs normalize EXIF
orientation and strip other metadata with disclosure. PNG transparency is
preserved only when PNG remains valid; converting transparent pixels to JPEG
is refused. Aspect-changing crop controls require explicit consent. Lossy
quality reduction and upscaling are warned before execution.

The worker enforces the core 32 MiB encoded and 24-megapixel decoded limits.
Animation/multiple-image inputs are refused. Every output is re-inspected and
validated against the confirmed target before download.

## HEIC gate

Public and default builds include the pinned lazy `libheif-js` 1.19.8 decoder
(D-028). It is imported only after HEIC magic; `index.html` must not reference
it eagerly. Notices ship as `THIRD_PARTY_NOTICES.md`. One decoded image is
accepted; zero/multiple images are refused. Decoded RGBA pixels then enter the
same core plan/execute/validate path.

`FITIFACT_HEIC_APPROVED=false` is a decoder-free proof build used in CI. It
must omit the `heic-decoder` / `wasm-bundle` chunks and keep the honest
unsupported heading.

## Build and verify

Node 24.6.0, npm 11.5.1, and wasm-pack 0.15.0 are pinned in the web project and
CI. From `web/`:

```console
npm ci
npm run lint
npm test
npm run build
npm run test:e2e
```

`npm run build` invokes the local pinned wasm-pack package and emits static
assets under `web/dist`. Browser verification covers Chromium, Firefox, and
WebKit at 1280 × 900 and 390 × 844.

CI performs a second default-gate build with `FITIFACT_BASE_PATH=/fitifact/`
and deploys that artifact only after the Rust, web, native-platform, MSRV, and
supply-chain jobs all pass on `main`.

To build without the decoder:

```powershell
$env:FITIFACT_HEIC_APPROVED = "false"
npm run build
Remove-Item Env:FITIFACT_HEIC_APPROVED
```

Third-party source/build/license details are copied into the static artifact as
`THIRD_PARTY_NOTICES.md`.
