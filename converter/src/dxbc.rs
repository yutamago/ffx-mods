//! Direct3D 11 shader bytecode (DXBC) patching: the container checksum and
//! a walk over the shader model 4 token stream (`SHDR`), enough to find and
//! rewrite single instructions in place. Instruction lengths never change,
//! so the chunk offsets stay valid; only the checksum is recomputed.
//!
//! Token format: the D3D10/11 tokenized program format (`d3d11TokenizedProgramFormat.hpp`
//! in the Windows SDK), summarised in ffx-rust's `crates/ffx-dxbc/src/sm4.rs`.

use anyhow::{Context, Result, bail, ensure};

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4).context("dxbc: read past the end")?.try_into()?,
    ))
}

/// Every DXBC container in `bytes` (a shader `.phyre` holds dozens): (offset, size).
pub fn containers(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(p) = bytes[at..].windows(4).position(|w| w == b"DXBC") {
        let start = at + p;
        let size = u32_at(bytes, start + 24).unwrap_or(0) as usize;
        if size < 32 || start + size > bytes.len() {
            at = start + 4;
            continue;
        }
        out.push((start, size));
        at = start + size;
    }
    out
}

// ---- checksum ---------------------------------------------------------------------------------

fn md5_transform(state: &mut [u32; 4], block: &[u8; 64]) {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
        14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
        21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k = |i: usize| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32;
    let m: [u32; 16] = std::array::from_fn(|i| u32::from_le_bytes(block[4 * i..4 * i + 4].try_into().expect("4")));
    let [mut a, mut b, mut c, mut d] = *state;
    for i in 0..64 {
        let (f, g) = match i / 16 {
            0 => ((b & c) | (!b & d), i),
            1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
            2 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let t = d;
        d = c;
        c = b;
        b = b.wrapping_add(a.wrapping_add(f).wrapping_add(k(i)).wrapping_add(m[g]).rotate_left(S[i]));
        a = t;
    }
    for (s, v) in state.iter_mut().zip([a, b, c, d]) {
        *s = s.wrapping_add(v);
    }
}

/// The checksum D3D stores at bytes 4..20 of a container: MD5 of everything
/// after it, with Microsoft's own padding (the bit length first in the last
/// block and `(bits >> 2) | 1` as its last word, instead of the 64-bit
/// length at the end).
pub fn checksum(container: &[u8]) -> [u8; 16] {
    let data = &container[20..];
    let bits = (data.len() as u32).wrapping_mul(8);
    let mut state = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    let full = data.len() & !63;
    for block in data[..full].chunks_exact(64) {
        md5_transform(&mut state, block.try_into().expect("64"));
    }
    let last = &data[full..];
    let mut block = [0u8; 64];
    if last.len() >= 56 {
        block[..last.len()].copy_from_slice(last);
        block[last.len()] = 0x80;
        md5_transform(&mut state, &block);
        block = [0u8; 64];
        block[..4].copy_from_slice(&bits.to_le_bytes());
    } else {
        block[..4].copy_from_slice(&bits.to_le_bytes());
        block[4..4 + last.len()].copy_from_slice(last);
        block[4 + last.len()] = 0x80;
    }
    block[60..].copy_from_slice(&((bits >> 2) | 1).to_le_bytes());
    md5_transform(&mut state, &block);
    let mut out = [0u8; 16];
    for (i, s) in state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&s.to_le_bytes());
    }
    out
}

/// Writes the checksum of the container at `bytes[at..at + size]`.
pub fn reseal(bytes: &mut [u8], at: usize, size: usize) {
    let sum = checksum(&bytes[at..at + size]);
    bytes[at + 4..at + 20].copy_from_slice(&sum);
}

/// A chunk of a container: (tag, offset of its data in the container, size).
pub fn chunks(c: &[u8]) -> Result<Vec<([u8; 4], usize, usize)>> {
    ensure!(c.get(..4) == Some(b"DXBC"), "not a DXBC container");
    let n = u32_at(c, 28)? as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let at = u32_at(c, 32 + 4 * i)? as usize;
        let tag: [u8; 4] = c.get(at..at + 4).context("chunk tag")?.try_into()?;
        let size = u32_at(c, at + 4)? as usize;
        ensure!(at + 8 + size <= c.len(), "chunk past the end");
        out.push((tag, at + 8, size));
    }
    Ok(out)
}

// ---- shader model 4 tokens --------------------------------------------------------------------

pub const OP_ADD: u32 = 0;
pub const OP_DP3: u32 = 16;
pub const OP_MAD: u32 = 50;
pub const OP_MAX: u32 = 52;
pub const OP_MUL: u32 = 56;
const OP_CUSTOMDATA: u32 = 53;
/// Opcode token bit: `_sat`.
pub const SATURATE: u32 = 1 << 13;

pub const REG_TEMP: u32 = 0;
pub const REG_IMMEDIATE32: u32 = 4;
pub const REG_CONSTANT_BUFFER: u32 = 8;

/// One operand, as far as the patches need it.
#[derive(Debug, Clone)]
pub struct Operand {
    /// Operand type (`REG_*`).
    pub reg: u32,
    /// Components: 0, 1 or 4.
    pub comps: u32,
    /// 4-component selection: 0 mask, 1 swizzle, 2 one component.
    pub mode: u32,
    /// The mask, swizzle or component bits (token bits 4-11).
    pub sel: u32,
    /// Immediate indices (`None` for a relative index).
    pub index: Vec<Option<u32>>,
    /// Token positions (in the program, in tokens) of an immediate's values.
    pub values: std::ops::Range<usize>,
    pub neg: bool,
    pub abs: bool,
}

