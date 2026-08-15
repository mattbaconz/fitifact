---
title: "Plugin and Transform Provider System"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - plugins
  - providers
---

# Plugin and transform provider system

## Goal

Orchestrate mature tools instead of reimplementing codecs/renderers.

## Provider interface

```text
Provider
- id
- version
- capabilities()
- probe_environment()
- execute(step, context)
```

Capabilities include:
- input predicates;
- output effects;
- side effects;
- costs;
- execution modes;
- license metadata.

## Candidate providers

### Media
FFmpeg / ffprobe, platform hardware encoders.

### Images
ImageMagick, libvips, platform image APIs, browser codecs.

### PDF
qpdf, Ghostscript, MuPDF or specialist tools after license/security review.

### Office
LibreOffice headless, Pandoc where semantic conversion is appropriate.

## Selection

Choose by:
- availability;
- quality;
- platform;
- license;
- local/cloud;
- hardware;
- security.

## Isolation

Providers parse hostile input. Use:
- minimal FS;
- no network by default;
- quotas;
- patched versions;
- executable allowlist.

## Plugin ecosystem

Defer arbitrary third-party executable plugins until sandbox/trust model exists.

## License boundary

Maintain a dependency ledger:
- provider;
- version;
- license;
- linking/subprocess;
- redistribution;
- build flags;
- codec patent notes.

## Fallback

If local provider missing:
> This plan needs H.264 encoding. Install the native provider or explicitly choose Shoehorn Cloud.

Never silently upload.
