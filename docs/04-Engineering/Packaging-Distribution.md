---
title: "Packaging and Distribution"
type: engineering
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - packaging
  - distribution
  - engineering
---
# Packaging and distribution

## Goal

Ship Fitifact as a small orchestrator with optional capability packs instead of one monolithic binary containing every transform engine.

## Distribution profiles

### Minimal CLI

Contains:
- schema/core;
- lightweight inspection;
- provider discovery.

Uses system or separately installed providers.

### Desktop Standard

Contains:
- core;
- thin UI;
- common image/media capabilities;
- OS-native adapters;
- portable fallback where justified.

### Browser

Contains:
- tiny initial JS;
- lazy WASM;
- optional native companion.

### Server

Contains:
- core;
- approved provider set;
- sandbox configuration;
- no consumer UI.

### Enterprise private worker

Contains only organization-approved providers.

## Provider packs

Conceptual:
- media;
- images;
- PDF;
- documents.

Do not expose package complexity to ordinary users. Capability can be installed/enabled on first use.

## Independent update domains

- core/planner;
- profiles;
- transform providers;
- UI/integrations.

A security update to a parser should not require a profile change.

## Native companion

The extension/web experience can delegate heavy local execution to a native host that:
- starts on demand;
- exposes narrow IPC;
- reports exact capabilities/version;
- does not remain resident without reason.

## Signed artifacts

Consumer trust requires:
- code signing/notarization where relevant;
- checksums;
- SBOM;
- release provenance.

## Installer transparency

Disclose:
- included providers;
- installed size;
- optional packs;
- update behavior.

## Provider changes during jobs

Do not switch provider versions in the middle of an active job.

After update:
- re-probe capabilities;
- new jobs use new version;
- reports record exact provider build.

## Naming

Fitifact is the selected executable name, but public publication is blocked
until owner/legal sign-off. v0.1 remains GitHub-only; crates.io, npm,
package-manager formulae, bundled FFmpeg, and OS signing are deferred.
