---
title: "v0.1 Release Checklist"
type: engineering
status: blocked
implementation: prepared
updated: 2026-08-16
canonical: true
tags:
  - release
  - checklist
  - provenance
---

# v0.1 release checklist

## Publication gate

This entire checklist is **blocked** until the owner and legal reviewer record
Fitifact naming approval. Preparing or rehearsing local commands does not
authorize creating the public repository, pushing tags, or publishing assets.

Before either tag:

- [ ] Record owner/legal sign-off and the final USPTO, WIPO, and EUIPO review.
- [ ] Create the public repository and apply [[04-Engineering/Repository-Rules]].
- [ ] Confirm the default branch is clean and every required CI check passes.
- [ ] Run `scripts/check-public-readiness.ps1 -RequireDependencyTools` from the
      exact candidate commit; review filenames only from the secret scan.
- [ ] Confirm every Cargo package still says `publish = false` and no registry,
      package-manager, signing, notarization, or bundled-FFmpeg target appeared.
- [ ] Run `dist plan --tag=v0.1.0` with cargo-dist 0.32.0 and review the four
      native archives, two installers, SHA-256 files, source archive,
      `fitifact-cli.cdx.xml`, and attestation scope.

## `v0.1.0-rc.1`

- [ ] Create the annotated RC tag from the reviewed commit and record the tag
      object, peeled commit SHA, workflow run, and generated source archive.
- [ ] Publish only through the reviewed release workflow; confirm it is marked
      prerelease and contains no crates.io/Homebrew/WinGet assets.
- [ ] On clean Windows x64, Linux GNU x64, macOS Intel, and macOS Apple Silicon
      machines, download (do not reuse CI workspaces), verify SHA-256 and GitHub
      attestation, extract/install, and run `fitifact --version` plus
      `fitifact doctor`.
- [ ] On each supported machine, run the three canonical behaviors: compatible
      `compatible-h264-aac.mp4` is a no-op, `remux-h264-aac.mov` is remuxed with
      streams copied, and `mismatch-hevc-aac.mp4` changes only HEVC video to
      H.264 while preserving AAC. Validate each produced artifact.
- [ ] Open `fitifact-cli.cdx.xml` with a CycloneDX-compatible validator and
      confirm it represents the candidate Cargo dependency graph.
- [ ] Confirm Windows displays the expected unsigned-binary warning and macOS
      displays the expected unsigned/not-notarized warning; release notes must
      disclose both without suggesting a bypass of platform security.
- [ ] Compare the RC source archive/tag tree to the reviewed commit and record
      any generated-file differences. Do not promote a mismatched build.

## Immutable `v0.1.0`

- [ ] Resolve every RC finding in a new reviewed commit and repeat all required
      CI, readiness, native install, doctor, scenario, checksum, SBOM,
      attestation, and provenance checks.
- [ ] Create `v0.1.0` once, from the approved commit. Never move or reuse the
      stable tag; enable GitHub immutable releases before publication.
- [ ] Verify the release page commit, tag object, source archives, checksums,
      SBOM, attestations, installers, and four platform archives all refer to
      the same immutable release provenance.
- [ ] Test the documented source fallback exactly:
      `cargo install --git https://github.com/mattbaconz/fitifact --locked fitifact-cli`.
- [ ] Preserve the completed checklist and clean-machine evidence with the
      release record. Any failed check blocks publication rather than becoming
      an undocumented exception.
