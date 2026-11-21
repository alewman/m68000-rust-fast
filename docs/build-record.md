# Build record

What happened when [handoff-brief.md](handoff-brief.md) was run, in order.
All on 2026-10-02 (PDT), i9-13900K shared with other people's jobs (load 2
to 17 during the session). The commit messages carry each measurement.

- **Fork** (`d0ee649`): history kept from m68000-rust; package renamed, crate
  not. The opcode table moved from a first-use `OnceLock` to `build.rs`
  (needed for `no_std`; a `const` table worked but added 34 s to every
  build); a word matching two rules still fails the build. The whole ladder
  ran in 387 s.
- **Measure** (`60e7567`): profiled through the conformance host, 40% of the
  replays' samples were the host and the manifest runner, not the core; the
  benchmark got a lean host of its own (`--check` proves it runs the same
  instructions and clocks). Baseline 0.462 s.
- **Machine load.** Separate bench runs stopped being comparable when the
  load rose past 10, so `scripts/ab.sh` was written: base and candidate
  alternately on one core, best-of. Every later number is a ratio from it.
- **Kept:** the `step()` fast path (x1.110), inlining the addressing-mode
  helpers (x1.099 replays, x1.051 synthetic), MOVE and TST specialized on
  the source kind (x1.082, x1.009). In all x1.314 on the replays.
- **Not kept,** each measured: a compile-time handler-pointer table (built
  too slowly), three narrower or wider inlining sets, a packed `GroupZero`,
  wider kind specialization, fat LTO. The README's table has the numbers.
- **Ladder:** rungs 1-5 clean on every kept commit, rung 3 included.
- **Tool note:** a `pkill -f` on a cargo pattern killed the session's own
  shell (exit 144), as the workspace notes warn; stop builds by PID.

## 2026-10-03: the optimization project resumed

- Merged m68000-rust's rung 6 and pin; the fork's ladder runs all six
  rungs (about 7 minutes) and is clean on every kept commit.
- The bench host again dominated `jump()`'s profile; it now decodes one
  device window and indexes a fixed array (`9b443fe`), and `ab.sh` gives
  both sides the same harness. Against the fork's baseline with that
  harness the earlier work measured x1.313 (replays), x1.207 (synthetic).
- Kept: MOVE specialized on size (x1.054). Not kept, each measured:
  `execute` inlined into `step`, TST on size, PGO (helps only what it was
  trained on), the fault record in the CPU with an empty error type (the
  dispatch redesign's cheap form: x1.000), the hot handlers inlined into
  dispatch, every handler out of line.
- `ab.sh` once timed stale binaries after a failed build; it now stops on a
  build failure, and those numbers were discarded.
- Final: x1.384 replays, x1.217 synthetic against `60e7567`, same harness.

## 2026-10-03: ready for phones, v0.1.0

- The user will test on phones and keep this core unless it is too slow.
  The core builds `no_std` for wasm32; `scripts/wasm-check.sh` runs the BCD
  rung and rungs 1 and 5 inside WebAssembly under Node: all agree, 23 of 23
  manifests, about 112 M instr/s. A planted Bcc timing fault fails it; a
  planted word-ADD carry fault does not (no manifest has such an ADD;
  rung 2 covers it natively).
- Tagged `v0.1.0`: the core is `ec30440`'s, with the whole native ladder
  clean on it.
