# Synthetic media fixtures

These six small binaries are tracked canonical test inputs. They contain only
FFmpeg `lavfi` test patterns, generated sine audio, and (for the corrupt case)
the first 24 bytes of another synthetic fixture. They contain no user or
copyrighted media.

| File | Expected purpose |
| --- | --- |
| `compatible-h264-aac.mp4` | Already-compatible MP4/H.264/AAC no-op |
| `mismatch-hevc-aac.mp4` | SDR 8-bit HEVC video requiring H.264 transcode; AAC is copied |
| `remux-h264-aac.mov` | H.264/AAC streams requiring lossless MOV-to-MP4 remux |
| `corrupt-truncated.mp4` | Deterministically truncated inspection failure |
| `unsupported-extra-video.mp4` | Two-video-stream topology that v0.1 refuses |
| `refusal-hdr10-hevc-aac.mp4` | BT.2020/PQ 10-bit HEVC input that v0.1 refuses |

## Provenance

The tracked files were generated on 2026-08-16 from
[`scripts/generate-fixtures.ps1`](../../scripts/generate-fixtures.ps1) on
Windows using:

- `ffmpeg version 8.1.1-full_build-www.gyan.dev`
- `ffprobe version 8.1.1-full_build-www.gyan.dev`
- FFmpeg `lavfi` sources `testsrc2`, `sine`, and `color`
- the FFmpeg `libx264`, `libx265`, and native `aac` encoders

[`SHA256SUMS`](SHA256SUMS) records the tracked bytes. A byte-for-byte second
generation with the same provider build produced the same six hashes.

The script prints every exact `ffmpeg` argv invocation before executing it. Its
literal argument arrays are the authoritative generation commands, including
fixed duration, dimensions, rate, metadata, threading, pixel format, and color
facts. The corrupt fixture is generated with .NET file APIs by copying exactly
the first 24 bytes of `compatible-h264-aac.mp4` to a new file.

The exact command record below uses repository-relative output paths; no
arguments are omitted (`generate-fixtures.ps1` passes each token as an argv
element, not through shell interpolation):

```text
ffmpeg -nostdin -hide_banner -loglevel error -n -f lavfi -i testsrc2=size=160x120:rate=10:duration=0.6 -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.6 -map 0:v:0 -map 1:a:0 -map_metadata -1 -metadata creation_time=1970-01-01T00:00:00Z -fflags +bitexact -flags:v +bitexact -flags:a +bitexact -threads 1 -c:a aac -b:a 48k -c:v libx264 -pix_fmt yuv420p -x264-params threads=1:lookahead_threads=1:sliced_threads=0 -color_range tv -colorspace bt709 -color_trc bt709 -color_primaries bt709 -movflags +faststart fixtures/media/compatible-h264-aac.mp4
ffmpeg -nostdin -hide_banner -loglevel error -n -f lavfi -i testsrc2=size=160x120:rate=10:duration=0.6 -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.6 -map 0:v:0 -map 1:a:0 -map_metadata -1 -metadata creation_time=1970-01-01T00:00:00Z -fflags +bitexact -flags:v +bitexact -flags:a +bitexact -threads 1 -c:a aac -b:a 48k -c:v libx265 -pix_fmt yuv420p -tag:v hvc1 -x265-params pools=none:frame-threads=1:wpp=0:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited -color_range tv -colorspace bt709 -color_trc bt709 -color_primaries bt709 -movflags +faststart fixtures/media/mismatch-hevc-aac.mp4
ffmpeg -nostdin -hide_banner -loglevel error -n -f lavfi -i testsrc2=size=160x120:rate=10:duration=0.6 -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.6 -map 0:v:0 -map 1:a:0 -map_metadata -1 -metadata creation_time=1970-01-01T00:00:00Z -fflags +bitexact -flags:v +bitexact -flags:a +bitexact -threads 1 -c:a aac -b:a 48k -c:v libx264 -pix_fmt yuv420p -x264-params threads=1:lookahead_threads=1:sliced_threads=0 -color_range tv -colorspace bt709 -color_trc bt709 -color_primaries bt709 -movflags +faststart fixtures/media/remux-h264-aac.mov
ffmpeg -nostdin -hide_banner -loglevel error -n -f lavfi -i testsrc2=size=160x120:rate=10:duration=0.6 -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.6 -f lavfi -i color=c=blue:size=32x32:rate=10:duration=0.6 -map 0:v:0 -map 1:a:0 -map 2:v:0 -map_metadata -1 -metadata creation_time=1970-01-01T00:00:00Z -fflags +bitexact -threads 1 -c:v libx264 -pix_fmt yuv420p -x264-params threads=1:lookahead_threads=1:sliced_threads=0 -c:a aac -b:a 48k -movflags +faststart fixtures/media/unsupported-extra-video.mp4
ffmpeg -nostdin -hide_banner -loglevel error -n -f lavfi -i testsrc2=size=160x120:rate=10:duration=0.6 -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.6 -map 0:v:0 -map 1:a:0 -map_metadata -1 -metadata creation_time=1970-01-01T00:00:00Z -fflags +bitexact -flags:v +bitexact -flags:a +bitexact -threads 1 -c:a aac -b:a 48k -c:v libx265 -pix_fmt yuv420p10le -tag:v hvc1 -x265-params pools=none:frame-threads=1:wpp=0:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:range=limited -color_range tv -colorspace bt2020nc -color_trc smpte2084 -color_primaries bt2020 -movflags +faststart fixtures/media/refusal-hdr10-hevc-aac.mp4
```

## Recreate

Install system `ffmpeg` and `ffprobe` with `libx264`, `libx265` (including
10-bit input support), and `aac`, then run from the repository root:

```powershell
powershell -NoProfile -File scripts/generate-fixtures.ps1 -Force
```

Without `-Force`, generation refuses existing outputs. With `-Force`, the
script removes only the six known fixture paths and still passes FFmpeg `-n`,
so FFmpeg itself never clobbers an output. Missing encoders and unsupported
10-bit encoding fail the run with a clear error; fixtures are never silently
skipped.
