#!/bin/sh
# Vercel build for fitifact.com. The build image has Node but no Rust, and the
# web app compiles the engine to WASM, so install rustup first. Static output
# only: web/dist, the app at / and the docs at /docs/.
set -eu
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
  . "$HOME/.cargo/env"
fi
rustup target add wasm32-unknown-unknown
node scripts/build-site.mjs
