---
title: "Differentiation"
type: research
status: active
updated: 2026-08-15
canonical: true
tags:
  - differentiation
---

# Differentiation

## Wedge

> **Destination-first + minimum mutation + validation.**

All three are required.

## Versus generic converters

Generic:
> What should I turn this into?

Shoehorn:
> What must this satisfy?

## Versus presets

Preset = known recipe.

Profile = constraints.

Same target, different inputs:
- input A may need remux;
- B selective transcode;
- C no-op.

Shoehorn should not apply the same recipe blindly.

## Versus Smart Converter

Smart Converter is strong prior art for preserving streams that do not need conversion.

Shoehorn extends:
- arbitrary constraints;
- multiple families;
- profile registry;
- developer API;
- validation.

## Versus upload platforms

Uploadcare/Filestack/Transloadit execute declared pipelines.

Shoehorn can become a higher-level **compiler**:
`policy -> transform plan`.

It may even emit a pipeline to another processor instead of executing itself.

## Versus Android transcoding

Android proves OS-level destination-aware adaptation.

Shoehorn aims to be portable, explicit, cross-platform, broader and developer-addressable.

## Defensibility stack

Weak:
1. UI

Moderate:
2. planner
3. provider ecosystem

Stronger:
4. verified registry
5. browser/OS/upload integrations
6. developer standard/API
7. acceptance/freshness operations

## Litmus test

If a competitor can replace Shoehorn by adding “Convert to MP4,” differentiation failed.

If replacement requires constraints, inspection, planner, validation, registry and integrations, the category is real.
