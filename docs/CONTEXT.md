---
title: "Fitifact Context Pack"
type: agent-context
status: active
updated: 2026-08-21
canonical: true
tags:
  - agent-context
---

# Fitifact context pack

Use this file when an agent needs minimal project context.

## Problem
Files are rejected because users do not know destination-specific constraints.

## Primitive
`adapt(file, constraints)`

## Pipeline
`inspect -> compile -> check -> plan -> execute -> validate`

## Differentiator
Destination-first + minimum mutation + validation.

## Non-negotiables
- no blind conversion;
- no extension-only inspection;
- no output without validation;
- no shell strings;
- no invented requirements;
- local-first;
- original preserved.

## Current v0.1
The frozen native media matrix remains MP4/H.264/AAC no-op,
MOV/H.264/AAC remux to MP4, or MP4/HEVC-to-H.264 video transcode while copying
compatible AAC audio. Media file-size and dimension constraints are check-only.

D-026 adds a static local consumer image product for parsed/edited JPEG/PNG
requirements, minimum-change image adaptation, crop consent, warnings, and
post-validation. HEIC decode is approved-gate only. **Your image stays on this
device.** Hosted/cloud execution remains absent.

## Human viability gate

The product is not commercially validated until ten real form/application
photo tasks meet the 8/10, 8/10, 5/10, and zero-harm continuation thresholds.

## Demo
MP4 container contains HEVC; target requires H.264. Change video only, preserve audio, validate.

## Repository boundary
Apache-2.0 public engine and CLI. Managed cloud/operations are deferred to a
separate private checkout.

## Brand
Fitifact had no material collision signal in the 2026-08-15 automated exact-name
checks, but legal review is pending and publication is blocked until sign-off.

## Read next
- [[AGENTS]]
- [[00-Foundation/Decision-Log]]
- [[02-Architecture/System-Architecture]]
- [[04-Engineering/MVP-Scope]]

## Lightweight architecture

Fitifact is **tiny compatibility plumbing**:
- small shared core;
- thin integrations;
- lazy providers;
- no mandatory idle daemon;
- no heavy browser codec runtime until a plan needs it;
- provider-independent planner.

Read:
- [[02-Architecture/Lightweight-Architecture]]
- [[03-Surfaces/Integration-Strategy]]
- [[04-Engineering/Performance-Budgets]]

