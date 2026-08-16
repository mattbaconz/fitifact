---
title: "Legal and Licensing"
type: engineering
status: active
implementation: mixed
updated: 2026-08-16
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
found no material collision signal; that is not legal clearance. USPTO, WIPO,
and EUIPO human/legal review is still pending, and public publication remains
blocked until owner/legal sign-off. See [[01-Product/Naming-Brand]] and D-023.

Still complete before publication:
- recorded USPTO/WIPO/EUIPO review and owner/legal sign-off;
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
