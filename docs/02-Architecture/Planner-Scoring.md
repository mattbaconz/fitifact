---
title: "Planner and Scoring"
type: architecture
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - planner
  - scoring
---

# Planner and scoring

## Objective

Find a plan that:
1. satisfies every hard constraint;
2. respects pinned preservation;
3. minimizes damage and operational cost.

## Avoid one magic score first

Recommended:

### Feasibility filter
Reject paths violating hard constraints, security policy or provider capability.

### Pareto frontier
Compare:
- semantic loss;
- quality loss;
- compute;
- latency;
- output size;
- compatibility confidence;
- privacy/execution location.

### Preference ranking
Apply user/default policy.

## Default priorities

1. avoid semantic loss;
2. avoid lossy conversion;
3. preserve pinned properties;
4. maximize compatibility confidence;
5. minimize changed components;
6. preserve perceptual quality;
7. minimize compute/latency;
8. minimize bytes only when relevant.

## Example dominance

Input:
`MOV/H264/AAC/18MB`

Target:
`MP4/H264/AAC <=25MB`

A: remux only.  
B: full video+audio re-encode.

A dominates B.

## Size fitting

Try:
1. lossless reduction;
2. remove optional data only if allowed;
3. bitrate adjustment;
4. resize/fps only if allowed and necessary;
5. bounded retries.

## Explainability

Plan stores reasons:
- target requires h264;
- audio already valid;
- remux is lossless;
- current size exceeds target.

## Unsatisfiable

Return blocking constraints and minimal relaxations, e.g.:
> Cannot satisfy 1 MB while preserving 4K/current quality. Allow 1080p or a larger limit.
