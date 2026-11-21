# The brief this repository was built from

## The request (2026-10-02)

After m68000-rust was published (private), the user: "After that is done I
will want you to do the optimized version like z80-rust-fast".

## How the session read it

z80-rust-fast's brief (in that repository's `docs/handoff-brief.md`) is the
model, transposed to the 68000:

- **Fork with history.** m68000-rust's transcription is the first commit, so
  every optimization is a diff against it. Package `m68000-rust-fast`; the
  crate keeps the name `m68000_rust`.
- **What must not change:** the `Bus` trait and `Cycle`, `M68000<B>`,
  `step()`, `reset()`, `set_ipl()`, `set_pc()`, `capture_state()` and
  `restore_state()`, `step_clocks`, the hooks and their defaults: a host
  written for m68000-rust compiles unchanged (additions are fine). The trace
  producer, the conformance host, `tests/data/` and every `scripts/rung*.sh`
  keep working unchanged. Behavior at every boundary: kind, clocks,
  instruction bytes, all fourteen `CpuState` fields, and every bus access in
  order, with `step_clocks` at each.
- **What may change:** everything else: dispatch, flag computation, register
  representation, inlining, cold paths, the build profile, the module
  layout. Readability against the Python stops being a requirement; each
  optimization is explained in its commit message with its measurement.
- **Milestones.** (1) Fork, baseline and harness, plus a `no_std` split of
  the core (`trace` and `conformance` behind a default `std` feature), since a
  WebAssembly build is the likely next step. (2) Profile before touching
  anything. (3) Optimize one change per commit, ladder-gated. (4) A README
  with the pins, which rungs pass at which commit, and a table of every
  optimization with its measured effect.

## What is different from the Z80

- **The ladder is cheap.** z80-rust-fast's rung 3 (ZEXALL and ZEXDOC) took
  about 7 hours each with 30 PyPy processes, so it ran only before a tag.
  Here rung 3 is the two MAME game replays, about 6 minutes each with one
  PyPy process, so **every rung runs on every commit that touches `src/`**.
- **The benchmark is real code.** The long-run measure is the two replay
  manifests (52.8 million instructions of Altered Beast, interrupts and
  device reads included) stepped bare, plus the reference's four synthetic
  workloads as secondary loops.
- **Timing is part of the contract.** The 68000 core counts clocks as it
  goes and hosts read `step_clocks` inside bus calls, so an optimization that
  reorders an access or batches clocks must keep every access's
  `step_clocks`; rung 2 checks it in 261,894 cases.

## Constraints carried over

Every claim pinned to a commit and reproduced; "the whole ladder" means
every rung. Shared machine: measure with nothing else building, pinned to
one core, under several code layouts (z80-rust-fast's method). Commit
locally; push when the user asks; the repository is private.
