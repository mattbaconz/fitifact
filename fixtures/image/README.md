# Image fixtures

Generated: **2026-08-16**

These two files are the D-025 canonical image pair. They are synthesized
in-process with the Rust `image` crate (8×8 sRGB red JPEG and PNG). They are
not FFmpeg/`lavfi` outputs.

- `compatible-jpeg.jpg` — already JPEG; adapt to JPEG is a no-op.
- `mismatch-png.png` — PNG; adapt to JPEG encodes without resizing.

Do not add HEIC, TIFF, WebP, or animation binaries here.
