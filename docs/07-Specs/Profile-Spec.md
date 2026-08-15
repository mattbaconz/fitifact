---
title: "Compatibility Profile Specification"
type: spec
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - profile
  - spec
---

# Compatibility profile specification

Status: draft.

## Example

```yaml
schema: fitifact.profile/v1
id: example/video-upload
name: Example Video Upload

scope:
  kind: web-upload
  platform: example
  feature: video

revision: 3
last_verified: 2026-08-15

trust:
  level: documented

constraints:
  - id: container
    field: media.container
    op: in
    value: [mp4]
    source: src-format

  - id: codec
    field: media.video.codec
    op: in
    value: [h264]
    source: src-format

  - id: bytes
    field: file.bytes
    op: lte
    value: 25000000
    source: src-size

preferences:
  preserve:
    resolution: high
    audio: high

sources:
  - id: src-format
    type: official-doc
    url: https://example.invalid/docs/video
    observed_at: 2026-08-15

  - id: src-size
    type: page
    url: https://example.invalid/upload
    observed_at: 2026-08-15

tests:
  - fixture: media/h264-aac-1080p.mp4
    expect: compatible
  - fixture: media/hevc-aac-1080p.mp4
    expect: incompatible
```

## Required

- schema;
- id;
- name;
- revision;
- scope;
- constraints;
- source information for public/non-local claims.

## IDs

Stable slash IDs:
`vendor/product/platform/feature`.

Avoid volatile version in ID when revision/range is enough.

## Scope

May include:
- app;
- platform;
- version range;
- region;
- account tier;
- feature;
- endpoint.

## Sources

Types:
- official-doc;
- official-api;
- observed-page;
- acceptance-test;
- user-config;
- community.

## Trust

Derived by registry policy, not author self-rating.

## Tests

Local compatibility checks plus real acceptance evidence where permitted.

## Overrides

Local profiles can `extends` upstream and preserve provenance.

## Security

Profiles are declarative data:
- no executable commands;
- no scripts;
- no arbitrary provider flags.
