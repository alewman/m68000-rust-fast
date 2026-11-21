#!/usr/bin/env bash
# Profile-guided optimization as an optional build (z80-rust-fast's shape):
# instrument, train, merge, rebuild with the profile, then time the PGO
# binary against the ordinary release build alternately on one core.
# `cargo build --release` is unchanged; a committed .profdata would be tied
# to one compiler version.
#
# Training is the Genesis replay and the synthetic workloads; the System 16B
# replay is held out, so the timing shows what PGO does on code it was not
# trained on as well as on all of it.
#
# Needs llvm-profdata from the rustc's own LLVM (rustup's llvm-tools for the
# matching toolchain); set PROFDATA if it is elsewhere.
# Usage: scripts/pgo.sh   env: CPU (default 4), ROUNDS (default 5), PROFDATA
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CPU=${CPU:-4}
ROUNDS=${ROUNDS:-5}
version=$(rustc --version | awk '{print $2}')
PROFDATA=${PROFDATA:-"$HOME/.rustup/toolchains/$version-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"}
[ -x "$PROFDATA" ] || { echo "llvm-profdata not found at $PROFDATA" >&2; exit 2; }
base_flags=$(sed -n 's/^rustflags = \["-C", "\(.*\)"\]$/-C \1/p' "$ROOT/.cargo/config.toml")
profiles="$ROOT/target/pgo-profiles"
rm -rf "$profiles"
mkdir -p "$profiles"
GENESIS="$ROOT/external/replay-genesis/manifest.json"
ALTBEAST="$ROOT/external/replay-altbeast/manifest.json"

RUSTFLAGS="$base_flags -C profile-generate=$profiles" CARGO_TARGET_DIR="$ROOT/target/pgo-gen" \
    cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
taskset -c "$CPU" "$ROOT/target/pgo-gen/release/m68k-replay-bench" --repeat 1 "$GENESIS" >/dev/null
taskset -c "$CPU" "$ROOT/target/pgo-gen/release/m68k-bench" 5000000 1 >/dev/null
"$PROFDATA" merge -o "$profiles/merged.profdata" "$profiles"/*.profraw
RUSTFLAGS="$base_flags -C profile-use=$profiles/merged.profdata" \
    CARGO_TARGET_DIR="$ROOT/target/pgo-use" cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
cargo build --release -q --manifest-path "$ROOT/Cargo.toml"

seconds() { sed -E 's/.* clocks, ([0-9.]+) s,.*/\1/'; }
min() { awk -v a="$1" -v b="$2" 'BEGIN{print (a == "" || b < a) ? b : a}'; }
echo "commit $(git -C "$ROOT" rev-parse --short HEAD), cpu $CPU, $(rustc --version | cut -d' ' -f1-2), load $(cut -d' ' -f1 /proc/loadavg)"
for set in held-out all; do
    files="$ALTBEAST"; [ $set = all ] && files="$GENESIS $ALTBEAST"
    plain=""; pgo=""
    for _ in $(seq "$ROUNDS"); do
        plain=$(min "$plain" "$(taskset -c "$CPU" "$ROOT/target/release/m68k-replay-bench" --repeat 1 $files | seconds)")
        pgo=$(min "$pgo" "$(taskset -c "$CPU" "$ROOT/target/pgo-use/release/m68k-replay-bench" --repeat 1 $files | seconds)")
    done
    echo "replays ($set): release $plain s, pgo $pgo s, x$(awk "BEGIN{printf \"%.3f\", $plain/$pgo}")"
done
plain=""; pgo=""
for _ in $(seq "$ROUNDS"); do
    plain=$(min "$plain" "$(taskset -c "$CPU" "$ROOT/target/release/m68k-bench" 5000000 1 | sed -nE 's/^total .* ([0-9.]+) s$/\1/p')")
    pgo=$(min "$pgo" "$(taskset -c "$CPU" "$ROOT/target/pgo-use/release/m68k-bench" 5000000 1 | sed -nE 's/^total .* ([0-9.]+) s$/\1/p')")
done
echo "synthetic (trained): release $plain s, pgo $pgo s, x$(awk "BEGIN{printf \"%.3f\", $plain/$pgo}")"
