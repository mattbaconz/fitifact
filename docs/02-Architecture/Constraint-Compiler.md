---
title: "Constraint Compiler"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - constraint-compiler
  - architecture
---

# Constraint compiler

## Inputs

- structured developer constraints;
- compatibility profile;
- user requirement text;
- rejection text;
- visible page hints;
- input `accept` hint.

## Pipeline

```text
normalize
  ↓
deterministic extractors
  ↓
typed candidate constraints
  ↓
schema validation
  ↓
conflict detection
  ↓
optional model-assisted leftovers
  ↓
evidence validation
  ↓
ConstraintSet
```

## Deterministic extraction

Handle common:
- byte limits;
- dimensions;
- aspect ratio;
- MIME/extensions;
- codec names;
- duration;
- fps;
- sample rate.

## Optional model-assisted parser

A model may help with messy prose, but:
1. output typed candidates only;
2. attach evidence spans;
3. never emit commands;
4. mark ambiguity;
5. pass deterministic schema validation;
6. never promote inference to verified fact.

## Ambiguity

“5 MB” may be decimal or binary. Target safely below the stricter interpretation when source is unclear.

“MP4” may mean extension/container and says nothing about codec.

“1080p” is shorthand; do not assume exact dimensions without context.

## Conflicts

Profile says 25 MB; current page says 10 MB:
- use current narrower page evidence for this operation;
- record conflict;
- flag registry freshness.

## Output

```text
ConstraintSet
- hard[]
- preferences[]
- unresolved[]
- conflicts[]
- provenance[]
- confidence_summary
```

If unresolved items materially affect plan, ask before execution.
