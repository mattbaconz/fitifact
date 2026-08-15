---
title: "Next Actions"
type: action-plan
status: active
updated: 2026-08-15
canonical: true
tags:
  - next
  - action-plan
---

# Next actions

1. **Complete the publication gate**
   - automated Fitifact exact-name checks found no material collision signal;
   - complete USPTO/WIPO/EUIPO human/legal review and obtain owner sign-off;
   - do not publish before sign-off.

2. **Harden the implemented v0.1 CLI/media slice**
   - Rust core and CLI;
   - FFprobe inspection;
   - typed constraints and bounded planner;
   - FFmpeg no-op/remux/selective-video-transcode behavior;
   - explicit refusal and post-validation.

3. **Create deterministic fixtures**
   - MP4 + HEVC mismatch;
   - remux-only MOV;
   - image wrong format/size is deferred to the later public MVP.

4. **Release the CLI/media slice after sign-off**
   - GitHub-only; no registry publishing or package-manager formulae.

5. **Later public MVP: build web/image flow (deferred)**
   - drop;
   - constraints;
   - plan;
   - adapt;
   - validate.

6. **Do not build cloud in the public repository**
   - managed operations are deferred to a separate private checkout.

7. **Prepare FOSS**
   - license audit;
   - security policy;
   - contribution/profile workflow.

8. **Record YouTube against a local target**
   - deterministic acceptance.

9. **After traction**
   - extension;
   - registry;
   - hosted API.

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

Before adding more file families, prove:

```text
cold start -> inspect -> no-op
```

without loading a transform provider.

Then, in deferred later work, prove:
- image workflow does not load media runtime;
- extension remains a thin IPC/UI layer.

For v0.1, prove local check/plan performs no network activity.
