---
title: "Integration Strategy"
type: surface
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - integrations
  - surfaces
  - architecture
---
# Integration strategy

## Core principle

> **One tiny compatibility core. Extremely thin integrations around it.**

Fitifact must not become six separate products with six separate compatibility implementations.

The browser extension, OS menu, mobile share target, CLI, web app, SDK, and cloud API should all reuse the same logical engine.

```text
Windows context menu ──────┐
macOS Quick Action ────────┤
Browser extension ─────────┤
Android share target ──────┤
iOS Share Extension ───────┼──> Fitifact Core
CLI ───────────────────────┤
Web SDK ───────────────────┤
REST API ──────────────────┘
```

An integration should mostly do:

```text
1. acquire file handle
2. acquire destination / constraints
3. call core / runtime
4. present result
```

Compatibility policy must not be duplicated inside integrations.

## Layering

```text
                     FITIFACT CORE
                         │
      inspect -> constraints -> plan -> validate
                         │
                  Runtime Adapter
                         │
        ┌────────────────┼─────────────────┐
        │                │                 │
      Native           WASM             Cloud
        │                │                 │
        ▼                ▼                 ▼
   OS codecs         browser-safe       workers
   FFmpeg/libvips    operations
```

## Thin-client rule

An integration must not:
- hard-code that a destination needs a specific codec;
- know FFmpeg/ImageMagick command syntax;
- duplicate the planner;
- duplicate inspection logic;
- bundle every transform provider by default;
- silently upload files.

Instead:
- destination requirements are profile/constraint data;
- transforms are provider capabilities;
- integrations are transport + UI.

## Windows

Preferred:
- context-menu action invokes a tiny bridge;
- bridge launches or messages the native Fitifact process;
- transformation occurs outside Explorer.

Never load heavy parsers/codecs directly inside Explorer.

## macOS

Preferred:
- Finder Quick Action / Services / Share integration;
- small bridge invokes native Fitifact;
- heavy processing happens in a dedicated process.

## Linux

Preferred:
- CLI as reference integration;
- Nautilus/Dolphin actions call the same CLI/runtime;
- no desktop-environment-specific compatibility policy.

## Browser extension

The extension should be an extremely small:
- DOM evidence collector;
- UX surface;
- Fitifact IPC/client.

Heavy local flow:

```text
extension
  ↓ Native Messaging
native Fitifact host
  ↓
core + provider
```

Do not bundle a large media transcoder into the base extension unless later measurements justify it.

## Android

Primary flow:

```text
Share -> Fitifact -> target -> adapt -> share onward
```

Receive a scoped content URI, call the same compatibility core, execute with platform/native provider, return a scoped output URI.

## iOS/iPadOS

Primary flow:

```text
Share Extension -> inspect/plan
               -> small local transform
               -> main app handoff for heavy work
```

Do not promise universal system interception.

## CLI as reference integration

The CLI is the best architecture purity test.

Given the same:
- file state;
- constraints;
- preferences;
- provider capabilities;

CLI, web, extension, and mobile should produce equivalent plans.

If they do not, compatibility logic has leaked into an integration.

## SDK

SDKs should be modular.

A developer who only wants `inspect/check/plan` should not have to ship an encoder.

Conceptual packages:

```text
schema
core
runtime-wasm
runtime-native
cloud-client
uploader-ui
```

Exact names depend on the final brand.

## Metadata-first cloud planning

When the client can inspect locally:

```text
local inspection
  ↓
structured metadata only
  ↓
cloud plan
  ↓
local execute
```

The file never needs to reach Fitifact Cloud unless managed execution is explicitly chosen.

## Integration acceptance criteria

A new integration is accepted only if:
- no destination-specific compatibility logic lives in it;
- it consumes the canonical schemas;
- it can show no-op;
- it accurately reports processing location;
- it surfaces validation failures;
- source file is preserved by default;
- permissions are minimal;
- heavy capability is lazy-loaded.
