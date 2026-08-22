---
title: "Consumer Image Moderated Test"
type: validation-protocol
status: ready-not-run
implementation: post-build-human-gate
updated: 2026-08-21
canonical: true
tags:
  - testing
  - consumer-image
  - viability
---

# Consumer image moderated test

This is the post-build human viability gate for D-026. It defines the study;
it does **not** report results. Recruit ten real participants with ten real
form/application photo tasks and execute only after the reviewed build is
available. Never fill the scorecard from engineering tests or invented data.

## Continuation thresholds

Continue this MVP only if all four thresholds hold:

1. **At least 8/10 task completions:** the participant reaches a downloaded,
   validated output without moderator takeover or codec/format instruction.
2. **At least 8/10 destination acceptances:** the real form/application accepts
   that output. A Fitifact validation badge alone does not count.
3. **At least 5/10 return intent:** after seeing the result, the participant
   says they would choose Fitifact for another similar photo requirement.
4. **Zero harmful outcomes:** no original overwrite, undisclosed data transfer,
   unapproved crop, silently flattened transparency, hidden lossy/upscale
   change, or claim of validation when a confirmed hard requirement failed.

A single zero-harm breach stops continuation even if the numeric rates pass.
Server rejection caused by an undocumented rule counts against destination
acceptance, but is not a harmful Fitifact outcome when the product accurately
said **“validated against the requirements you confirmed.”**

## Recruitment and task rules

- Ten adults, one scored task each; do not score a moderator or contributor.
- Use an actual application/form photo requirement the participant currently
  needs to satisfy. Record the destination and requirement text with consent;
  redact account identifiers.
- Include a practical mix of JPEG, PNG, phone-origin HEIC where the approved
  build is legally enabled, size limits, dimension limits, and at least two
  tasks that require crop consent. Do not manufacture a crop just to pass a
  quota.
- Use the participant's own device where practical. Do not move their image to
  a researcher machine unless separately consented.
- Do not ask participants to risk a deadline-critical or irreversible
  submission. Test a draft/preview upload where the destination permits it.

## Moderator script

1. Say: “Use this page to make your image meet the form's instructions. Think
   aloud. I won't explain image formats, but you can stop at any time.”
2. Ask the participant to paste/type the destination requirement, review the
   normalized target, choose the photo, and proceed as they think appropriate.
3. Do not explain the four workflow steps, interpret warnings, select crop
   consent, or tell them which button to press. Record any takeover.
4. Before download, ask: “What will Fitifact change? What stays on this device?
   What does validation mean here?” Record comprehension without correcting it.
5. Let the participant submit the downloaded result to the real destination.
   Record accepted/rejected and the verbatim destination message when safe.
6. Ask: “Would you choose this for another similar photo requirement? Why or
   why not?” Record yes/no before discussion.
7. Debrief, correct any privacy/acceptance misunderstanding, and delete study
   copies from the researcher-controlled device.

## Per-task scorecard

| Task | Real destination/task | Input | Requirement types | Crop required/approved | Warnings understood | Completed unaided | Destination accepted | Would use again | Harm observed | Notes/rejection text |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 01 | Not run | — | — | — | — | — | — | — | — | — |
| 02 | Not run | — | — | — | — | — | — | — | — | — |
| 03 | Not run | — | — | — | — | — | — | — | — | — |
| 04 | Not run | — | — | — | — | — | — | — | — | — |
| 05 | Not run | — | — | — | — | — | — | — | — | — |
| 06 | Not run | — | — | — | — | — | — | — | — | — |
| 07 | Not run | — | — | — | — | — | — | — | — | — |
| 08 | Not run | — | — | — | — | — | — | — | — | — |
| 09 | Not run | — | — | — | — | — | — | — | — | — |
| 10 | Not run | — | — | — | — | — | — | — | — | — |

## Study summary (leave blank until executed)

| Gate | Required | Observed | Pass/fail |
| --- | ---: | ---: | --- |
| Unaided task completion | ≥ 8/10 | Not run | Not evaluated |
| Real destination acceptance | ≥ 8/10 | Not run | Not evaluated |
| Return intent | ≥ 5/10 | Not run | Not evaluated |
| Harmful outcomes | 0 | Not run | Not evaluated |

The study owner must attach consent-safe notes, build commit, browser/device,
destination date, and signed decision. Engineering completion must never be
reported as passing this human gate.
