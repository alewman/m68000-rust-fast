# m68000-rust-fast

A fast fork of [m68000-rust](https://github.com/alewman/m68000-rust), the
Rust transcription of [m68000-python](https://github.com/alewman/m68000-python),
that stays provably equivalent to the reference at every processor
boundary: the same ladder, every rung, on every commit that touches `src/`.
Each optimization is one commit with its measurement; a change that does
not pay is reverted and recorded below as not kept.

The crate keeps the name `m68000_rust` and m68000-rust's API (`Bus`,
`Cycle`, `M68000<B>`, `step()`, `reset()`, `set_ipl()`, `set_pc()`,
`capture_state()`/`restore_state()`, `step_clocks`), so a host written for
m68000-rust compiles unchanged. The core builds without `std`
(`cargo build --no-default-features --lib`); the trace producer, the
conformance host and the binaries are behind the default `std` feature.
[docs/handoff-brief.md](docs/handoff-brief.md) is the request and its
reading, [docs/build-record.md](docs/build-record.md) what happened.

## Speed

The measure is `m68k-replay-bench`: the two MAME game replays of rung 3
(Genesis and System 16B Altered Beast, 52,850,830 instructions, interrupts
and device reads included) stepped bare on a lean host. The secondary
measure is `m68k-bench`, the reference's four synthetic workloads (copy
loop, memory, arithmetic, exceptions), so no change is judged on the
replays alone: 70% of the replays' instructions are one busy-wait shape
(`tst.w`/`move.w` of an absolute short address, then Bcc).

