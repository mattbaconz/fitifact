---
title: "FOSS Strategy"
type: business
status: active
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

Shoehorn should be genuinely useful without Shoehorn Cloud.

## Open source

Proposed:
- artifact/inspection schemas;
- constraint engine;
- planner;
- provider framework;
- local execution;
- CLI;
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

## Proposed license

**Apache-2.0** is favored for Shoehorn-owned code because broad commercial embedding is strategically valuable.

Not a final legal conclusion. Review:
- bundled providers;
- linking boundaries;
- FFmpeg build/license;
- PDF/document engines;
- codec patents;
- app-store distribution.

## What cloud sells

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

Shoehorn can go further:
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
