//! PS2 effect textures: the oef2 blocks of the spell overlays
//! (`_host0/magic/mag_NNNN.bin`) and of the field and battle maps' particle
//! effects (`mapout.txa`). Layout: `docs/effects.md`.
//!
//! An oef2 comes in two parts. The *table* holds, per texture, the `TEX0`
//! the effect draws with and the upload rectangle, and per CLUT its address.
//! The *data* block holds the offsets of the pixels and palettes. The files
//! carry no index of their oef2s, so [`tables`] finds them by structure.

use crate::phyretex::Rgba;

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> usize {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes")) as usize
}

pub const PSMCT32: u8 = 0x00;
pub const PSMT8: u8 = 0x13;
pub const PSMT4: u8 = 0x14;

fn bits_per_texel(psm: u8) -> Option<usize> {
    match psm {
        PSMCT32 => Some(32),
        PSMT8 => Some(8),
        PSMT4 => Some(4),
        _ => None,
    }
}

/// Table header: offset of the texture table, the CLUT table, the data
/// block; texture and CLUT counts.
const H_TEXTURES: usize = 0x08;
const H_CLUTS: usize = 0x0c;
const H_DATA: usize = 0x3c;
const H_SET: usize = 0x40;
const H_TEXTURE_COUNT: usize = 0x44;
const H_CLUT_COUNT: usize = 0x46;
const HEADER: usize = 0x54;
const TEXTURE_ENTRY: usize = 0x20;
const CLUT_ENTRY: usize = 0x10;

/// One texture-table entry (32 bytes) with its pixels.
#[derive(Debug, Clone)]
pub struct Texture {
    /// Index in the table.
    pub index: usize,
    /// `TEX0`: where and how the effect reads it.
    pub tbp: u16,
    pub tbw: u8,
    pub psm: u8,
    /// log2 of the width and height.
    pub tw: u8,
    pub th: u8,
    pub cbp: u16,
    pub csa: u8,
    /// The upload rectangle (`sceGsSetDefLoadImage`).
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub dbw: u8,
    /// Offset of the pixels in the file (linear, as uploaded).
    pub data: usize,
}

