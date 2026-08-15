---
title: "UX Specification"
type: ux-spec
status: active
updated: 2026-08-15
canonical: true
tags:
  - ux
  - product
---

# UX specification

## Homepage
Headline:
> **Make this file work.**

Primary:
`[ Drop a file ]`

Then:
`Where does it need to work?`
or
`Paste requirements / rejection`

Do not show format tiles above the fold.

## Inspection card

```text
video.mp4
Container       MP4
Video           HEVC Main10
Audio           AAC
Resolution      1920×1080
Size            41.8 MB
```

Simple translation:
> The file is MP4, but the video inside uses HEVC.

## Requirement card

```text
Target needs
✓ MP4
✕ H.264 video
✓ AAC audio
✕ under 25 MB
```

Every requirement has source/provenance.

## Plan card

```text
I need to change 2 things
1. HEVC -> H.264
2. Reduce bitrate enough to fit under 25 MB

Keeping:
✓ 1080p
✓ audio
✓ frame rate
```

## No-op
> ✓ This file already fits. Nothing needs to change.

## Impossible
> I can't meet all requirements without breaking your priorities.

Show conflict and minimal relaxations.

## Confidence
Prefer categorical:
- Verified
- Documented
- Inferred
- User-provided
- Unknown

Avoid fake probabilities.

## Advanced mode
Expose codecs, streams, planner costs, sources, providers and candidate plans.

## Privacy
Always state:
`Processing: On this device`
or
`Processing: Shoehorn Cloud — region/retention`

## Accessibility
Keyboard, screen-reader labels, non-color-only status, reduced motion, cancellable progress.
