//! What a Delphi executable says about itself: its classes (virtual method tables with
//! names, parents, published methods) and its enumeration types, and its string literals.

use crate::pe::Image;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Class {
    /// Address of the VMT (what an object's first field points to).
    pub vmt: u32,
    pub name: String,
    pub parent: Option<u32>,
    pub instance_size: u32,
    /// Virtual methods in slot order (code addresses).
    pub virtuals: Vec<u32>,
    /// Published methods: (code, name).
    pub published: Vec<(u32, String)>,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub at: u32,
    pub name: String,
    pub values: Vec<String>,
}

/// Where the fields of a VMT lie below it: Delphi 3..2007 and Delphi 2009+ (which added
/// Equals, GetHashCode and ToString and moved everything 12 bytes down).
#[derive(Clone, Copy)]
struct Layout {
    self_ptr: i64,
    method_table: i64,
    class_name: i64,
    instance_size: i64,
    parent: i64,
}
const OLD: Layout = Layout { self_ptr: -76, method_table: -52, class_name: -44, instance_size: -40, parent: -36 };
const NEW: Layout = Layout { self_ptr: -88, method_table: -64, class_name: -56, instance_size: -52, parent: -48 };

fn ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

pub fn classes(img: &Image) -> Vec<Class> {
    let a = classes_with(img, OLD);
    let b = classes_with(img, NEW);
    if b.len() > a.len() { b } else { a }
}

fn classes_with(img: &Image, l: Layout) -> Vec<Class> {
    #[allow(non_snake_case)]
    let (SELF, METHOD_TABLE, CLASS_NAME, INSTANCE_SIZE, PARENT) = (l.self_ptr, l.method_table, l.class_name, l.instance_size, l.parent);
    let mut found: Vec<Class> = Vec::new();
    let end = img.base + img.mem.len() as u32;
    let mut va = img.base;
    while va + 4 <= end {
        if let Some(v) = img.u32(va) {
            // vmtSelfPtr points to the VMT, 76 bytes past itself
            if v as i64 == va as i64 - SELF {
                let vmt = v;
                let at = |off: i64| img.u32((vmt as i64 + off) as u32);
                if let Some(name) = at(CLASS_NAME).and_then(|p| img.short_string(p)).filter(|n| ident(n) && n.len() > 1) {
                    let parent = at(PARENT).filter(|&p| p != 0).and_then(|p| {
                        // a pointer to the parent's VMT pointer (or, in some builds, the VMT)
                        img.u32(p).filter(|&q| img.u32((q as i64 + SELF) as u32) == Some(q)).or_else(|| {
                            (img.u32((p as i64 + SELF) as u32) == Some(p)).then_some(p)
                        })
                    });
                    let mut published = Vec::new();
                    if let Some(mt) = at(METHOD_TABLE).filter(|&p| p != 0) {
                        if let Some(n) = img.u16(mt) {
                            let mut p = mt + 2;
                            for _ in 0..n.min(2000) {
                                let (Some(len), Some(code), Some(mname)) = (img.u16(p), img.u32(p + 2), img.short_string(p + 6)) else { break };
                                if len < 7 {
                                    break;
                                }
                                published.push((code, mname));
                                p += len as u32;
                            }
                        }
                    }
                    found.push(Class {
                        vmt,
                        name,
                        parent,
                        instance_size: at(INSTANCE_SIZE).unwrap_or(0),
                        virtuals: Vec::new(),
                        published,
                    });
                }
            }
        }
        va += 4;
    }
    // the virtual methods run from the VMT up to the next class's data
    let mut starts: Vec<u32> = found.iter().map(|c| (c.vmt as i64 + SELF) as u32).collect();
    starts.sort_unstable();
    for c in &mut found {
        let limit = starts.iter().copied().find(|&s| s > c.vmt).unwrap_or(c.vmt + 4096).min(c.vmt + 4096);
        let mut p = c.vmt;
        while p + 4 <= limit {
            match img.u32(p) {
                Some(f) if img.is_code(f) => c.virtuals.push(f),
                _ => break,
            }
            p += 4;
        }
    }
    found.sort_by_key(|c| c.vmt);
    found
}

