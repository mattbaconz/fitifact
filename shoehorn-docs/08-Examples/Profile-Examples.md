---
title: "Profile Examples"
type: examples
status: active
updated: 2026-08-15
canonical: true
tags:
  - examples
  - profiles
---

# Profile examples

Illustrative only, not claims about real platforms.

## Avatar

```yaml
schema: shoehorn.profile/v1
id: demo/avatar
name: Demo Avatar
revision: 1
last_verified: 2026-08-15

scope:
  kind: upload
  feature: avatar

constraints:
  - field: image.format
    op: in
    value: [jpeg, png]
    source: demo

  - field: file.bytes
    op: lte
    value: 5000000
    source: demo

  - field: image.aspect_ratio
    op: eq
    value: "1:1"
    source: demo

sources:
  - id: demo
    type: user-config
    note: illustrative
```

## Enterprise image policy

```yaml
schema: shoehorn.profile/v1
id: acme/marketing/upload
name: ACME Marketing Upload

scope:
  kind: private-policy

constraints:
  - field: image.format
    op: in
    value: [jpeg, png, webp]

  - field: file.bytes
    op: lte
    value: 10000000

  - field: image.width
    op: lte
    value: 6000

preferences:
  preserve:
    color_profile: high
    metadata: low
```

## Real profile rule

A public real-world profile must include precise scope, sources, evidence dates and tests.
