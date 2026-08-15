---
title: "MVP Scope"
type: engineering
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - mvp
  - engineering
  - canonical
---

# MVP scope

This document separates the implemented **v0.1 CLI/media release** from the
broader **later public MVP**. “Public MVP” elsewhere in the vault does not mean
the feature exists in v0.1.

## v0.1 CLI/media slice — implemented

v0.1 proves real inspection, typed destination constraints, compatibility
diagnosis, minimum-mutation planning, safe local execution, post-validation,
human explanation, and structured JSON reports.

Executable adaptation behavior is limited to:

1. MP4/H.264/AAC that already satisfies the target: no-op without starting an
   encoder;
2. acceptable media streams in the wrong container: remux without re-encoding;
3. HEVC video targeting H.264 with compatible AAC audio: transcode video and
   copy audio;
4. all other requested mutations: explicit refusal.

The CLI accepts file-size and video-dimension constraints for inspection and
compatibility checking. They are **check-only**: v0.1 cannot resize video, change
frame rate, or fit a byte target. A plan must never pretend otherwise.

v0.1 uses system FFmpeg/ffprobe, is distributed only through GitHub, performs no
telemetry or network activity, never overwrites originals or existing outputs,
never silently discards streams, and validates every generated output.

## Later public MVP — deferred

The broader public MVP may add a one-click web experience and common image
adaptation (JPEG, PNG, WebP, HEIC/HEIF where viable, and TIFF). Potential image
constraints include bytes, dimensions, aspect ratio, alpha, and animation.
Those surfaces, formats, transforms, providers, and packaging are not
implemented in v0.1.

A few sourced destination profiles, profile registry workflows, browser-local
processing, and richer explanation UI are also deferred. Natural-language
requirements parsing is not part of v0.1.

## Public and private boundary

The public Apache-2.0 repository owns the engine, schemas, planner, local
provider framework, CLI, tests, fixtures, and public docs. Managed cloud/API
execution, private profiles, credentials, infrastructure, metering, continuous
verification operations, and enterprise controls are deferred to a separate
private checkout. They must not be added here or presented as available.

## Acceptance criteria

### v0.1

- detect codec inside the real container;
- no-op when valid without provider execution;
- preserve compatible streams;
- choose remux before lossy transcode when sufficient;
- refuse unsupported mutations;
- never overwrite originals or existing outputs;
- post-validate against the same hard constraints;
- return deterministic structured results;
- work without telemetry, network calls, or implicit cloud upload.

### Later public MVP (deferred)

- nontechnical one-click web flow;
- common image compatibility workflow;
- browser initial path without eager heavyweight media runtime;
- image work without initializing the media provider.

## Anti-scope rule

A new format or surface is not enough reason to expand v0.1. Any expansion
requires an explicit decision that updates D-019 or D-020.
