#!/usr/bin/env bash
# The phone/browser target checked on its own terms: build wasm-check for
# wasm32-unknown-unknown and run it under Node. Inside WebAssembly: the BCD
# rung (SHA-256 of the hardware-verified table), rungs 1 and 5 (the 23
# manifests of tests/data against the reference traces), and the reference's
# "base" loop for a speed figure.
# Needs the wasm32-unknown-unknown target and node.
#   env: TOOLCHAIN (rustup toolchain with the wasm32 target, default stable)
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TOOLCHAIN=${TOOLCHAIN:-stable}
RUSTC=$(rustup which --toolchain "$TOOLCHAIN" rustc) \
    rustup run "$TOOLCHAIN" cargo build --release -q \
    --manifest-path "$ROOT/wasm-check/Cargo.toml" --target wasm32-unknown-unknown
wasm="$ROOT/wasm-check/target/wasm32-unknown-unknown/release/m68000_wasm_check.wasm"
echo "$(basename "$wasm"): $(stat -c %s "$wasm") bytes"
node "$ROOT/scripts/wasm-check.mjs" "$wasm"