impl Texture {
    pub fn width(&self) -> u32 {
        1 << self.tw
    }
    pub fn height(&self) -> u32 {
        1 << self.th
    }
    /// The HD name of the texture as the effect reads it:
    /// `TBP_PSM_0_0_W_H` from `TEX0`.
    pub fn tex0_name(&self, set: usize) -> String {
        format!(
            "{}_{}_0_{set}_{}_{}",
            self.tbp,
            self.psm,
            self.width(),
            self.height()
        )
    }
    /// The HD names of the uploaded rectangle: `TBP_PSM_X_Y_W_H` (map
    /// particles), `TBP_X_Y_BPP_W_H` (monster particles).
    pub fn upload_names(&self) -> [String; 2] {
        let bpp = bits_per_texel(self.psm).unwrap_or(0);
        [
            format!(
                "{}_{}_{}_{}_{}_{}",
                self.tbp, self.psm, self.x, self.y, self.w, self.h
            ),
            format!(
                "{}_{}_{}_{bpp}_{}_{}",
                self.tbp, self.x, self.y, self.w, self.h
            ),
        ]
    }
    fn rect_overlaps(&self, o: &Texture) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

/// A palette by its GS address, in index order (CSM1 undone), alpha scaled
/// from the GS's 0x80 to 255.
#[derive(Debug, Clone)]
pub struct Clut {
    pub cbp: u16,
    pub colours: Vec<[u8; 4]>,
}

#[derive(Debug, Clone)]
pub struct Table {
    /// File offsets of the table and the data block it was paired with
    /// (for debugging).
    #[allow(dead_code)]
    pub offset: usize,
    #[allow(dead_code)]
    pub data: usize,
    pub textures: Vec<Texture>,
    pub cluts: Vec<Clut>,
}

fn gs_alpha(a: u8) -> u8 {
    if a >= 0x80 {
        255
    } else {
        ((u32::from(a) * 255 + 64) / 128) as u8
    }
}

/// CSM1 stores a 256-entry palette with index bits 3 and 4 swapped.
fn csm1(i: usize) -> usize {
    (i & 0xe7) | (i & 8) << 1 | (i & 0x10) >> 1
}

/// A table entry, if it is a plausible one; `Ok(None)` for an all-zero
/// (unused) entry.
fn texture_entry(b: &[u8], o: usize, index: usize) -> Result<Option<Texture>, ()> {
    let e = &b[o..o + TEXTURE_ENTRY];
    if e.iter().all(|&c| c == 0) {
        return Ok(None);
    }
    let tex0 = u64::from_le_bytes(e[0..8].try_into().expect("8 bytes"));
    let t = Texture {
        index,
        tbp: (tex0 & 0x3fff) as u16,
        tbw: (tex0 >> 14 & 0x3f) as u8,
        psm: (tex0 >> 20 & 0x3f) as u8,
        tw: (tex0 >> 26 & 0xf) as u8,
        th: (tex0 >> 30 & 0xf) as u8,
        cbp: (tex0 >> 37 & 0x3fff) as u16,
        csa: (tex0 >> 56 & 0x1f) as u8,
        x: u32::from(e[8]) * 16,
        y: u32::from(e[9]) * 16,
        dbw: (u32::from(e[10]) * 16 / 64) as u8,
        w: u32::from(u16_at(e, 0xc)),
        h: u32::from(u16_at(e, 0xe)),
        data: 0,
    };
    let size = u32_at(e, 0x10);
    let upload_psm = e[0x18];
    let dbp = u16_at(e, 0) & 0x3fff;
    let Some(bpp) = bits_per_texel(t.psm) else {
        return Err(());
    };
    if upload_psm != t.psm
        || dbp != t.tbp
        || !(1..=1024).contains(&t.w)
        || !(1..=1024).contains(&t.h)
        || t.tw > 10
        || t.th > 10
        || size != t.w as usize * t.h as usize * bpp / 8
    {
        return Err(());
    }
    Ok(Some(t))
}

/// The texture entries of a table at `t`, if it is one.
fn table_entries(b: &[u8], t: usize) -> Option<Vec<Option<Texture>>> {
    let n = usize::from(u16_at(b, t + H_TEXTURE_COUNT));
    let off = u32_at(b, t + H_TEXTURES);
    if n == 0
        || n > 512
        || off < HEADER
        || !off.is_multiple_of(16)
        || t + off + TEXTURE_ENTRY * n > b.len()
    {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(texture_entry(b, t + off + TEXTURE_ENTRY * i, i).ok()?);
    }
    out.iter().any(Option::is_some).then_some(out)
}

/// The texture offsets of a data block at `d`, if it is one.
fn data_offsets(b: &[u8], d: usize) -> Option<Vec<usize>> {
    let n = usize::from(u16_at(b, d + H_TEXTURE_COUNT));
    if u16_at(b, d + H_SET) != 0 || n == 0 || n > 512 {
        return None;
    }
    let off = u32_at(b, d + H_TEXTURES);
    if off < HEADER || !off.is_multiple_of(4) || d + off + 4 * n > b.len() {
        return None;
    }
    let offs: Vec<usize> = (0..n).map(|i| u32_at(b, d + off + 4 * i)).collect();
    let mut last = off;
    let mut any = false;
    for &o in &offs {
        if o == 0 {
            continue;
        }
        if o < last || d + o > b.len() {
            return None;
        }
        last = o;
        any = true;
    }
    any.then_some(offs)
}

/// Every oef2 table in a file, each with the data block that holds its
/// pixels: its own (`+0x3c`) if that one has them, otherwise every other
/// data block with as many textures whose gaps fit the sizes.
pub fn tables(b: &[u8]) -> Vec<Table> {
    if b.len() < HEADER {
        return Vec::new();
    }
    let last = b.len() - HEADER;
    let mut datas = Vec::new();
    for d in (0..=last).step_by(16) {
        if let Some(o) = data_offsets(b, d) {
            datas.push((d, o));
        }
    }
    let mut out = Vec::new();
    for t in (0..=last).step_by(16) {
        let Some(entries) = table_entries(b, t) else {
            continue;
        };
        let n = entries.len();
        let size =
            |e: &Texture| e.w as usize * e.h as usize * bits_per_texel(e.psm).unwrap_or(0) / 8;
        let fits = |d: usize, offs: &[usize]| -> bool {
            if offs.len() != n {
                return false;
            }
            for (i, e) in entries.iter().enumerate() {
                let Some(e) = e else { continue };
                if offs[i] == 0 {
                    continue;
                }
                let end = offs[i] + size(e);
                if d + end > b.len() {
                    return false;
                }
                if offs[i + 1..]
                    .iter()
                    .find(|&&o| o != 0)
                    .is_some_and(|&next| next < end)
                {
                    return false;
                }
            }
            true
        };
        let own = t + u32_at(b, t + H_DATA);
        let mut chosen: Vec<&(usize, Vec<usize>)> =
            datas.iter().filter(|(d, o)| fits(*d, o)).collect();
        if chosen.iter().any(|(d, _)| *d == own) {
            chosen.retain(|(d, _)| *d == own);
        }
        for (d, offs) in chosen {
            let textures = entries
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    let mut e = e.clone()?;
                    (offs[i] != 0).then(|| {
                        e.data = d + offs[i];
                        e
                    })
                })
                .collect();
            out.push(Table {
                offset: t,
                data: *d,
                textures,
                cluts: cluts(b, t, *d),
            });
        }
    }
    out
}

