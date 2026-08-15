---
title: "Testing and QA"
type: engineering
status: active
implementation: mixed
updated: 2026-08-15
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
provider provenance, and SHA-256 manifest are committed with it. Generated-temp
provider tests remain in addition to these canonical fixtures.

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

v0.1 CI narrows that claim to native Windows x64, Linux GNU x64, macOS Intel,
and macOS Apple Silicon runners. Browser testing remains deferred.

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

Ask a nontechnical person to make a file work without explaining codecs first.

## Failure injection

Disk full, worker killed, provider missing, timeout, corrupt output, registry unavailable, cloud interruption.
