---
title: "Validation"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - validation
  - architecture
---

# Validation

## Principle

Validation is the difference between **conversion completed** and **compatibility achieved**.

## Pipeline

```text
output bytes
   ↓
fresh inspection
   ↓
same hard ConstraintSet
   ↓
pass/fail/unknown
```

Do not reuse predicted output state as validation.

## Validation layers

### Structural
Output can be parsed and has expected family/container.

### Constraint
Every hard constraint evaluates pass.

### Integrity
Basic checks:
- non-zero output;
- expected stream/page/frame presence;
- duration/content sanity where applicable.

### Semantic preservation
When a plan claims preservation:
- preserved stream hashes where remux can prove identity;
- page count;
- dimensions;
- duration tolerance;
- audio presence;
- animation count.

## Exact preservation

Where possible, prove:
- copied audio stream unchanged;
- remuxed elementary stream unchanged.

This is stronger than saying “quality preserved.”

## Tolerances

Some fields are approximate:
- duration rounding;
- frame rate;
- bitrate;
- target byte estimate.

Hard destination bounds still remain exact when required.

## Validation unknown

If a target constraint relies on a fact the inspector cannot verify:
- output cannot be labeled fully verified;
- report `unknown`;
- profile confidence/UI must reflect it.

## Destination acceptance

Local validation proves the file satisfies **known constraints**, not that a third-party server will definitely accept it if hidden rules exist.

Wording:
> “Matches the known requirements.”

Use “Accepted” only when actual destination confirms.

## Validation report

For every constraint:
- expected;
- actual;
- result;
- source.

## Retry

Only adaptation strategies designed for convergence, such as size targeting, should automatically retry after validation near-miss.

## Regression

Validator behavior must be versioned because changing inspection semantics can change compatibility outcomes.
