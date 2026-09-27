//! The Material Symbols in `assets/icons/material` as `(name, svg)` pairs.
use std::io::Write;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/material");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".svg")).collect())
        .unwrap_or_default();
    names.sort();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs");
    let mut f = std::fs::File::create(out).unwrap();
    writeln!(f, "pub static ICONS: &[(&str, &str)] = &[").unwrap();
    for n in names {
        let path = dir.join(&n).canonicalize().unwrap();
        writeln!(f, "    ({:?}, include_str!({:?})),", n.trim_end_matches(".svg"), path.display().to_string()).unwrap();
    }
    writeln!(f, "];").unwrap();
}
