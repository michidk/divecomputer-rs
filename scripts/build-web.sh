#!/bin/sh
# Build the browser assets in-place. Generated bindings stay out of Git.
set -eu
cd "$(dirname "$0")/.."

expected=$(sed -n 's/^wasm-bindgen = "=\([^"]*\)"/\1/p' Cargo.toml)
actual=$(wasm-bindgen --version)
if [ "$actual" != "wasm-bindgen $expected" ]; then
  echo "Install the matching CLI: cargo install wasm-bindgen-cli --version $expected --locked" >&2
  exit 1
fi
cargo build --locked --release -p divecomputer-wasm --target wasm32-unknown-unknown
cargo_target=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
wasm-bindgen --target web --out-dir crates/divecomputer-wasm/www/pkg \
  "$cargo_target/wasm32-unknown-unknown/release/divecomputer_wasm.wasm"
