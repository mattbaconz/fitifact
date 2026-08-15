---
title: "Browser Extension"
type: surface
status: active
updated: 2026-08-15
canonical: true
tags:
  - browser-extension
  - surface
---

# Browser extension

## Promise

> **Fix this upload.**

Not universal interception.

## Evidence sources

With explicit user action:
- input `accept`;
- visible instructions;
- rejection text;
- known domain/profile;
- user-pasted error.

`accept` is a hint, not server policy.

## Permissions

Prefer narrow permissions and `activeTab`-style explicit access. Avoid permanent all-sites access unless necessary.

## Flow

```text
upload fails
-> user opens extension
-> use page requirements?
-> select file
-> diagnose
-> adapt
-> save/reselect
```

## File input limitation

Do not assume arbitrary upload fields can always be programmatically populated/retried. Browser security can require manual reselection.

## Native messaging

For large media:
extension -> strict schema -> official native companion -> adapted file.

Websites must never directly invoke native arbitrary commands.

## Security/privacy

No hidden page scraping, no implicit cloud, no arbitrary page strings passed to commands.

## Distribution

Keep extension broadly free; it is a retention/discovery weapon.

## Footprint target

The base extension should be primarily:
- page/DOM evidence collection;
- compact UI;
- IPC/client.

Do not bundle a heavy FFmpeg/WASM runtime by default.

Preferred heavy local path:

```text
extension -> Native Messaging -> native Shoehorn host -> provider
```

The native host starts on demand rather than remaining resident.
