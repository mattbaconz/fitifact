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

## Fix Round 1

Independent review identified five blocking gaps. All were fixed with regressions that would have failed before this round:

- The public `encode_jpeg_bytes` path now enforces the 32 MiB encoded and 24 MP decoded limits before pixel allocation. `ImageProvider` checks file metadata before reading an oversized file, then inherits decoded-limit enforcement. The typed planner and built-in renderer both reject targets above 24 MP before resize allocation.
- The legacy PNG→JPEG path now inspects alpha and returns an explicit transparency-flattening refusal instead of calling `to_rgb8` on alpha-bearing PNG input.
- Format-alternative parsing now recognizes only standalone `or` or `/` connectors and marks only precise connector/format spans. It no longer covers intervening evidence, so `JPEG, exactly 1200×630, or PNG` retains both format and exact-dimension constraints. Words such as `for` no longer count as `or`.
- Recognizable malformed numeric dimension and MB/MiB/byte targets now return `INPUT_INVALID`, including fractional width/height, malformed exact pairs, signs, repeated/locale decimal separators, and fractional raw bytes.
- Aspect comparison now uses `u128` cross-products with a bounded 1% near-equality allowance. Material changes including 2:1→1:1 and small 3:2→2:1 require approved crop consent. Uncropped rendering and proportional fitting use the aspect-preserving `resize` path; only an explicitly approved crop may use exact target resampling.

Focused regression verification:

- `cargo test -p fitifact --test constraints_contract --test requirements_contract --test image_contract --test image_adapt_contract --locked` — PASS, 57 passed, 0 failed (17 constraint, 10 requirements, 11 legacy image, 19 image-adapt).

Final verification after the last fix:

- `cargo fmt --all -- --check` — PASS.
- `cargo test --workspace --all-targets --locked` — PASS, 169 passed, 0 failed, 11 ignored (the pre-existing 10 opt-in live FFmpeg tests and 1 opt-in bench test).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS, zero warnings.

Fix-round files changed:

- `crates/fitifact/src/image.rs`
- `crates/fitifact/src/image_adapt.rs`
- `crates/fitifact/src/requirements.rs`
- `crates/fitifact/tests/image_contract.rs`
- `crates/fitifact/tests/image_adapt_contract.rs`
- `crates/fitifact/tests/requirements_contract.rs`
- `.superpowers/sdd/consumer-image-upload-mvp/task-1-report.md`

Fix-round self-review found no remaining correctness concern. Resource gates now exist at every public execution/allocation boundary in scope, and all changed output still follows inspect → check → plan → execute → re-inspect → validate.

## Fix Round 2

Scoped re-review found two remaining edge cases. Both were fixed with focused regressions:

- Declared target aspect ratios are now compared by exact `u128` cross-products. Therefore every genuine ratio change, including 100×100→100×101, is planned as a crop and fails with `SECURITY_BLOCKED` without explicit consent. Approved normalized crops retain a narrowly bounded integer-pixel rounding allowance before exact target rendering. Same-ratio scaled dimensions remain crop-free.
- Malformed `W×H` prevalidation now requires either two numeric neighbors or an explicit dimension qualifier/adjacent dimension term. Unsupported multiplier prose such as `make it 2x faster` remains unresolved, while dimension-qualified incomplete input such as `exactly 2x` and the previously covered malformed numeric pairs still return `INPUT_INVALID`.

Test-first evidence:

- `cargo test -p fitifact --test image_adapt_contract near_equal_aspect_change_requires_and_honors_crop_consent --locked` — RED before the source fix because `plan.target.crop.required` was false; PASS after the fix.
- `cargo test -p fitifact --test requirements_contract unsupported_prose_with_an_x_suffix_remains_unresolved --locked` — RED before the source fix because parsing returned `INPUT_INVALID`; PASS after the fix.
- `cargo test -p fitifact --test requirements_contract dimension_qualified_incomplete_exact_pair_is_rejected --locked` — PASS, locking the existing rejection side of the narrowed parser boundary.

Focused regression verification:

- `cargo test -p fitifact --test constraints_contract --test requirements_contract --test image_contract --test image_adapt_contract --locked` — PASS, 60 passed, 0 failed (17 constraint, 12 requirements, 11 legacy image, 20 image-adapt).

Final verification after the last source/test edit:

- `cargo fmt --all -- --check` — PASS.
- `cargo test --workspace --all-targets --locked` — PASS, 172 passed, 0 failed, 11 ignored (the pre-existing 10 opt-in live FFmpeg tests and 1 opt-in bench test).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS, zero warnings.

Fix-round files changed:

- `crates/fitifact/src/image_adapt.rs`
- `crates/fitifact/src/requirements.rs`
- `crates/fitifact/tests/image_adapt_contract.rs`
- `crates/fitifact/tests/requirements_contract.rs`
- `.superpowers/sdd/consumer-image-upload-mvp/task-1-report.md`

Fix-round self-review found no remaining correctness or scope concern. Exact target planning cannot take a non-crop path for a changed ratio, crop quantization remains explicit and consent-gated, and unsupported numeric-looking prose is no longer promoted into the supported requirement grammar.
