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

1. **Resolve naming**
   - Shoehorn is a strong codename but has serious software collisions.
   - Generate alternatives and check packages/domains/trademarks.

2. **Build architecture spike**
   - chosen core language;
   - FFprobe inspection;
   - image inspection;
   - constraints;
   - graph planner;
   - one FFmpeg provider.

3. **Create deterministic fixtures**
   - MP4 + HEVC mismatch;
   - remux-only MOV;
   - image wrong format/size.

4. **Build internal CLI first**
   - fastest way to validate semantics.

5. **Build web magic**
   - drop;
   - constraints;
   - plan;
   - adapt;
   - validate.

6. **Do not build cloud yet**
   - prove demand.

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

Then prove:
- image workflow does not load media runtime;
- extension remains a thin IPC/UI layer;
- cloud is not contacted during local check/plan.
