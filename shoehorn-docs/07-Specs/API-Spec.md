---
title: "API Specification"
type: spec
status: active
updated: 2026-08-15
canonical: true
tags:
  - api
  - spec
---

# API specification

Status: design draft.

## Principles

- resource-oriented;
- asynchronous for expensive jobs;
- plans are inspectable;
- idempotent mutation endpoints;
- no implicit cloud upload;
- typed stable errors.

## Local logical API

```text
inspect(input) -> Artifact
compile(sources) -> ConstraintSet
check(artifact, constraints) -> CompatibilityReport
plan(artifact, constraints, preferences) -> PlanResponse
execute(input, plan, options) -> ExecutionResult
validate(output, constraints) -> ValidationReport
adapt(...) -> AdaptationResult
```

## Hosted REST sketch

### `POST /v1/uploads`
Returns a scoped presigned upload URL and `upload_id`.

### `POST /v1/inspections`
```json
{
  "upload_id": "upl_...",
  "options": {}
}
```

### `POST /v1/plans`
```json
{
  "artifact": {},
  "constraints": {},
  "preferences": {}
}
```

Response includes:
- compatible;
- violations;
- recommended plan;
- optional alternatives;
- estimated cloud units;
- warnings.

### `POST /v1/jobs`
```json
{
  "upload_id": "upl_...",
  "plan_id": "plan_..."
}
```

### `GET /v1/jobs/{id}`

States:
- queued;
- running;
- validating;
- completed;
- failed;
- validation_failed;
- cancelled.

### `POST /v1/adapt`
Convenience endpoint combining inspection/planning/execution while still returning the generated plan.

## Why expose planning

Developers may require:
- user consent;
- cost preview;
- quality review;
- policy approval;
- local/cloud selection.

## Idempotency

Mutation endpoints accept:
`Idempotency-Key`.

## Authentication

- scoped API keys;
- rotate/revoke;
- enterprise service identity later.

## Webhooks

Signed events:
- job.started;
- job.completed;
- job.failed;
- job.validation_failed.

## Privacy

API must document:
- where payload is stored;
- retention;
- region;
- deletion semantics.

## Versioning

Use `/v1` for API plus explicit schema versions inside artifacts/profiles/plans.
