#!/usr/bin/env bash
# Time the replay benchmark (m68k-replay-bench: the two MAME game replays
# stepped bare), pinned to one core, under several code layouts, and report
# each plus the minimum. z80-rust-fast's method: byte-identical hot code can
# differ by several percent by placement alone, so a change is judged by the
# minimum over layouts, not one build.
#   default  `cargo build --release` (.cargo/config.toml aligns branch-target
#            blocks to 64 bytes)
#   noalign  no alignment flag (RUSTFLAGS replaces the config's)
#   nf5      32-byte alignment
# Usage: scripts/bench.sh [m68k-replay-bench options...]
#   env: CPU (logical core, default 4), LAYOUTS, ROOT (checkout to measure)
set -euo pipefail
ROOT=${ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
CPU=${CPU:-4}
LAYOUTS=${LAYOUTS:-"default noalign nf5"}
declare -A FLAGS=(
    [noalign]="-C opt-level=3"
    [nf5]="-C llvm-args=-align-all-nofallthru-blocks=5"
)
dirty=$(git -C "$ROOT" diff --quiet HEAD -- . ':!README.md' ':!docs' 2>/dev/null || echo " (dirty)")
echo "commit $(git -C "$ROOT" rev-parse --short HEAD)$dirty, cpu $CPU, $(rustc --version | cut -d' ' -f1-2), load $(cut -d' ' -f1 /proc/loadavg)"
best=""
best_layout=""
for layout in $LAYOUTS; do
    if [ "$layout" = default ]; then
        cargo build --release -q --manifest-path "$ROOT/Cargo.toml" --bin m68k-replay-bench
        bin="$ROOT/target/release/m68k-replay-bench"
    else
        RUSTFLAGS="${FLAGS[$layout]}" CARGO_TARGET_DIR="$ROOT/target/layout-$layout" \
            cargo build --release -q --manifest-path "$ROOT/Cargo.toml" --bin m68k-replay-bench
        bin="$ROOT/target/layout-$layout/release/m68k-replay-bench"
    fi
    line=$(taskset -c "$CPU" "$bin" "$@")
    echo "$layout: $line"
    seconds=$(sed -E 's/.* clocks, ([0-9.]+) s,.*/\1/' <<<"$line")
    if [ -z "$best" ] || awk "BEGIN{exit !($seconds < $best)}"; then
        best=$seconds
        best_layout=$layout
    fi
done
echo "min over layouts: $best s ($best_layout)"
