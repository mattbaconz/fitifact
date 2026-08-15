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
- [ ] Configure the protected `public-release` GitHub Environment with required
      release-owner reviewers, and create the repository variable
      `FITIFACT_PUBLICATION_APPROVED` with the default value `false`.
- [ ] Protect `v*` tags against deletion, update, and unauthorized creation.
- [ ] Confirm the default branch is clean and every required CI check passes.
- [ ] Run `scripts/check-public-readiness.ps1 -RequireDependencyTools` from the
      exact candidate commit; review filenames only from the secret scan.
- [ ] Confirm every Cargo package still says `publish = false` and no registry,
      package-manager, signing, notarization, or bundled-FFmpeg target appeared.
- [ ] Confirm `Cargo.toml`, `Cargo.lock`, `fitifact --version`, and the RC tag all
      identify `0.1.0-rc.1`; do not tag stable from this candidate commit.
- [ ] Run `dist plan --tag=v0.1.0-rc.1` with cargo-dist 0.32.0 and review the four
      native archives, two installers, SHA-256 files, source archive,
      `fitifact-cli.cdx.xml`, and attestation scope.
- [ ] Run `scripts/check-workflows.ps1` and confirm it finds exactly one
      `gh release create`, after `actions/attest`, with no `dist host
      --steps=create`, `gh release upload`, release API, or release-action
      alternative.

Exact cargo-dist 0.32.0 source is fixed at tag commit
`6886366640dd4da83d33ba55cc04aa58423cbad2`. Its
[`do_host`](https://github.com/axodotdev/cargo-dist/blob/6886366640dd4da83d33ba55cc04aa58423cbad2/cargo-dist/src/host.rs#L19-L43)
explicitly leaves GitHub hosting "implemented in CI backend" and only saves the
merged manifest. The matching
[generated host template](https://github.com/axodotdev/cargo-dist/blob/6886366640dd4da83d33ba55cc04aa58423cbad2/cargo-dist/templates/ci/github/release.yml.j2#L540-L555)
then includes the
[GitHub publication partial](https://github.com/axodotdev/cargo-dist/blob/6886366640dd4da83d33ba55cc04aa58423cbad2/cargo-dist/templates/ci/github/partials/publish_github.yml.j2#L1-L46),
where attestation precedes the generated `gh release create`. Thus the checked-in
`dist host --steps=upload --steps=release` invocation is a manifest step, not a
second GitHub publication primitive; the later `gh release create` is the sole
release creation/upload operation and must remain after successful attestation.

## `v0.1.0-rc.1`

- [ ] Publish and test the RC from the reviewed commit whose checked-in package
      and binary version is exactly `0.1.0-rc.1`. Create the annotated RC tag
      from that commit and record the tag object, peeled commit SHA, workflow
      run, and generated source archive.
- [ ] After owner/legal sign-off, set the repository variable
      `FITIFACT_PUBLICATION_APPROVED` to `true`; approve the `public-release`
      Environment only after verifying the candidate run. A matching tag may
      build with approval absent, but the `host` publication job must remain
      skipped or waiting and no GitHub Release may be created.
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
- [ ] Reset `FITIFACT_PUBLICATION_APPROVED` to `false` immediately after the RC
      publication window; stable publication requires a fresh explicit gate.

## Immutable `v0.1.0`

- [ ] Accept the RC only after every required clean-machine and provenance check
      passes. Resolve findings in new reviewed commits and repeat RC acceptance.
- [ ] Only after RC acceptance, bump the workspace/package version and internal
      dependency to `0.1.0`, update `Cargo.lock` and the changelog from the RC
      candidate to stable, and update the CLI version expectation.
- [ ] Rerun the complete stable verification suite and
      `dist plan --tag=v0.1.0`; commit that stable-version bump separately.
      Confirm `fitifact --version` reports `0.1.0` from that exact commit.
- [ ] Create `v0.1.0` once from the reviewed stable-version commit, never from
      the RC-version commit. Never move or reuse the stable tag; enable GitHub
      immutable releases before publication.
- [ ] Verify the release page commit, tag object, source archives, checksums,
      SBOM, attestations, installers, and four platform archives all refer to
      the same immutable release provenance.
- [ ] Test the documented source fallback exactly:
      `cargo install --git https://github.com/mattbaconz/fitifact --locked fitifact-cli`.
- [ ] Preserve the completed checklist and clean-machine evidence with the
      release record. Any failed check blocks publication rather than becoming
      an undocumented exception.
- [ ] Reset `FITIFACT_PUBLICATION_APPROVED` to `false` after the approved
      publication window; required Environment review remains mandatory for
      every future release.
