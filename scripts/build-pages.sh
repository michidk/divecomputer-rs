#!/bin/sh
# The deployable artifact contains the existing book and both browser tools.
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-web.sh
mdbook build docs
mkdir -p docs/book/tools
cp crates/divecomputer-wasm/www/*.html crates/divecomputer-wasm/www/*.js \
  crates/divecomputer-wasm/www/*.css docs/book/tools/
# Copy only the generated web runtime, not package metadata or generated docs.
mkdir -p docs/book/tools/pkg
cp crates/divecomputer-wasm/www/pkg/*.js crates/divecomputer-wasm/www/pkg/*.wasm \
  docs/book/tools/pkg/