impl Operand {
    /// The single component a destination writes (a one-bit mask).
    pub fn dst_component(&self) -> Option<u32> {
        (self.comps == 2 && self.mode == 0 && (self.sel & 0xf).count_ones() == 1)
            .then(|| (self.sel & 0xf).trailing_zeros())
    }
    /// Destination mask (all four for a non-mask operand).
    pub fn dst_mask(&self) -> u32 {
        if self.comps == 2 && self.mode == 0 { self.sel & 0xf } else { 0xf }
    }
    /// A temp register `rN`.
    pub fn temp(&self) -> Option<u32> {
        (self.reg == REG_TEMP && self.index.len() == 1).then(|| self.index[0]).flatten()
    }
    /// `cb<slot>[<reg>]` with immediate indices.
    pub fn cbuffer(&self) -> Option<(u32, u32)> {
        if self.reg != REG_CONSTANT_BUFFER || self.index.len() != 2 {
            return None;
        }
        Some((self.index[0]?, self.index[1]?))
    }
}

/// One instruction: where it is and its operands.
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Token position of the opcode token in the program.
    pub at: usize,
    pub opcode: u32,
    pub saturate: bool,
    pub operands: Vec<Operand>,
}

fn operand(t: &[u32], at: &mut usize) -> Result<Operand> {
    let tok = *t.get(*at).context("operand past the end")?;
    *at += 1;
    let comps = tok & 3;
    let mode = (tok >> 2) & 3;
    let sel = (tok >> 4) & 0xff;
    let reg = (tok >> 12) & 0xff;
    let dims = (tok >> 20) & 3;
    let (mut neg, mut abs) = (false, false);
    let mut ext = tok >> 31 != 0;
    while ext {
        let e = *t.get(*at).context("extended operand")?;
        *at += 1;
        if e & 0x3f == 1 {
            let m = (e >> 6) & 0xff;
            neg |= m & 1 != 0;
            abs |= m & 2 != 0;
        }
        ext = e >> 31 != 0;
    }
    let mut index = Vec::new();
    for d in 0..dims {
        let rep = (tok >> (22 + 3 * d)) & 7;
        match rep {
            0 => {
                index.push(Some(*t.get(*at).context("index")?));
                *at += 1;
            }
            1 => {
                index.push(None);
                *at += 2;
            }
            2 => {
                index.push(None);
                operand(t, at)?;
            }
            3 => {
                index.push(None);
                *at += 1;
                operand(t, at)?;
            }
            4 => {
                index.push(None);
                *at += 2;
                operand(t, at)?;
            }
            _ => bail!("index representation {rep}"),
        }
    }
    let mut values = *at..*at;
    if reg == REG_IMMEDIATE32 || reg == 5 {
        let n = match comps {
            1 => 1,
            2 => 4,
            _ => 0,
        } * if reg == 5 { 2 } else { 1 };
        values = *at..*at + n;
        *at += n;
    }
    Ok(Operand { reg, comps, mode, sel, index, values, neg, abs })
}

/// The tokens of a `SHDR`/`SHEX` chunk (version, length, then the program).
pub fn tokens(chunk: &[u8]) -> Vec<u32> {
    chunk.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().expect("4"))).collect()
}

/// The arithmetic instructions of a program, with operands decoded for the
/// opcodes in `decode` (others keep an empty operand list).
pub fn instructions(t: &[u32], decode: &[u32]) -> Result<Vec<Instruction>> {
    let len = (*t.get(1).context("program length")? as usize).min(t.len());
    let mut out = Vec::new();
    let mut at = 2;
    while at < len {
        let tok = t[at];
        let opcode = tok & 0x7ff;
        let n = if opcode == OP_CUSTOMDATA {
            *t.get(at + 1).context("customdata length")? as usize
        } else {
            ((tok >> 24) & 0x7f) as usize
        };
        ensure!(n > 0 && at + n <= len, "bad instruction length at token {at}");
        let mut operands = Vec::new();
        if decode.contains(&opcode) && opcode != OP_CUSTOMDATA {
            let mut p = at + 1;
            let mut ext = tok >> 31 != 0;
            while ext {
                ext = t[p] >> 31 != 0;
                p += 1;
            }
            while p < at + n {
                operands.push(operand(&t[..at + n], &mut p)?);
            }
        }
        out.push(Instruction { at, opcode, saturate: tok & SATURATE != 0, operands });
        at += n;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MD5 itself (RFC 1321 test vector) through the transform: the DXBC
    /// variant only differs in the padding.
    #[test]
    fn md5_transform_matches_rfc() {
        let mut block = [0u8; 64];
        block[..3].copy_from_slice(b"abc");
        block[3] = 0x80;
        block[56..].copy_from_slice(&24u64.to_le_bytes());
        let mut s = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
        md5_transform(&mut s, &block);
        let hex: String = s.iter().flat_map(|w| w.to_le_bytes()).map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "900150983cd24fb0d6963f7d28e17f72");
    }
}
