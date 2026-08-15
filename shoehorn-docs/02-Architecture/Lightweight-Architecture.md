---
title: "Lightweight Architecture"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - lightweight
  - performance
  - architecture
---
# Lightweight architecture

## Product requirement

Shoehorn should be **small, dormant when unused, and lazy about loading heavy capability**.

This reinforces the same philosophy as minimum-mutation file adaptation:

> **Minimum mutation of files. Minimum footprint on the computer.**

## Desired shape

```text
tiny schema/core
      +
lazy capability providers
      +
thin integrations
```

Not:

```text
huge app runtime
+ every codec
+ every document engine
+ permanent daemon
```

## Four-layer model

```text
┌─────────────────────────────────────┐
│ 1. Schema                           │
│ constraints / plans / profiles      │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 2. Core                             │
│ check / planner / validation logic  │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 3. Providers                        │
│ media / image / PDF / OS codecs     │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│ 4. Integrations                     │
│ web / CLI / ext / OS / mobile / API │
└─────────────────────────────────────┘
```

The core must not know Chrome, Android, Discord, PowerPoint, or any specific integration UI exists.

## Lazy capability loading

Rules:

1. Do not initialize an encoder before a plan proves encoding is necessary.
2. Image-only work does not load the media runtime.
3. PDF/document providers are loaded only for those families.
4. No-op requires inspection + checking only.
5. Remux does not initialize lossy encoders.
6. Browser media WASM is lazy-loaded only when a selected plan needs it.
7. NLP/model parsing is optional and lazy.
8. Destination registry data is loaded incrementally where possible.

## Modular providers

Potential native capability packs:

```text
core
media
images
pdf
documents
```

The user still sees one product.

A provider can be:
- bundled;
- system-provided;
- installed on demand;
- cloud-only.

## Use the operating system

Where exact semantics and validation are sufficient, support platform providers:

```text
Windows     Media Foundation
macOS/iOS   VideoToolbox / platform image APIs
Android     MediaCodec
Fallback    FFmpeg / libvips / other mature providers
```

Shoehorn does not need to own the encoder. It needs a provider capable of producing the target state.

## No mandatory idle daemon

Preferred:
- no resident process while idle;
- no background scanning;
- native helper starts on explicit invocation;
- browser native host starts on demand.

A future persistent service must justify:
- memory;
- wakeups;
- battery;
- privacy;
- user value.

## File I/O

For large artifacts:
- stream when possible;
- avoid full buffering;
- avoid redundant copies;
- pass file handles rather than byte arrays;
- use temporary files only when required;
- clean intermediates aggressively.

## Fingerprints

Do not full-hash a 20 GB file only to cache inspection metadata unless cryptographic identity is required.

Use explicit identity levels:
- weak local cache fingerprint;
- strong cryptographic content hash for integrity/idempotency.

Never present a weak fingerprint as a strong hash.

## Core startup goals

Measure rather than market prematurely, but design for:
- millisecond-scale core initialization;
- no network dependency;
- no GUI dependency;
- no provider startup until required.

## Browser bundle

Initial:
- UI;
- schema;
- planner/checker;
- lightweight inspection.

Lazy:
- image runtime;
- media runtime;
- PDF runtime;
- natural-language parser;
- extra registry data.

The homepage must not download a transcoder merely to display a drop zone.

## Desktop UI

Avoid Electron by default.

Prefer:
- native shell;
- Tauri/system-webview style shell;
- or platform-native UI.

Shoehorn's UI is too small to justify a permanent bundled browser runtime without measured benefits.

## Optimization hierarchy

1. avoid unnecessary transformation;
2. avoid unnecessary provider loading;
3. avoid unnecessary network;
4. avoid unnecessary file copies;
5. improve data structures/algorithms;
6. micro-optimize only after profiling.

## Product-aligned messaging

After benchmarks exist, truthful claims could include:

> Shoehorn only loads the parts it needs.

> If your file already fits, Shoehorn changes nothing.

> If a local provider can do the job, your file stays on your device.

Do not publish hard binary/RAM numbers before reproducible measurement.
