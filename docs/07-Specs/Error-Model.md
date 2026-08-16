---
title: "Error Model"
type: spec
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - errors
  - spec
---

# Error model

## Categories — implemented in v0.1

These codes exist on `fitifact.error/v1` and in the Rust `ErrorCode` enum:

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
- `SECURITY_BLOCKED`

`ALREADY_COMPATIBLE` is a success status, not an error.

## Categories — deferred

These names are reserved for later surfaces. They are not present in v0.1
`ErrorCode` and must not appear in CLI envelopes:

- `PROFILE_STALE`
- `CLOUD_QUOTA`

## CLI exit mapping

Engine failures requested as JSON still use the `fitifact.error/v1` envelope.
Process exit codes for those codes are:

- `INPUT_INVALID`, `INSPECTION_UNSUPPORTED`, `INSPECTION_LIMIT` → 4
- `NO_VALID_PLAN` → 3
- `PROVIDER_MISSING`, `EXECUTION_FAILED`, `EXECUTION_LIMIT` → 5
- `VALIDATION_FAILED` → 6
- `SECURITY_BLOCKED` → 7
- `REQUIREMENTS_AMBIGUOUS`, `REQUIREMENTS_CONFLICT`, and any other unmapped
  engine or usage error → **64**

Constraint compile conflicts are therefore exit 64, not exit 4. See
[[07-Specs/CLI-Spec]].

## Object

```json
{
  "schema": "fitifact.error/v1",
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

Reusable error values carry `fitifact.error/v1`. Constraint parse errors use
stable top-level codes (`INPUT_INVALID` or `REQUIREMENTS_CONFLICT`) and stable
reason prefixes in their messages. Planner refusal is not an execution error:
`fitifact.plan/v1` returns typed blocking codes in `cannot_satisfy`.

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