Every number below is from `scripts/ab.sh`: the base commit and the
candidate built under three code layouts (64- and 32-byte branch-target
alignment and none, z80-rust-fast's method) and run alternately on one core
of a shared i9-13900K, best of 5 or 7 each. The machine's load moved between
4 and 17 during this work, which makes separate runs incomparable; run
alternately, both sides see the same load. The ratio is the result.

| Commit | Change | Replays (min over layouts) | Synthetic |
| --- | --- | ---: | ---: |
| `60e7567` | baseline: the transcription and the build-time opcode table (0.462 s on the first lean host; 0.382 s on the current one) | -- | -- |
| `1b8dbbb` | `step()`: one combined test for the common case, the rest `#[cold]` | x1.110 | (not measured: the synthetic total line came after) |
| `0ce1db4` | inline the addressing-mode helpers, the sized read and the refills | x1.099 | x1.051 |
| `67c0972` | MOVE and TST specialized on the source addressing kind (const generic) | x1.082 | x1.009 |
| `9b443fe` | bench host: one device-window compare and a fixed-size memory (a harness change: every row is re-measurable with the same harness, since `ab.sh` copies the current bench into the base) | at this commit 0.287 s, 184 M instr/s; against `60e7567` with this harness x1.313 | x1.207 against `60e7567` |
| `ec30440` | MOVE specialized on its size as well as its source kind (36 instances) | x1.054 | x1.009 |
| | **at `ec30440` against `60e7567`, same harness** | **x1.384: 0.382 s to 0.276 s, 191 M instr/s** | **x1.217** |

Measured and not kept (each A/B against the commit it would have followed):

| Change | Replays | Synthetic | Why not |
| --- | ---: | ---: | --- |
| a compile-time table of handler function pointers per host type | -- | -- | const evaluation of 65,536 pointers ran over ten minutes |
| inlining only `extension`, `prefetch`, `jump` | x1.017 | x1.000 | below the noise |
| `GroupZero` packed into a `NonZeroU64`, so `Step` returns in a register | x0.933 | x1.025 | slower on the main measure |
| inlining `jump`, `read_program_word` and the bus read helpers as well | x0.982 | x0.959 | slower |
| ADD/SUB/AND/OR, CMP, CMPI, ADDQ/SUBQ, the unary group, MOVEA, LEA specialized like MOVE | x0.972 | x1.017 | slower on the main measure (code growth) |
| fat LTO, one codegen unit | x0.972 | x0.951 | slower |
| `execute()` inlined into `step()` | x0.909 | x0.967 | slower |
| TST specialized on size as well as kind | x0.968 | x1.009 | slower on the main measure |
| the fault's details kept in the CPU and an empty error type, so every `Result` returns in registers | x1.000 | x0.958 | no gain: returning through memory was not the cost |
| Bcc, MOVE and TST inlined into the dispatch `match` | x0.887 | x0.935 | slower |
| every handler forced out of line (the dispatch a bare jump table) | x0.993 | x1.000 | no gain |
| profile-guided optimization (`scripts/pgo.sh`: trained on the Genesis replay and the synthetic loops, System 16B held out), at `16e40a4` | held-out x0.978, both x0.996 | x1.139 (trained on them) | helps only the loops it was trained on; kept as an optional script, not the build |

**Where it stands.** At `ec30440` the replays run at about 191 M
instructions/s, roughly 29 machine cycles per emulated instruction with the
host included. Bcc's taken path is about 40 machine instructions, most of
them its two queue reads. The last experiments, which attacked dispatch and
call overhead from every side, were all neutral or slower: what remains is
each instruction's own bus accesses, flags, and the prefetch-queue and
clock bookkeeping the conformance contract requires at every boundary.
Going much further would mean a different kind of core (decoded-block
caching or translation), which would have to keep producing every
boundary and every access's `step_clocks` to stay on the ladder.

Through the full conformance host (bus-error and device range scans and the
access log on every access, a state capture per boundary) the same replays
took 0.744 s at the baseline; that host is for proving equivalence, not for
speed, and is unchanged.

## Certification

Pinned oracles, as m68000-python's `docs/conformance.md` requires:

| What | Version |
| --- | --- |
| m68000-python (reference core and conformance kit) | 0.1.0 plus the conformance kit at commit `b3df005` (branch `conformance-kit`, [m68000-python#5](https://github.com/alewman/m68000-python/pull/5): main `38dbd0f` + the kit + the reader fix this port found) |
| Trace schema | version 1 |
| SingleStepTests/m68000 corpus | revision `64b253116a3de04aaac4346c43680960dc9b67e5` |
| SingleStepTests/680x0 (rung 6, a detector) | revision `e0d5ece9670205cc84a0101081837deb446f86a3`, the `68000/v1` files |
| flamewing/68k-bcd-verifier | commit `39a01be528b0744302bf1dc9b3463fc22a3fc45f`, table SHA-256 `8432868c…80147e5` |

Ladder status at `67c0972` (and on every commit above, by `scripts/ladder.sh`), each rung reproduced with the script named
(`scripts/`) on Linux x86_64 with rustc 1.93.1, CPython 3.14.4 and PyPy
7.3.20 / Python 3.11.13:

| Rung | What | Result | Script |
| --- | --- | --- | --- |
| 1 | The example manifests of the reference's kit diff clean | `exceptions`, `flags-and-branches`, `tas-drop`: `traces are identical` | `rung1.sh` |
| 2 | SingleStepTests/m68000, 127 files: registers, SR, both stack pointers, the prefetch address and pair, RAM, clocks, every bus transaction with strobes and function code, and `step_clocks` at every access end of the cases without an address error | `TOTAL: 317500 passed, 0 failed / 317500 cases in 127 files; step_clocks checked in 261894` | `rung2.sh` |
| 3 | The reference's two MAME 0.285 lockstep recordings as `replay` manifests (real game code, its interrupts, every device read replayed), diffed in lockstep through a pipe | System 16B `altbeast`, 30 s: 24,597,382 records, 263,504,451 clocks; Genesis Altered Beast, 40 s: 28,253,448 records, 306,763,197 clocks; both `traces are identical`, 5 min 6 s and 5 min 49 s wall with the reference on PyPy | `rung3.sh` |
| 4 | ABCD, SBCD and NBCD over every input against flamewing's hardware-verified table (T1) | `BCD tables: all 525,312 inputs agree` | `rung4.sh` |
| 5 | The interrupt and STOP scenarios as manifests with events (`examples/conformance/interrupts/` in the reference) | all 20 `traces are identical` | `rung5.sh` |
| 6 | SingleStepTests/680x0 (Tom Harte's corpus, CLK-generated, T3) as a detector: every disagreement sorted into the reference's named causes, and the table required to be the reference's line for line, as z80-rust's FUSE rung requires FUSE's six divergences | `787,660 of 1,000,060 cases agree`, 19 causes, identical to the reference's `scripts/classify_680x0.py` (9 s here, 33 s for the reference on PyPy) | `rung6.sh` |

A gate that cannot fail proves nothing. Rung 6 fails on the first ADD.w
case when V is never set for ADD.w, and rung 2 was checked against two
seeded faults: V never set for ADD.w fails 250 cases, and TAS's write moved
two clocks earlier (same total) fails 2,108 on `step_clocks` alone.

CI (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy -D
warnings` and `cargo test` (which replays the example manifests against
copies of the reference traces without Python), and rungs 2 and 4 on every
push; rungs 1 and 5 need the reference checked out (below).

The replay manifests are made from ROMs and MAME traces, so neither
repository commits them: `python validation/lockstep.py export` in the
reference builds them from its recorded runs (its `docs/conformance.md`,
rung 3). The export follows MAME's registers before every instruction of
both games, so the replayed machine is the game, not an approximation of it.

### Reproducing

```text
scripts/fetch_m68000python.sh        # m68000-python at the pin into external/, with a venv
scripts/fetch_test_vectors.sh        # SingleStepTests/m68000 at the pinned revision into external/
scripts/rung1.sh
scripts/rung2.sh
scripts/rung3.sh                     # needs the replay manifests, below
scripts/rung4.sh
scripts/rung5.sh
scripts/fetch_680x0.sh               # SingleStepTests/680x0 (68000/v1) at the pinned revision
scripts/rung6.sh
```

## On WebAssembly (phones and browsers)

The core builds `no_std` for `wasm32-unknown-unknown`, and
`scripts/wasm-check.sh` proves it there rather than assuming the native
results carry over: it builds `wasm-check/` (a 0.9 MB module with every
manifest and reference trace of `tests/data` built in) and runs it under
Node. Inside WebAssembly, at `v0.1.0` (2026-10-03):

| Check | Result |
| --- | --- |
| Rung 4: ABCD, SBCD, NBCD over every input against flamewing's hardware-verified table (SHA-256) | all 525,312 inputs agree |
| Rungs 1 and 5: the 3 example and 20 interrupt manifests, each record against the reference trace | 23 of 23 match |
| The reference's "base" benchmark loop | about 112 M instr/s under Node 22 on the i9 (native: 134 M) |

Two extra clocks on every taken Bcc make `flags-and-branches` fail and the
script exit 1, so the check bites. Rungs 2, 3 and 6 run natively only; the
module runs the same source. A phone will be slower than this desktop; how
much is what testing there will show.

## Using the core

```rust
use m68000_rust::{Bus, BusError, Cycle, M68000};

struct Flat(Vec<u8>);

impl Bus for Flat {
    fn read_byte(&mut self, a: u32, _: Cycle) -> Result<u8, BusError> { Ok(self.0[a as usize]) }
    fn read_word(&mut self, a: u32, _: Cycle) -> Result<u16, BusError> {
        Ok(u16::from_be_bytes([self.0[a as usize], self.0[a as usize + 1]]))
    }
    fn write_byte(&mut self, a: u32, v: u8, _: Cycle) -> Result<(), BusError> {
        self.0[a as usize] = v;
        Ok(())
    }
    fn write_word(&mut self, a: u32, v: u16, _: Cycle) -> Result<(), BusError> {
        self.0[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
}

let mut memory = vec![0u8; 1 << 24];
memory[0x1000..0x1002].copy_from_slice(&[0x70, 0x2A]); // moveq #42, D0
let mut cpu = M68000::new(Flat(memory));
cpu.set_pc(0x1000).unwrap();
assert_eq!(cpu.step(), 4);
assert_eq!(cpu.r[0], 42);
```

The embedding contract is the reference's (its README, "The embedding
contract"), in Rust form:

- The host implements `Bus`: four accessors over a 24-bit address space,
  each told the access's function code and `step_clocks` (where in the step
  the access ends) in a `Cycle`. Returning `Err(BusError)` asserts BERR.
- Its provided methods are the reference's optional hooks, with the same
  defaults: `acknowledge(level)` (autovector), `tas_write` (an ordinary
  byte write; the Genesis drops it), `address_error` and `reset_devices`.
- `step()` runs one instruction or exception entry and returns its clocks;
  `reset()`, `set_ipl()`, `set_pc()`, `capture_state()` and `restore_state()`
  are the reference's.

## Notes for the reference

Rung 3 found one bug, in the reference's trace reader rather than its core:
`disassemble_bytes` keyed an instruction's words at the full address while
the disassembler reads at the 24-bit one, so any record at a PC above
`$FFFFFF` was rejected. Genesis Altered Beast runs code at `$FFFFF200`,
reached through a sign-extended short jump. The test and fix are commits
`f7dec3f` and `b3df005` on the reference's `conformance-kit` branch. No
divergence between the two cores was found at any rung.

## Binaries

- `m68k-trace <manifest>`: run a conformance manifest and stream its trace
  for `python -m m68000_python.conformance diff manifest -`.
- `m68k-vectors <dir>`: the SingleStepTests/m68000 runner (rung 2).
- `m68k-bcd [table]`: the BCD table check (rung 4).
- `m68k-680x0 <dir>`: the 680x0 corpus classified as the reference classifies it (rung 6).
- `m68k-bench`: the reference's four benchmark workloads.
- `scripts/wasm-check.sh`: the WebAssembly check above.
- `m68k-replay-bench [--check]`: the replays on a lean host, the speed
  measure; `--check` requires the same counts as the conformance host.

Scripts beyond the rungs: `scripts/ladder.sh` (every rung, about 6.5
minutes), `scripts/ab.sh BASE` (the A/B above; `BENCH=synthetic` for the
secondary measure), `scripts/bench.sh` (absolute numbers under three
layouts, for a quiet machine).

## License

MIT, like the reference. The SingleStepTests/m68000 corpus (MIT) and
flamewing's verifier (GPL-3.0; only the SHA-256 of its table is used) are
external and not bundled.
