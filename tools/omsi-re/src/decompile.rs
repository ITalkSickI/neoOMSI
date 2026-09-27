//! Pseudo-code of a function: its blocks as labels, every instruction lifted to a C-like
//! statement where the idiom is known (moves, arithmetic, compare-and-branch, calls with the
//! Delphi `register` convention's arguments), with the names, string literals and floating
//! point constants it refers to written out.

use crate::analysis::{Function, Program};
use iced_x86::{ConditionCode, Formatter, Instruction, IntelFormatter, Mnemonic, OpKind, Register};
use std::collections::HashMap;
use std::fmt::Write;

pub fn pseudo(p: &Program, f: &Function, asm: bool) -> String {
    let mut out = String::new();
    let insns = p.instructions(f);
    let labels: std::collections::BTreeSet<u32> = f.blocks.keys().copied().collect();
    let _ = writeln!(out, "// {} @ {:06x} .. {:06x}, {} blocks", p.name_of(f.start), f.start, f.end(), f.blocks.len());
    if let Some(refs) = p.xrefs.get(&f.start) {
        let callers: std::collections::BTreeSet<String> = refs.iter().filter_map(|r| p.function_of(*r)).map(|c| p.name_of(c)).collect();
        let _ = writeln!(out, "// called from: {}", callers.into_iter().take(12).collect::<Vec<_>>().join(", "));
    }
    let mut fmt = IntelFormatter::new();
    fmt.options_mut().set_hex_prefix("0x");
    fmt.options_mut().set_hex_suffix("");
    fmt.options_mut().set_uppercase_hex(false);
    let mut last_cmp: Option<(String, String, bool)> = None; // (a, b, is_test)
    // what the argument registers were last set to (for call arguments)
    let mut regs: HashMap<Register, String> = HashMap::new();
    for i in &insns {
        if labels.contains(&i.ip32()) {
            let _ = writeln!(out, "L_{:06x}:", i.ip32());
            regs.clear();
        }
        let raw = {
            let mut s = String::new();
            fmt.format(i, &mut s);
            s
        };
        let line = if asm { None } else { lift(p, i, &mut last_cmp, &mut regs) };
        let note = format!("{}{}", annotate(p, i), float_imm(i));
        match line {
            Some(l) => {
                let _ = writeln!(out, "    {l:<60} // {:06x}: {raw}{note}", i.ip32());
            }
            None => {
                let _ = writeln!(out, "    {:06x}: {raw}{note}", i.ip32());
            }
        }
    }
    out
}

fn reg(r: Register) -> String {
    format!("{r:?}").to_ascii_lowercase()
}

/// An operand as an expression.
fn operand(p: &Program, i: &Instruction, k: u32) -> String {
    match i.op_kind(k) {
        OpKind::Register => reg(i.op_register(k)),
        OpKind::Immediate8 | OpKind::Immediate16 | OpKind::Immediate32 | OpKind::Immediate8to32 | OpKind::Immediate8to16 => {
            let v = i.immediate(k) as u32;
            value(p, v)
        }
        OpKind::NearBranch32 => p.name_of(i.near_branch32()),
        OpKind::Memory => memory(p, i),
        _ => "?".into(),
    }
}

/// A constant: a string literal, a named address or a number.
fn value(p: &Program, v: u32) -> String {
    if let Some(s) = p.strings.get(&v) {
        return format!("{:?}", truncate(s, 60));
    }
    if let Some(n) = p.names.get(&v) {
        return n.clone();
    }
    if p.img.contains(v) && p.img.is_code(v) && p.functions.contains_key(&v) {
        return format!("&{}", p.name_of(v));
    }
    if v > 9 {
        format!("0x{v:x}")
    } else {
        format!("{v}")
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s.to_string()
    }
}

fn memory(p: &Program, i: &Instruction) -> String {
    let base = i.memory_base();
    let index = i.memory_index();
    let d = i.memory_displacement32() as i32;
    let size = match i.memory_size().size() {
        1 => "u8",
        2 => "u16",
        4 => "u32",
        8 => "u64",
        10 => "f80",
        _ => "",
    };
    if base == Register::None && index == Register::None {
        let a = d as u32;
        if let Some(n) = p.names.get(&a) {
            return format!("[{n}]");
        }
        if let Some(s) = p.strings.get(&a) {
            return format!("{:?}", truncate(s, 60));
        }
        return format!("g_{a:06x}");
    }
    if base == Register::EBP && index == Register::None {
        return if d < 0 { format!("local_{:x}", -d) } else { format!("arg_{d:x}") };
    }
    if base == Register::ESP && index == Register::None {
        return format!("stack_{d:x}");
    }
    let mut s = String::new();
    if base != Register::None {
        s.push_str(&reg(base));
    }
    if index != Register::None {
        if !s.is_empty() {
            s.push('+');
        }
        let _ = write!(s, "{}*{}", reg(index), i.memory_index_scale());
    }
    if index == Register::None && base != Register::None {
        // a field of the object the base register points at
        return format!("{}->{}f_{:x}", reg(base), if size.is_empty() { "" } else { "" }, d as u32);
    }
    if d != 0 {
        let _ = write!(s, "{}0x{:x}", if d < 0 { "-" } else { "+" }, d.unsigned_abs());
    }
    format!("*({size}*)({s})")
}

