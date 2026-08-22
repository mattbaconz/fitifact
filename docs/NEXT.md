---
title: "Next Actions"
type: action-plan
status: active
updated: 2026-08-22
canonical: true
tags:
  - next
  - action-plan
---

# Next actions

The v0.1 CLI/media slice remains frozen at `0.1.0-rc.1`. The
**only allowed `v0.1.0-rc.1` tag target** is commit
`b033552cb2729e96ca97c649a7bb4a223f2ad900`
(`feat: freeze unpublished 0.1.0-rc.1 local candidate`). Annotated tag
`v0.1.0-rc.1` peels to that SHA. Do not tag later image/web commits as
`v0.1.0-rc.1`. Do not reopen the media matrix unless a review finds a
false-safe defect. Keep crates `publish = false`. GitHub Release stays off until `FITIFACT_PUBLICATION_APPROVED`. The freeze tree
is missing `[profile.dist]`; artifact jobs for that tag failed. Do not move
the tag. `v0.1.0-rc.3` is the first published buildable candidate.
`v0.1.0-rc.4` is the D-026 consumer image candidate. `0.1.0-rc.5` is the 0.2
usable session (file-first web, still WebP in, public lazy HEIC via D-028). Do
not invent a `v0.2.0` tag. Create `v0.1.0-rc.5` only from the reviewed version
commit after `main` CI passes. `v0.1.0-rc.2` built native
archives but failed the global SBOM step because cargo-cyclonedx also wrote a
wasm `.cdx.xml`. Do not move any existing tag.

Local identity: the public name is Fitifact; the product git checkout is
`C:\fitifact\fitifact` under the `C:\fitifact` umbrella; in-repo `docs/` is
canonical. Do not edit the sibling vault copy. Preserve Shoehorn only as
historical rename/collision context.

1. **0.2 session (this candidate)** — `0.1.0-rc.5` makes rejected uploads
   obviously fixable: drop the file first, paste what the form said, hide the
   schema, decode still WebP and public HEIC locally. Human gate is next.

2. **Run the human continuation gate** — post-build, not fabricated:
   - execute [[04-Engineering/Consumer-Image-Moderated-Test]] with ten real
     form/application photo tasks using drop-then-paste;
   - require 8/10 completion, 8/10 real destination acceptance, 5/10 return
     intent, and zero harmful outcomes.
   - Do not start `@fitifact/browser` or cloud until those numbers exist.

3. **RC5 release** — publish the protected `v0.1.0-rc.5` prerelease and deploy
   the default lazy-HEIC static build to GitHub Pages after every required CI
   job passes. Keep a decoder-free `FITIFACT_HEIC_APPROVED=false` job.

4. **Publication gate** (owner directed public GitHub 2026-08-18)
   - 2026-08-16 naming packet is in [[01-Product/Naming-Brand]]; it is not
     billed clearance; owner directed GitHub create/push/tag;
   - GitHub Release stays off until `FITIFACT_PUBLICATION_APPROVED=true` and
     Environment `public-release` is approved;
   - follow [[04-Engineering/Release-Checklist]] GitHub-only.

5. **Owner runbook after a public repo exists** (cannot run from this Windows
   workspace today): on clean Windows x64, Linux GNU x64, macOS Intel, and
   macOS Apple Silicon, download CI/GitHub Release assets (do not reuse this
   checkout), verify SHA-256 and attestation, then `fitifact --version`,
   `fitifact doctor`, and the three canonical media fixtures. See the
   `v0.1.0-rc.1` section of the release checklist.

6. **Do not build cloud in the public repository**
   - managed operations stay in the separate private checkout.

7. **After traction (deferred)**
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
