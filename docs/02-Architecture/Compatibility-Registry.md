---
title: "Compatibility Registry"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - registry
  - architecture
  - moat
---

# Compatibility registry

## Purpose

Turn a destination identifier into evidence-backed constraints.

Example IDs:
```text
app/platform/feature
generic/browser/video
private/company/avatar
```

## Profile contents

- identity;
- scope;
- constraints;
- provenance;
- revision;
- verification date;
- trust level;
- test fixtures;
- quirks.

## Trust levels

- community;
- documented;
- tested;
- verified.

Use explicit labels, not an opaque score.

## Scope must be precise

App, platform, version range, region/account tier if relevant, feature/endpoint.

Do not flatten macOS/web/windows into one profile without evidence.

## Provenance

Each constraint can cite a different source.

## Freshness

Profiles can expire based on volatility.

## Testing

Fixtures:
- known valid;
- known invalid;
- boundaries;
- codec/dimension variants.

Real destination acceptance tests must respect terms and rate limits.

## Distribution

Open registry:
- Git/PR;
- sources/tests;
- signed releases.

Managed verified registry:
- monitoring;
- historical versions;
- stronger freshness;
- SLA/private profiles.

## Local overrides

Users can extend/override without silently changing upstream.

## Long-term asset

A structured “Can I Use for files” dataset may become independently valuable.
