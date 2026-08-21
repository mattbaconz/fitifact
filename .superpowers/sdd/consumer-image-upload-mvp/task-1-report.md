# Task 1 report — Complete the image compatibility engine

## Status

DONE. The image compatibility engine is implemented in `crates/fitifact` with a provider-neutral `image.adapt` plan/execution boundary, deterministic requirements parsing, bounded resource use, real-output reinspection, and behavioral contract coverage. No cloud, web, deployment, publishing, tag, or media-matrix files were changed.

## Implementation

- Extended `fitifact.constraints/v1` image width/height validation to accept positive `eq`, `gte`, and `lte` integer values through `u32::MAX`, retained legacy `lte` documents, and reject conflicting exact/min/max ranges.
- Added the `fitifact.requirements/v1` parser and public types. It deterministically recognizes JPEG/JPG/PNG, decimal MB and binary MiB (plus whole-byte) ceilings, exact `W x H`/`W×H`, and textual/symbolic min/max dimensions. Results include normalized constraints, byte-accurate source spans, explicit ambiguities, and unresolved text.
- Added a typed, provider-neutral `image.adapt` contract with source facts and a target containing format, dimensions, byte ceiling, preservation claims, metadata policy, quality/upscale warnings, crop-consent requirements, and proportional-reduction policy.
- Added `ImageAdaptOptions` and validated normalized crop rectangles. Required aspect-changing crops fail with `SECURITY_BLOCKED` unless both a crop rectangle and explicit consent are present.
- Added a cancellable execution boundary (`CancellationSignal`, `AtomicCancellation`) and a built-in in-process Rust image provider. No FFmpeg provider is constructed.
- Added EXIF-orientation-aware inspection and execution. Adapted output applies orientation, omits source metadata, discloses stripping, preserves alpha only in PNG, and refuses implicit alpha-to-JPEG flattening.
- Implemented true no-op, source-format preservation, approved crop, aspect-preserving resize, explicit upscale warning, lossless PNG output, JPEG quality fitting from 95 down to 50 with at most seven actual JPEG encodes, and at most three proportional dimension reductions when no exact/minimum dimension can be violated.
- Enforced the exact 32 MiB encoded-input and 24 MP decoded-image limits. APNG and JPEG/MPO inputs are identified and refused.
- Every changed output is re-inspected and checked against the original hard constraints. Output format/dimensions, preservation claims, alpha policy, metadata stripping, encode/reduction caps, and provider output are independently post-validated.

## Files changed

- `crates/fitifact/src/constraints.rs`
- `crates/fitifact/src/contract.rs`
- `crates/fitifact/src/error.rs`
- `crates/fitifact/src/image.rs`
- `crates/fitifact/src/image_adapt.rs` (new)
- `crates/fitifact/src/requirements.rs` (new)
- `crates/fitifact/src/lib.rs`
- `crates/fitifact/tests/constraints_contract.rs`
- `crates/fitifact/tests/image_contract.rs`
- `crates/fitifact/tests/image_adapt_contract.rs` (new)
- `crates/fitifact/tests/requirements_contract.rs` (new)

## Verification

Focused iteration:

- `cargo test -p fitifact --test constraints_contract --locked` — PASS, 17 passed.
- `cargo test -p fitifact --test requirements_contract --locked` — PASS, 7 passed.
- `cargo test -p fitifact --test image_adapt_contract --locked` — PASS, 16 passed.
- Combined focused suite (`constraints_contract`, `requirements_contract`, `image_contract`, `image_adapt_contract`) — PASS, 48 passed.

Final required verification after the last source/test edit:

- `cargo fmt --all -- --check` — PASS.
- `cargo test --workspace --all-targets --locked` — PASS, 160 passed, 0 failed, 11 ignored (the pre-existing 10 opt-in live FFmpeg tests and 1 opt-in bench test).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS, zero warnings.

## Behavioral coverage

The added contracts cover no-op; format-only; resize-only; JPEG byte fitting; PNG proportional byte fitting; combined format/resize/byte adaptation; approved and missing crop consent; impossible targets; alpha preservation/refusal; EXIF orientation normalization and metadata removal; exact byte and resource-limit boundaries; cancellation; APNG/MPO refusal; upscale disclosure; provider-returned wrong output; and provider-returned metadata-bearing output.

## Self-review

- Corrected JPEG fitting so the reported seven-encode ceiling counts every actual encode, including dimension-reduction rounds.
- Tightened post-validation to verify preservation claims and metadata removal, rather than accepting constraint compatibility alone.
- Replaced naive MPO/APNG marker scans with container-structure checks to avoid false positives in compressed payloads.
- Added `u32::MAX` validation for numeric image dimensions after noticing the public constraint value type is wider than the image dimension domain.
- Verified the existing media planner/runtime behavior and the frozen v0.1 media tests remain unchanged and green.

## Concerns

None. The natural-language grammar is intentionally bounded: unsupported wording is returned in `unresolved`, and multiple formats without an explicit “or” are returned as an ambiguity rather than inferred.
