---
title: "Enterprise"
type: business
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - enterprise
---

# Enterprise

## Value

Companies already build validation/normalization pipelines. Fitifact can provide a declarative compatibility layer with audit and private execution.

## Use cases
- customer uploads;
- internal downstream-system compatibility;
- legacy migration;
- company media/document policies.

## Deployment

### Managed workers
Standard cloud.

### Dedicated region
Higher tier.

### Private worker
Control plane sends signed job; payload stays customer-side.

### Fully self-hosted
Possible under FOSS, with paid support/control-plane.

## Controls
- SSO/OIDC/SAML;
- RBAC;
- audit;
- API scopes;
- retention;
- allowed providers;
- max quality loss;
- custom profiles;
- local/cloud policy.

## Example policy

```text
Marketing image:
- JPEG/PNG/WebP
- <=10 MB
- max 6000×6000
- preserve color profile
- optionally strip GPS
```

GPS stripping is privacy policy, not compatibility; report reasons separately.

## Procurement

Eventually prepare:
- security whitepaper;
- DPA/subprocessors;
- data residency;
- incident process;
- relevant compliance.

Do not claim certifications before they exist.