/// The CLUTs of a table `t` with the data block `d`. A CLUT entry is
/// `u16 CBP`, `u16` byte size (0x400: 256 colours, 0x40: 16), ...; the
/// data block lists the offsets of the colours.
fn cluts(b: &[u8], t: usize, d: usize) -> Vec<Clut> {
    let m = usize::from(u16_at(b, t + H_CLUT_COUNT)).min(usize::from(u16_at(b, d + H_CLUT_COUNT)));
    let (ct, dt) = (t + u32_at(b, t + H_CLUTS), d + u32_at(b, d + H_CLUTS));
    let mut out = Vec::new();
    for j in 0..m {
        let (e, p) = (ct + CLUT_ENTRY * j, dt + 4 * j);
        if e + CLUT_ENTRY > b.len() || p + 4 > b.len() {
            break;
        }
        let (cbp, bytes, off) = (u16_at(b, e), usize::from(u16_at(b, e + 2)), u32_at(b, p));
        let count = bytes / 4;
        if off == 0 || !(count == 16 || count == 256) || d + off + bytes > b.len() {
            continue;
        }
        let raw = &b[d + off..d + off + bytes];
        let colours = (0..count)
            .map(|i| {
                // 256 colours go up as a 16x16 CT32 rectangle (CSM1 order);
                // 16 colours as 8x2, which is linear.
                let j = if count == 256 { csm1(i) } else { i };
                let c = &raw[4 * j..4 * j + 4];
                [c[0], c[1], c[2], gs_alpha(c[3])]
            })
            .collect();
        out.push(Clut { cbp, colours });
    }
    out
}

// GS local memory addressing (GSdx/PCSX2 `GSLocalMemory`; tools/py/gsmem.py).

