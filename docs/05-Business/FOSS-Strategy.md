---
title: "FOSS Strategy"
type: business
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - foss
  - business
  - canonical
---

# FOSS strategy

## Strategy

**Open engine, paid operation.**

Fitifact should be genuinely useful without Fitifact Cloud.

## Open source

Implemented publicly in v0.1:
- artifact/inspection schemas;
- constraint engine;
- planner;
- provider framework;
- local execution;
- CLI;
- tests, fixtures, and public documentation.

Deferred public work:

- SDK core;
- community registry;
- browser/desktop integrations where practical;
- self-hosting.

## Why open the actual engine

Benefits:
- developer trust;
- distribution;
- contributions;
- ecosystem;
- easier embedding;
- FOSS credibility;
- avoids “marketing wrapper” perception.

## License

**Apache-2.0** applies to Fitifact-owned code in this public repository because
broad commercial embedding is strategically valuable.

Not a final legal conclusion. Review:
- bundled providers;
- linking boundaries;
- FFmpeg build/license;
- PDF/document engines;
- codec patents;
- app-store distribution.

## Private managed operations — deferred

No cloud implementation is present in this repository or in v0.1. If approved,
managed execution, infrastructure, credentials, metering, private profiles,
continuous verification operations, and enterprise control-plane code belong in
a separate private checkout. The public core must remain independently useful.

Potential managed offerings are design direction only:

## What a future cloud service could sell

### Managed adaptation
No setup.

### Heavy compute
Fast CPU/GPU workers.

### Verified registry
Maintained destination profiles.

### Scale
Queueing, retries, batch and concurrency.

### Enterprise
Private workers, SSO, audit, residency, SLA/support.

### Developer operations
Keys, usage, webhooks, teams and analytics.

## Do not paywall

Avoid:
- OSS limited to 3 formats;
- watermarks;
- arbitrary local file-size cap;
- closed planner;
- closed profile schema;
- no self-hosting.

## remove.bg lesson

remove.bg exposes open integration tooling around a commercial service rather than a fully self-hostable core engine.

Fitifact can go further:
- open the compatibility engine;
- monetize operation and verification.

## Registry split

Open:
- schema;
- community profiles;
- sources/tests.

Managed:
- freshness guarantees;
- continuous verification;
- historical tracking;
- private profiles;
- SLA.

## Flywheel

```text
YouTube/GitHub
 -> users
 -> profiles/contributors
 -> better compatibility
 -> developer embeds
 -> hosted API demand
 -> revenue
 -> better core
```

## Why lightweight helps monetization

A tiny local core makes the generous FOSS strategy economically stronger:
- more jobs stay local;
- cloud cost is reserved for users explicitly choosing managed compute;
- developers can embed check/plan without bloating their apps;
- local use feels genuinely superior, not intentionally crippled.

Cloud becomes a convenience/performance product, not a ransom for a bloated client.
