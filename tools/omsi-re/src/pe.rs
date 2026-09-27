//! A 32-bit PE image mapped by virtual address.

use anyhow::{anyhow, bail, Result};

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub va: u32,
    pub vsize: u32,
    pub raw_size: u32,
    pub exec: bool,
}

pub struct Image {
    pub base: u32,
    pub entry: u32,
    pub sections: Vec<Section>,
    /// The whole image laid out at its virtual addresses (from `base`).
    pub mem: Vec<u8>,
    /// Imported functions by the address of their IAT slot: "dll!name".
    pub imports: Vec<(u32, String)>,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

impl Image {
    pub fn load(bytes: &[u8]) -> Result<Image> {
        if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
            bail!("not an MZ executable");
        }
        let pe = u32_at(bytes, 0x3c) as usize;
        if &bytes[pe..pe + 4] != b"PE\0\0" {
            bail!("no PE header");
        }
        let nsec = u16_at(bytes, pe + 6) as usize;
        let opt_size = u16_at(bytes, pe + 20) as usize;
        let opt = pe + 24;
        if u16_at(bytes, opt) != 0x10b {
            bail!("only 32-bit images are handled");
        }
        let entry = u32_at(bytes, opt + 16);
        let base = u32_at(bytes, opt + 28);
        let image_size = u32_at(bytes, opt + 56) as usize;
        let header_size = u32_at(bytes, opt + 60) as usize;
        let mut mem = vec![0u8; image_size];
        let hs = header_size.min(bytes.len()).min(image_size);
        mem[..hs].copy_from_slice(&bytes[..hs]);
        let mut sections = Vec::new();
        let st = opt + opt_size;
        for i in 0..nsec {
            let s = st + i * 40;
            let name = String::from_utf8_lossy(&bytes[s..s + 8]).trim_end_matches('\0').to_string();
            let vsize = u32_at(bytes, s + 8);
            let va = u32_at(bytes, s + 12);
            let raw_size = u32_at(bytes, s + 16);
            let raw = u32_at(bytes, s + 20);
            let flags = u32_at(bytes, s + 36);
            let n = raw_size.min(vsize.max(raw_size)) as usize;
            let (r, v) = (raw as usize, va as usize);
            if r < bytes.len() && v < mem.len() {
                let n = n.min(bytes.len() - r).min(mem.len() - v);
                mem[v..v + n].copy_from_slice(&bytes[r..r + n]);
            }
            sections.push(Section { name, va, vsize, raw_size, exec: flags & 0x2000_0000 != 0 });
        }
        let mut img = Image { base, entry: base + entry, sections, mem, imports: Vec::new() };
        img.read_imports(opt);
        Ok(img)
    }

    fn read_imports(&mut self, opt: usize) {
        let dir = u32_at(&self.mem, opt + 96 + 8) as usize;
        if dir == 0 {
            return;
        }
        let mut d = dir;
        let mut out = Vec::new();
        while d + 20 <= self.mem.len() {
            let ilt = u32_at(&self.mem, d) as usize;
            let name = u32_at(&self.mem, d + 12) as usize;
            let iat = u32_at(&self.mem, d + 16) as usize;
            if name == 0 && iat == 0 {
                break;
            }
            let dll = self.cstr_rva(name).unwrap_or_default();
            let table = if ilt != 0 { ilt } else { iat };
            let mut k = 0;
            loop {
                let t = table + k * 4;
                if t + 4 > self.mem.len() {
                    break;
                }
                let e = u32_at(&self.mem, t);
                if e == 0 {
                    break;
                }
                let fname = if e & 0x8000_0000 != 0 {
                    format!("#{}", e & 0xffff)
                } else {
                    self.cstr_rva(e as usize + 2).unwrap_or_default()
                };
                out.push((self.base + (iat + k * 4) as u32, format!("{dll}!{fname}")));
                k += 1;
            }
            d += 20;
        }
        self.imports = out;
    }

    fn cstr_rva(&self, rva: usize) -> Option<String> {
        let b = self.mem.get(rva..)?;
        let end = b.iter().position(|&c| c == 0)?;
        Some(String::from_utf8_lossy(&b[..end]).into_owned())
    }

    pub fn contains(&self, va: u32) -> bool {
        va >= self.base && ((va - self.base) as usize) < self.mem.len()
    }

    pub fn bytes(&self, va: u32, n: usize) -> Option<&[u8]> {
        if !self.contains(va) {
            return None;
        }
        let o = (va - self.base) as usize;
        self.mem.get(o..o + n)
    }

    pub fn u8(&self, va: u32) -> Option<u8> {
        self.bytes(va, 1).map(|b| b[0])
    }

    pub fn u32(&self, va: u32) -> Option<u32> {
        self.bytes(va, 4).map(|b| u32_at(b, 0))
    }

    pub fn i32(&self, va: u32) -> Option<i32> {
        self.u32(va).map(|v| v as i32)
    }

    pub fn u16(&self, va: u32) -> Option<u16> {
        self.bytes(va, 2).map(|b| u16_at(b, 0))
    }

    /// A Delphi ShortString (length byte, then the characters).
    pub fn short_string(&self, va: u32) -> Option<String> {
        let n = self.u8(va)? as usize;
        let b = self.bytes(va + 1, n)?;
        Some(b.iter().map(|&c| c as char).collect())
    }

    pub fn is_code(&self, va: u32) -> bool {
        self.sections.iter().any(|s| s.exec && va >= self.base + s.va && va < self.base + s.va + s.vsize.max(s.raw_size))
    }

    /// The address range of the first executable section.
    pub fn code_range(&self) -> Result<(u32, u32)> {
        let s = self.sections.iter().find(|s| s.exec).ok_or_else(|| anyhow!("no code section"))?;
        Ok((self.base + s.va, self.base + s.va + s.vsize.max(s.raw_size)))
    }
}
