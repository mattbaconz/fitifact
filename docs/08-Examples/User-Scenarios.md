---
title: "User Scenarios"
type: examples
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - examples
---

# User scenarios

## 1 — MP4 that still fails

Input:
```text
demo.mp4
container: mp4
video: hevc
audio: aac
size: 41.8 MB
```

Target:
```text
container: mp4
video: h264
max: 25 MB
```

Violations:
- video codec;
- size.

Plan:
1. transcode video only;
2. fit bitrate below safe margin;
3. preserve AAC audio;
4. validate.

Explanation:
> Your file is MP4, but the video inside it uses HEVC. This target needs H.264. The file is also too large. Fitifact will leave the audio untouched.

## 2 — remux only

Input MOV already contains H.264/AAC.
Target needs MP4/H.264/AAC.

Plan:
- remux only;
- no re-encode.

## 3 — already valid

All hard constraints pass.

Plan:
- no-op.

## 4 — HEIC avatar

Input:
`4032×3024 HEIC, 8.3 MB`

Target:
`JPEG/PNG, square, >=512, <=5 MB`

Fitifact must ask crop vs padding; never crop silently.

## 5 — impossible size

4K long video, target 1 MB, preserve 4K/current quality.

Result:
`cannot_satisfy`.

Offer relaxation:
- allow lower resolution;
- increase size;
- send link instead.

## 6 — conflicting requirements

Registry 25 MB, current page 10 MB.

Use page for operation, record conflict, mark registry potentially stale.

## 7 — vague rejection

“Unsupported file.”

If page/profile evidence insufficient:
> I can inspect the file, but I don't have enough evidence to guarantee what this target requires.
