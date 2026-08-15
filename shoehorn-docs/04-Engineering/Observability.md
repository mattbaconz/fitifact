---
title: "Observability"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - observability
---

# Observability

## Principle

Observe operations without surveilling file contents.

## Core metrics
- inspection duration;
- plan duration;
- candidate count;
- no-op rate;
- transform class;
- validation pass/fail.

## Runtime
- CPU;
- peak memory;
- bytes in/out;
- queue time;
- retries.

## Product
- constraints supplied;
- mismatch found;
- plan accepted;
- adaptation validated;
- accepted-after-adaptation feedback;
- local/cloud choice.

## Registry
- profile freshness;
- failing tests;
- source changes;
- rejection reports.

## Logs

Structured:
`job_id, provider, version, step, status, duration, error_code`

No raw filename/content by default.

## Debug bundle

Explicit opt-in export:
- sanitized inspection;
- constraints;
- plan;
- provider versions;
- logs.

Never include original payload automatically.
