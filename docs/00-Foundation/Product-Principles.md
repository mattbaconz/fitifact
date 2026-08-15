---
title: "Product Principles"
type: principles
status: active
updated: 2026-08-15
canonical: true
tags:
  - principles
  - canonical
---

# Product principles

1. **Destination before format.** Ask what must work, not what extension the user wants.
2. **Minimum mutation.** No-op > metadata/container-only > lossless > selective lossy > broad lossy > semantic conversion.
3. **Hard constraints are sacred.** A byte over a hard limit is failure.
4. **Preferences are not constraints.** “Preserve resolution” is soft unless pinned.
5. **Explain the why.** Diagnose, show changes, show preservation, show confidence.
6. **Never hallucinate compatibility.** Unknown must stay unknown.
7. **Local-first.** Cloud is explicit.
8. **Validate the output, not the command.** Exit code 0 is not compatibility.
9. **Preserve originals.** Never overwrite by default.
10. **Separate intelligence and execution.** Fitifact decides what; mature providers do how.
11. **Profiles require provenance.** Source, scope, verification date, confidence.
12. **Simple surface, technical depth.** Jargon belongs in expandable details.
13. **Security before format count.**
14. **FOSS must be real.** The planner/compatibility model is open.
15. **Refuse bad adaptations.** If constraints require unacceptable damage, explain and offer alternatives.

## 16. Minimum footprint

The same philosophy used for files applies to Fitifact itself:

> **Load and mutate only what is necessary.**

Fitifact should:
- remain dormant when unused;
- avoid mandatory background services;
- load transform providers lazily;
- keep integrations thin;
- avoid huge initial web/extension bundles;
- prefer operating-system capabilities where appropriate.

See [[02-Architecture/Lightweight-Architecture]].
