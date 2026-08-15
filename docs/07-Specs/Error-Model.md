---
title: "Error Model"
type: spec
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - errors
  - spec
---

# Error model

## Categories

- `INPUT_INVALID`
- `INSPECTION_UNSUPPORTED`
- `INSPECTION_LIMIT`
- `REQUIREMENTS_AMBIGUOUS`
- `REQUIREMENTS_CONFLICT`
- `NO_VALID_PLAN`
- `PROVIDER_MISSING`
- `EXECUTION_FAILED`
- `EXECUTION_LIMIT`
- `VALIDATION_FAILED`
- `PROFILE_STALE`
- `SECURITY_BLOCKED`
- `CLOUD_QUOTA`

`ALREADY_COMPATIBLE` is a success status, not an error.

## Object

```json
{
  "code": "VALIDATION_FAILED",
  "message": "The output is still larger than the 25 MB limit.",
  "details": {
    "actualBytes": 25210000,
    "maxBytes": 25000000
  },
  "retryable": true,
  "suggestions": []
}
```

## User messaging

Never stop at raw provider output such as “FFmpeg exited 1.”

Translate known failures; expose raw diagnostics only in advanced/debug views.

## Retryable

Explicitly classify:
- transient infra: yes;
- corrupt file: no;
- unsupported codec/provider: no until environment changes;
- near-miss size fit: bounded retry possible.

## Validation

Output exists but failed constraints = first-class failure.