/// Enumeration types from their RTTI (`tkEnumeration`): the value names in order.
pub fn enums(img: &Image) -> Vec<Enum> {
    let mut out = Vec::new();
    let end = img.base + img.mem.len() as u32;
    let mut va = img.base;
    while va + 16 < end {
        if img.u8(va) == Some(3) {
            if let Some(name) = img.short_string(va + 1).filter(|n| n.len() >= 2 && ident(n)) {
                let td = va + 2 + name.len() as u32;
                let (Some(ord), Some(min), Some(max)) = (img.u8(td), img.i32(td + 1), img.i32(td + 5)) else {
                    va += 1;
                    continue;
                };
                if ord <= 5 && min == 0 && (1..512).contains(&max) {
                    let mut p = td + 13;
                    let mut values = Vec::new();
                    for _ in 0..=max {
                        match img.short_string(p).filter(|s| ident(s)) {
                            Some(s) => {
                                p += 1 + s.len() as u32;
                                values.push(s);
                            }
                            None => break,
                        }
                    }
                    if values.len() == max as usize + 1 {
                        out.push(Enum { at: va, name, values });
                        va = p;
                        continue;
                    }
                }
            }
        }
        va += 1;
    }
    out
}

/// String literals: Delphi's constant AnsiStrings (reference count -1, length, text, NUL)
/// by the address of their first character, and NUL-terminated C strings of 5 characters or
/// more in data sections.
pub fn strings(img: &Image) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let b = &img.mem;
    let text = |c: u8| c >= 0x20 || c == b'\t' || c == b'\r' || c == b'\n';
    let mut o = 0usize;
    while o + 9 < b.len() {
        if b[o] == 0xff && b[o + 1] == 0xff && b[o + 2] == 0xff && b[o + 3] == 0xff {
            let len = u32::from_le_bytes([b[o + 4], b[o + 5], b[o + 6], b[o + 7]]) as usize;
            if (1..20000).contains(&len) && o + 8 + len < b.len() && b[o + 8 + len] == 0 && b[o + 8..o + 8 + len].iter().all(|&c| text(c)) {
                let s: String = b[o + 8..o + 8 + len].iter().map(|&c| cp1252(c)).collect();
                out.insert(img.base + (o + 8) as u32, s);
                o += 8 + len;
                continue;
            }
        }
        o += 1;
    }
    // UnicodeString literals (Delphi 2009+): element size 2 before the reference count
    let mut o = 2usize;
    while o + 12 < b.len() {
        if b[o] == 0xff && b[o + 1] == 0xff && b[o + 2] == 0xff && b[o + 3] == 0xff && b[o - 2] == 2 && b[o - 1] == 0 {
            let len = u32::from_le_bytes([b[o + 4], b[o + 5], b[o + 6], b[o + 7]]) as usize;
            let st = o + 8;
            if (1..20000).contains(&len) && st + 2 * len + 2 <= b.len() && b[st + 2 * len] == 0 && b[st + 2 * len + 1] == 0 {
                let units: Vec<u16> = (0..len).map(|k| u16::from_le_bytes([b[st + 2 * k], b[st + 2 * k + 1]])).collect();
                if units.iter().all(|&u| u >= 0x20 || u == 9 || u == 10 || u == 13) {
                    out.insert(img.base + st as u32, String::from_utf16_lossy(&units));
                    o = st + 2 * len;
                    continue;
                }
            }
        }
        o += 1;
    }
    // C strings outside the code
    for s in img.sections.iter().filter(|s| !s.exec) {
        let (a, e) = (s.va as usize, (s.va + s.vsize.max(s.raw_size)) as usize);
        let mut o = a;
        while o < e.min(b.len()) {
            let start = o;
            while o < e.min(b.len()) && b[o] != 0 && (b[o] >= 0x20 || b[o] == b'\t' || b[o] == b'\n' || b[o] == b'\r') && b[o] < 0x7f {
                o += 1;
            }
            if o - start >= 5 && o < b.len() && b[o] == 0 {
                out.entry(img.base + start as u32).or_insert_with(|| b[start..o].iter().map(|&c| c as char).collect());
            }
            o += 1;
        }
    }
    out
}

fn cp1252(c: u8) -> char {
    match c {
        0x80 => '€',
        0x84 => '„',
        0x93 => '“',
        0x94 => '”',
        _ => c as char,
    }
}
