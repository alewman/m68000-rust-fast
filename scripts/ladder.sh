#!/usr/bin/env bash
# Every rung, in order, against the reference checkout in external/.
# Rung 3 (the two game replays, about 6 minutes each with PyPy) runs both
# replays in parallel; SKIP3=1 leaves it out (and then this is not "the
# whole ladder").
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REFERENCE=${REFERENCE:-"$ROOT/external/m68000-python"}
PY=${PY:-/data/emu/m68000-python/.venv/bin/python}
PYPY=${PYPY:-/data/emu/m68000-python/.venv-pypy/bin/pypy3}
echo "commit $(git -C "$ROOT" rev-parse --short HEAD), reference $(git -C "$REFERENCE" rev-parse --short HEAD)"
start=$(date +%s)
"$ROOT/scripts/rung1.sh" "$REFERENCE" "$PY"
"$ROOT/scripts/rung2.sh" "$ROOT/external/m68000_test_vectors/v1" | tail -1
"$ROOT/scripts/rung4.sh"
"$ROOT/scripts/rung5.sh" "$REFERENCE" "$PY" | tail -1
if [ "${SKIP3:-0}" != 1 ]; then
    for game in genesis altbeast; do
        "$ROOT/scripts/rung3.sh" "$ROOT/external/replay-$game/manifest.json" "$REFERENCE" "$PYPY" \
            > "$ROOT/target/rung3-$game.log" 2>&1 &
    done
    wait
    cat "$ROOT"/target/rung3-*.log
    grep -q "traces are identical" "$ROOT/target/rung3-genesis.log"
    grep -q "traces are identical" "$ROOT/target/rung3-altbeast.log"
fi
echo "ladder clean in $(( $(date +%s) - start )) s"
