---
title: "Product Requirements"
type: prd
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - prd
  - product
---

# Product requirements

P0 describes the product contract; v0.1 implements only the CLI/media subset in
[[04-Engineering/MVP-Scope]]. P1 and P2 are deferred.

## PRD objective

Define what must be true for Fitifact to deserve the phrase **file adapter** rather than “smart converter.”

## P0 requirements

### P0-1 Actual inspection
The system must identify real file internals independently of filename extension.

### P0-2 Structured constraints
The target must compile into machine-readable hard constraints and preferences.

### P0-3 Compatibility check
The system must identify which target constraints pass, fail or remain unknown.

### P0-4 Minimum-mutation planning
The planner must prefer a less-destructive valid plan over a broader/lossier valid plan.

### P0-5 No-op
Already-compatible files must not be converted.

### P0-6 Selective transformation
For media, valid streams should remain untouched when the target only requires another stream to change.

### P0-7 Post-validation
Every generated output must be re-inspected and checked against the same hard constraints.

### P0-8 Explanation
User must see why original failed and what changed.

### P0-9 Original safety
Original file remains unchanged by default.

### P0-10 Local transparency
UI must accurately disclose local/native/cloud execution.

## P1 requirements — deferred

- pasted requirement/rejection compiler;
- destination profiles;
- confidence/provenance UI;
- size-target fitting;
- CLI JSON;
- native companion;
- browser extension.

## P2 requirements — deferred

- managed API;
- verified registry;
- mobile integration;
- private profiles;
- batch;
- enterprise workers.

## Non-functional

### Security
Untrusted input sandboxing and resource ceilings.

### Privacy
Local-first and explicit cloud.

### Performance
No-op/check must feel instant for ordinary files.

### Reliability
A produced output that fails validation is not success.

### Explainability
Every transform step has a reason linked to a constraint/preference.

## Consumer acceptance test

Give a tester a file rejected for a codec/size reason. Without teaching them codecs, they should be able to:
1. understand the mismatch;
2. approve an adaptation;
3. receive a validated file.

## Developer acceptance test

A developer can define a target entirely through structured constraints and receive deterministic check/plan semantics without selecting an output format.
