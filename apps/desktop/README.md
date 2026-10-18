# Fitifact desktop

Thin Tauri 2 host for the same local Fitifact UI. Photos adapt in-process.
Video and audio inspect and adapt with **shipped** `ffmpeg` and `ffprobe`
sidecars. You do not install FFmpeg or edit PATH to use the app.

The command-line tool still uses FFmpeg from PATH. This crate is **not** a
Cargo workspace member. Build it from this directory.

## Requirements

- Node.js and the workspace `web/` UI (`npm ci` in `web/`)
- Rust toolchain
- Tauri 2 CLI (this package pins `@tauri-apps/cli`)

`npm run sidecar` downloads the pinned FFmpeg build named in
[`FFMPEG-NOTICE.md`](FFMPEG-NOTICE.md) into `src-tauri/binaries/`, checks its
SHA-256, and extracts its license and build notes beside the programs. Those
files are gitignored. CI (`desktop.yml`) packages an unsigned Windows installer
and uploads the `desktop-windows` artifact.

## Run

```console
cd web
npm ci
cd ../apps/desktop
npm ci
npm run sidecar
npm run dev
```

First launch opens Setup: declare destinations (Fitifact cannot see Discord
Nitro). Drop a photo, video, or sound file, tap where it has to work, and
Fitifact makes only the changes needed. Compatible files are a no-op.

## Package

```console
cd apps/desktop
npm run build
```

The UI build writes `web/dist`. Tauri bundles that, the YAML profiles under
`profiles/`, and the FFmpeg sidecars with their notices. Unsigned Windows and
macOS binaries will show OS warnings; see the
[code signing policy](../../README.md#code-signing-policy).
This RC packages NSIS only (`1.0.0-rc.1` is not a valid MSI prerelease).
FFmpeg is a separate process with its own license; see
[`FFMPEG-NOTICE.md`](FFMPEG-NOTICE.md).

Video sources: MP4, MOV, WebM, MKV. Photos: JPEG, PNG, WebP, HEIC, TIFF, BMP,
GIF. Audio: MP3, WAV, FLAC, M4A, OGG. PDF, ZIP, Office files, and SVG are refused.
