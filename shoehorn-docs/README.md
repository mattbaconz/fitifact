---
title: "Shoehorn"
type: root-index
status: active
updated: 2026-08-15
canonical: true
tags:
  - shoehorn
  - index
---

# Shoehorn

> **Any file. Any destination. Make it work.**

Shoehorn is a proposed **file compatibility layer**. It is not primarily a file converter. Its job is to take a file, understand what a destination accepts, and make the **smallest necessary set of changes** so the file works there.

The canonical product contract is:

```text
adapt(file, constraints) -> compatible file + explanation
```

Examples:

```text
video.mov + "PowerPoint" -> compatible video
assignment.pdf + "< 2 MB" -> compatible PDF
photo.heic + "JPEG/PNG, square, < 5 MB" -> accepted image
video.mp4 + "MP4/H.264 only" -> diagnose HEVC-in-MP4 and adapt only the video stream
```

The public mental model is intentionally simple:

```text
FILE DOESN'T FIT
      ↓
   SHOEHORN
      ↓
   FILE FITS
```

## Start here

- [[00-Foundation/Executive-Summary]]
- [[00-Foundation/Vision]]
- [[01-Product/Product-Definition]]
- [[01-Product/Use-Cases]]
- [[02-Architecture/System-Architecture]]
- [[04-Engineering/MVP-Scope]]
- [[05-Business/FOSS-Strategy]]
- [[06-Research/Competitors]]
- [[06-Research/Threats]]
- [[05-Business/YouTube-Launch]]
- [[AGENTS]]

## Locked product principles

1. **Destination-first, not format-first.**
2. **Compatibility is the outcome. Conversion is an implementation detail.**
3. **Prefer minimum mutation.**
4. **Explain what changed and why.**
5. **Local-first where feasible.**
6. **The core compatibility engine should be genuinely FOSS.**
7. **Cloud monetizes managed compute, verified compatibility knowledge, scale, and enterprise operations—not artificial crippling of local use.**
8. **Never invent destination requirements. Confidence and provenance matter.**
9. **Do not position Shoehorn as “another universal converter.”**
10. **The signature interaction is rejected → adapt → accepted.**

## Still open

- Final public name. **Shoehorn is a working codename and has serious naming collisions.** See [[01-Product/Naming-Brand]].
- Exact implementation language and crate/package boundaries, though Rust is the current recommendation.
- Whether the first public MVP includes PDFs or launches with images + media only.
- Which cloud provider(s), if any.
- Pricing.
- Which destination profiles are officially verified at launch.
- Whether natural-language requirement parsing is purely deterministic at launch or optionally model-assisted.

## Repository / vault philosophy

This vault is both:

- an **Obsidian knowledge base**, using YAML frontmatter and `[[wikilinks]]`; and
- an **agent execution spec**, with explicit invariants, non-goals, acceptance criteria, threat models, and source-of-truth documents.

If an implementation agent encounters a conflict, follow [[AGENTS]] and [[00-Foundation/Decision-Log]].

## Core one-liner

> **Shoehorn figures out what a file needs to become in order to work somewhere, then changes as little as possible.**

## YouTube launch thesis

The strongest current launch title is:

> **I Made an Adapter for Files**

See [[05-Business/YouTube-Launch]].

## Research timestamp

Competitive and prior-art research in this vault was refreshed on **2026-08-15**. Markets, product features, pricing, names, and platform constraints change. Treat research files as time-bounded and re-verify before public claims.

## Lightweight integration philosophy

Shoehorn should be architected as **tiny compatibility plumbing**:

```text
thin integrations
      ↓
small schema/core
      ↓
lazy providers
```

The browser extension, desktop context menu, mobile share target, CLI, SDK, web app and API must reuse the same compatibility semantics.

Heavy providers are loaded only after inspection/planning proves they are required.

See:
- [[02-Architecture/Lightweight-Architecture]]
- [[03-Surfaces/Integration-Strategy]]
- [[04-Engineering/Performance-Budgets]]
- [[04-Engineering/Packaging-Distribution]]
