---
title: "Next Actions"
type: action-plan
status: active
updated: 2026-08-16
canonical: true
tags:
  - next
  - action-plan
---

# Next actions

The v0.1 CLI/media slice is frozen locally as unpublished `0.1.0-rc.1`. The
**only allowed RC tag target** is commit
`b033552cb2729e96ca97c649a7bb4a223f2ad900`
(`feat: freeze unpublished 0.1.0-rc.1 local candidate`). Do not tag later
image/web commits as `v0.1.0-rc.1`. Do not reopen the media matrix unless a
review finds a false-safe defect. Keep the package unpublished.

Local identity: the public name is Fitifact; the product git checkout is
`C:\fitifact\fitifact` under the `C:\fitifact` umbrella; in-repo `docs/` is
canonical. Do not edit the sibling vault copy. Preserve Shoehorn only as
historical rename/collision context.

1. **Local RC verification** — done on the freeze commit; stay unpublished.

2. **Publication gate** (owner — **paused here**)
   - 2026-08-16 naming packet is in [[01-Product/Naming-Brand]]; it is not
     clearance; owner/legal sign-off is unchecked;
   - do not create `mattbaconz/fitifact`, push, tag, or release before sign-off;
   - after sign-off, follow [[04-Engineering/Release-Checklist]] GitHub-only.

3. **Owner runbook after a public repo exists** (cannot run from this Windows
   workspace today): on clean Windows x64, Linux GNU x64, macOS Intel, and
   macOS Apple Silicon, download CI/GitHub Release assets (do not reuse this
   checkout), verify SHA-256 and attestation, then `fitifact --version`,
   `fitifact doctor`, and the three canonical media fixtures. See the
   `v0.1.0-rc.1` section of the release checklist.

4. **Image then local-only web** — landed on commits after the freeze SHA
   (do not tag them `v0.1.0-rc.1`):
   - JPEG no-op and PNG→JPEG (D-025);
   - static WASM drop flow for that image matrix only;
   - no ffmpeg.wasm, no uploads, no cloud in this repository.

5. **Do not build cloud in the public repository**
   - managed operations stay in the separate private checkout.

6. **After traction (deferred)**
   - destination profiles and registry workflow;
   - extension;
   - hosted API;
   - record YouTube against a local target.

## First milestone

```text
Input:
MP4 + HEVC + AAC

Target:
MP4 + H264 + AAC

Expected plan:
transcode video only

Expected:
audio preserved
final validation passes
```

If the core cannot reliably make this decision, do not add more formats.

## Optimization milestone

v0.1 proves locally:

```text
cold start -> inspect -> no-op
```

without constructing or spawning a transform provider. `check`/`plan` spawn only
`ffprobe` and the workspace `Cargo.lock` contains no HTTP client or `tokio`
runtime. Run `fitifact bench` for the measured report.

Then, in deferred later work, prove:
- image workflow does not load media runtime;
- extension remains a thin IPC/UI layer.
