//! The long-run speed measure: the MAME game replays stepped bare, on a lean host.
//!
//! The conformance host (`m68000_rust::conformance`) checks bus-error ranges,
//! device windows and an access log on every access, and the manifest runner
//! captures the whole state before every boundary for its stop rules; a
//! profile of the replays run through it spent about 40% of its samples
//! there, not in the core. A board host decodes addresses with a compare or
//! two, so this benchmark runs the same manifests on a host that does only
//! what a replay manifest without bus errors or `at_pc` needs: flat memory,
//! the device windows answering from the reads stream, the TAS drop, and the
//! `ipl`/`reset` events at their steps. `--check` also runs each manifest
//! through the conformance runner and requires the same instruction and
//! clock counts.
//!
//! Prints instructions (boundaries), clocks, wall time (best of `--repeat`,
//! default 3, per manifest, summed), instructions per second and the
//! equivalent clock rate.
//!
//! Usage: `m68k-replay-bench [--repeat N] [--check] [manifest...]`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use m68000_rust::conformance::{load_manifest, trace_manifest, EventKind, Manifest, Start};
use m68000_rust::trace::{next_boundary, BoundaryKind};
use m68000_rust::{Bus, BusError, Cycle, M68000};

struct ReplayHost {
    memory: Vec<u8>,
    devices: Vec<(u32, u32)>,
    reads: Vec<u16>,
    next: usize,
    tas_drop: bool,
}

impl ReplayHost {
    #[inline]
    fn device(&self, address: u32) -> bool {
        self.devices
            .iter()
            .any(|&(start, length)| address.wrapping_sub(start) < length)
    }

    #[inline]
    fn replayed(&mut self) -> u16 {
        let value = self.reads.get(self.next).copied().unwrap_or(0xFFFF);
        self.next += 1;
        value
    }
}

impl Bus for ReplayHost {
    #[inline]
    fn read_byte(&mut self, address: u32, _cycle: Cycle) -> Result<u8, BusError> {
        if self.device(address) {
            return Ok(self.replayed() as u8);
        }
        Ok(self.memory[address as usize])
    }

    #[inline]
    fn read_word(&mut self, address: u32, _cycle: Cycle) -> Result<u16, BusError> {
        if self.device(address) {
            return Ok(self.replayed());
        }
        let a = address as usize;
        Ok(u16::from_be_bytes([
            self.memory[a],
            self.memory[(a + 1) & 0xFF_FFFF],
        ]))
    }

    #[inline]
    fn write_byte(&mut self, address: u32, value: u8, _cycle: Cycle) -> Result<(), BusError> {
        if !self.device(address) {
            self.memory[address as usize] = value;
        }
        Ok(())
    }

    #[inline]
    fn write_word(&mut self, address: u32, value: u16, _cycle: Cycle) -> Result<(), BusError> {
        if !self.device(address) {
            let a = address as usize;
            let [high, low] = value.to_be_bytes();
            self.memory[a] = high;
            self.memory[(a + 1) & 0xFF_FFFF] = low;
        }
        Ok(())
    }

    #[inline]
    fn tas_write(&mut self, address: u32, value: u8, cycle: Cycle) -> Result<(), BusError> {
        if self.tas_drop {
            return Ok(()); // the Genesis bus drops it
        }
        self.write_byte(address, value, cycle)
    }
}

/// One bare run of `manifest` on the lean host: (instructions, clocks).
fn run(manifest: &Manifest) -> (u64, u64) {
    let mut memory = vec![0u8; 1 << 24];
    for (address, data) in &manifest.memory {
        let start = *address as usize;
        memory[start..start + data.len()].copy_from_slice(data);
    }
    let host = ReplayHost {
        memory,
        devices: manifest.devices.clone(),
        reads: manifest.reads.clone(),
        next: 0,
        tas_drop: manifest.tas_drop,
    };
    let mut cpu = M68000::new(host);
    match &manifest.start {
        Start::Reset => {
            cpu.reset();
        }
        Start::State { state, fill_queue } => {
            let mut state = state.clone();
            if *fill_queue {
                let read = |cpu: &M68000<ReplayHost>, a: u32| {
                    let a = (a & 0xFF_FFFF) as usize;
                    u16::from_be_bytes([cpu.bus.memory[a], cpu.bus.memory[a + 1]])
                };
                state.ir = read(&cpu, state.pc);
                state.irc = read(&cpu, state.pc.wrapping_add(2));
            }
            cpu.restore_state(&state);
        }
    }
    let events = &manifest.events;
    let mut next_event = 0;
    let mut clocks = 0u64;
    let mut steps = 0u64;
    while steps < manifest.max_steps {
        while next_event < events.len() && events[next_event].at_step == steps {
            match events[next_event].kind {
                EventKind::Ipl(level) => cpu.set_ipl(level),
                EventKind::Reset => {
                    cpu.reset();
                }
            }
            next_event += 1;
        }
        if manifest.on_idle && (cpu.halted || cpu.stopped) && next_event == events.len() {
            let kind = next_boundary(&cpu.capture_state());
            if matches!(kind, BoundaryKind::HaltedIdle | BoundaryKind::StoppedIdle) {
                break;
            }
        }
        clocks += u64::from(cpu.step());
        steps += 1;
    }
    (steps, clocks)
}

fn main() {
    let mut repeat = 3;
    let mut check = false;
    let mut manifests: Vec<PathBuf> = Vec::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--repeat" => {
                repeat = arguments
                    .next()
                    .and_then(|n| n.parse().ok())
                    .expect("--repeat N");
            }
            "--check" => check = true,
            _ => manifests.push(argument.into()),
        }
    }
    if manifests.is_empty() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("external");
        manifests = ["replay-genesis", "replay-altbeast"]
            .iter()
            .map(|name| root.join(name).join("manifest.json"))
            .collect();
    }
    let (mut steps, mut clocks, mut seconds) = (0u64, 0u64, 0f64);
    for path in &manifests {
        let manifest = load_manifest(path).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            manifest.bus_error.is_empty() && manifest.at_pc.is_empty(),
            "{}: the lean host covers replay manifests without bus errors or at_pc",
            path.display()
        );
        let mut best = f64::MAX;
        let mut counts = (0, 0);
        for _ in 0..repeat {
            let started = Instant::now();
            counts = run(&manifest);
            best = best.min(started.elapsed().as_secs_f64());
        }
        if check {
            let reference = trace_manifest(&manifest, false, |_| {});
            assert_eq!(
                counts,
                (reference.steps, reference.clocks),
                "{}: the lean host's run differs from the conformance host's",
                path.display()
            );
            eprintln!("{}: same counts as the conformance host", path.display());
        }
        steps += counts.0;
        clocks += counts.1;
        seconds += best;
    }
    println!(
        "{steps} instructions, {clocks} clocks, {seconds:.3} s, {:.1} M instr/s, {:.0} MHz",
        steps as f64 / seconds / 1e6,
        clocks as f64 / seconds / 1e6
    );
}
