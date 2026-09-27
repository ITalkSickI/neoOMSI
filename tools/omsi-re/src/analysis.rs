//! Functions, their blocks and who refers to what, found by following the code from every
//! known entry (the program's entry point, the classes' methods, every call target, the
//! `case` jump tables and the prologues between what was found).

use crate::delphi::{self, Class, Enum};
use crate::pe::Image;
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic, OpKind, Register};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct Block {
    pub start: u32,
    /// Address after the last instruction.
    pub end: u32,
    pub succ: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Function {
    pub start: u32,
    pub blocks: BTreeMap<u32, Block>,
}

impl Function {
    pub fn end(&self) -> u32 {
        self.blocks.values().map(|b| b.end).max().unwrap_or(self.start)
    }
}

pub struct Program {
    pub img: Image,
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
    pub strings: HashMap<u32, String>,
    pub names: HashMap<u32, String>,
    pub functions: BTreeMap<u32, Function>,
    /// Target → the instructions that call, jump to or name it.
    pub xrefs: HashMap<u32, Vec<u32>>,
    /// Instruction address → the function it belongs to.
    pub owner: HashMap<u32, u32>,
}

pub fn decode_at(img: &Image, va: u32) -> Option<Instruction> {
    let b = img.bytes(va, 16).or_else(|| {
        let left = (img.base as usize + img.mem.len()).saturating_sub(va as usize);
        img.bytes(va, left.min(16))
    })?;
    let mut d = Decoder::with_ip(32, b, va as u64, DecoderOptions::NONE);
    let i = d.decode();
    (!i.is_invalid()).then_some(i)
}

impl Program {
    pub fn build(img: Image, names_file: Option<&str>) -> Program {
        let classes = delphi::classes(&img);
        let enums = delphi::enums(&img);
        let strings = delphi::strings(&img);
        let mut names: HashMap<u32, String> = HashMap::new();
        for (slot, n) in &img.imports {
            names.insert(*slot, n.clone());
        }
        let by_vmt: HashMap<u32, &Class> = classes.iter().map(|c| (c.vmt, c)).collect();
        for c in &classes {
            for (code, m) in &c.published {
                names.entry(*code).or_insert_with(|| format!("{}.{}", c.name, m));
            }
        }
        for c in &classes {
            for (k, f) in c.virtuals.iter().enumerate() {
                // a slot the parent has at the same address is the parent's method
                let inherited = c.parent.and_then(|p| by_vmt.get(&p)).map(|p| p.virtuals.get(k) == Some(f)).unwrap_or(false);
                if !inherited {
                    names.entry(*f).or_insert_with(|| format!("{}.virtual_{k:02}", c.name));
                }
            }
        }
        if let Some(path) = names_file {
            if let Ok(text) = std::fs::read_to_string(path) {
                for line in text.lines() {
                    let line = line.split('#').next().unwrap_or("").trim();
                    let mut it = line.splitn(2, char::is_whitespace);
                    if let (Some(a), Some(n)) = (it.next(), it.next()) {
                        if let Ok(a) = u32::from_str_radix(a.trim_start_matches("0x"), 16) {
                            names.insert(a, n.trim().to_string());
                        }
                    }
                }
            }
        }
        let mut p = Program { img, classes, enums, strings, names, functions: BTreeMap::new(), xrefs: HashMap::new(), owner: HashMap::new() };
        p.discover();
        p
    }

    fn discover(&mut self) {
        let mut seeds: BTreeSet<u32> = BTreeSet::new();
        seeds.insert(self.img.entry);
        for c in &self.classes {
            seeds.extend(c.virtuals.iter().copied());
            seeds.extend(c.published.iter().map(|x| x.0));
        }
        let mut done: BTreeSet<u32> = BTreeSet::new();
        let (code_lo, code_hi) = self.img.code_range().unwrap_or((0, 0));
        loop {
            while let Some(f) = seeds.pop_first() {
                if done.contains(&f) || !self.img.is_code(f) || self.owner.contains_key(&f) {
                    done.insert(f);
                    continue;
                }
                done.insert(f);
                let (func, calls) = self.follow(f);
                for c in calls {
                    if !done.contains(&c) {
                        seeds.insert(c);
                    }
                }
                self.functions.insert(f, func);
            }
            // prologues in what nothing reached (event handlers stored in data, ...)
            let mut more = Vec::new();
            let mut va = code_lo & !3;
            while va + 3 < code_hi {
                if !self.owner.contains_key(&va) && !done.contains(&va) {
                    if let Some(b) = self.img.bytes(va, 3) {
                        if b == [0x55, 0x8b, 0xec] {
                            more.push(va);
                        }
                    }
                }
                va += 4;
            }
            if more.is_empty() {
                break;
            }
            seeds.extend(more);
        }
    }

    /// Follow one function from `start`: its blocks, and the calls it makes.
    fn follow(&mut self, start: u32) -> (Function, Vec<u32>) {
        let mut todo = vec![start];
        let mut starts: BTreeSet<u32> = BTreeSet::new();
        let mut insns: BTreeMap<u32, Instruction> = BTreeMap::new();
        let mut calls = Vec::new();
        let mut edges: HashMap<u32, Vec<u32>> = HashMap::new();
        while let Some(b) = todo.pop() {
            if !starts.insert(b) {
                continue;
            }
            let mut va = b;
            loop {
                if insns.contains_key(&va) || (va != b && self.owner.get(&va).is_some_and(|o| *o != start)) {
                    break;
                }
                let Some(i) = decode_at(&self.img, va) else { break };
                insns.insert(va, i);
                let next = i.next_ip32();
                self.note_refs(&i);
                let mut ends = false;
                match i.flow_control() {
                    FlowControl::Return => ends = true,
                    FlowControl::Interrupt if i.mnemonic() == Mnemonic::Int3 => ends = true,
                    FlowControl::UnconditionalBranch => {
                        let t = i.near_branch32();
                        if self.img.is_code(t) {
                            // a jump to another function's start is a tail call
                            if self.functions.contains_key(&t) || self.names.contains_key(&t) || t < start || t > start + 0x20000 {
                                calls.push(t);
                            } else {
                                todo.push(t);
                                edges.entry(va).or_default().push(t);
                            }
                        }
                        ends = true;
                    }
                    FlowControl::ConditionalBranch => {
                        let t = i.near_branch32();
                        todo.push(t);
                        todo.push(next);
                        edges.entry(va).or_default().extend([t, next]);
                        ends = true;
                    }
                    FlowControl::IndirectBranch => {
                        // `jmp [table + reg*4]`: a case statement's table follows
                        if i.op0_kind() == OpKind::Memory && i.memory_base() == Register::None && i.memory_index_scale() == 4 {
                            let mut t = i.memory_displacement32();
                            let mut n = 0;
                            while let Some(target) = self.img.u32(t) {
                                if !self.img.is_code(target) || n > 1024 || (n > 0 && insns.contains_key(&t)) {
                                    break;
                                }
                                todo.push(target);
                                edges.entry(va).or_default().push(target);
                                t += 4;
                                n += 1;
                            }
                        }
                        ends = true;
                    }
                    FlowControl::Call => {
                        let t = i.near_branch32();
                        if self.img.is_code(t) {
                            calls.push(t);
                        }
                    }
                    _ => {}
                }
                // `push offset label`: a finally / except handler inside this function
                if i.mnemonic() == Mnemonic::Push && i.op0_kind() == OpKind::Immediate32 {
                    let t = i.immediate32();
                    if self.img.is_code(t) && t > start && t < start + 0x4000 {
                        todo.push(t);
                    }
                }
                if ends {
                    break;
                }
                va = next;
                if starts.contains(&va) {
                    edges.entry(i.ip32()).or_default().push(va);
                    break;
                }
            }
        }
        // split into blocks at every start
        let mut func = Function { start, blocks: BTreeMap::new() };
        let all_starts: BTreeSet<u32> = starts.iter().copied().filter(|s| insns.contains_key(s)).collect();
        let addrs: Vec<u32> = insns.keys().copied().collect();
        let mut cur: Option<Block> = None;
        for (k, a) in addrs.iter().enumerate() {
            let i = insns[a];
            if cur.is_none() || all_starts.contains(a) || cur.as_ref().map(|c| c.end != *a).unwrap_or(false) {
                if let Some(mut c) = cur.take() {
                    if c.succ.is_empty() && c.end == *a {
                        c.succ.push(*a);
                    }
                    func.blocks.insert(c.start, c);
                }
                cur = Some(Block { start: *a, end: *a, succ: Vec::new() });
            }
            let c = cur.as_mut().unwrap();
            c.end = i.next_ip32();
            if let Some(e) = edges.get(a) {
                c.succ.extend(e.iter().copied());
            }
            self.owner.insert(*a, start);
            let _ = k;
        }
        if let Some(c) = cur {
            func.blocks.insert(c.start, c);
        }
        (func, calls)
    }

    /// Record what an instruction refers to: its branch target, and any address of the
    /// image among its immediates and displacements.
    fn note_refs(&mut self, i: &Instruction) {
        let from = i.ip32();
        let mut add = |t: u32| {
            self.xrefs.entry(t).or_default().push(from);
        };
        match i.flow_control() {
            FlowControl::Call | FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch => {
                if i.op0_kind() == OpKind::NearBranch32 {
                    add(i.near_branch32());
                }
            }
            _ => {}
        }
        for k in 0..i.op_count() {
            match i.op_kind(k) {
                OpKind::Immediate32 => {
                    let v = i.immediate32();
                    if self.img.contains(v) {
                        add(v);
                    }
                }
                OpKind::Memory => {
                    let d = i.memory_displacement32();
                    if self.img.contains(d) {
                        add(d);
                    }
                }
                _ => {}
            }
        }
    }

    /// The function containing an address.
    pub fn function_of(&self, va: u32) -> Option<u32> {
        if self.functions.contains_key(&va) {
            return Some(va);
        }
        self.owner.get(&va).copied()
    }

    pub fn name_of(&self, va: u32) -> String {
        match self.names.get(&va) {
            Some(n) => n.clone(),
            None => format!("sub_{va:06x}"),
        }
    }

    /// An address given as hex (0x5d4b7f / 5d4b7f) or as a name.
    pub fn resolve(&self, s: &str) -> Option<u32> {
        if let Ok(v) = u32::from_str_radix(s.trim_start_matches("0x"), 16) {
            if self.img.contains(v) {
                return Some(v);
            }
        }
        let low = s.to_ascii_lowercase();
        self.names.iter().find(|(_, n)| n.to_ascii_lowercase() == low).map(|(a, _)| *a).or_else(|| {
            self.names.iter().filter(|(_, n)| n.to_ascii_lowercase().contains(&low)).map(|(a, _)| *a).min()
        })
    }

    /// The instructions of a function in address order.
    pub fn instructions(&self, f: &Function) -> Vec<Instruction> {
        let mut out = Vec::new();
        for b in f.blocks.values() {
            let mut va = b.start;
            while va < b.end {
                match decode_at(&self.img, va) {
                    Some(i) => {
                        va = i.next_ip32();
                        out.push(i);
                    }
                    None => break,
                }
            }
        }
        out
    }
}
