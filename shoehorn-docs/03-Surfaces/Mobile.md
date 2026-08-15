---
title: "Mobile"
type: surface
status: active
updated: 2026-08-15
canonical: true
tags:
  - mobile
  - surface
---

# Mobile

## Android

Natural:
`Share file -> Shoehorn -> Adapt for -> Share result`

Use intents/document picker and platform codecs where suitable.

## iOS/iPadOS

Natural:
- Share Extension;
- Files picker;
- Shortcuts;
- main app handoff for heavy work.

Do not claim system-wide interception.

## Constraints

- memory;
- battery;
- thermal limits;
- sandbox;
- background execution;
- temporary storage.

These justify optional cloud, not implicit cloud.

## UX

Three-tap aspiration:
file -> Shoehorn -> destination.

No-op should return quickly with “Already compatible.”

## Privacy

Always disclose on-device vs cloud and what metadata leaves device.

## Monetization

Mobile should primarily distribute the product. Heavy cloud credits can be paid later.

## Lightweight mobile model

The share extension/intent is a thin entry point.

Prefer platform-native capabilities where they meet target semantics and hand heavy work to:
- the main app;
- or explicitly selected cloud.

Do not package the entire desktop provider universe into the mobile app.
