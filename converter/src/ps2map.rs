//! PS2 field and battle map textures: `mapout.txa` (texture uploads and the
//! CLUT bank) and the GS registers in `mapout.vpa` (TEX0 writes, texture
//! swap frames), replayed into an emulated GS memory. Format: `docs/maps.md`.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use crate::gsmem::{self, Gs};

fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4)
        .map_or(0, |s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    b.get(o..o + 8)
        .map_or(0, |s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// One record of a `yiArc` render file (magic `0x214365`, version `0x110`).
#[derive(Debug, Clone, Copy)]
pub struct Record {
    pub kind: u32,
    /// Offset of the 0x40-byte header in the file.
    pub at: usize,
    /// Data length (after the header).
    pub len: usize,
}

impl Record {
    pub fn header(&self, b: &[u8], word: usize) -> u32 {
        u32_at(b, self.at + 4 * word)
    }
    pub fn data<'a>(&self, b: &'a [u8]) -> &'a [u8] {
        let s = (self.at + 0x40).min(b.len());
        &b[s..(s + self.len).min(b.len())]
    }
}

/// The records of every render file in a `MAP1` file's sections.
pub fn records(b: &[u8]) -> Vec<Record> {
    let mut out = Vec::new();
    if b.len() < 0x20 || &b[..4] != b"MAP1" {
        return out;
    }
    let mut sections: Vec<usize> = (0x10..0x80.min(b.len()))
        .step_by(4)
        .map(|o| u32_at(b, o) as usize)
        .filter(|&s| s != 0 && s < b.len())
        .collect();
    sections.sort_unstable();
    sections.dedup();
    for s in sections {
        if u32_at(b, s) != 0x21_4365 || u32_at(b, s + 4) != 0x110 {
            continue;
        }
        let count = u32_at(b, s + 0xc) as usize;
        let mut at = s + 0x40;
        for _ in 0..count {
            if at + 0x40 > b.len() {
                break;
            }
            let len = u32_at(b, at + 4) as usize;
            out.push(Record {
                kind: u32_at(b, at),
                at,
                len,
            });
            at += 0x40 + len;
        }
    }
    out
}

/// A texture record (kind 2) of a `.txa`.
#[derive(Debug, Clone)]
pub struct TexRec {
    pub dbp: u32,
    pub dbw: u32,
    pub psm: u32,
    pub w: u32,
    pub h: u32,
    /// Pixel data offset and length in the file.
    pub data: usize,
    pub size: usize,
}

impl TexRec {
    /// GS blocks the texture covers when read with its own PSM.
    pub fn blocks(&self) -> u32 {
        let bits = match self.psm {
            gsmem::PSMT4 => 4,
            gsmem::PSMT8 => 8,
            0x02 => 16,
            _ => 32,
        };
        (self.w * self.h * bits / 8).div_ceil(256)
    }
}

/// `yiClutAlloc_makepacket` (PS2 `0x0018aba8`): the GS address of CLUT
/// `i` for allocation mode 2, 3 or 4.
pub fn clut_cbp(mode: u32, i: u32) -> Option<u32> {
    let (base, split, off): (u32, u32, [u32; 4]) = match mode {
        2 => (0xb4000, 32, [0x400, 0x500, 0x600, 0x700]),
        3 => (0x18000, 32, [0x100, 0x300, 0x500, 0x700]),
        4 => (0x82000, 16, [0x100, 0x300, 0x500, 0x700]),
        _ => return None,
    };
    let words = if i < split {
        base + 0x800 * (i / 4) + off[(i % 4) as usize]
    } else {
        0xb8000 + 0x100 * (i - split)
    };
    Some(words >> 6)
}

/// `yiMapFileRead` uploads at most this many CLUTs (`0x48`).
pub const CLUT_UPLOADS: usize = 0x48;