fn cond(c: ConditionCode, a: &str, b: &str, test: bool) -> String {
    if let Some(call) = a.strip_prefix('@') {
        return match c {
            ConditionCode::e => format!("{call} says equal/zero"),
            ConditionCode::ne => format!("{call} says different/non-zero"),
            _ => format!("{c:?} after {call}"),
        };
    }
    if test {
        let e = if a == b { a.to_string() } else { format!("({a} & {b})") };
        return match c {
            ConditionCode::e => format!("{e} == 0"),
            ConditionCode::ne => format!("{e} != 0"),
            ConditionCode::s => format!("{e} < 0"),
            ConditionCode::ns => format!("{e} >= 0"),
            ConditionCode::le => format!("{e} <= 0"),
            ConditionCode::g => format!("{e} > 0"),
            _ => format!("{c:?}({e})"),
        };
    }
    let op = match c {
        ConditionCode::e => "==",
        ConditionCode::ne => "!=",
        ConditionCode::l => "<",
        ConditionCode::ge => ">=",
        ConditionCode::le => "<=",
        ConditionCode::g => ">",
        ConditionCode::b => "<u",
        ConditionCode::ae => ">=u",
        ConditionCode::be => "<=u",
        ConditionCode::a => ">u",
        _ => return format!("{c:?}({a}, {b})"),
    };
    format!("{a} {op} {b}")
}

fn lift(p: &Program, i: &Instruction, last: &mut Option<(String, String, bool)>, regs: &mut HashMap<Register, String>) -> Option<String> {
    let o = |k| operand(p, i, k);
    let set = |regs: &mut HashMap<Register, String>, r: Register, v: String| {
        if i.op_kind(0) == OpKind::Register {
            regs.insert(r.full_register32(), v);
        }
    };
    let s = match i.mnemonic() {
        Mnemonic::Mov | Mnemonic::Movzx | Mnemonic::Movsx => {
            let (a, b) = (o(0), o(1));
            set(regs, i.op0_register(), b.clone());
            format!("{a} = {b};")
        }
        Mnemonic::Lea => {
            let b = o(1);
            set(regs, i.op0_register(), format!("&{b}"));
            format!("{} = &{b};", o(0))
        }
        Mnemonic::Xor if i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Register && i.op0_register() == i.op1_register() => {
            set(regs, i.op0_register(), "0".into());
            format!("{} = 0;", o(0))
        }
        Mnemonic::Add => format!("{} += {};", o(0), o(1)),
        Mnemonic::Sub => format!("{} -= {};", o(0), o(1)),
        Mnemonic::And => format!("{} &= {};", o(0), o(1)),
        Mnemonic::Or => format!("{} |= {};", o(0), o(1)),
        Mnemonic::Xor => format!("{} ^= {};", o(0), o(1)),
        Mnemonic::Shl | Mnemonic::Sal => format!("{} <<= {};", o(0), o(1)),
        Mnemonic::Shr => format!("{} >>= {};", o(0), o(1)),
        Mnemonic::Sar => format!("{} >>= {}; /*signed*/", o(0), o(1)),
        Mnemonic::Inc => format!("{}++;", o(0)),
        Mnemonic::Dec => format!("{}--;", o(0)),
        Mnemonic::Neg => format!("{0} = -{0};", o(0)),
        Mnemonic::Not => format!("{0} = ~{0};", o(0)),
        Mnemonic::Imul if i.op_count() == 3 => format!("{} = {} * {};", o(0), o(1), o(2)),
        Mnemonic::Imul if i.op_count() == 2 => format!("{} *= {};", o(0), o(1)),
        Mnemonic::Cmp => {
            *last = Some((o(0), o(1), false));
            return None;
        }
        Mnemonic::Test => {
            *last = Some((o(0), o(1), true));
            return None;
        }
        Mnemonic::Push => format!("push({});", o(0)),
        Mnemonic::Pop => format!("pop({});", o(0)),
        Mnemonic::Ret => "return;".into(),
        Mnemonic::Jmp if i.op0_kind() == OpKind::NearBranch32 => {
            let t = i.near_branch32();
            if p.functions.contains_key(&t) && p.function_of(i.ip32()) != Some(t) {
                format!("return {}(); /*tail call*/", p.name_of(t))
            } else {
                format!("goto L_{t:06x};")
            }
        }
        Mnemonic::Call => {
            let target = o(0);
            // Delphi's register convention: eax, edx, ecx, then the stack
            let args: Vec<String> = [Register::EAX, Register::EDX, Register::ECX]
                .iter()
                .map(|r| regs.get(r).cloned().unwrap_or_else(|| reg(*r)))
                .collect();
            regs.clear();
            // what the flags say after a call: its result (the RTL's string compare sets
            // ZF when equal)
            *last = Some((format!("@{target}"), String::new(), false));
            format!("eax = {target}({});", args.join(", "))
        }
        m if i.condition_code() != ConditionCode::None && i.op0_kind() == OpKind::NearBranch32 && format!("{m:?}").starts_with('J') => {
            let (a, b, t) = last.clone().unwrap_or(("flags".into(), "0".into(), false));
            format!("if ({}) goto L_{:06x};", cond(i.condition_code(), &a, &b, t), i.near_branch32())
        }
        Mnemonic::Setne | Mnemonic::Sete | Mnemonic::Setl | Mnemonic::Setg | Mnemonic::Setle | Mnemonic::Setge | Mnemonic::Setb | Mnemonic::Seta | Mnemonic::Setbe | Mnemonic::Setae => {
            let (a, b, t) = last.clone().unwrap_or(("flags".into(), "0".into(), false));
            format!("{} = ({});", o(0), cond(i.condition_code(), &a, &b, t))
        }
        Mnemonic::Fld => format!("fpush({});", o(0)),
        Mnemonic::Fild => format!("fpush((float){});", o(0)),
        Mnemonic::Fstp => format!("{} = fpop();", o(0)),
        Mnemonic::Fst => format!("{} = st0;", o(0)),
        Mnemonic::Fistp => format!("{} = (int)fpop();", o(0)),
        Mnemonic::Fadd | Mnemonic::Faddp => format!("st0 += {};", if i.op_count() > 0 { o(i.op_count() - 1) } else { "st1".into() }),
        Mnemonic::Fsub | Mnemonic::Fsubp => format!("st0 -= {};", if i.op_count() > 0 { o(i.op_count() - 1) } else { "st1".into() }),
        Mnemonic::Fmul | Mnemonic::Fmulp => format!("st0 *= {};", if i.op_count() > 0 { o(i.op_count() - 1) } else { "st1".into() }),
        Mnemonic::Fdiv | Mnemonic::Fdivp => format!("st0 /= {};", if i.op_count() > 0 { o(i.op_count() - 1) } else { "st1".into() }),
        Mnemonic::Fcomp | Mnemonic::Fcom | Mnemonic::Fcompp => {
            *last = Some(("st0".into(), if i.op_count() > 0 { o(0) } else { "st1".into() }, false));
            return None;
        }
        Mnemonic::Nop => return Some(String::new()),
        _ => return None,
    };
    Some(s)
}

