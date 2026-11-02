//! PS2 GS local memory: the swizzled layouts of the pixel formats FFX's
//! maps use, enough to upload data in one format and read it in another.
//! Tables as in GSdx/PCSX2 `GSLocalMemory` (port of `tools/py/gsmem.py`).
//!
//! `bp` is a block address (256-byte blocks), `bw` a buffer width in
//! 64-pixel units. A page is 32 blocks (8 KiB).

/// 4 MiB of GS memory.
pub const MEM_SIZE: usize = 4 << 20;

pub const PSMCT32: u32 = 0x00;
pub const PSMT8: u32 = 0x13;
pub const PSMT4: u32 = 0x14;
pub const PSMT8H: u32 = 0x1b;
pub const PSMT4HH: u32 = 0x24;
pub const PSMT4HL: u32 = 0x2c;

/// Bits per texel of an indexed format, or `None`.
pub fn index_bits(psm: u32) -> Option<u32> {
    match psm {
        PSMT8 | PSMT8H => Some(8),
        PSMT4 | PSMT4HH | PSMT4HL => Some(4),
        _ => None,
    }
}

/// Block order inside a PSMCT32 page (8x4 blocks of 8x8 pixels); the same
/// for PSMT8 (blocks of 16x16).
const BLOCK32: [[u32; 8]; 4] = [
    [0, 1, 4, 5, 16, 17, 20, 21],
    [2, 3, 6, 7, 18, 19, 22, 23],
    [8, 9, 12, 13, 24, 25, 28, 29],
    [10, 11, 14, 15, 26, 27, 30, 31],
];

/// Block order inside a PSMT4 page (4x8 blocks of 32x16).
const BLOCK4: [[u32; 4]; 8] = [
    [0, 2, 8, 10],
    [1, 3, 9, 11],
    [4, 6, 12, 14],
    [5, 7, 13, 15],
    [16, 18, 24, 26],
    [17, 19, 25, 27],
    [20, 22, 28, 30],
    [21, 23, 29, 31],
];

/// Word inside an 8x8 PSMCT32 block.
const COLUMN32: [[u32; 8]; 8] = [
    [0, 1, 4, 5, 8, 9, 12, 13],
    [2, 3, 6, 7, 10, 11, 14, 15],
    [16, 17, 20, 21, 24, 25, 28, 29],
    [18, 19, 22, 23, 26, 27, 30, 31],
    [32, 33, 36, 37, 40, 41, 44, 45],
    [34, 35, 38, 39, 42, 43, 46, 47],
    [48, 49, 52, 53, 56, 57, 60, 61],
    [50, 51, 54, 55, 58, 59, 62, 63],
];

