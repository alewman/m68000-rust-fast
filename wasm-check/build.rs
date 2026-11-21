//! Embed every manifest and reference trace of tests/data (rungs 1 and 5).
use std::fmt::Write;
use std::path::Path;

fn main() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/data");
    let mut manifests = Vec::new();
    for directory in [data.clone(), data.join("interrupts")] {
        println!("cargo:rerun-if-changed={}", directory.display());
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "json") {
                manifests.push(path.canonicalize().unwrap());
            }
        }
    }
    manifests.sort();
    let mut out = String::from("&[\n");
    for path in &manifests {
        let name = path.file_stem().unwrap().to_string_lossy();
        writeln!(
            out,
            "    ({name:?}, include_str!({:?}), include_str!({:?})),",
            path,
            path.with_extension("jsonl")
        )
        .unwrap();
    }
    out.push(']');
    let target = Path::new(&std::env::var("OUT_DIR").unwrap()).join("cases.rs");
    std::fs::write(target, out).unwrap();
}
