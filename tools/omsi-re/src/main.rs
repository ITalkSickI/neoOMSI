//! omsi-re: a reverse-engineering workbench for Omsi.exe.
//!
//! usage: omsi-re [--exe <Omsi.exe>] <command> [args]
//!   info                      sections, classes, functions found
//!   classes [filter]          Delphi classes (VMT, parent, size, method counts)
//!   class <name>              one class: parents, published and virtual methods
//!   enums [filter]            enumeration types with their value names
//!   strings <text>            string literals containing <text>, with the functions using them
//!   xrefs <addr|name>         who refers to an address
//!   func <addr|name> [--asm]  a function as pseudo-code (or plain assembly)
//!   callers <addr|name>       the functions calling a function
//!   callees <addr|name>       the functions a function calls
//!   name <addr> <name>        remember a name (names.txt, loaded every run)
//!
//! The executable is `$OMSI_ROOT/Omsi.exe`, the path remembered in `~/.openomsi-root`,
//! or `--exe`. Names learnt go into tools/omsi-re/names.txt.

mod analysis;
mod decompile;
mod delphi;
mod pe;

use analysis::Program;
use anyhow::{anyhow, Result};
use std::collections::BTreeSet;

const NAMES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/names.txt");

fn exe_path(args: &mut Vec<String>) -> Result<String> {
    if let Some(k) = args.iter().position(|a| a == "--exe") {
        let p = args.get(k + 1).cloned().ok_or_else(|| anyhow!("--exe needs a path"))?;
        args.drain(k..k + 2);
        return Ok(p);
    }
    if let Ok(r) = std::env::var("OMSI_ROOT") {
        return Ok(format!("{r}/Omsi.exe"));
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if let Ok(r) = std::fs::read_to_string(format!("{home}/.openomsi-root")) {
        return Ok(format!("{}/Omsi.exe", r.trim()));
    }
    Err(anyhow!("no Omsi.exe: give --exe or set OMSI_ROOT"))
}

/// Pseudo-code without the compiler's noise: Delphi's range and overflow checks (`cmp i,
/// [arr-4]; jb ok; call RangeError`, `jno ok; call IntOverflow`), register saves, NULL tests
/// that only guard a range check, and the labels nothing jumps to any more.
fn clean(text: &str) -> String {
    let noise = |l: &str| {
        let t = l.trim_start();
        l.contains("RangeError") || l.contains("IntOverflow") || l.contains("if (no after") || l.contains("->f_fffffffc")
            || t.starts_with("push(e") || t.starts_with("pop(e")
            || (t.contains(": test e") && t.contains(",e") && t.split("//").next().is_some_and(|x| x.trim().is_empty() || x.trim().ends_with(':')))
            || (t.contains(": cmp e") && t.contains("-4]"))
    };
    let mut kept: Vec<&str> = text.lines().filter(|l| !noise(l)).collect();
    // a jump to the very next line (what a removed check jumped over) and the test before it
    let goto_of = |l: &str| l.split("goto ").nth(1).map(|x| x.split(';').next().unwrap_or("").trim().to_string());
    let raw = |l: &str| l.trim_start().split(':').next().is_some_and(|a| a.len() == 6 && a.chars().all(|c| c.is_ascii_hexdigit()));
    let mut k = 0;
    while k + 1 < kept.len() {
        if kept[k].trim_start().starts_with("if (") && goto_of(kept[k]).is_some_and(|t| {
            kept[k + 1..].iter().take_while(|l| l.starts_with("L_") && l.ends_with(':')).any(|l| *l == format!("{t}:"))
        }) {
            kept.remove(k);
            if k > 0 && raw(kept[k - 1]) {
                kept.remove(k - 1);
                k -= 1;
            }
            continue;
        }
        k += 1;
    }
    // `if (x == 0) goto L` right before the label L ... that only led to the check
    let targets: std::collections::HashSet<String> = kept
        .iter()
        .filter_map(|l| l.split("goto ").nth(1).map(|x| x.trim_end_matches(|c: char| c == ';' || c.is_whitespace()).split(';').next().unwrap_or("").to_string()))
        .collect();
    let mut out = String::new();
    for l in kept {
        if let Some(label) = l.strip_suffix(':').filter(|x| x.starts_with("L_")) {
            if !targets.contains(label) {
                continue;
            }
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}

fn main() -> Result<()> {
    // `omsi-re func … | head`: a closed pipe ends the program quietly
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        eprintln!("{msg}");
    }));
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let exe = exe_path(&mut args)?;
    let cmd = args.first().cloned().unwrap_or_else(|| "info".into());
    if cmd == "name" {
        let (a, n) = (args.get(1).ok_or_else(|| anyhow!("name <addr> <name>"))?, args.get(2).ok_or_else(|| anyhow!("name <addr> <name>"))?);
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(NAMES)?;
        writeln!(f, "{} {}", a.trim_start_matches("0x"), n)?;
        println!("{a} = {n}");
        return Ok(());
    }
    let t = std::time::Instant::now();
    let bytes = std::fs::read(&exe)?;
    let img = pe::Image::load(&bytes)?;
    let p = Program::build(img, Some(NAMES));
    let arg = |k: usize| args.get(k).cloned().unwrap_or_default();
    let addr = |s: &str| p.resolve(s).ok_or_else(|| anyhow!("unknown address or name: {s}"));
    match cmd.as_str() {
        "info" => {
            for s in &p.img.sections {
                println!("{:8} {:08x} {:8x} {}", s.name, p.img.base + s.va, s.vsize, if s.exec { "code" } else { "" });
            }
            println!(
                "{} classes, {} enums, {} strings, {} functions, {} imports, {} names; analysed in {:.2} s",
                p.classes.len(),
                p.enums.len(),
                p.strings.len(),
                p.functions.len(),
                p.img.imports.len(),
                p.names.len(),
                t.elapsed().as_secs_f64()
            );
        }
        "classes" => {
            let f = arg(1).to_ascii_lowercase();
            for c in p.classes.iter().filter(|c| c.name.to_ascii_lowercase().contains(&f)) {
                let parent = c.parent.and_then(|v| p.classes.iter().find(|x| x.vmt == v)).map(|x| x.name.as_str()).unwrap_or("-");
                println!("{:06x} {:32} : {:24} size {:5}  {} virtual, {} published", c.vmt, c.name, parent, c.instance_size, c.virtuals.len(), c.published.len());
            }
        }
        "class" => {
            let n = arg(1).to_ascii_lowercase();
            let c = p.classes.iter().find(|c| c.name.to_ascii_lowercase() == n).ok_or_else(|| anyhow!("no class {n}"))?;
            let mut chain = vec![c.name.clone()];
            let mut cur = c.parent;
            while let Some(v) = cur {
                match p.classes.iter().find(|x| x.vmt == v) {
                    Some(x) => {
                        chain.push(x.name.clone());
                        cur = x.parent;
                    }
                    None => break,
                }
            }
            println!("{} (VMT {:06x}, instance size {})", chain.join(" : "), c.vmt, c.instance_size);
            for (code, m) in &c.published {
                println!("  published {code:06x} {m}");
            }
            for (k, f) in c.virtuals.iter().enumerate() {
                println!("  virtual {k:3} {f:06x} {}", p.name_of(*f));
            }
        }
        "enums" => {
            let f = arg(1).to_ascii_lowercase();
            for e in p.enums.iter().filter(|e| e.name.to_ascii_lowercase().contains(&f) || e.values.iter().any(|v| v.to_ascii_lowercase().contains(&f))) {
                println!("{:06x} {} = ({})", e.at, e.name, e.values.join(", "));
            }
        }
        "strings" => {
            let f = arg(1).to_ascii_lowercase();
            let mut hits: Vec<(&u32, &String)> = p.strings.iter().filter(|(_, s)| s.to_ascii_lowercase().contains(&f)).collect();
            hits.sort();
            for (a, s) in hits {
                let users: BTreeSet<String> = p.xrefs.get(a).into_iter().flatten().filter_map(|r| p.function_of(*r)).map(|f| format!("{} ({f:06x})", p.name_of(f))).collect();
                println!("{a:06x} {:?}", s.chars().take(100).collect::<String>());
                for u in users {
                    println!("         used in {u}");
                }
            }
        }
        "xrefs" => {
            let a = addr(&arg(1))?;
            for r in p.xrefs.get(&a).into_iter().flatten() {
                let f = p.function_of(*r).map(|f| p.name_of(f)).unwrap_or_default();
                println!("{r:06x} in {f}");
            }
        }
        "func" => {
            let a = addr(&arg(1))?;
            let f = p.function_of(a).ok_or_else(|| anyhow!("{a:06x} is in no function found"))?;
            let asm = args.iter().any(|x| x == "--asm");
            let text = decompile::pseudo(&p, &p.functions[&f], asm);
            print!("{}", if asm || args.iter().any(|x| x == "--raw") { text } else { clean(&text) });
        }
        "callers" => {
            let a = addr(&arg(1))?;
            let set: BTreeSet<u32> = p.xrefs.get(&a).into_iter().flatten().filter_map(|r| p.function_of(*r)).collect();
            for f in set {
                println!("{f:06x} {}", p.name_of(f));
            }
        }
        "callees" => {
            let a = addr(&arg(1))?;
            let f = p.function_of(a).ok_or_else(|| anyhow!("no function"))?;
            let mut set = BTreeSet::new();
            for i in p.instructions(&p.functions[&f]) {
                if i.is_call_near() {
                    set.insert(i.near_branch32());
                }
            }
            for c in set {
                println!("{c:06x} {}", p.name_of(c));
            }
        }
        // `search <text>`: every instruction whose assembly contains the text (e.g.
        // "[eax+edx*8+4]"), with its function
        "search" => {
            use iced_x86::Formatter;
            let needle = arg(1).to_ascii_lowercase();
            let mut fmt = iced_x86::IntelFormatter::new();
            fmt.options_mut().set_hex_prefix("0x");
            fmt.options_mut().set_hex_suffix("");
            for (start, f) in &p.functions {
                for i in p.instructions(f) {
                    let mut s = String::new();
                    fmt.format(&i, &mut s);
                    if s.to_ascii_lowercase().contains(&needle) {
                        println!("{:06x} {:40} {s}", i.ip32(), p.name_of(*start));
                    }
                }
            }
        }
        other => return Err(anyhow!("unknown command {other}")),
    }
    Ok(())
}