const MEM: usize = 4 << 20;
const BLOCK32: [[usize; 8]; 4] = [
    [0, 1, 4, 5, 16, 17, 20, 21],
    [2, 3, 6, 7, 18, 19, 22, 23],
    [8, 9, 12, 13, 24, 25, 28, 29],
    [10, 11, 14, 15, 26, 27, 30, 31],
];
const BLOCK4: [[usize; 4]; 8] = [
    [0, 2, 8, 10],
    [1, 3, 9, 11],
    [4, 6, 12, 14],
    [5, 7, 13, 15],
    [16, 18, 24, 26],
    [17, 19, 25, 27],
    [20, 22, 28, 30],
    [21, 23, 29, 31],
];
const COLUMN32: [[usize; 8]; 8] = [
    [0, 1, 4, 5, 8, 9, 12, 13],
    [2, 3, 6, 7, 10, 11, 14, 15],
    [16, 17, 20, 21, 24, 25, 28, 29],
    [18, 19, 22, 23, 26, 27, 30, 31],
    [32, 33, 36, 37, 40, 41, 44, 45],
    [34, 35, 38, 39, 42, 43, 46, 47],
    [48, 49, 52, 53, 56, 57, 60, 61],
    [50, 51, 54, 55, 58, 59, 62, 63],
];
const C8: [[usize; 16]; 8] = [
    [0, 4, 16, 20, 32, 36, 48, 52, 2, 6, 18, 22, 34, 38, 50, 54],
    [
        8, 12, 24, 28, 40, 44, 56, 60, 10, 14, 26, 30, 42, 46, 58, 62,
    ],
    [33, 37, 49, 53, 1, 5, 17, 21, 35, 39, 51, 55, 3, 7, 19, 23],
    [
        41, 45, 57, 61, 9, 13, 25, 29, 43, 47, 59, 63, 11, 15, 27, 31,
    ],
    [
        96, 100, 112, 116, 64, 68, 80, 84, 98, 102, 114, 118, 66, 70, 82, 86,
    ],
    [
        104, 108, 120, 124, 72, 76, 88, 92, 106, 110, 122, 126, 74, 78, 90, 94,
    ],
    [
        65, 69, 81, 85, 97, 101, 113, 117, 67, 71, 83, 87, 99, 103, 115, 119,
    ],
    [
        73, 77, 89, 93, 105, 109, 121, 125, 75, 79, 91, 95, 107, 111, 123, 127,
    ],
];
const C4: [[usize; 32]; 8] = [
    [
        0, 8, 32, 40, 64, 72, 96, 104, 2, 10, 34, 42, 66, 74, 98, 106, 4, 12, 36, 44, 68, 76, 100,
        108, 6, 14, 38, 46, 70, 78, 102, 110,
    ],
    [
        16, 24, 48, 56, 80, 88, 112, 120, 18, 26, 50, 58, 82, 90, 114, 122, 20, 28, 52, 60, 84, 92,
        116, 124, 22, 30, 54, 62, 86, 94, 118, 126,
    ],
    [
        65, 73, 97, 105, 1, 9, 33, 41, 67, 75, 99, 107, 3, 11, 35, 43, 69, 77, 101, 109, 5, 13, 37,
        45, 71, 79, 103, 111, 7, 15, 39, 47,
    ],
    [
        81, 89, 113, 121, 17, 25, 49, 57, 83, 91, 115, 123, 19, 27, 51, 59, 85, 93, 117, 125, 21,
        29, 53, 61, 87, 95, 119, 127, 23, 31, 55, 63,
    ],
    [
        192, 200, 224, 232, 128, 136, 160, 168, 194, 202, 226, 234, 130, 138, 162, 170, 196, 204,
        228, 236, 132, 140, 164, 172, 198, 206, 230, 238, 134, 142, 166, 174,
    ],
    [
        208, 216, 240, 248, 144, 152, 176, 184, 210, 218, 242, 250, 146, 154, 178, 186, 212, 220,
        244, 252, 148, 156, 180, 188, 214, 222, 246, 254, 150, 158, 182, 190,
    ],
    [
        129, 137, 161, 169, 193, 201, 225, 233, 131, 139, 163, 171, 195, 203, 227, 235, 133, 141,
        165, 173, 197, 205, 229, 237, 135, 143, 167, 175, 199, 207, 231, 239,
    ],
    [
        145, 153, 177, 185, 209, 217, 241, 249, 147, 155, 179, 187, 211, 219, 243, 251, 149, 157,
        181, 189, 213, 221, 245, 253, 151, 159, 183, 191, 215, 223, 247, 255,
    ],
];

/// Byte address of a PSMCT32 texel (4 bytes).
fn addr32(x: usize, y: usize, bp: usize, bw: usize) -> usize {
    let page = (y >> 5) * bw + (x >> 6);
    let block = bp + page * 32 + BLOCK32[(y >> 3) & 3][(x >> 3) & 7];
    ((block << 6) + COLUMN32[y & 7][x & 7]) * 4 % MEM
}

/// Byte address of a PSMT8 texel. `bw` in 64-texel units; a page is
/// 128 wide, so an odd `bw` rounds down, as on the GS.
fn addr8(x: usize, y: usize, bp: usize, bw: usize) -> usize {
    let page = (y >> 6) * (bw >> 1) + (x >> 7);
    let block = bp + page * 32 + BLOCK32[(y >> 4) & 3][(x >> 4) & 7];
    let row = y & 15;
    let col = if row < 8 {
        C8[row][x & 15]
    } else {
        C8[row - 8][x & 15] + 128
    };
    ((block << 8) + col) % MEM
}

/// Nibble address of a PSMT4 texel.
fn addr4(x: usize, y: usize, bp: usize, bw: usize) -> usize {
    let page = (y >> 7) * (bw >> 1) + (x >> 7);
    let block = bp + page * 32 + BLOCK4[(y >> 4) & 7][(x >> 5) & 3];
    let row = y & 15;
    let col = if row < 8 {
        C4[row][x & 31]
    } else {
        C4[row - 8][x & 31] + 256
    };
    ((block << 9) + col) % (MEM * 2)
}

/// GS memory that remembers which upload wrote each byte, so that every
/// texel is read through the palette of the upload it came from.
pub struct Gs {
    mem: Vec<u8>,
    owner: Vec<u16>,
    dirty: Vec<usize>,
}

const NOBODY: u16 = u16::MAX;

