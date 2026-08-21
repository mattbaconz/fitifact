---
title: "Consumer Image UI Polish Audit"
type: verification-evidence
status: reviewed
updated: 2026-08-21
canonical: false
tags:
  - ui
  - evidence
---

# Consumer image UI polish audit

Functional precondition: the Task 2 production worker/WASM workflow passed 36
browser flows, and the Task 3 owned HEIC fixture passed the real approved
decoder test before this audit.

Reviewed baseline evidence:

- [desktop, 1280 × 1351](before-desktop.png) captured from a 1280 × 900
  Chromium viewport;
- [mobile, 1170 × 4638 physical pixels](before-mobile.png) captured from a
  390 × 844 CSS-pixel iPhone 14 viewport at device scale factor 3.

## Five highest-impact issues recorded before changes

1. **Hierarchy:** the oversized editorial hero dominates the actual four-step
   tool, making a work surface read like a marketing landing page.
2. **Spacing/density:** large hero/card gaps and generous mobile padding create
   avoidable scrolling; empty workflow states consume as much space as active
   controls.
3. **State clarity:** step 4 initially says “Describe the upload requirement,”
   duplicating step 1 and obscuring that this area is where plans, validation,
   and the download appear. The gated file input has no visible prerequisite.
4. **Accessibility/keyboard visibility:** the visually hidden file input can
   receive keyboard focus without a reliable visible focus ring on its drop
   target. State tone is carried too heavily by subtle border color.
5. **Mobile behavior:** the headline, privacy badge, card headings, and drop
   target scale too large at 390 CSS pixels, producing a long, low-density page
   before any result exists.

## Scope of the pass

Frontend presentation and exact result copy only: compact hierarchy and
spacing, clearer status structure/prerequisite text, keyboard focus visibility,
stronger state cues, and mobile density. No information-architecture rewrite,
new product feature, backend/worker behavior, gradient, decorative dashboard
card, fake metric, or marketing redesign.

## Problems fixed

- Reduced the hero's maximum type size and vertical footprint so the workflow
  is visible sooner and reads as the primary product.
- Tightened card/gap/drop-target spacing and removed avoidable empty-card
  height by aligning grid items to their content.
- Made step 4 consistently “Review and download,” separated the live state
  heading, exposed the pre-file prerequisite, and added the exact validation
  boundary plus undocumented-rule caveat.
- Added a focus-within ring for the visually hidden file input/drop target and
  stronger top-border state cues that do not rely on subtle color alone.
- Reduced mobile heading, header, badge, card, and drop-target density while
  retaining 44-pixel controls and readable single-column fields.

## Files changed

- `web/src/App.tsx`
- `web/src/styles.css`
- `web/e2e/workflow.spec.ts` (state/copy assertions only)

## Not changed (intentional)

No worker/core architecture, feature set, information architecture, font/CDN,
branding system, format scope, or destination promise changed. The active
mobile result remains a long page because it shows editable requirements,
warnings, every validation check, and the download in one honest sequence; the
review found no horizontal overflow, clipped control, or obscured action.

## Reviewed after evidence

- [completed desktop workflow, 1280 × 1657](after-desktop.png), captured from a
  1280 × 900 Chromium viewport after adapting the tracked PNG fixture;
- [completed mobile workflow, 1170 × 7302 physical pixels](after-mobile.png),
  captured from a 390 × 844 CSS-pixel iPhone 14 viewport at device scale factor
  3 after the same real worker/WASM adaptation.

Both screenshots were visually reviewed on 2026-08-21. State hierarchy,
spacing, validation wording, checklist/download visibility, and responsive
containment passed. No remaining visual defect blocks the MVP.

## Manual verify steps

1. Build the default static product and serve `web/dist` without the HEIC flag.
2. At 1280 × 900 and 390 × 844, review the default state and tab to the hidden
   file control after requirements are ready; confirm the drop target shows the
   focus ring.
3. Adapt `fixtures/image/mismatch-png.png` to the default target and confirm the
   warnings, all-pass checklist, validation boundary, and download remain
   visible with no horizontal overflow.
