use std::io::Write;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/i18n");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("locales.rs");
    let mut f = std::fs::File::create(out).unwrap();
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    writeln!(f, "pub static LOCALES: &[(&str, &str)] = &[").unwrap();
    for n in names {
        let path = dir.join(&n).canonicalize().unwrap();
        writeln!(
            f,
            "    ({:?}, include_str!({:?})),",
            n.trim_end_matches(".json"),
            path.display().to_string()
        )
            .unwrap();
    }
    writeln!(f, "];").unwrap();
}