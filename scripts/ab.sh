#!/usr/bin/env bash
# A/B the replay benchmark: a base revision against the working tree,
# alternately, on one core, best of ROUNDS each, under each layout.
#
# Why: the machine is shared, and separate runs move by tens of percent with
# other people's load. Two builds timed alternately on one core see the
# same load, and interference only ever slows a sample, so best-of recovers
# the undisturbed rate. The ratio is the result. (z80-python's
# benchmarks/compare_revisions.py, the same reasoning.)
#
# Usage: scripts/ab.sh [BASE (default HEAD)]
#   env: CPU (default 4), ROUNDS (default 7), LAYOUTS (default "default noalign nf5"),
#        BENCH (replay, the default, or synthetic: m68k-bench's four workloads)
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BASE=${1:-HEAD}
CPU=${CPU:-4}
ROUNDS=${ROUNDS:-7}
LAYOUTS=${LAYOUTS:-"default noalign nf5"}
BENCH=${BENCH:-replay}
BIN=m68k-replay-bench
[ "$BENCH" = synthetic ] && BIN=m68k-bench
declare -A FLAGS=(
    [default]=""
    [noalign]="-C opt-level=3"
    [nf5]="-C llvm-args=-align-all-nofallthru-blocks=5"
)
BASE_TREE="$ROOT/target/ab-base"
rev=$(git -C "$ROOT" rev-parse --short "$BASE")
if [ ! -d "$BASE_TREE" ]; then
    git -C "$ROOT" worktree add -q --detach "$BASE_TREE" "$rev"
fi
git -C "$BASE_TREE" checkout -q --detach "$rev"
ln -sfn "$ROOT/external" "$BASE_TREE/external"
build() { # tree layout -> binary path
    local tree=$1 layout=$2 target
    target="$tree/target/ab-$layout"
    if [ "$layout" = default ]; then
        CARGO_TARGET_DIR="$target" cargo build --release -q --manifest-path "$tree/Cargo.toml" --bin "$BIN"
    else
        RUSTFLAGS="${FLAGS[$layout]}" CARGO_TARGET_DIR="$target" \
            cargo build --release -q --manifest-path "$tree/Cargo.toml" --bin "$BIN"
    fi
    echo "$target/release/$BIN"
}
seconds() { if [ "$BENCH" = synthetic ]; then sed -nE 's/^total .* ([0-9.]+) s$/\1/p'; else sed -E 's/.* clocks, ([0-9.]+) s,.*/\1/'; fi; }
echo "$BENCH: base $rev vs working tree, cpu $CPU, $ROUNDS rounds, load $(cut -d' ' -f1 /proc/loadavg)"
best_base_all=""; best_new_all=""
for layout in $LAYOUTS; do
    a=$(build "$BASE_TREE" "$layout")
    b=$(build "$ROOT" "$layout")
    best_a=""; best_b=""
    for _ in $(seq "$ROUNDS"); do
        for side in a b; do
            bin=$a; [ $side = b ] && bin=$b
            if [ "$BENCH" = synthetic ]; then
                t=$(taskset -c "$CPU" "$bin" 5000000 1 | seconds)
            else
                t=$(taskset -c "$CPU" "$bin" --repeat 1 "$ROOT/external/replay-genesis/manifest.json" "$ROOT/external/replay-altbeast/manifest.json" | seconds)
            fi
            if [ $side = a ]; then
                [ -z "$best_a" ] || awk "BEGIN{exit !($t < $best_a)}" && best_a=$t
            else
                [ -z "$best_b" ] || awk "BEGIN{exit !($t < $best_b)}" && best_b=$t
            fi
        done
    done
    echo "$layout: base $best_a s, new $best_b s, x$(awk "BEGIN{printf \"%.3f\", $best_a/$best_b}")"
    [ -z "$best_base_all" ] || awk "BEGIN{exit !($best_a < $best_base_all)}" && best_base_all=$best_a
    [ -z "$best_new_all" ] || awk "BEGIN{exit !($best_b < $best_new_all)}" && best_new_all=$best_b
done
echo "min over layouts: base $best_base_all s, new $best_new_all s, x$(awk "BEGIN{printf \"%.3f\", $best_base_all/$best_new_all}")"
