---
title: "Legal and Licensing"
type: engineering
status: active
implementation: mixed
updated: 2026-08-21
canonical: true
tags:
  - legal
  - licensing
---

# Legal and licensing

> This is an engineering risk checklist, not legal advice.

## Fitifact-owned code

Current proposed license: Apache-2.0 to maximize adoption and embedding.

Final selection requires review before public release.

## Provider licensing

Fitifact will likely rely on external tools whose license can vary by build/configuration.

Maintain a machine-readable and human-readable ledger for every redistributed provider:

```text
name
version
source
license
build flags
linked vs subprocess
redistribution terms
notices
codec/patent notes
```

## FFmpeg

Do not assume one universal “FFmpeg license” in documentation. Licensing obligations can depend on how it is built and which optional components are enabled.

Action:
- define approved local/cloud builds;
- audit build configuration;
- ship notices/source offers as required.

## In-process image crate

D-025 JPEG/PNG work uses the Rust `image` crate (MIT OR Apache-2.0) with only
the `jpeg` and `png` features. It is linked into the Fitifact binary and WASM
module, not invoked as a subprocess. Re-check `cargo deny` when adding image
formats. This is not ImageMagick and not FFmpeg.

## Approval-gated HEIC decoder

D-026 pins `libheif-js` 1.19.8. It is LGPL-3.0 and ships an embedded WASM build
of libheif; package/upstream source, build form, license, and no-CDN behavior
are recorded in `web/public/THIRD_PARTY_NOTICES.md`, which is copied into the
static build. It is imported only when HEIC magic is present **and** the build
sets `FITIFACT_HEIC_APPROVED=true`. Default builds emit no decoder chunk.

The approval flag is a legal/build gate, not a user preference. Before enabling
it for distribution, re-review LGPL obligations, corresponding-source/notice
delivery, the exact libheif build configuration and codecs, regional patent
exposure, and browser redistribution. Do not silently replace the decoder or
enable it to claim broad HEIC support.

## PDF/document tooling

Tools such as Ghostscript, LibreOffice, MuPDF and others have distinct licenses. Evaluate each before bundling or hosted use.

## Codec patents

Some codecs can have patent/licensing considerations depending on region and commercial usage.

Action:
- legal review before large commercial hosted transcoding;
- prefer provider/build choices with understood obligations;
- do not market “every codec” casually.

## Profile data

Compatibility facts may be derived from:
- official docs;
- observed UI;
- tests.

Need policy on:
- quoting vs. facts;
- source attribution;
- platform trademarks;
- redistribution.

Profiles should store facts and source URLs, not copy entire copyrighted documentation.

## Brand/trademark

Fitifact is the selected public name. Automated exact-name checks on 2026-08-15
and a 2026-08-16 human review packet found no material exact-name collision
signal; that is not legal clearance. USPTO, WIPO, and EUIPO counsel review and
owner sign-off are still unchecked, and public publication remains blocked.
See [[01-Product/Naming-Brand]] and D-023.

Still complete before publication:
- owner/legal sign-off on the 2026-08-16 packet plus official TESS / WIPO GBD /
  EUIPO eSearch;
- domain search;
- app-store/package search;
- company/name conflict review.

## User content

Terms/privacy must cover:
- temporary processing;
- retention;
- subprocessors;
- deletion;
- lawful content.

## Open-source contributions

Define:
- DCO or CLA decision;
- contribution license;
- profile data license;
- third-party fixture policy.

## Fixtures

Prefer generated/public-domain/appropriately licensed media. Do not commit random copyrighted files to the test suite.

The D-026 image fixtures are generated from owned synthetic pixels and pinned
by SHA-256. The HEIC fixture is encoded locally with the installed Microsoft
HEIF Image Extension; it was not copied from a website or user photo. Its
generator, codec version, nondeterministic-encoder caveat, and provenance are
recorded in `fixtures/image/README.md`.
