---
title: "Release Strategy"
type: engineering
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - release
---

# Release strategy

## Versioning

Semantic version core/CLI. Profile revisions version separately.

## Channels
- stable;
- beta;
- nightly.

The first prepared public sequence is narrower: `v0.1.0-rc.1` acceptance first,
then immutable `v0.1.0`. See [[04-Engineering/Release-Checklist]].

## Before 1.0
Schemas may evolve with migration notes.

## After 1.0
Document API/schema deprecation windows.

## Build integrity
Aim for:
- pinned dependencies;
- checksums;
- SBOM;
- signed releases;
- reproducible builds where feasible.

## Provider security
Provider security updates can justify release without product features.

## Profile releases
Must not require full binary release.

## FOSS launch checklist
- license;
- security policy;
- contribution guide;
- architecture;
- fixtures;
- issue templates;
- roadmap;
- naming decision;
- no secrets.

## Rollback
Cloud must support app/provider/profile rollback.

## Release blockers
- validator bypass;
- critical parser vulnerability;
- known false compatibility claim;
- accidental original overwrite.
- missing owner/legal Fitifact sign-off;
- failed checksum, SBOM, attestation, or source/tag provenance verification;
- a supported target that was not exercised on its native hosted runner.
