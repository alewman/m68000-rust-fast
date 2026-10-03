//! The checks that matter on a phone, run inside WebAssembly.
//!
//! Exports, called by scripts/wasm-check.mjs (Node) or any page:
//! - `bcd_ok()`: ABCD, SBCD and NBCD over every input in bcd-gen's layout,
//!   1 when the table's SHA-256 is flamewing's hardware-verified table's
//!   (rung 4, as `m68k-bcd` does natively);
//! - `manifest_count()` and `manifest_failures()`: the 3 example and 20
//!   interrupt manifests of tests/data run through the conformance host, each
//!   record compared with the reference trace (rungs 1 and 5, as
//!   tests/rung1.rs does natively); the failing manifests' names go to
//!   `report()`;
//! - `base_loop(n)`: the reference's "base" benchmark program for n
//!   instructions, for a speed figure on the device.

use std::path::Path;

use m68000_rust::conformance::{manifest_from_json, trace_manifest};
use m68000_rust::trace::record_to_json;
use m68000_rust::{Bus, BusError, CpuState, Cycle, M68000};
use serde_json::Value;

const CASES: &[(&str, &str, &str)] = include!(concat!(env!("OUT_DIR"), "/cases.rs"));
const TABLE_SHA256: &str = "8432868c9aa93c92574bae48bebd4efb2834e298340eb09530625365b80147e5";

static mut REPORT: String = String::new();

/// Every word reads as NOP, for the prefetch; writes go nowhere.
struct Nops;

impl Bus for Nops {
    fn read_byte(&mut self, _: u32, _: Cycle) -> Result<u8, BusError> {
        Ok(0x4E)
    }
    fn read_word(&mut self, _: u32, _: Cycle) -> Result<u16, BusError> {
        Ok(0x4E71)
    }
    fn write_byte(&mut self, _: u32, _: u8, _: Cycle) -> Result<(), BusError> {
        Ok(())
    }
    fn write_word(&mut self, _: u32, _: u16, _: Cycle) -> Result<(), BusError> {
        Ok(())
    }
}

fn bcd_case(cpu: &mut M68000<Nops>, opcode: u16, destination: u32, source: u32, x: bool, z: bool) -> [u8; 2] {
    let mut d = [0; 8];
    d[0] = destination;
    d[1] = source;
    cpu.restore_state(&CpuState {
        d,
        sr: 0x2700 | if x { 0x11 } else { 0 } | if z { 0x04 } else { 0 },
        pc: 0x1000,
        ir: opcode,
        irc: 0x4E71,
        ..CpuState::default()
    });
    cpu.step();
    [(cpu.sr & 0x1F) as u8, cpu.r[0] as u8]
}

#[no_mangle]
pub extern "C" fn bcd_ok() -> u32 {
    let mut cpu = M68000::new(Nops);
    let mut table = Vec::with_capacity(1_050_624);
    for opcode in [0xC101u16, 0x8101] {
        for source in 0..256 {
            for destination in 0..256 {
                for x in [false, true] {
                    for z in [false, true] {
                        table.extend(bcd_case(&mut cpu, opcode, destination, source, x, z));
                    }
                }
            }
        }
    }
    for operand in 0..256 {
        for x in [false, true] {
            for z in [false, true] {
                table.extend(bcd_case(&mut cpu, 0x4800, operand, 0, x, z));
            }
        }
    }
    u32::from(sha256(&table) == TABLE_SHA256)
}

fn manifest_matches(manifest: &str, reference: &str) -> bool {
    let value: Value = serde_json::from_str(manifest).unwrap();
    let Ok(loaded) = manifest_from_json(&value, Path::new(".")) else {
        return false;
    };
    let mut produced: Vec<Value> = Vec::new();
    trace_manifest(&loaded, true, |record| {
        produced.push(serde_json::from_str(&record_to_json(&record)).unwrap());
    });
    let mut expected: Vec<Value> = reference
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for record in &mut expected {
        if let Some(instruction) = record["instruction"].as_object_mut() {
            instruction.remove("mnemonic");
            instruction.remove("operands");
        }
    }
    produced == expected
}

#[no_mangle]
pub extern "C" fn manifest_count() -> u32 {
    CASES.len() as u32
}

#[no_mangle]
pub extern "C" fn manifest_failures() -> u32 {
    let mut failures = 0;
    let mut report = String::new();
    for (name, manifest, reference) in CASES {
        if !manifest_matches(manifest, reference) {
            failures += 1;
            report.push_str(name);
            report.push('\n');
        }
    }
    // SAFETY: the module is single-threaded; nothing else touches REPORT.
    unsafe { REPORT = report };
    failures
}

/// The failing manifests' names: (pointer, length) packed as ptr << 32 | len.
#[no_mangle]
pub extern "C" fn report() -> u64 {
    // SAFETY: as in manifest_failures.
    let report = unsafe { &*std::ptr::addr_of!(REPORT) };
    ((report.as_ptr() as u64) << 32) | report.len() as u64
}

struct Flat(Vec<u8>);

impl Bus for Flat {
    fn read_byte(&mut self, a: u32, _: Cycle) -> Result<u8, BusError> {
        Ok(self.0[(a & 0xFFFF) as usize])
    }
    fn read_word(&mut self, a: u32, _: Cycle) -> Result<u16, BusError> {
        let a = (a & 0xFFFE) as usize;
        Ok(u16::from_be_bytes([self.0[a], self.0[a + 1]]))
    }
    fn write_byte(&mut self, a: u32, v: u8, _: Cycle) -> Result<(), BusError> {
        self.0[(a & 0xFFFF) as usize] = v;
        Ok(())
    }
    fn write_word(&mut self, a: u32, v: u16, _: Cycle) -> Result<(), BusError> {
        let a = (a & 0xFFFE) as usize;
        self.0[a..a + 2].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
}

/// The reference benchmark's "base" program (a copy loop through (An)+ and
/// DBF) for `count` instructions; returns the clocks, so nothing is elided.
#[no_mangle]
pub extern "C" fn base_loop(count: u32) -> u32 {
    #[rustfmt::skip]
    let program: [u16; 18] = [
        0x41F9, 0x0000, 0x2000, 0x43F9, 0x0000, 0x3000, 0x303C, 0x00FF, 0x3218,
        0xD241, 0x32C1, 0x0C41, 0x1234, 0x6702, 0x5282, 0x51C8, 0xFFF0, 0x60E2,
    ];
    let mut memory = vec![0u8; 0x10000];
    for (index, word) in program.iter().enumerate() {
        memory[0x1000 + 2 * index..0x1002 + 2 * index].copy_from_slice(&word.to_be_bytes());
    }
    let mut cpu = M68000::new(Flat(memory));
    cpu.set_pc(0x1000).unwrap();
    let mut clocks = 0u32;
    for _ in 0..count {
        clocks = clocks.wrapping_add(cpu.step());
    }
    clocks
}

/// SHA-256 (FIPS 180-4), so the gate needs no dependency.
fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in message.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes(word.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (state, value) in h.iter_mut().zip(v) {
            *state = state.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