impl Default for Gs {
    fn default() -> Self {
        Self {
            mem: vec![0; MEM],
            owner: vec![NOBODY; MEM],
            dirty: Vec::new(),
        }
    }
}

impl Gs {
    fn clear(&mut self) {
        for &a in &self.dirty {
            self.mem[a] = 0;
            self.owner[a] = NOBODY;
        }
        self.dirty.clear();
    }

    fn set(&mut self, a: usize, v: u8, who: u16) {
        self.mem[a] = v;
        if self.owner[a] == NOBODY {
            self.dirty.push(a);
        }
        self.owner[a] = who;
    }

    /// Host-to-local transfer of `t`'s pixels.
    fn upload(&mut self, file: &[u8], t: &Texture, who: u16) {
        let (bp, bw) = (usize::from(t.tbp), usize::from(t.dbw));
        let (x0, y0, w, h) = (t.x as usize, t.y as usize, t.w as usize, t.h as usize);
        let src = &file[t.data..];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (px, py) = (x0 + x, y0 + y);
                match t.psm {
                    PSMT8 => self.set(addr8(px, py, bp, bw), src[i], who),
                    PSMT4 => {
                        let v = src[i >> 1] >> (4 * (i & 1)) & 0xf;
                        let a = addr4(px, py, bp, bw);
                        let old = self.mem[a >> 1];
                        let new = if a & 1 == 1 {
                            old & 0x0f | v << 4
                        } else {
                            old & 0xf0 | v
                        };
                        self.set(a >> 1, new, who);
                    }
                    _ => {
                        let a = addr32(px, py, bp, bw);
                        for k in 0..4 {
                            self.set(a + k, src[4 * i + k], who);
                        }
                    }
                }
            }
        }
    }

    /// Reads `t`'s `TEX0` rectangle; each texel through the CLUT of the
    /// upload that wrote it. Texels nothing wrote are transparent.
    fn read(&self, t: &Texture, pal: &[Option<&Clut>]) -> Rgba {
        self.read_rect(t, t.tbw, (0, 0, t.width(), t.height()), pal)
    }

    /// Reads a rectangle of `t`'s buffer (`bw` wide).
    fn read_rect(
        &self,
        t: &Texture,
        bw: u8,
        (x0, y0, w, h): (u32, u32, u32, u32),
        pal: &[Option<&Clut>],
    ) -> Rgba {
        let (bp, bw) = (usize::from(t.tbp), usize::from(bw));
        let mut img = Rgba::new(w, h);
        for py in 0..h {
            for px in 0..w {
                let (x, y) = ((x0 + px) as usize, (y0 + py) as usize);
                let (a, index) = match t.psm {
                    PSMT8 => {
                        let a = addr8(x, y, bp, bw);
                        (a, usize::from(self.mem[a]))
                    }
                    PSMT4 => {
                        let a = addr4(x, y, bp, bw);
                        (a >> 1, usize::from(self.mem[a >> 1] >> (4 * (a & 1)) & 0xf))
                    }
                    _ => (addr32(x, y, bp, bw), 0),
                };
                let who = self.owner[a];
                if who == NOBODY {
                    continue;
                }
                let c = if t.psm == PSMCT32 {
                    let m = &self.mem[a..a + 4];
                    [m[0], m[1], m[2], gs_alpha(m[3])]
                } else {
                    let Some(Some(clut)) = pal.get(usize::from(who)) else {
                        continue;
                    };
                    let i = if t.psm == PSMT4 {
                        usize::from(t.csa) * 16 + index
                    } else {
                        index
                    };
                    clut.colours.get(i).copied().unwrap_or([0; 4])
                };
                img.put(px, py, c);
            }
        }
        img
    }
}

/// One image the effect can show, with the HD names it may go by.
pub struct Image {
    /// Index of the table in the file (0 = first).
    pub set: usize,
    /// `TEX0` name, with `_N` for the N-th upload to the same place
    /// (`13440_19_0_0_256_256_2`).
    pub tex0_name: String,
    /// Upload-rectangle names, for single uploads.
    pub upload_names: Vec<String>,
    /// The `TEX0` name with the set index in the fourth field
    /// (`13632_19_0_1_256_256`), for sets after the first.
    pub set_name: Option<String>,
    pub image: Rgba,
    /// Texture indices that make it up.
    pub parts: Vec<usize>,
    /// Problems (missing palette).
    pub missing_clut: bool,
}

