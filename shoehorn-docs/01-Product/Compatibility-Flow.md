---
title: "Compatibility Flow"
type: product
status: active
updated: 2026-08-15
canonical: true
tags:
  - compatibility
  - flow
---

# Compatibility flow

## State machine

```text
UNINSPECTED
  ↓
INSPECTED
  ↓
CONSTRAINTS_READY
  ↓
CHECKING
  ├─ valid -> COMPATIBLE
  ├─ plan -> PLANNED -> EXECUTING -> VALIDATING -> ADAPTED
  └─ none -> UNSATISFIABLE
```

Execution can also end:
- FAILED
- FAILED_VALIDATION
- SECURITY_BLOCKED

## Detection vs. repair

`check(file,constraints)` is independently useful and should return mismatches and possible plan families without execution.

## Requirement precedence

1. explicit user hard constraint;
2. exact verified profile;
3. official/current page requirement;
4. server rejection text;
5. HTML hints such as `accept`;
6. inferred heuristic.

Conflicts are surfaced.

## Safe size margin

Do not target the exact ceiling. Profiles/plans can use a safety margin such as 98%.

## Iterative adaptation

Size fitting can be:
`estimate -> encode -> inspect -> adjust -> validate`
with bounded retries and compute budget.

## Feedback

Accepted/rejected user feedback is telemetry, not automatic profile truth.
