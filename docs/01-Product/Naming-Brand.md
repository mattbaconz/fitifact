---
title: "Naming and Brand"
type: brand
status: active
updated: 2026-08-18
canonical: true
tags:
  - brand
  - naming
  - risk
---

# Naming and brand

## Fitifact is selected, not legally cleared

Fitifact replaces the historical **Shoehorn** working codename. It evokes a
small artifact made to satisfy a particular fit or compatibility requirement
and works as both a product name and CLI command.

The name is **not legally cleared**. Owner directed public GitHub create, push,
and freeze-tag on 2026-08-18 against the human pass below. That is not billed
attorney clearance. Do not publish packages, buy assets, or make trademark
claims. This document is a search record, not legal advice, and must not be
described as trademark clearance, registrability, ownership, or freedom to
operate.

The `ft.` ligature in `docs/brand/` is a visual monogram for documentation and
future icon use, not a trademark filing; owner/legal sign-off remains
unchecked. Pair the mark with the word **Fitifact** in public copy.

## Collision-search record — 2026-08-15

Automated exact-name checks covered:

- GitHub repositories and organizations;
- crates.io package names, including Fitifact hyphen and underscore variants;
- npm package names;
- executable/command-name collisions;
- ICANN registration data and RDAP lookups.

Those automated checks found **no material collision signal**. Search absence is
not proof of availability, registrability, ownership, or freedom to operate.

## Human review packet — 2026-08-16

Re-checked from this workspace on 2026-08-16. Owner directed public GitHub
create/push/tag on 2026-08-18. That direction is not billed clearance.

### Exact-name software and package checks

| Surface | Query | Result |
| --- | --- | --- |
| GitHub repositories | `fitifact` | `total_count: 0` |
| GitHub user/org | `fitifact` | 404 |
| crates.io crate | `fitifact`, `fiti_fact` | 404 / empty search |
| npm package | `fitifact` | 404; search `total: 0` |
| Command name | Windows `Get-Command fitifact` | no third-party PATH collision |

Nearby names that are **not** exact Fitifact: Factifai, Factify, FictFact,
rustifact, pretifact, `@fitfak/*`, and various Garmin `.fit` parsers. Those are
recorded so a reviewer can judge confusing similarity. They are not an exact
crate, npm, or GitHub collision on `fitifact`.

### Domain / RDAP

| Name | Result |
| --- | --- |
| `fitifact.com` | Verisign RDAP 404 (not in the .com registry snapshot queried) |
| `fitifact.dev` | nic.dev RDAP unavailable (503) at query time; re-check at publication |

Absence of a .com RDAP record is not a purchase instruction and is not proof
the name will remain unregistered.

### USPTO / WIPO / EUIPO human pass

Public web and secondary-index review, not a billed attorney search and not a
complete TESS/TSDR, WIPO Global Brand Database, or EUIPO eSearch screenshot
packet:

- No live exact **FITIFACT** USPTO word mark surfaced. Dead nearby marks
  include **FIT FACTS** (serial 74363546, class 16) and **FITFACTS** (serial
  76462032 / registration 2814634, class 16, cancelled). **Fitfact, Inc.** is
  a New York corporation name, not a software-class federal mark in this pass.
- WIPO Global Brand Database interactive search was not completed here (portal
  challenge). No secondary-index hit for an exact Fitifact international mark
  was found. Owner/counsel must still search [branddb.wipo.int](https://branddb.wipo.int/en)
  and Madrid Monitor before sign-off.
- No exact **FITIFACT** EUTM appeared in public EUIPO-adjacent indexes. Nearby
  **IFIT** registrations (iFIT Inc., fitness/education classes) are a different
  mark and class story for counsel, not an exact hit.

**Owner/legal sign-off:** [x] owner directed 2026-08-18. Public GitHub
create, `main` push, and annotated `v0.1.0-rc.1` on the freeze SHA were
authorized from this packet. This is not billed attorney clearance, not a
completed WIPO Global Brand Database interactive search, and not a trademark
registration. GitHub Release publication stays gated by
`FITIFACT_PUBLICATION_APPROVED` and Environment `public-release`.

## Publication gate

Owner directed public GitHub on 2026-08-18. Remaining gates before a GitHub
Release, registry package, domain purchase, or public launch:

1. keep USPTO, WIPO, and EUIPO review recorded; WIPO interactive search is
   still incomplete;
2. resolve any confusingly similar marks in relevant software/service classes;
3. set `FITIFACT_PUBLICATION_APPROVED` only for an explicit Release window;
4. re-check package, command, social, and domain availability at publication
   time.

## Historical context

The Shoehorn codename was rejected because `shoehorn.dev`,
`@total-typescript/shoehorn`, and other software uses created material collision
and searchability risk. Historical research URLs remain in the source ledger
only as evidence for that retired name.

## Naming criteria

The public name should be easy to spell, work as a CLI noun/verb, evoke fit or
compatibility, avoid major collision signals, span consumer and developer use,
and not depend on an AI positioning.

The launch title does not need the product name: **“I Made an Adapter for
Files.”**
