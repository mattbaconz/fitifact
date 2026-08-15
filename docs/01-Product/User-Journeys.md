---
title: "User Journeys"
type: product
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - user-journeys
---

# User journeys

## A — impulse web user
1. Sees video.
2. Opens web app.
3. Drops file.
4. Chooses destination or pastes requirement.
5. Sees current facts, problem, proposed changes.
6. Clicks Adapt.
7. Local/native/cloud processing with explicit location.
8. Output validates.
9. Saves result.
10. Optional “Was it accepted?” feedback.

## B — browser extension
1. Upload rejected.
2. User explicitly opens extension.
3. Extension reads allowed page hints with permission.
4. User selects file.
5. Fitifact merges page evidence, rejection and profile.
6. Shows confidence.
7. Adapts.
8. Saves/reselects where possible.
9. User retries.

## C — OS context menu
Right-click → `Adapt for...` → choose destination → plan → output sibling file.

## D — mobile share sheet
Share → Fitifact → destination → adapt → share result onward.

## E — developer API
Declare constraints → check → plan → execute → validate → use report.

## F — enterprise/private worker
Control plane sends signed job metadata → customer worker processes in customer environment → validation/audit returned.

## Emotional flow

```text
confusion
"why won't this upload?"
    ↓
clarity
"the codec is wrong"
    ↓
confidence
"only video changes"
    ↓
relief
"accepted"
```
