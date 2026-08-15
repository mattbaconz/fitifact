# Changelog

All notable changes to Fitifact are documented here.

## [Unreleased]

### [added]

- Release-ready public repository documentation and governance files.
- Versioned constraint, artifact, check, plan, adaptation, error, and doctor
  contracts with strict YAML parsing and normalized all-stream media facts.

### [changed]

- Renamed the Shoehorn prototype to Fitifact across crates, CLI, schemas,
  examples, fixtures, and documentation.
- Defined v0.1 as the CLI/media slice distributed through GitHub only.
- Replaced deprecated `serde_yaml` with maintained `yaml_serde` and bounded the
  planner to provider-neutral remux and selective video-transcode operations.

### [fixed]

- Distinguished executable adaptation constraints from check-only constraints.
- Labelled later image, web, profile, natural-language, cloud, packaging, and
  operations work as deferred.
- Refused unsafe stream topology, unsupported mutation classes, HDR/bit-depth
  conversion, and uncertain post-transform size claims.
- Made overlapping target sets order-independent, added strict public JSON
  constraint compilation, and refused unproved pixel/color conversion.
