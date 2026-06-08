#!/usr/bin/env bash
# Build the WASM package for the web app.
#
# Requires: rustup with the wasm32-unknown-unknown target, and wasm-pack.
#   rustup target add wasm32-unknown-unknown
#   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

wasm-pack build "$root/tuner-core" \
  --target web \
  --out-dir "$root/tuner-web/pkg" \
  --release \
  --no-default-features \
  --features wasm

echo "WASM package written to tuner-web/pkg"
