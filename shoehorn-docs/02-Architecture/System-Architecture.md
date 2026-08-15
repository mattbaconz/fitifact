---
title: "System Architecture"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - architecture
  - canonical
---

# System architecture

## Goal

Separate **compatibility reasoning** from **file transformation mechanics**.

```text
                 ┌────────────────────┐
                 │      Surfaces      │
                 │ web/ext/CLI/API/OS │
                 └─────────┬──────────┘
                           │
                 ┌─────────▼──────────┐
                 │   Constraint In   │
                 └─────────┬──────────┘
                           │
┌───────────────┐  ┌───────▼────────┐
│ File Inspector│->│ Compatibility   │
│ actual state  │  │ Core            │
└───────────────┘  │ check + plan    │
                   └───────┬────────┘
                           │ plan
                 ┌─────────▼──────────┐
                 │ Execution Runtime  │
                 │ provider sandbox   │
                 └─────────┬──────────┘
                           │ artifact
                 ┌─────────▼──────────┐
                 │ Validator          │
                 └─────────┬──────────┘
                           │
                    result/report
```

## Logical components

### Artifact subsystem
Content hash, source handle, detected family, normalized metadata, streams/components, provenance.

### Inspector
File-family probes produce normalized facts.

### Constraint compiler
Takes structured constraints, profiles, user text, page hints and rejection messages and emits typed requirements.

### Compatibility checker
Pure evaluation:
`check(artifact_state, constraints) -> violations`

### Transformation graph
Catalog of available state transitions.

### Planner
Searches graph for valid candidates while minimizing mutation, semantic loss, quality loss, risk and compute.

### Runtime
Executes provider-neutral plan with sandboxed transform providers.

### Validator
Re-inspects output and checks hard constraints.

### Registry
Stores destination profiles, versions, evidence, tests and trust state.

### Report builder
Produces machine JSON, human explanation and audit metadata.

## Recommended repo shape

```text
/
├── crates/
│   ├── core/
│   ├── artifact/
│   ├── constraints/
│   ├── planner/
│   ├── registry/
│   ├── runtime/
│   └── inspectors/
├── providers/
│   ├── ffmpeg/
│   ├── images/
│   ├── pdf/
│   └── ...
├── apps/
│   ├── cli/
│   ├── web/
│   ├── extension/
│   └── desktop/
├── sdk/
├── profiles/
├── fixtures/
└── docs/
```

This is a target shape, not permission to over-modularize v0.

## Provider-neutral plans

Planner emits:

```text
TranscodeVideo {
  to: h264,
  preserve_resolution: true
}
```

not shell strings.

A provider translates typed intent to a safe argument vector.

## Idempotency

`check` and `plan` should be pure relative to:
- artifact facts;
- constraints;
- provider capability snapshot;
- planner version.

Cloud execution should use job IDs, hashes and idempotency keys.

## Versioning

Version independently:
- constraint schema;
- profile schema;
- planner;
- provider catalog;
- API.

Every report records relevant versions.

## Failure philosophy

Fail closed:
- uncertain parse -> unknown;
- transform error -> failed;
- validation mismatch -> validation failed;
- stale profile -> warn/block per policy.

Never reinterpret provider success as destination compatibility.

## Footprint and dependency direction

Shoehorn's practical dependency direction should preserve:

```text
schema/core
    ↓
provider interfaces
    ↓
runtime bindings
    ↓
thin integrations
```

The core must not know specific integrations or destination UI exist.

Heavy providers are discovered and loaded on demand, not treated as mandatory cold-start dependencies.

See [[02-Architecture/Lightweight-Architecture]].
