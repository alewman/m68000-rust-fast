//! The long-run speed measure: replay manifests stepped bare.
//!
//! Runs each manifest given (by default the two MAME game replays of rung 3)
//! through the conformance host with no records and no access log, the same
//! loop `m68k-trace --no-trace` runs, and prints instructions (boundaries),
//! clocks, wall time, instructions per second and the equivalent clock rate.
//! Each manifest runs `--repeat` times (default 3); the best time counts.
//!
//! Usage: `m68k-replay-bench [--repeat N] [manifest...]`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use m68000_rust::conformance::{load_manifest, trace_manifest};

fn main() {
    let mut repeat = 3;
    let mut manifests: Vec<PathBuf> = Vec::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--repeat" {
            repeat = arguments
                .next()
                .and_then(|n| n.parse().ok())
                .expect("--repeat N");
        } else {
            manifests.push(argument.into());
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
        let mut best = f64::MAX;
        let mut run = None;
        for _ in 0..repeat {
            let started = Instant::now();
            let result = trace_manifest(&manifest, false, |_| {});
            best = best.min(started.elapsed().as_secs_f64());
            run = Some(result);
        }
        let run = run.unwrap();
        steps += run.steps;
        clocks += run.clocks;
        seconds += best;
    }
    println!(
        "{steps} instructions, {clocks} clocks, {seconds:.3} s, {:.1} M instr/s, {:.0} MHz",
        steps as f64 / seconds / 1e6,
        clocks as f64 / seconds / 1e6
    );
}