impl Image {
    /// Every name the image may go by.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.tex0_name.as_str())
            .filter(|n| !n.is_empty())
            .chain(self.upload_names.iter().map(String::as_str))
            .chain(self.set_name.as_deref())
    }
}

/// Decodes all images of a file's tables. Textures with the same `TEX0`
/// name are drawn together (an effect may upload a texture in parts, each
/// with its own palette); a part that lands on an earlier one starts the
/// next variant.
pub fn images(file: &[u8], tables: &[Table], gs: &mut Gs) -> Vec<Image> {
    let mut out = Vec::new();
    for (set, table) in tables.iter().enumerate() {
        // Group by TEX0 name, keeping first-seen order.
        let mut groups: Vec<(String, Vec<Vec<&Texture>>)> = Vec::new();
        for t in &table.textures {
            let name = t.tex0_name(0);
            let g = match groups.iter_mut().find(|(n, _)| *n == name) {
                Some(g) => g,
                None => {
                    groups.push((name, Vec::new()));
                    groups.last_mut().expect("just pushed")
                }
            };
            match g.1.last_mut() {
                Some(v) if !v.iter().any(|o| o.rect_overlaps(t)) => v.push(t),
                _ => g.1.push(vec![t]),
            }
        }
        for (name, variants) in groups {
            for (k, parts) in variants.iter().enumerate() {
                gs.clear();
                for (who, t) in parts.iter().enumerate() {
                    gs.upload(file, t, who as u16);
                }
                let mut missing = false;
                let pal: Vec<Option<&Clut>> = parts
                    .iter()
                    .map(|t| table.cluts.iter().find(|c| c.cbp == t.cbp))
                    .collect();
                for t in parts {
                    if t.psm != PSMCT32 && !table.cluts.iter().any(|c| c.cbp == t.cbp) {
                        missing = true;
                    }
                }
                let image = gs.read(parts[0], &pal);
                let suffix = if k == 0 {
                    String::new()
                } else {
                    format!("_{}", k + 1)
                };
                let first = parts[0];
                out.push(Image {
                    set,
                    tex0_name: format!("{name}{suffix}"),
                    upload_names: if parts.len() == 1 && k == 0 {
                        first.upload_names().to_vec()
                    } else {
                        Vec::new()
                    },
                    set_name: (set > 0).then(|| format!("{}{suffix}", first.tex0_name(set))),
                    image,
                    parts: parts.iter().map(|t| t.index).collect(),
                    missing_clut: missing,
                });
            }
        }
        // Partial uploads also go by their own rectangle (field particles:
        // `13888_19_64_0_64_64`).
        for t in &table.textures {
            if (t.x, t.y, t.w, t.h) == (0, 0, t.width(), t.height()) {
                continue;
            }
            gs.clear();
            gs.upload(file, t, 0);
            let clut = table.cluts.iter().find(|c| c.cbp == t.cbp);
            let rect = gs.read_rect(t, t.dbw, (t.x, t.y, t.w, t.h), &[clut]);
            // Some HD textures named after the rectangle hold the whole
            // `TEX0` area (monster particles); offer both.
            let whole = out
                .iter()
                .find(|i| i.set == set && i.parts.contains(&t.index))
                .map(|i| i.image.clone());
            for image in std::iter::once(rect).chain(whole) {
                out.push(Image {
                    set,
                    tex0_name: String::new(),
                    upload_names: t.upload_names().to_vec(),
                    set_name: None,
                    image,
                    parts: vec![t.index],
                    missing_clut: t.psm != PSMCT32 && clut.is_none(),
                });
            }
        }
    }
    gs.clear();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T8 written and read back with the same buffer width is the identity.
    #[test]
    fn t8_roundtrip() {
        let mut gs = Gs::default();
        let t = Texture {
            index: 0,
            tbp: 0x3400,
            tbw: 2,
            psm: PSMT8,
            tw: 7,
            th: 6,
            cbp: 0,
            csa: 0,
            x: 0,
            y: 0,
            w: 128,
            h: 64,
            dbw: 2,
            data: 0,
        };
        let file: Vec<u8> = (0..128 * 64).map(|i| (i * 7 % 251) as u8).collect();
        gs.upload(&file, &t, 0);
        let grey = Clut {
            cbp: 0,
            colours: (0..256).map(|i| [i as u8, 0, 0, 255]).collect(),
        };
        let img = gs.read(&t, &[Some(&grey)]);
        for (i, px) in img.pixels.chunks(4).enumerate() {
            assert_eq!(px[0], file[i]);
        }
    }
}
