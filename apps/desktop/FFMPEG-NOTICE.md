# FFmpeg sidecar notice

The Fitifact desktop app may ship `ffmpeg` and `ffprobe` next to the app so
video and audio work without a PATH install. Those programs are separate
processes. Fitifact (Apache-2.0) does not link against FFmpeg.

FFmpeg has its own license, which depends on the build:

- Project: https://ffmpeg.org
- Source: https://ffmpeg.org/download.html
- Windows packaging fallback used by `scripts/fetch-desktop-ffmpeg.ps1`:
  https://www.gyan.dev/ffmpeg/builds/

Builds that include libx264 are typically GPL. If this app copied FFmpeg from
your PATH, that binary’s license applies.

The Fitifact command-line tool does not ship FFmpeg; it uses PATH.
