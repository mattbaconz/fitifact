# Image fixtures

Generated/expanded: **2026-08-25** from owned synthetic pixels. No fixture was
copied from the web or a third-party photo.

`cargo run --locked -p fitifact --example generate_image_fixtures` creates the
JPEG/PNG/WebP/TIFF/BMP/GIF set with the pinned Rust `image` crate.
`compatible-jpeg.jpg` and `mismatch-png.png` remain the original 8 × 8 D-025
canonical pair. `still-webp.webp` is an owned 8 × 8 still WebP.
`still-tiff.tiff`, `still-bmp.bmp`, and `still-gif.gif` are owned 8 × 8 still
images. `animated-gif.gif` is an owned two-frame GIF used to prove first-frame
consent. `transparent-png.png` exercises alpha refusal, `crop-grid.png` makes
crop consent visually reviewable, `malformed-image.jpg` is a deliberately
truncated JPEG-signature input, and `oversized-pixels.png` is a valid
6,001 × 4,000 solid PNG that exceeds the 24-megapixel decoded limit while
remaining repository-safe.

`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
scripts/generate-heic-fixture.ps1 -Force` encodes `synthetic-single.heic` from
an owned 16 × 12 RGBA pattern through the installed Windows HEIF encoder. The
canonical artifact used Microsoft HEIF Image Extension 1.2.48.0 (x64). Its
HEVC bitstream is not promised byte-for-byte reproducible across encoder runs,
so the checked-in result is checksum-pinned and the script does not overwrite
it without `-Force`. The approved-gate web test decodes it as exactly one image;
zero- and multi-image rejection semantics are covered separately.

Run `./scripts/generate-image-fixtures.ps1` to reproduce the deterministic set
and retain the canonical HEIC, then `./scripts/check-fixtures.ps1` to verify the
exact inventory, sizes, checksums, and this provenance record.
