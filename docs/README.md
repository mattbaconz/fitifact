---
title: "Fitifact documentation"
type: root-index
status: active
updated: 2026-08-18
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

## Current public release: v0.1 CLI/media slice

Implemented:

- Rust `fitifact` library and `fitifact` CLI;
- local inspection through system `ffprobe`;
- typed YAML or flag constraints, compatibility checks, bounded planning, and
  machine-readable reports;
- MP4/H.264/AAC no-op;
- MOV/H.264/AAC remux to MP4;
- MP4/HEVC-to-H.264 video transcode while copying compatible AAC audio;
- post-execution re-inspection and validation;
- GitHub-only distribution, Apache-2.0, no telemetry or network activity.

The CLI can **check** file-size and video-dimension constraints, but its v0.1
provider cannot execute size fitting or resizing. Requests requiring unsupported
mutations are refused. Originals and existing outputs are never overwritten,
streams are never silently discarded, and provider success alone never counts
as compatibility.

## Later public MVP (deferred)

The broader public MVP described in product and design notes adds images and a
one-click web experience only after the CLI/media milestone is reliable.
Browser/desktop/mobile surfaces, profiles, natural-language parsing, WASM,
bundled providers, registry publication, signing, and package-manager
distribution are also deferred. Examples of those features are design sketches,
not working interfaces.

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

The strongest current launch description remains **“I Made an Adapter for
Files.”** Owner directed public GitHub on 2026-08-18; the name is still not
billed-cleared. GitHub Release publication stays gated. See
[[01-Product/Naming-Brand]].
