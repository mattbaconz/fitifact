---
title: "Desktop and OS Integration"
type: surface
status: active
implementation: deferred
updated: 2026-08-15
canonical: true
tags:
  - desktop
  - surface
---

# Desktop and OS integration

## Goal

Make `Adapt for...` a normal file operation.

## Windows
Potential:
- Explorer context action;
- Send To;
- drag/drop app;
- native companion.

## macOS
Potential:
- Finder Quick Action;
- Services;
- Share;
- Shortcuts;
- drag/drop.

## Linux
Potential:
- desktop actions;
- Nautilus/Dolphin integration;
- CLI;
- Flatpak.

## Native companion

Provides:
- fast local execution;
- bundled provider environment;
- bridge for extension;
- filesystem integration.

## Packaging

Bundling providers improves predictability but increases size and license obligations. Using system tools reduces bundle complexity but harms reproducibility.

## Output naming

Default sibling:
`original-adapted.ext`

Never overwrite by default.

## Trust

Signing/notarization and transparent provider versions are important.

## Offline

Inspection, custom constraints, local profiles, planning and execution should work without account/network when providers are installed.

## Process isolation

Shell integrations should invoke or communicate with a separate Fitifact process.

Do not load codecs, complex parsers or heavy runtime directly inside Explorer/Finder host processes.

The shell surface is a thin bridge:

```text
selected file -> action -> Fitifact
```
