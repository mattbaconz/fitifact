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

The v0.1 CLI/media slice is implemented locally: inspection, typed constraints,
bounded planning, no-op/remux/selective-video-transcode, explicit refusal,
post-validation, deterministic fixtures, CI, and GitHub-only packaging
preparation. Do not reopen that slice unless a review finds a false-safe defect.

1. **Finish local RC verification**
   - confirm `0.1.0-rc.1` on the reviewed commit;
   - run fmt, Clippy, unit/integration tests, live FFmpeg tests, fixture and
     workflow checks, and `dist plan --tag=v0.1.0-rc.1`;
   - keep the package unpublished.

2. **Complete local identity**
   - the public name is Fitifact;
   - the checkout folder should match (`C:\fitifact`);
   - preserve Shoehorn only as historical rename/collision context.

3. **Complete the publication gate**
   - automated Fitifact exact-name checks found no material collision signal;
   - complete USPTO/WIPO/EUIPO human/legal review and obtain owner sign-off;
   - do not create the public repository, push, tag, or release before sign-off.

4. **Release the CLI/media slice after sign-off**
   - GitHub-only; no registry publishing or package-manager formulae;
   - follow [[04-Engineering/Release-Checklist]].

5. **Later public MVP: build web/image flow (deferred)**
   - drop;
   - constraints;
   - plan;
   - adapt;
   - validate.

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

Before adding more file families, prove:

```text
cold start -> inspect -> no-op
```

without loading a transform provider.

Then, in deferred later work, prove:
- image workflow does not load media runtime;
- extension remains a thin IPC/UI layer.

For v0.1, prove local check/plan performs no network activity.
