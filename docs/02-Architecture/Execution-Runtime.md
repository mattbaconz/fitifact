---
title: "Execution Runtime"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - runtime
  - architecture
---

# Execution runtime

## Role

Translate provider-neutral typed steps into safe operations. Runtime must not reinterpret product intent.

## Safe provider boundary

Planner:
`TranscodeVideo(codec=h264)`

Provider:
- validates enum;
- builds argument array;
- invokes known executable;
- monitors resources;
- returns structured result.

Never concatenate user input into shell commands.

## Modes

### Browser
WASM/Web Workers. Best for smaller workloads.

### Native
Fast local providers and OS integrations.

### Cloud
Ephemeral isolated worker with quotas.

## Job lifecycle

```text
prepare
verify input
allocate workspace
execute steps
inspect intermediates if needed
inspect final
validate
cleanup
```

## Intermediates

- unique workspace;
- generated paths;
- no traversal;
- cleanup on failure;
- retention only by explicit policy.

## Resource budgets

- wall time;
- CPU;
- memory;
- disk;
- output bytes/count;
- decoded dimensions/frame/page count.

## Retry

Retry transient infra failures, not deterministic corrupt-file errors.

## Hardware acceleration

Model as provider capability and record exact provider. Faster may trade quality/reproducibility.

## Output validation

Cannot be bypassed.

## Logs

Separate:
- user explanation;
- structured operational logs;
- raw provider debug logs;
- security audit.

## Provider loading policy

Provider initialization is lazy.

Examples:
- already-compatible file: no transformer initialization;
- remux-only plan: no lossy encoder startup;
- image adaptation: no media/PDF providers;
- browser page load: no large WASM runtime until the plan requires it.

Capability metadata should be cheap to query separately from expensive provider initialization.
