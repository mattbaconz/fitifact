# Contributing to Fitifact

Thank you for helping make file compatibility safer and more predictable.

## Before you start

- Read [`AGENTS.md`](AGENTS.md) and the accepted decisions in
  [`docs/00-Foundation/Decision-Log.md`](docs/00-Foundation/Decision-Log.md).
- Keep changes inside the v0.1 CLI/media boundary unless a decision explicitly
  expands it.
- Open an issue before substantial contract or architecture changes.
- Follow the [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## Development

Install a Rust toolchain and run:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

When media behavior changes, add deterministic unit coverage and update fixtures
or fixture-generation instructions. Live FFmpeg tests are ignored by default;
their opt-in command is documented in `crates/fitifact/tests/live_ffmpeg.rs`.

## Pull requests

Keep pull requests focused. Explain the compatibility problem, the constraints
involved, the minimal-change behavior, validation evidence, and any deferred
work. Update `CHANGELOG.md` under `[added]`, `[changed]`, or `[fixed]` when the
change is user-visible. Do not add generated media fixtures or dependencies
without documenting their source and license.

By contributing, you agree that your contribution is licensed under Apache-2.0.
