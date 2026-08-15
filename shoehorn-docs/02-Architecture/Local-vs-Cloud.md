---
title: "Local vs Cloud Architecture"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - local-first
  - cloud
---

# Local vs cloud architecture

## Principle

Local is trust baseline. Cloud is explicit speed/scale convenience.

## Local advantages
- privacy;
- offline;
- no upload latency;
- no operator bandwidth cost;
- FOSS credibility.

## Local disadvantages
- variable hardware;
- missing codecs;
- browser/mobile limits;
- support matrix.

## Cloud advantages
- predictable providers;
- CPU/GPU;
- huge files;
- queues/batch;
- APIs/webhooks.

## Cloud disadvantages
- privacy/compliance;
- storage/egress;
- abuse;
- legal/codec exposure.

## Decision

```text
if operation is practical locally:
    local default
else:
    offer native companion or explicit cloud
```

Never auto-upload because local is slow.

## Cloud worker

```text
API -> queue -> ephemeral worker -> scoped input
    -> transform -> validate -> output -> webhook
```

## Private enterprise worker

Shoehorn control plane can send signed jobs while files remain in customer storage/environment.

## Retention

Design short-lived input/output retention and explicit deletion before launch.

## Cryptography

- TLS;
- encryption at rest;
- short-lived signed URLs;
- integrity hashes.

## Parity

Local/cloud can produce different binary encodes, but both must satisfy the same semantic constraints and report provider differences.

## Plan before transfer

Where practical, inspect and plan locally before any cloud upload:

```text
local inspection
    ↓
artifact metadata + target constraints
    ↓
plan
    ↓
local execute OR explicit cloud upload
```

This reduces bandwidth, cloud cost, latency and privacy exposure.

Never require a multi-gigabyte upload just to discover the file was already compatible.
