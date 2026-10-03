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
