# Desktop FFmpeg sidecars

The Fitifact desktop app ships `ffmpeg` and `ffprobe` next to the app so video
and audio work without a PATH install. Binaries are not committed.

From `apps/desktop`:

```console
npm run sidecar
```

On Windows that downloads the pinned FFmpeg build named in
`apps/desktop/FFMPEG-NOTICE.md`, checks its SHA-256, and writes the programs
plus `FFMPEG-LICENSE.txt` and `FFMPEG-BUILD.txt`, which the installer ships
beside them. Elsewhere it copies FFmpeg from PATH for development only. Tauri
names the programs `ffmpeg-<target-triple>` and `ffprobe-<target-triple>`.

FFmpeg stays a separate process with its own license. Fitifact does not link it.
The command-line tool still uses PATH.
