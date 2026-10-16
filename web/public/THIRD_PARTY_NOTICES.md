# Third-party notices

## Interface fonts

Geist Sans and Geist Mono are self-hosted with the application and documentation.
Copyright 2024 The Geist Project Authors. Licensed under the SIL Open Font
License 1.1; the full license is included in [brand/GEIST-LICENSE.txt](brand/GEIST-LICENSE.txt).
Font source: https://github.com/vercel/geist-font.

## Optional HEIC decoder

The HEIC path dynamically imports `libheif-js` 1.19.8, an Emscripten browser build of
libheif, only after local HEIF magic detection. Public and default web builds include
this lazy decoder unless `FITIFACT_HEIC_APPROVED=false`.

- JavaScript package source: https://github.com/catdad-experiments/libheif-js/tree/1.19.8
- Upstream libheif source: https://github.com/strukturag/libheif
- Build form: `libheif-js/wasm-bundle`, distributed by the package as an embedded WebAssembly build
- Package license: LGPL-3.0 (the installed package includes its license text)
- Upstream licenses and bundled codec license details: see the source distributions above and
  `node_modules/libheif-js` for the exact installed artifact.

Fitifact does not load this decoder unless HEIC magic is present, and never fetches it from a CDN.
A decoder-free build can set `FITIFACT_HEIC_APPROVED=false` to omit the chunk.

## Desktop FFmpeg sidecar

Desktop builds may include `ffmpeg` and `ffprobe` as sidecar processes. Fitifact
does not link those binaries. See `apps/desktop/FFMPEG-NOTICE.md` and
https://ffmpeg.org for license terms. The public web app does not ship FFmpeg.
