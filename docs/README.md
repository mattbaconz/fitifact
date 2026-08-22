---
title: "Fitifact documentation"
type: root-index
status: active
updated: 2026-08-22
canonical: true
tags:
  - fitifact
  - index
---

# Fitifact documentation

Fitifact is a destination-first file compatibility engine: inspect the real
file, apply typed constraints, choose the minimum supported mutation, then
validate the output.

## How to read implementation status

`status: active` means the document is maintained; it does **not** mean every
feature it discusses exists. Documents with `implementation: deferred` describe
future work. In a document marked `implementation: mixed`, only a section that
explicitly says **current** or **implemented in v0.1** is an implementation
claim; all other feature designs are deferred. The repository-root
[`AGENTS.md`](../AGENTS.md) and the decision log take precedence over design
examples.

## Current public candidate: v0.1 CLI/media + local image workflow

Implemented:

- Rust `fitifact` library and `fitifact` CLI;
- local inspection through system `ffprobe`;
- typed YAML or flag constraints, compatibility checks, bounded planning, and
  machine-readable reports;
- MP4/H.264/AAC no-op;
- MOV/H.264/AAC remux to MP4;
- MP4/HEVC-to-H.264 video transcode while copying compatible AAC audio;
- post-execution re-inspection and validation;
- deterministic JPEG/PNG requirement parsing, bounded adaptation, explicit
  crop consent, and output validation in the local static web product;
- default-off gated HEIC decoding with owned fixture and license notices;
- GitHub-only distribution, Apache-2.0, no telemetry or network activity.

The CLI can **check** file-size and video-dimension constraints, but its v0.1
provider cannot execute size fitting or resizing. Requests requiring unsupported
mutations are refused. Originals and existing outputs are never overwritten,
streams are never silently discarded, and provider success alone never counts
as compatibility.

## Implemented static consumer surface

The D-026 Vite/React worker product and WASM bridge are implemented and
deployed at `https://mattbaconz.github.io/fitifact/`. Processing stays on the
device. Hosted processing, profiles, browser extensions, desktop/mobile shells,
registry publication, signing, and package-manager distribution remain
deferred.

## Open-core and private operations boundary

This public repository contains the Apache-2.0 core, schemas, planner, local
provider framework, CLI, tests, fixtures, and public documentation. Any managed
cloud execution, control plane, credentials, infrastructure, metering, private
profiles, continuous verification operations, or enterprise services are
deferred and belong in a separate private checkout. No private cloud checkout
exists as part of v0.1.

## Start here

- [[00-Foundation/Decision-Log]]
- [[00-Foundation/Product-Principles]]
- [[01-Product/Product-Definition]]
- [[04-Engineering/MVP-Scope]]
- [[03-Surfaces/CLI]]
- [[07-Specs/CLI-Spec]]
- [[04-Engineering/Roadmap]]

The consumer launch description is **“Make your image pass the upload.”** Owner
directed public GitHub on 2026-08-18 and RC4 release on 2026-08-22; the name is
still not billed-cleared. GitHub Release publication stays protected by its
one-window gate. See [[01-Product/Naming-Brand]].
