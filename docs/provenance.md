# Provenance

Audit of 2026-11-13 (provenance audit wave 1). This crate is a speed fork of
m68000-rust, itself a transcription of m68000-python (all MIT, same author).
It inherits their verdicts: see m68000-python's `docs/provenance.md` and
m68000-rust's. Only this fork's own additions were checked. No GPL source was
opened.

## Inherited

| Piece | Verdict |
| --- | --- |
| `src/alu.rs` `divide_unsigned_clocks`, `divide_signed_clocks` | **Rewritten 2026-11-19 (wave 2)**: old bodies moved aside unread (`trash/`, git-ignored); transcribed from m68000-python `7b11f4d` (via m68000-rust `7eb17ca`): a shift-and-subtract rule FITTED to SingleStepTests/m68000 (MIT), within UM Table 8-4. **clean** |
| divide-by-zero flags (`src/system.rs`) | **Rewritten 2026-11-19** from m68000-python's rewrite (closed-box WinUAE run results, OBSERVED); old body moved aside unread. **OBSERVED; accepted by the owner 2026-11-19** (a closed-box basis is acceptable where no permitted source defines it). |
| `src/bcd.rs` | cite (Musashi MIT), as m68000-python. |
| `src/core.rs`, `src/cpu.rs`, and the hidden-state steps in `ea.rs`, `loads.rs`, `bits.rs`, `bcd.rs`, `control.rs`, `alu.rs`, `flags.rs`, `system.rs` | **Re-sourced 2026-11-19 (wave 2, second pass)**, carried down from m68000-python through m68000-rust: nothing rests on MAME's header-less microcode files. Each hidden-state step is pinned by SingleStepTests/m68000 (37 of 43, the reference's `scripts/ablate_hidden_state.py`), by WinUAE run as a closed box (2, OBSERVED), or is the boundary model (2, HYPOTHESIS); the 2 pinned by nothing (the MOVE.L -(An) and PEA decoder hand-overs in `loads.rs`) are removed. The E-clock rule cites MAME's `m68000.cpp` (BSD-3-Clause). **clean**; `cpu.rs` **cite** (NOTICE). |
| the rest of `src/` | clean. |

## Own additions

| Piece | Source | Verdict |
| --- | --- | --- |
| `build.rs`, `src/opcode_map.rs`, `src/ea_kinds.rs` | the opcode rules moved to build time; own | clean |
| Specialised MOVE/TST handlers, inlining, `no_std` core | own restructuring of the transcription | clean |
| `src/bin/m68k-replay-bench.rs`, `scripts/ab.sh`, `bench.sh`, `pgo.sh`, `ladder.sh`, `wasm-check.*` | own tooling | clean |
| `scripts/fetch_*.sh`, `tests/data/*` | as m68000-rust (SST MIT; 680x0 no licence, run only; flamewing GPL-3.0 run, hash only; own manifests) | clean |
| SPDX headers | `// SPDX-License-Identifier: MIT` on every `src/`, `tests/` file and `build.rs` (2026-11-19) | clean |

Counts: clean 4 (own additions) + inherited (after wave 2: no rewrite, no unclear; cite: Musashi in `bcd.rs`, MAME `m68000.cpp` in `cpu.rs`).

## History

Public (alewman/m68000-rust-fast). The divide clocks are in every commit
since `e00bb12` (2026-10-02), on `main` (origin) and in tag `v0.1.0`. Not on
crates.io. Compiled into megadrive-rust-fast and so into the play-md site's
WebAssembly build: when the divide clocks are rewritten, that build should be
redeployed.

## Wave 2 (2026-11-19)

In place. `scripts/ladder.sh` against m68000-python `7b11f4d` (local, not yet
pushed), whole ladder including rung 3, clean in 3,124 s on a machine at
load ~90: rung 1 identical (3 manifests), rung 2 317,500 / 317,500, rung 4
525,312 agree, rung 5 20 identical, rung 6 the reference's table line for
line, rung 3 both replays identical (24,597,382 and 28,253,448 records).
Before: rung 2 317,500 / 317,500 on the old code. The divide functions give
the Python's value on all 633,847 inputs of an equivalence check (random,
small quotients, quotient -32768). `scripts/wasm-check.sh`: BCD 525,312
agree, 23 of 23 manifests, 57.6 M instr/s (load-affected; no A/B speed run
was made, the machine being shared and loaded).

Downstream: megadrive-rust-fast pins this crate by git rev (`b6af29a`); it
needs a repin to this commit once pushed, its lockstep gate, then a play-md
rebuild and redeploy.

Also inherited from m68000-python's wave 2 and **resolved** in its second
pass (below): what the core said it learned from MAME 0.285's microcode
rested partly on MAME files with no licence header (`m68000gen.py`,
`m68000.lst`, the generated handlers), GPL under MAME's COPYING.

## Wave 2, second pass (2026-11-19): MAME's header-less files

The same source change as m68000-rust's (applied as its diff, which applied
cleanly here): comments cite the corpus, the manuals, closed-box runs or
MAME's BSD-3-Clause `m68000.cpp` instead of the microcode, and the two
decoder hand-overs pinned by nothing are removed from `loads.rs`. No rung
reaches the one behaviour they could show (the IR of a bus error on
MOVE.L -(An)'s second write, outside the contract). Not a speed change: no
A/B was needed (the removed statements were stores to a field the next
boundary overwrites).

`scripts/ladder.sh` against m68000-python `27c3792`, whole ladder including
rung 3, clean in 1,052 s on a shared machine at load ~30: rung 1 identical,
rung 2 317,500 / 317,500 (with `step_clocks`), rung 4 525,312 agree, rung 5
20 identical, rung 6 the reference's table line for line, rung 3 both
replays identical (24,597,382 and 28,253,448 records). Before: this crate's
`fb4ef51`, ladder clean against `7b11f4d` (Wave 2, above); not rerun, the
machine being loaded (m68000-rust's rungs were rerun before and after,
identical). `scripts/wasm-check.sh`: BCD 525,312 agree, 23 of 23 manifests,
104.6 M instr/s (load-affected; no speed claim). clippy and fmt clean.

`scripts/fetch_m68000python.sh` pins m68000-python `27c3792` (local until
its history rewrite and push).
