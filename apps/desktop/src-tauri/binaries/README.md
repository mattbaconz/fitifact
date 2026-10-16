# Desktop FFmpeg sidecars

The Fitifact desktop app ships `ffmpeg` and `ffprobe` next to the app so video
and audio work without a PATH install. Binaries are not committed.

From `apps/desktop`:

```console
npm run sidecar
```

That copies a local FFmpeg install when one is already on PATH, or downloads a
pinned essentials build. Tauri names the files `ffmpeg-<target-triple>` and
`ffprobe-<target-triple>`.

FFmpeg stays a separate process with its own license. Fitifact does not link it.
The command-line tool still uses PATH.