/// Comments: what a memory operand holds when it is a constant (a float in the code, a
/// string), and the class of a VMT an instruction names.
fn annotate(p: &Program, i: &Instruction) -> String {
    let mut out = String::new();
    for k in 0..i.op_count() {
        let a = match i.op_kind(k) {
            OpKind::Memory if i.memory_base() == Register::None && i.memory_index() == Register::None => i.memory_displacement32(),
            OpKind::Immediate32 => i.immediate32(),
            _ => continue,
        };
        if let Some(c) = p.classes.iter().find(|c| c.vmt == a) {
            let _ = write!(out, "  ; class {}", c.name);
        }
        let fpu = format!("{:?}", i.mnemonic()).starts_with('F') || i.memory_size().is_packed() || format!("{:?}", i.mnemonic()).ends_with("ss") || format!("{:?}", i.mnemonic()).ends_with("sd");
        if i.op_kind(k) == OpKind::Memory && p.img.is_code(a) && fpu {
            match i.memory_size().size() {
                4 => {
                    if let Some(v) = p.img.u32(a) {
                        let _ = write!(out, "  ; = {}f", f32::from_bits(v));
                    }
                }
                8 => {
                    if let Some(b) = p.img.bytes(a, 8) {
                        let v = f64::from_le_bytes(b.try_into().unwrap());
                        let _ = write!(out, "  ; = {v}");
                    }
                }
                10 => {
                    if let Some(b) = p.img.bytes(a, 10) {
                        let _ = write!(out, "  ; = {}", f80(b));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Comments for immediates that read as a float (a field set to 0.6 is `mov [x], 0x3f19999a`).
pub fn float_imm(i: &Instruction) -> String {
    if i.mnemonic() == Mnemonic::Mov && i.op_count() == 2 && i.op_kind(1) == OpKind::Immediate32 && i.memory_size().size() == 4 {
        let v = i.immediate32();
        let f = f32::from_bits(v);
        let e = (v >> 23) & 0xff;
        if v != 0 && (100..=160).contains(&e) && f.is_finite() && (f * 1000.0).round() / 1000.0 == (f * 1000.0 * 1.0).round() / 1000.0 {
            let r = (f * 10000.0).round() / 10000.0;
            if (r - f).abs() < 1e-4 * f.abs().max(1.0) {
                return format!("  ; = {r}f");
            }
        }
    }
    String::new()
}

/// An x87 extended float.
fn f80(b: &[u8]) -> f64 {
    let mant = u64::from_le_bytes(b[0..8].try_into().unwrap());
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = (se & 0x7fff) as i32;
    if exp == 0 && mant == 0 {
        return 0.0;
    }
    sign * (mant as f64 / (1u64 << 63) as f64) * 2f64.powi(exp - 16383)
}