/// Byte inside a 16x16 PSMT8 block (rows 8..15: the same plus 128).
const COLUMN8: [[u32; 16]; 8] = [
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

/// Nibble inside a 32x16 PSMT4 block (rows 8..15: the same plus 256).
const COLUMN4: [[u32; 32]; 8] = [
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

/// Word address of a PSMCT32 pixel.
pub fn addr32(x: u32, y: u32, bp: u32, bw: u32) -> usize {
    let bw = bw.max(1);
    let page = (y >> 5) * bw + (x >> 6);
    let block = bp + page * 32 + BLOCK32[(y as usize >> 3) & 3][(x as usize >> 3) & 7];
    ((block as usize) << 6) + COLUMN32[y as usize & 7][x as usize & 7] as usize
}

/// Byte address of a PSMT8 texel.
pub fn addr8(x: u32, y: u32, bp: u32, bw: u32) -> usize {
    let page = (y >> 6) * (bw >> 1).max(1) + (x >> 7);
    let block = bp + page * 32 + BLOCK32[(y as usize >> 4) & 3][(x as usize >> 4) & 7];
    let col = COLUMN8[y as usize & 7][x as usize & 15] + if y & 8 != 0 { 128 } else { 0 };
    ((block as usize) << 8) + col as usize
}

/// Nibble address of a PSMT4 texel.
pub fn addr4(x: u32, y: u32, bp: u32, bw: u32) -> usize {
    let page = (y >> 7) * (bw >> 1).max(1) + (x >> 7);
    let block = bp + page * 32 + BLOCK4[(y as usize >> 4) & 7][(x as usize >> 5) & 3];
    let col = COLUMN4[y as usize & 7][x as usize & 31] + if y & 8 != 0 { 256 } else { 0 };
    ((block as usize) << 9) + col as usize
}

/// GS local memory.
#[derive(Clone)]
pub struct Gs {
    pub mem: Vec<u8>,
}

impl Default for Gs {
    fn default() -> Self {
        Self::new()
    }
}

impl Gs {
    pub fn new() -> Self {
        Self {
            mem: vec![0; MEM_SIZE],
        }
    }

    pub fn word(&self, a: usize) -> u32 {
        let a = (a * 4) % MEM_SIZE;
        u32::from_le_bytes(self.mem[a..a + 4].try_into().expect("4 bytes"))
    }

    fn set_word(&mut self, a: usize, v: u32) {
        let a = (a * 4) % MEM_SIZE;
        self.mem[a..a + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// A host-to-local transfer (`BITBLTBUF`/`TRXPOS`/`TRXREG` + IMAGE data)
    /// of a `w` x `h` rectangle at (`dx`, `dy`). The data is in transfer
    /// order: rows top first, 4-bit formats low nibble first. Returns false
    /// for formats this emulation does not know (nothing is written).
    #[allow(clippy::too_many_arguments)]
    pub fn upload(
        &mut self,
        psm: u32,
        bp: u32,
        bw: u32,
        (dx, dy): (u32, u32),
        (w, h): (u32, u32),
        data: &[u8],
    ) -> bool {
        let n = (w * h) as usize;
        let byte = |i: usize| data.get(i).copied().unwrap_or(0);
        let nib = |i: usize| (byte(i >> 1) >> ((i & 1) * 4)) & 15;
        match psm {
            PSMCT32 => {
                for i in 0..n {
                    let (x, y) = (i as u32 % w + dx, i as u32 / w + dy);
                    let v = u32::from_le_bytes([
                        byte(4 * i),
                        byte(4 * i + 1),
                        byte(4 * i + 2),
                        byte(4 * i + 3),
                    ]);
                    self.set_word(addr32(x, y, bp, bw), v);
                }
            }
            PSMT8 => {
                for i in 0..n {
                    let (x, y) = (i as u32 % w + dx, i as u32 / w + dy);
                    self.mem[addr8(x, y, bp, bw) % MEM_SIZE] = byte(i);
                }
            }
            PSMT4 => {
                for i in 0..n {
                    let (x, y) = (i as u32 % w + dx, i as u32 / w + dy);
                    let a = addr4(x, y, bp, bw) % (MEM_SIZE * 2);
                    let sh = (a & 1) * 4;
                    let m = &mut self.mem[a >> 1];
                    *m = (*m & !(15 << sh)) | (nib(i) << sh);
                }
            }
            PSMT8H | PSMT4HL | PSMT4HH => {
                for i in 0..n {
                    let (x, y) = (i as u32 % w + dx, i as u32 / w + dy);
                    let a = addr32(x, y, bp, bw);
                    let v = self.word(a);
                    let v = match psm {
                        PSMT8H => (v & 0x00ff_ffff) | (u32::from(byte(i)) << 24),
                        PSMT4HL => (v & !0x0f00_0000) | (u32::from(nib(i)) << 24),
                        _ => (v & !0xf000_0000) | (u32::from(nib(i)) << 28),
                    };
                    self.set_word(a, v);
                }
            }
            _ => return false,
        }
        true
    }

    /// The index of an indexed texel.
    pub fn index(&self, psm: u32, tbp: u32, tbw: u32, x: u32, y: u32) -> u8 {
        match psm {
            PSMT8 => self.mem[addr8(x, y, tbp, tbw) % MEM_SIZE],
            PSMT4 => {
                let a = addr4(x, y, tbp, tbw) % (MEM_SIZE * 2);
                (self.mem[a >> 1] >> ((a & 1) * 4)) & 15
            }
            PSMT8H => (self.word(addr32(x, y, tbp, tbw)) >> 24) as u8,
            PSMT4HL => ((self.word(addr32(x, y, tbp, tbw)) >> 24) & 15) as u8,
            PSMT4HH => (self.word(addr32(x, y, tbp, tbw)) >> 28) as u8,
            _ => 0,
        }
    }

    /// The colours an indexed texture reads with `TEX0` CBP/CSA, CPSM =
    /// PSMCT32, CSM = CSM1 (all FFX map TEX0s): 256 entries for 8-bit
    /// formats, 16 (entries `csa * 16 ..`) for 4-bit ones. Raw GS RGBA.
    pub fn clut(&self, cbp: u32, csa: u32, bits: u32) -> Vec<[u8; 4]> {
        let (first, n) = if bits == 8 { (0, 256) } else { (csa * 16, 16) };
        (first..first + n)
            .map(|i| {
                let j = csm1(i & 0xff);
                self.word(addr32(j & 15, j >> 4, cbp, 1)).to_le_bytes()
            })
            .collect()
    }
}

/// CSM1: a 256-colour CLUT stored as a 16x16 PSMCT32 rectangle holds index
/// `i` at position `csm1(i)` (index bits 3 and 4 swapped).
pub fn csm1(i: u32) -> u32 {
    (i & 0xe7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)
}

/// GS alpha (0x80 = opaque) to 8-bit alpha.
pub fn alpha(a: u8) -> u8 {
    if a >= 0x80 {
        255
    } else {
        ((u32::from(a) * 255 + 64) / 128) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A T4 image written as PSMCT32 at half width and quarter height
    /// reads back as T4 the way the closed-form PSMT4 <-> PSMCT32 relation
    /// says: every T4 address maps to one distinct nibble.
    #[test]
    fn addresses_are_bijective() {
        let mut seen = vec![false; 2 * 8192 * 2];
        for y in 0..128 {
            for x in 0..128 {
                let a = addr4(x, y, 0, 2);
                assert!(!seen[a], "T4 collision");
                seen[a] = true;
            }
        }
        let mut seen = vec![false; 8192 * 2];
        for y in 0..64 {
            for x in 0..256 {
                let a = addr8(x, y, 0, 4);
                assert!(!seen[a], "T8 collision");
                seen[a] = true;
            }
        }
    }

    #[test]
    fn t8_roundtrip() {
        let mut gs = Gs::new();
        let data: Vec<u8> = (0..256 * 256).map(|i| (i * 7 + i / 256) as u8).collect();
        gs.upload(PSMT8, 0x100, 4, (0, 0), (256, 256), &data);
        for y in 0..256 {
            for x in 0..256 {
                assert_eq!(
                    gs.index(PSMT8, 0x100, 4, x, y),
                    data[(y * 256 + x) as usize]
                );
            }
        }
    }
}
