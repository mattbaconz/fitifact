---
title: "Planner and Scoring"
type: architecture
status: active
implementation: implemented-v0.1
updated: 2026-08-16
canonical: true
tags:
  - planner
  - scoring
---

# Planner and scoring

## Objective

Find a plan that satisfies every hard constraint using only proven operations,
then changes as little as possible. Unknown facts are never pass, and a changed
or uncertain fact is never predicted to remain compatible.

## v0.1 search and ranking

Decision D-022 defines breadth-first search over the bounded capability catalog
to maximum depth 2. Feasible candidates rank lexicographically by:

1. semantic loss;
2. lossy steps;
3. streams changed;
4. step count.

This is not a weighted score or Pareto frontier. Pareto ranking remains deferred
until the catalog contains enough proven alternatives to justify it.

The only v0.1 edges are lossless MP4 remux and HEVC-to-H.264 video transcode to
MP4 with already-valid AAC copied. A compatible MP4/H.264/AAC input is a no-op;
MOV/H.264/AAC selects remux; MP4/HEVC/AAC selects one video-transcode step.

## Feasibility and refusal

Before search, the planner rejects targets or inputs requiring non-MP4 output,
audio transcode, resizing, size fitting, semantic/HDR conversion,
greater-than-8-bit conversion, unsupported codecs/containers, or unsafe stream
topology. A passing size constraint becomes uncertain after remux or transcode,
so a mutation with any size limit is also refused.

`cannot_satisfy` returns stable machine-readable blocking codes, related hard
constraint IDs, and readable messages. It never emits a speculative plan.

## Explainability

Serialized plans are provider-neutral. Typed steps carry targets, linked
reasons, proven expected facts, preservation claims, and warnings. Provider
names, commands, shell strings, and argv belong only to execution.
