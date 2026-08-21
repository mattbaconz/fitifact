+# Third-party notices

## Optional HEIC decoder

The HEIC path dynamically imports `libheif-js` 1.19.8, an Emscripten browser build of
libheif, only after local HEIF magic detection and only when the build-time
`FITIFACT_HEIC_APPROVED=true` gate is present.

- JavaScript package source: https://github.com/catdad-experiments/libheif-js/tree/1.19.8
- Upstream libheif source: https://github.com/strukturag/libheif
- Build form: `libheif-js/wasm-bundle`, distributed by the package as an embedded WebAssembly build
- Package license: LGPL-3.0 (the installed package includes its license text)
- Upstream licenses and bundled codec license details: see the source distributions above and
  `node_modules/libheif-js` for the exact installed artifact.

Fitifact does not load this optional decoder when the gate is absent and never fetches it from a CDN.

