---
title: "Shoehorn Context Pack"
type: agent-context
status: active
updated: 2026-08-15
canonical: true
tags:
  - agent-context
---

# Shoehorn context pack

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

## MVP
Images + common media.

## Demo
MP4 container contains HEVC; target requires H.264. Change video only, preserve audio, validate.

## FOSS
Open engine, paid managed operation.

## Brand
Shoehorn is a codename with current software naming collisions.

## Read next
- [[AGENTS]]
- [[00-Foundation/Decision-Log]]
- [[02-Architecture/System-Architecture]]
- [[04-Engineering/MVP-Scope]]

## Lightweight architecture

Shoehorn is **tiny compatibility plumbing**:
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

