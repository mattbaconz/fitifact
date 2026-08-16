# Changelog

All notable changes to Fitifact are documented here.

## [0.1.0-rc.1] - unpublished candidate

### [added]

- Release-ready public repository documentation and governance files.
- Versioned constraint, artifact, check, plan, adaptation, error, and doctor
  contracts with strict YAML parsing and normalized all-stream media facts.
- Stable JSON CLI/error/doctor envelopes, bounded transform timeouts, strict
  human-size flags, FFmpeg capability diagnostics, and structured validation
  provenance claims.
- Deterministic synthetic media fixtures, native four-target CI, dependency
  policy, and cargo-dist 0.32.0 release assets with checksums, CycloneDX SBOM,
  installers, and GitHub artifact attestations.

### [changed]

- Renamed the Shoehorn prototype to Fitifact across crates, CLI, schemas,
  examples, fixtures, and documentation.
- Defined v0.1 as the CLI/media slice distributed through GitHub only.
- Replaced deprecated `serde_yaml` with maintained `yaml_serde` and bounded the
  planner to provider-neutral remux and selective video-transcode operations.
- Hardened system FFmpeg execution to typed file-only/no-clobber argv, bounded
  process output/time, hidden sibling staging, atomic create-if-absent
  persistence, and defensive typed-plan validation.

### [fixed]

- Distinguished executable adaptation constraints from check-only constraints.
- Labelled later image, web, profile, natural-language, cloud, packaging, and
  operations work as deferred.
- Refused unsafe stream topology, unsupported mutation classes, HDR/bit-depth
  conversion, and uncertain post-transform size claims.
- Made overlapping target sets order-independent, added strict public JSON
  constraint compilation, and refused unproved pixel/color conversion.
- Prevented existing-output overwrite, partial-stage leakage, silent stream
  loss, raw provider diagnostic leakage, and false adaptation success by fresh
  topology/fact/duration checks plus SHA-256 copied-stream provenance.
- Bound staging and cleanup to atomically reserved workspaces and stable file
  identities, reject post-validation replacement, identity-confirm the
  published hard link, and preserve validated finals when cleanup cannot
  complete.
- Removed pathname rollback deletion: Windows cleanup is handle-bound, Unix
  workspaces are atomically mode-0700, ordinary provider partials are claimed
  and removed, ambiguous or replaced objects are retained, and
  publication/cleanup uncertainty returns structured warning paths.
- Refused ambiguous MOV-family demuxer labels without a recognized MP4 or
  QuickTime brand, rejected mixed constraint-file/flag targets, bounded
  constraint-file reads to 1 MiB, and locked GitHub publication to one
  attestation-ordered release creation command.
- Treated FFprobe's shared `matroska,webm` label as unknown, kept unknown
  container facts from being reinterpreted during checks, and aligned planning
  and execution to the exact MOV/H.264 remux and MP4/HEVC transcode sources.
- Restricted MP4/MOV brand interpretation to ISO-BMFF/QuickTime demuxer labels
  so a borrowed `isom`/`qt` tag cannot promote Matroska or WebM evidence.