pub struct Txa {
    pub textures: Vec<TexRec>,
    /// Kind-3 allocation mode (2, 3, 4; 0 = not uploaded).
    pub clut_mode: Option<u32>,
    /// The bank's 256-colour palettes, raw (16x16 CT32 upload order).
    pub bank: Vec<Vec<u8>>,
    /// Record order: `Ok(texture index)` or `Err(())` for the CLUT bank.
    order: Vec<Result<usize, ()>>,
}

impl Txa {
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.len() >= 0x20 && &b[..4] == b"MAP1", "not a MAP1 file");
        let mut t = Txa {
            textures: Vec::new(),
            clut_mode: None,
            bank: Vec::new(),
            order: Vec::new(),
        };
        for r in records(b) {
            match r.kind {
                2 => {
                    let h = r.at + 0x40;
                    t.order.push(Ok(t.textures.len()));
                    t.textures.push(TexRec {
                        dbp: u32_at(b, h + 4),
                        dbw: u32_at(b, h + 8),
                        psm: u32_at(b, h + 0xc),
                        w: u32_at(b, h + 0x10),
                        h: u32_at(b, h + 0x14),
                        data: r.at + 0x80,
                        size: r.len.saturating_sub(0x40),
                    });
                }
                3 => {
                    t.clut_mode = Some(r.header(b, 9));
                    t.bank = r.data(b).chunks_exact(0x400).map(<[u8]>::to_vec).collect();
                    t.order.push(Err(()));
                }
                _ => {}
            }
        }
        Ok(t)
    }

    /// The CBPs the CLUT bank is uploaded to (index = bank palette).
    pub fn clut_cbps(&self) -> Vec<u32> {
        let mode = self.clut_mode.unwrap_or(0);
        (0..self.bank.len().min(CLUT_UPLOADS) as u32)
            .map_while(|i| clut_cbp(mode, i))
            .collect()
    }

    /// GS memory after the map's textures and CLUTs are loaded, in file
    /// order, as `yiMapFileRead` does (textures at `0x3c00` and above go
    /// through the scene's VRAM copy on PC; on PS2 they are uploaded the
    /// same way).
    pub fn vram(&self, b: &[u8]) -> Gs {
        let mut gs = Gs::new();
        for o in &self.order {
            match *o {
                Ok(i) => {
                    let t = &self.textures[i];
                    let data = &b[t.data.min(b.len())..(t.data + t.size).min(b.len())];
                    if t.psm == gsmem::PSMT4 {
                        // pre-swizzled: sent as PSMCT32 at half width and
                        // quarter height
                        gs.upload(
                            gsmem::PSMCT32,
                            t.dbp,
                            (t.dbw >> 1).max(1),
                            (0, 0),
                            (t.w / 2, t.h / 4),
                            data,
                        );
                    } else {
                        gs.upload(t.psm, t.dbp, t.dbw, (0, 0), (t.w, t.h), data);
                    }
                }
                Err(()) => {
                    for (i, cbp) in self.clut_cbps().into_iter().enumerate() {
                        gs.upload(gsmem::PSMCT32, cbp, 1, (0, 0), (16, 16), &self.bank[i]);
                    }
                }
            }
        }
        gs
    }
}

/// The fields of a `TEX0` register FFX's maps use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tex0 {
    pub tbp: u32,
    pub psm: u32,
    pub tbw: u32,
    pub cbp: u32,
    pub csa: u32,
}

impl Tex0 {
    pub fn decode(v: u64) -> Self {
        Self {
            tbp: (v & 0x3fff) as u32,
            tbw: (v >> 14 & 63) as u32,
            psm: (v >> 20 & 63) as u32,
            cbp: (v >> 37 & 0x3fff) as u32,
            csa: (v >> 56 & 31) as u32,
        }
    }
}

