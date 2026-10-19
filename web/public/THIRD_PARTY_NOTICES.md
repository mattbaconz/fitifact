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

## Browser video tools

The web app's video path dynamically imports Mediabunny 1.61.1 in its worker, only after a
video is opened. It reads MP4, MOV, WebM, and MKV containers and copies their streams into
MP4; it does not decode or encode. Fitifact never fetches it from a CDN, and photo-only visits
never download it.

- Source: https://github.com/Vanilagy/mediabunny and the published package
  https://www.npmjs.com/package/mediabunny/v/1.61.1, which includes its TypeScript source
- License: Mozilla Public License 2.0 (the installed package includes its license text)

## Desktop FFmpeg sidecars

The Windows desktop app ships `ffmpeg.exe` and `ffprobe.exe` from FFmpeg 9.0.2,
the gyan.dev essentials build, licensed under the GNU General Public License
version 3. Fitifact runs them as separate processes and does not link them.
The installer puts three files next to them: `FFMPEG-NOTICE.md` (the exact
build, its source, and its license), `FFMPEG-LICENSE.txt` (the GPLv3 text), and
`FFMPEG-BUILD.txt` (its configuration and library versions).

- FFmpeg source for this build: https://github.com/FFmpeg/FFmpeg/commit/946fcce07b
- Build provider: https://www.gyan.dev/ffmpeg/builds/

The public web app does not ship FFmpeg.
