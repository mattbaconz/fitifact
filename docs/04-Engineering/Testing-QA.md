---
title: "Testing and QA"
type: engineering
status: active
implementation: mixed
updated: 2026-08-21
canonical: true
tags:
  - testing
  - qa
---

# Testing and QA

## Layers

### Unit
Constraints, compatibility predicates, planner, profile resolution, schema validation.

### Fixture integration
Real tiny files with known properties.

The tracked canonical media set is under `fixtures/media`; its generator,
provider provenance, and SHA-256 manifest are committed with it. The image set
under `fixtures/image` contains owned synthetic JPEG, PNG, still WebP, transparent PNG,
crop-grid, malformed, decoded-limit, and single-image HEIC fixtures with a
generator, provenance, and SHA-256 manifest. Generated-temp provider tests
remain in addition to these canonical fixtures.

### Provider integration
Execute actual providers.

### End-to-end
Input -> output -> validation.

### Destination acceptance
Only where legally/operationally permitted.

## Golden fixtures

Generate legal fixtures for:
- H.264/AAC MP4;
- HEVC/AAC MP4;
- MOV;
- transparent image;
- animated image;
- oversized dimensions;
- malformed/truncated input.

## Planner properties

- valid input yields no-op if no preference requires changes;
- hard constraints can never be ignored;
- unnecessary lossy edge cannot outrank equivalent lossless edge;
- adding stricter constraints cannot turn invalid into compatible.

## Boundary tests

- exact max bytes;
- one byte over;
- dimension boundaries;
- aspect rounding;
- rational FPS;
- zero/huge duration;
- giant dimensions;
- corrupt metadata.

## Security fuzzing

Fuzz parsers, schema, profile parser, text normalization and archive readers if added.

## Cross-platform

Windows, macOS, Linux, browsers, ARM/x64 where relevant.

CI covers native Windows x64, Linux GNU x64, macOS Intel, and macOS Apple
Silicon, plus Chromium, Firefox, and WebKit at desktop and mobile widths for the
static image product. Default builds decode the owned HEIC fixture lazily;
`FITIFACT_HEIC_APPROVED=false` keeps the decoder absent.

## Performance regressions

Track inspect latency, plan latency, memory, throughput, startup size.

## Provider update regression

Replay fixtures and compare:
- inspection;
- plan;
- output validation.

## Profile PR requirements

Source + schema validation + valid/invalid/boundary fixtures where possible.

## Human QA

The exact ten-task, real-destination protocol and continuation scorecard are in
[[04-Engineering/Consumer-Image-Moderated-Test]]. Its 8/10, 8/10, 5/10, and
zero-harm gates are post-build human validation. No engineering test can fill
or pass that scorecard.

## Failure injection

Disk full, worker killed, provider missing, timeout, corrupt output, registry unavailable, cloud interruption.
