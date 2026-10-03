//! Build the 65,536-entry dispatch table from `src/opcode_map.rs`.
//!
//! The rules are evaluated for every first word here, once per build, and
//! written out as a literal array that `dispatch.rs` includes. A word that
//! matches two rules panics `decode`, which fails the build.

use std::fmt::Write;

#[path = "src/ea_kinds.rs"]
#[allow(dead_code)]
mod ea_kinds;
#[path = "src/opcode_map.rs"]
#[allow(dead_code)]
mod opcode_map;

fn main() {
    println!("cargo:rerun-if-changed=src/ea_kinds.rs");
    println!("cargo:rerun-if-changed=src/opcode_map.rs");
    let mut out = String::from("[");
    for opcode in 0..=0xFFFFu16 {
        write!(out, "Op::r#{:?},", opcode_map::decode(opcode)).unwrap();
    }
    out.push(']');
    let path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("names.rs");
    std::fs::write(path, out).unwrap();
}
