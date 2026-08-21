---
title: "API Specification"
type: spec
status: active
implementation: local-api-implemented-hosted-deferred
updated: 2026-08-21
canonical: true
tags:
  - api
  - spec
---

# API specification

Status: local Rust/WASM contract implemented; hosted REST remains a design
draft and is not exposed by this repository.

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

The WASM bridge exposes deterministic JSON/byte equivalents:

```text
compile_requirements(text) -> requirements report
compile_constraints(json) -> fitifact.constraints/v1 or fitifact.error/v1
image_limits() -> fitifact.image-limits/v1
inspect_bytes(bytes) -> Artifact
plan_bytes(bytes, constraints) -> fitifact.web-plan/v1
adapt_bytes(bytes, constraints, ImageAdaptOptions) -> report + transferable bytes
plan_rgba(rgba, width, height, constraints) -> report + safe PNG preview
adapt_rgba(rgba, width, height, constraints, options) -> report + transferable bytes
validate_bytes(bytes, constraints) -> CompatibilityReport
```

These functions perform no network activity. The worker owns request/source
generation, and output bytes are transferred rather than published. Every
adaptation follows inspect → check → plan → execute → re-inspect → validate.
`ImageAdaptOptions` accepts only a normalized crop rectangle and explicit crop
consent. Errors use `fitifact.error/v1`.

## Hosted REST sketch — deferred

### `POST /v1/uploads`
Returns a scoped presigned upload URL and `upload_id`.

No such endpoint exists in D-026. **Your image stays on this device.** This
section must not be cited as an implemented upload/cloud capability.

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
