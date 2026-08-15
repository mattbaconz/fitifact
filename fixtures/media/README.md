# Generated media fixtures

These files are generated, not committed. They are tiny lavfi test patterns (public-domain synthetic media).

```powershell
powershell -File scripts/generate-fixtures.ps1
```

| File | Purpose |
| --- | --- |
| `h264-aac.mp4` | already compatible |
| `hevc-aac.mp4` | video-only transcode |
| `h264-aac.mov` | remux-only |

Requires `ffmpeg` with `libx264` (and `libx265` for the HEVC fixture).
