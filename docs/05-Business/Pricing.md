---
title: "Pricing Strategy"
type: business
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - pricing
  - business
---

# Pricing strategy

## Status
Exploratory. No public numbers until cost benchmarking.

## Principles

- local OSS free;
- local web free;
- cloud tracks resource use;
- spend predictable;
- hard budget caps;
- no download ransom.

## Potential tiers

### Free
Local engine, CLI, community profiles, browser/local adaptation, trial cloud credits.

### Developer
API, included units, standard concurrency, webhooks, verified public profiles.

### Scale
Batch, higher concurrency, priority, analytics, configurable retention.

### Enterprise
Private workers, SSO, audit, custom region, support/SLA.

## Adaptation units

Possible internal model:
`base + bytes + compute seconds + accelerator factor`

Externally simplify.

## Cost preview

Plan endpoint can return:
- estimated cloud units;
- expected latency;
- local alternative.

## Hard spend controls
- monthly cap;
- per-job max;
- alerts;
- reject-on-cap.

## Registry

Public verified profiles may stay free for ecosystem adoption.

Charge for:
- SLA;
- private profiles;
- continuous verification;
- change notifications.

## Research needed

Refresh competitor pricing before launch:
CloudConvert, ConvertAPI, Transloadit, Uploadcare, Filestack, Cloudinary and raw compute/egress.