/// Every A+D write of `TEX0_1`/`TEX0_2` (a value followed by register 6
/// or 7) in the file, at any 4-byte alignment (VIF codes shift the GIF
/// data by a word), with how often each occurs.
pub fn scan_tex0(b: &[u8]) -> BTreeMap<Tex0, u32> {
    let mut out = BTreeMap::new();
    let mut o = 0;
    while o + 16 <= b.len() {
        let reg = u64_at(b, o + 8);
        if reg == 6 || reg == 7 {
            let t = Tex0::decode(u64_at(b, o));
            if gsmem::index_bits(t.psm).is_some() {
                *out.entry(t).or_insert(0) += 1;
            }
        }
        o += 4;
    }
    out
}

/// One frame of a texture-swap animation (vpa kind 6): an image upload
/// and, usually, its own CLUT.
#[derive(Debug, Clone)]
pub struct SwapFrame {
    /// Animation slot (`data +0`).
    pub anim: u32,
    /// Frame number within the slot (file order).
    pub frame: u32,
    pub dbp: u32,
    pub dbw: u32,
    pub dpsm: u32,
    pub pos: (u32, u32),
    pub size: (u32, u32),
    pub pixels: Vec<u8>,
    /// The frame's CLUT in index order (raw GS RGBA), if it has one.
    pub clut: Option<Vec<[u8; 4]>>,
}

/// Texture-swap frames of a `.vpa` (`yiMapFileRead` case 6: `+0x58` packet
/// quadwords, packet at `+0x80`; `+0x64` CLUT quadwords right after it).
pub fn swap_frames(b: &[u8]) -> Vec<SwapFrame> {
    let mut out = Vec::new();
    let mut count: BTreeMap<u32, u32> = BTreeMap::new();
    for r in records(b).into_iter().filter(|r| r.kind == 6) {
        let anim = u32_at(b, r.at + 0x40);
        let qwc = r.header(b, 0x16) as usize;
        let clut_qwc = r.header(b, 0x19) as usize;
        let start = r.at + 0x80;
        let end = (start + qwc * 16).min(b.len());
        let (mut bitblt, mut pos, mut reg) = (None, (0, 0), None);
        let mut image = None;
        let mut o = start;
        while o + 16 <= end {
            let lo = u64_at(b, o);
            let hi = u64_at(b, o + 8);
            match hi {
                0x50 => bitblt = Some(lo),
                0x51 => pos = ((lo >> 32 & 0x7ff) as u32, (lo >> 48 & 0x7ff) as u32),
                0x52 => reg = Some(((lo & 0xfff) as u32, (lo >> 32 & 0xfff) as u32)),
                _ => {}
            }
            // GIFtag, FLG = IMAGE
            if bitblt.is_some() && reg.is_some() && lo >> 58 & 3 == 2 && hi == 0 {
                let nloop = (lo & 0x7fff) as usize;
                image = Some((o + 16, nloop * 16));
                break;
            }
            o += 16;
        }
        let (Some(bb), Some(size), Some((data, len))) = (bitblt, reg, image) else {
            continue;
        };
        let pixels = b[data.min(b.len())..(data + len).min(b.len())].to_vec();
        let clut = (clut_qwc > 0).then(|| {
            let c = &b[end.min(b.len())..(end + clut_qwc * 16).min(b.len())];
            let raw: Vec<[u8; 4]> = c
                .chunks_exact(4)
                .map(|e| [e[0], e[1], e[2], e[3]])
                .collect();
            if raw.len() == 256 {
                (0..256).map(|i| raw[gsmem::csm1(i) as usize]).collect()
            } else {
                raw
            }
        });
        let n = count.entry(anim).or_insert(0);
        out.push(SwapFrame {
            anim,
            frame: *n,
            dbp: (bb >> 32 & 0x3fff) as u32,
            dbw: (bb >> 48 & 63) as u32,
            dpsm: (bb >> 56 & 63) as u32,
            pos,
            size,
            pixels,
            clut,
        });
        *n += 1;
    }
    out
}
