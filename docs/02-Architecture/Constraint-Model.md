---
title: "Constraint Model"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - constraints
  - architecture
---

# Constraint model

## Purpose

Destinations care about properties, not conversion labels.

```text
container ∈ {mp4}
video.codec ∈ {h264}
file.bytes <= 25_000_000
width <= 1920
```

## Types

- equality;
- set membership;
- numeric bounds;
- presence/absence;
- ratios;
- relational constraints;
- conditional groups;
- alternatives.

## Hard vs soft

Hard:
`max_bytes <= 5_000_000`

Soft preference:
`preserve resolution strongly`

Hard determines success. Preferences rank plans.

## Unknown

Checks return:
- pass;
- fail;
- unknown.

## Provenance

Each constraint stores:
- source kind;
- URI if any;
- observed timestamp;
- confidence;
- evidence span when extracted from text.

## Merge

Never silently relax a hard constraint.

Exact page requirements can be narrower than a general profile. Conflicts are retained in report.

## Preferences

Examples:
- preserve quality;
- preserve resolution;
- preserve frame rate;
- preserve audio;
- preserve metadata;
- prefer local execution;
- minimize compute;
- prefer open codec.

## Bundles

Named destination profiles compile into the same constraint primitives as API/user constraints. No hidden platform-specific planner path.

## Anti-pattern

`"PowerPoint compatible"` is a profile identifier, not a constraint.
