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

The v0.1 native provider uses `Command` with typed argv and system
`ffmpeg`/`ffprobe` from `PATH`; it never invokes a shell. Media input is limited
to the `file` protocol. Probe timeout is 30 seconds. Transform timeout defaults
to 30 minutes and is bounded by the CLI. Process stdout is capped at 1 MiB and
only the final 256 KiB of stderr is retained; timeout kills, waits for, and
reaps the child. User errors contain stable summaries rather than raw provider
argv, environment, paths, or stderr.

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

Every changed final is first written inside an atomically reserved, random
hidden workspace beside the destination. Fitifact records stable directory and
file identity (volume/file ID on Windows, device/inode on Unix) and holds
protective handles. Unix creates the workspace atomically with mode `0700`;
Windows deletes owned objects by handle disposition rather than by pathname.
A stage is fully hashed, inspected, and validated before a same-filesystem
hard-link create-if-absent publishes it. Hard-link creation is the last fallible
publication operation. The final link is immediately identity-checked against
the still-held stage; an unconfirmed final is preserved and reported with its
path, never rolled back. Cleanup failure similarly retains the published final
and emits a structured warning. If the atomic no-clobber primitive or stable
identity is unavailable, execution refuses.

Portable filesystem APIs cannot defend against a malicious process running as
the same OS account and controlling the same directory. Fitifact therefore
trusts other same-account processes. The random private workspace, atomic Unix
permissions, Windows protected handles, and identity checks defend against
accidental collisions and untrusted path replacement outside that boundary.

Remux accepts only the planner-proven MOV/H.264/optional-AAC source and uses
`-map 0 -c copy`. Selective transcode accepts only the planner-proven
MP4/HEVC/optional-AAC source with one video stream, maps those exact streams,
encodes video with `libx264`, and copies audio. Provider entry points
defensively reject source-container, plan-version, target, expected-fact,
preservation, or topology forgeries.

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
