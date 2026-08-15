---
title: "SaaS Business Model"
type: business
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - saas
  - business
---

# SaaS business model

## Thesis

Consumers make Fitifact understandable. Developers and enterprises can make it economically significant.

## Ladder

1. Free OSS
2. Free web/local
3. Managed cloud
4. Developer API
5. Teams
6. Enterprise/private workers

## Segments

### Consumer
Occasional rejection, low subscription willingness.

### Prosumer/creator
Frequent media adaptation; possible compute spend.

### SaaS developer
Strong early commercial target: user uploads and normalization.

### Enterprise
Internal policies, migration, private execution and audit.

## API value

Commodity conversion API:
`file -> specified format`

Fitifact:
`file + policy -> validated compliant file`

Value includes:
- inspection;
- planning;
- transformation;
- retries;
- validation;
- explanation.

## Cost centers

- ingress/egress;
- storage;
- CPU/GPU;
- provider licensing if any;
- registry verification;
- support;
- abuse.

## Pricing model direction

Do not price every request equally. A tiny image and multi-GB video differ dramatically.

Possible internal meter:
- bytes;
- compute time;
- accelerator class;
- transform complexity.

Package externally into predictable units.

## Consumer

Prefer free local + optional one-off cloud credits over forced subscription.

## Developer

Monthly included units + overage, concurrency tiers, webhooks and verified registry.

## Enterprise

Annual/custom:
- private workers;
- SSO;
- audit;
- region;
- support/SLA;
- custom profiles.

## Moats

Weak:
- UI;
- format count;
- FFmpeg wrapper.

Stronger:
- verified registry;
- planner quality;
- integrations;
- developer standard;
- provider ecosystem;
- enterprise trust.

## Abuse

Cloud needs:
- auth/rate limits;
- payload expiry;
- no permanent public hosting by default;
- per-job compute budgets;
- egress controls.

## Plan-only and metadata-first API

A strong acquisition strategy is to make inspection/check/planning free or extremely cheap.

Clients can inspect locally, send structured metadata, and request a plan without uploading the payload.

Charge primarily when Fitifact Cloud executes heavyweight transformations.

This lets developers integrate deeply before incurring meaningful spend and reduces Fitifact bandwidth/compute costs.
