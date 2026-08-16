---
title: "Performance Budgets"
type: engineering
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - performance
  - budgets
  - engineering
---
# Performance budgets

## Status

Design targets, not marketing promises.

“Heavily optimized” must eventually be enforced with measurements.

## Budget categories

Track:
- core cold start;
- idle footprint;
- initial web bundle;
- extension package size;
- planner latency;
- full-file passes;
- peak RAM;
- provider startup time;
- cloud transfer volume.

## Core

Requirements (v0.1, measured — not marketing SLAs):
- no network during core initialization;
- no GUI dependency;
- `check/plan` does not spawn transform providers (only `ffprobe` for inspect);
- no-op path does not initialize encoders or construct the FFmpeg transform provider.

`fitifact bench` records these proofs on the canonical fixtures. Wall-clock
numbers are observational; CI asserts outcomes and spawn proofs, not budgets.

## Web

Rule:

> **The transcoder is not part of initial page load.**

Measure:
- compressed transfer;
- parsed JS;
- time to interactive;
- memory after file selection.

## Extension

Requirements:
- small UI/content logic;
- no heavy media runtime by default;
- no persistent background CPU;
- minimal permissions.

## Native

Measure:
- core binary size without optional packs;
- optional provider pack sizes;
- cold start;
- idle RSS;
- peak inspection RAM;
- peak transform RAM.

## File I/O

Track full-file passes and temporary copies.

A streaming plan should not create multiple giant copies without a documented reason.

## Planner

Planner latency should stay negligible relative to file inspection and transformation, even with a large capability catalog.

## Regression policy

Benchmark CI should flag:
- major core cold-start regression;
- major initial web bundle growth;
- accidental provider load on no-op;
- new redundant file copy;
- significant memory regressions.

Initial numeric thresholds should be established from baseline measurements rather than guessed.

## Optimization hierarchy

1. eliminate work;
2. lazy-load capability;
3. reduce network;
4. reduce copies;
5. optimize algorithms;
6. micro-optimize after profiling.

## User-perceived metrics

Primary:
> time from incompatible file selection to validated compatible output.

Secondary:
> time to determine that an already-compatible file needs no change.

The no-op path should be exceptionally fast.
