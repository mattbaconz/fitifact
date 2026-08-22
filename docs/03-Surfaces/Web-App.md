---
title: "Web App"
type: surface
status: active
implementation: implemented-static-local
updated: 2026-08-21
canonical: true
tags:
  - web
  - surface
---

# Web app

## Implemented surface

`web/` is a static Vite + React + TypeScript product backed by
`fitifact-wasm`. The default-off-HEIC build is deployed from a fully green
`main` commit to [GitHub Pages](https://mattbaconz.github.io/fitifact/). The
deployment contains static files only and does not add a server, upload path,
telemetry endpoint, or cloud fallback.

```text
parse requirements -> review target -> choose image -> inspect/check/plan
                   -> approve crop if needed -> execute -> re-inspect/validate
                   -> download
```

The consumer headline is **“Make your image pass the upload.”** The persistent
privacy disclosure is **“Your image stays on this device.”** Successful results
are described as **“validated against the requirements you confirmed”**, never
as guaranteed acceptance by the destination server.

## Execution and trust boundary

The main thread transfers file bytes to a dedicated module worker. The worker
loads the Rust WASM bridge, performs typed parsing/planning/adaptation, and
returns transferable output bytes. Cancellation terminates the worker. Raster
previews use revocable object URLs only after inspection; SVG/HTML input is
never rendered. There are no telemetry calls, payload uploads, CDN fonts,
remote decoders, or implicit cloud fallback.

JPEG and PNG use the in-process Rust provider. Changed outputs normalize EXIF
orientation and strip other metadata with disclosure. PNG transparency is
preserved only when PNG remains valid; converting transparent pixels to JPEG
is refused. Aspect-changing crop controls require explicit consent. Lossy
quality reduction and upscaling are warned before execution.

The worker enforces the core 32 MiB encoded and 24-megapixel decoded limits.
Animation/multiple-image inputs are refused. Every output is re-inspected and
validated against the confirmed target before download.

## HEIC gate

HEIC magic detection does not load a decoder. Default builds produce an honest
unsupported state and emit no decoder chunk. Only
`FITIFACT_HEIC_APPROVED=true` includes the isolated, lazy `libheif-js` 1.19.8
decoder. The approval decision must cover its LGPL-3.0 notice and embedded WASM
build. One decoded image is accepted; zero/multiple images are refused. Decoded
RGBA pixels then enter the same core plan/execute/validate path.

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

To audit the approved decoder without publishing it:

```powershell
$env:FITIFACT_HEIC_APPROVED = "true"
npm run test:heic
npm run build
Remove-Item Env:FITIFACT_HEIC_APPROVED
```

Third-party source/build/license details are copied into the static artifact as
`THIRD_PARTY_NOTICES.md`.
