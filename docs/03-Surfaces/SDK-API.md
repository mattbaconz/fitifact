---
title: "SDK and API Surface"
type: surface
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - sdk
  - api
  - surface
---

# SDK and API surface

## Local SDK

Convenience:
```ts
const result = await adapt(file, constraints);
```

Transparent flow:
```ts
const artifact = await inspect(file);
const check = checkCompatibility(artifact, constraints);
const plan = planAdaptation(artifact, constraints, preferences);
const result = await execute(file, plan);
```

## Hosted API

Potential:
- inspections;
- plans;
- jobs;
- profile resolution;
- convenience adapt.

## Why separate plan

Developers may need:
- consent for lossy work;
- cost preview;
- policy review;
- local/cloud choice.

## Upload

Prefer presigned object uploads for large payloads.

## Webhooks
Signed:
- job.started;
- job.completed;
- job.failed;
- job.validation_failed.

## SDKs

Keep thin. Compatibility semantics live in shared core, not reimplemented per language.

## Killer uploader

Long term:
```text
invalid upload
-> show plan
-> repair automatically
-> upload validated output
```

## Modular SDK packaging

A developer using only `inspect/check/plan` should not have to ship a media transcoder.

Conceptual package split:

```text
schema/core
runtime-wasm
runtime-native
cloud-client
optional uploader/UI
```

Plan/check remain useful when no execution provider is installed.
