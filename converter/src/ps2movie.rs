//! The PS2 FMVs: `jppc/movie/adpcm/<name>.dat` with its `<name>.sz`
//! (`docs/movies.md`).
//!
//! `.sz`: the IPU's intra and non-intra quantiser matrices (64 bytes each,
//! uploaded with `SETIQ`), then one `u32` per frame: the size of that
//! frame's chunk in `.dat` (0 ends the list).
//!
//! A chunk is a run of 16-byte-headed records. The first is the picture
//! (`u32` bitstream size, `u32`, then bytes `+8` type (0 intra, 1
//! predicted), `+9` intra quantiser, `+0xa` non-intra quantiser, `+0xb`/`+0xc`
//! width/height in macroblocks, `+0xe`/`+0xf` IPU control bits); then sound
//! (`0x40` delta-coded, `0x50`/`0x51` LZ-packed delta-coded, `0x7800` raw
//! PCM), a camera record (`0x30`, for movies with a 3D scene on top) and an
//! all-zero end record.
//!
//! The picture bitstream is MPEG-2 macroblock syntax the PS2 decoded with
//! the IPU (`VDEC`/`BDEC`) and the EE (motion compensation): FFX.exe still
//! has the PS2 player (`MovieViewer_IntraDecode` `0x0077b480`, the
//! predicted-picture decoder `0x0077b630`, `IpuLoadMacroblock_0077e210`).
//! The IPU itself is not in FFX.exe; the block decode and IDCT follow
//! PCSX2's IPU emulation.

use std::sync::OnceLock;

use anyhow::{Context, Result, bail, ensure};

/// A decoded picture, YCbCr 4:2:0 (studio range, BT.601).
#[derive(Clone)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub y: Vec<u8>,
    pub cb: Vec<u8>,
    pub cr: Vec<u8>,
}

impl Picture {
    fn new(width: usize, height: usize) -> Self {
        Picture {
            width,
            height,
            y: vec![0; width * height],
            cb: vec![128; width * height / 4],
            cr: vec![128; width * height / 4],
        }
    }

    /// RGBA through the IPU's colour conversion (`CSC`), for previews.
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for yy in 0..self.height {
            for xx in 0..self.width {
                let c = (yy / 2) * (self.width / 2) + xx / 2;
                let lum = (0x95 * (i32::from(self.y[yy * self.width + xx]) - 16).max(0)) >> 6;
                let cr = i32::from(self.cr[c]) - 128;
                let cb = i32::from(self.cb[c]) - 128;
                let r = (lum + ((0xcc * cr) >> 6) + 1) >> 1;
                let g = (lum + ((-0x68 * cr) >> 6) + ((-0x32 * cb) >> 6) + 1) >> 1;
                let b = (lum + ((0x102 * cb) >> 6) + 1) >> 1;
                out.extend([r, g, b].map(|v| v.clamp(0, 255) as u8));
                out.push(255);
            }
        }
        out
    }

    /// The picture scaled horizontally to `width` (linear). Some movies
    /// switch between 640- and 512-wide pictures; both fill the 4:3 screen.
    pub fn widen(&self, width: usize) -> Picture {
        if width == self.width {
            return self.clone();
        }
        let scale = |src: &[u8], sw: usize, dw: usize, h: usize| {
            let mut out = vec![0u8; dw * h];
            for row in 0..h {
                for x in 0..dw {
                    // Sample centres line up: (x + 0.5) * sw / dw - 0.5.
                    let pos = ((2 * x + 1) * sw) as f64 / (2 * dw) as f64 - 0.5;
                    let pos = pos.max(0.0);
                    let i = pos.floor() as usize;
                    let f = pos - i as f64;
                    let a = f64::from(src[row * sw + i.min(sw - 1)]);
                    let b = f64::from(src[row * sw + (i + 1).min(sw - 1)]);
                    out[row * dw + x] = (a + (b - a) * f).round() as u8;
                }
            }
            out
        };
        Picture {
            width,
            height: self.height,
            y: scale(&self.y, self.width, width, self.height),
            cb: scale(&self.cb, self.width / 2, width / 2, self.height / 2),
            cr: scale(&self.cr, self.width / 2, width / 2, self.height / 2),
        }
    }
}

/// The `.sz` file: quantiser matrices and chunk sizes.
pub struct Index {
    pub intra: [u8; 64],
    pub non_intra: [u8; 64],
    pub sizes: Vec<u32>,
}

impl Index {
    pub fn parse(sz: &[u8]) -> Result<Index> {
        ensure!(sz.len() >= 128 && sz.len() % 4 == 0, "movie size file of {} bytes", sz.len());
        let mut intra = [0; 64];
        let mut non_intra = [0; 64];
        intra.copy_from_slice(&sz[..64]);
        non_intra.copy_from_slice(&sz[64..128]);
        let sizes = sz[128..]
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .take_while(|&s| s != 0)
            .collect();
        Ok(Index { intra, non_intra, sizes })
    }
}

/// One chunk decoded: the picture and the sound that came with it
/// (interleaved stereo, 48 kHz).
pub struct Unit {
    pub picture: Picture,
    pub sound: Vec<i16>,
}

/// Statistics a decoder collects (for the log and checks).
#[derive(Default, Clone, Debug)]
pub struct Stats {
    pub frames: usize,
    pub intra: usize,
    /// Pictures whose bitstream ended more than 4 bytes off the record size.
    pub misaligned: usize,
    pub widths: Vec<usize>,
}

pub struct Decoder {
    iq: [u8; 64],
    niq: [u8; 64],
    /// The macroblock store predicted pictures read: the last intra
    /// picture (the PS2 player writes it only in the intra decoder;
    /// predicting from the previous picture falls apart within frames).
    reference: Option<Picture>,
    pub stats: Stats,
}

impl Decoder {
    pub fn new(index: &Index) -> Decoder {
        Decoder {
            iq: index.intra,
            niq: index.non_intra,
            reference: None,
            stats: Stats::default(),
        }
    }

    pub fn chunk(&mut self, c: &[u8]) -> Result<Unit> {
        ensure!(c.len() >= 16, "chunk of {} bytes", c.len());
        let size = u32_at(c, 0) as usize;
        let body = c.get(16..16 + size).context("picture record past the chunk")?;
        let h = &c[8..16];
        let kind = h[0];
        let (q_intra, q_non) = (h[1], h[2]);
        let (mbx, mby) = (usize::from(h[3]), usize::from(h[4]));
        ensure!(mbx > 0 && mby > 0 && mbx <= 64 && mby <= 64, "picture of {mbx}x{mby} macroblocks");
        // IPU_CTRL: IDP (DC precision), AS (alternate scan), QST (q scale type).
        let idp = u32::from(h[7] >> 1) & 3;
        let alt_scan = h[6] & 1 != 0;
        let nonlinear_q = h[7] & 1 != 0;
        let p = Params {
            idp,
            alt_scan,
            nonlinear_q,
            iq: &self.iq,
            niq: &self.niq,
        };
        let mut bits = Bits::new(body);
        let picture = if kind == 0 {
            let pic = decode_intra(&mut bits, &p, mbx, mby, q_intra)?;
            self.reference = Some(pic.clone());
            self.stats.intra += 1;
            pic
        } else {
            let reference = self
                .reference
                .as_ref()
                .filter(|r| r.width == mbx * 16 && r.height == mby * 16)
                .cloned()
                .unwrap_or_else(|| Picture::new(mbx * 16, mby * 16));
            decode_predicted(&mut bits, &p, mbx, mby, q_intra, q_non, &reference)?
        };
        // The record is padded: the end of picture comes within 32 bytes.
        let used = bits.pos.div_ceil(8);
        if used > size || size - used > 32 {
            self.stats.misaligned += 1;
        }
        self.stats.frames += 1;
        if !self.stats.widths.contains(&picture.width) {
            self.stats.widths.push(picture.width);
        }
        let sound = sound(c, 16 + size)?;
        Ok(Unit {
            picture,
            sound,
        })
    }
}

/// The sound of a chunk alone (interleaved stereo, 48 kHz), without
/// decoding its picture.
pub fn chunk_sound(c: &[u8]) -> Result<Vec<i16>> {
    ensure!(c.len() >= 16, "chunk of {} bytes", c.len());
    sound(c, 16 + u32_at(c, 0) as usize)
}

/// A 48 kHz stereo 16-bit WAV around little-endian PCM bytes.
pub fn wav(pcm: &[u8]) -> Vec<u8> {
    let mut w = Vec::with_capacity(44 + pcm.len());
    w.extend(b"RIFF");
    w.extend((36 + pcm.len() as u32).to_le_bytes());
    w.extend(b"WAVEfmt ");
    w.extend(16u32.to_le_bytes());
    w.extend(1u16.to_le_bytes());
    w.extend(2u16.to_le_bytes());
    w.extend(48000u32.to_le_bytes());
    w.extend((48000u32 * 4).to_le_bytes());
    w.extend(4u16.to_le_bytes());
    w.extend(16u16.to_le_bytes());
    w.extend(b"data");
    w.extend((pcm.len() as u32).to_le_bytes());
    w.extend(pcm);
    w
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

// ---------------------------------------------------------------- sound

/// The sound records after the picture (`MovieDecoder_runCommand_0077e460`):
/// each gives blocks of 256 left then 256 right samples.
fn sound(c: &[u8], mut off: usize) -> Result<Vec<i16>> {
    let mut out = Vec::new();
    while off + 16 <= c.len() {
        let w = [u32_at(c, off), u32_at(c, off + 4), u32_at(c, off + 8), u32_at(c, off + 12)];
        let body = &c[off + 16..];
        let samples: Vec<i16> = match w[0] {
            // Delta codes, LZ-packed (`0x0077b270`); the first w[2] - w[3]
            // bytes are the first track.
            0x50 | 0x51 => {
                let packed = body
                    .get(..(w[2] - w[3]) as usize)
                    .context("sound record past the chunk")?;
                let codes = crate::disc::sa_decode(packed)?;
                let n = (w[1] as usize).min(codes.len());
                off += 16 + w[2] as usize;
                delta(&codes[..n])
            }
            // Delta codes (`0x0077b1f0`).
            0x40 => {
                let codes = body.get(..w[1] as usize).context("sound record past the chunk")?;
                off += 16 + w[2] as usize;
                delta(codes)
            }
            // Raw PCM (`0x0077b060`).
            0x6400 | 0x7800 => {
                let n = w[0] as usize;
                let raw = body.get(..n).context("sound record past the chunk")?;
                off += 16 + n;
                raw.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
            }
            // Camera for the 3D scene drawn over the movie; the PC has its
            // own (`video/<id>.dat`).
            0x30 => {
                off += 16 + 0x30;
                continue;
            }
            0 => break,
            other => bail!("unknown movie record {other:#x}"),
        };
        for block in samples.chunks(512) {
            let (l, r) = block.split_at(block.len().min(256));
            for (i, &s) in l.iter().enumerate() {
                out.push(s);
                out.push(r.get(i).copied().unwrap_or(0));
            }
        }
    }
    Ok(out)
}

/// The movie sound's delta code: a byte with bit 7 set adds its low 7 bits
/// (signed); otherwise two bytes add a 15-bit signed value.
fn delta(codes: &[u8]) -> Vec<i16> {
    let mut out = Vec::with_capacity(codes.len());
    let mut s: i16 = 0;
    let mut i = 0;
    while i < codes.len() {
        let c = codes[i];
        let d = if c & 0x80 != 0 {
            i += 1;
            i16::from(((c << 1) as i8) >> 1)
        } else {
            let hi = i16::from(((c << 1) as i8) >> 1);
            let lo = i16::from(codes.get(i + 1).copied().unwrap_or(0));
            i += 2;
            (hi << 8) | lo
        };
        s = s.wrapping_add(d);
        out.push(s);
    }
    out
}

// ---------------------------------------------------------------- bits

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Bits { data, pos: 0 }
    }

    /// The next `n` (<= 24) bits, zeros past the end.
    fn peek(&self, n: u32) -> u32 {
        let byte = self.pos / 8;
        let mut w: u32 = 0;
        for k in 0..4 {
            w = (w << 8) | u32::from(self.data.get(byte + k).copied().unwrap_or(0));
        }
        (w << (self.pos % 8)) >> (32 - n)
    }

    fn skip(&mut self, n: u32) {
        self.pos += n as usize;
    }

    fn get(&mut self, n: u32) -> u32 {
        let v = self.peek(n);
        self.skip(n);
        v
    }

    fn get_signed(&mut self, n: u32) -> i32 {
        let v = self.get(n) as i32;
        (v << (32 - n)) >> (32 - n)
    }

    fn past_end(&self) -> bool {
        self.pos > self.data.len() * 8
    }
}

// ---------------------------------------------------------------- VLC tables

/// A prefix-code lookup: `bits`-bit index -> (length, value); length 0 is
/// an invalid code.
struct Vlc {
    bits: u32,
    table: Vec<(u8, i16)>,
}

impl Vlc {
    fn new(bits: u32, codes: &[(u32, u32, i16)]) -> Vlc {
        let mut table = vec![(0u8, 0i16); 1 << bits];
        for &(code, len, value) in codes {
            let shift = bits - len;
            let base = (code << shift) as usize;
            for e in &mut table[base..base + (1 << shift)] {
                *e = (len as u8, value);
            }
        }
        Vlc { bits, table }
    }

    fn read(&self, b: &mut Bits) -> Result<i16> {
        let (len, value) = self.table[b.peek(self.bits) as usize];
        ensure!(len > 0, "bad code at bit {}", b.pos);
        b.skip(u32::from(len));
        Ok(value)
    }
}

const ESCAPE: i16 = 111;
const EOB: i16 = 112;

struct Tables {
    mbai: Vlc,
    cbp: Vlc,
    mv: Vlc,
    dc_lum: Vlc,
    dc_chroma: Vlc,
    /// B.14 and B.15: value = index into RUN/LEVEL, or ESCAPE/EOB.
    b14: Vlc,
    b15: Vlc,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut mbai: Vec<(u32, u32, i16)> = MBAI
            .iter()
            .enumerate()
            .map(|(i, &(c, l))| (u32::from(c), u32::from(l), i as i16 + 1))
            .collect();
        mbai.push((0x8, 11, 0x23)); // macroblock_escape (the IPU's 0x23)
        let cbp = CBP
            .iter()
            .enumerate()
            .map(|(i, &(c, l))| (u32::from(c), u32::from(l), i as i16))
            .collect::<Vec<_>>();
        let mv = MV
            .iter()
            .enumerate()
            .map(|(i, &(c, l))| (u32::from(c), u32::from(l), i as i16))
            .collect::<Vec<_>>();
        let dc = |codes: &[(u16, u8)]| {
            codes
                .iter()
                .enumerate()
                .map(|(i, &(c, l))| (u32::from(c), u32::from(l), i as i16))
                .collect::<Vec<_>>()
        };
        let dct = |t: &[(u16, u8)]| {
            t.iter()
                .enumerate()
                .map(|(i, &(c, l))| (u32::from(c), u32::from(l), i as i16))
                .collect::<Vec<_>>()
        };
        Tables {
            mbai: Vlc::new(11, &mbai),
            cbp: Vlc::new(9, &cbp),
            mv: Vlc::new(10, &mv),
            dc_lum: Vlc::new(9, &dc(&DC_LUM)),
            dc_chroma: Vlc::new(10, &dc(&DC_CHROMA)),
            b14: Vlc::new(16, &dct(&B14)),
            b15: Vlc::new(16, &dct(&B15)),
        }
    })
}

// ---------------------------------------------------------------- blocks

struct Params<'a> {
    idp: u32,
    alt_scan: bool,
    nonlinear_q: bool,
    iq: &'a [u8; 64],
    niq: &'a [u8; 64],
}

impl Params<'_> {
    fn scan(&self) -> &'static [u8; 64] {
        if self.alt_scan { &SCAN_ALT } else { &SCAN_NORM }
    }

    fn quantiser(&self, qsc: u8) -> i32 {
        let q = usize::from(qsc & 31);
        if self.nonlinear_q { NONLINEAR_Q[q] } else { (q as i32) << 1 }
    }
}

fn saturate(v: i32) -> i32 {
    v.clamp(-2048, 2047)
}

/// One run/level code of a block.
enum Coef {
    End,
    /// A table code: run, level magnitude, negative.
    Code(usize, i32, bool),
    /// An escape: run, signed 12-bit level.
    Escape(usize, i32),
}

fn coefficient(b: &mut Bits, vlc: &Vlc, first_non_intra: bool) -> Result<Coef> {
    if first_non_intra && b.peek(1) == 1 {
        // B.14's first coefficient: '1s' is run 0, level 1.
        b.skip(1);
        return Ok(Coef::Code(0, 1, b.get(1) == 1));
    }
    Ok(match vlc.read(b)? {
        EOB => Coef::End,
        ESCAPE => {
            let run = b.get(6) as usize;
            Coef::Escape(run, b.get_signed(12))
        }
        k => {
            let k = k as usize;
            Coef::Code(RUN[k] as usize, i32::from(LEVEL[k]), b.get(1) == 1)
        }
    })
}

/// Reads a block's run/level codes from scan position `i`; `level(code, i)`
/// dequantises one. Rounding follows PCSX2: a table code is dequantised as
/// a magnitude and then negated, an escape with its sign.
fn block_codes(
    b: &mut Bits,
    vlc: &Vlc,
    mut i: usize,
    non_intra: bool,
    scan: &[u8; 64],
    blk: &mut [i32; 64],
    level: impl Fn(&Coef, usize) -> i32,
) -> Result<()> {
    loop {
        let c = coefficient(b, vlc, non_intra && i == 0)?;
        let run = match c {
            Coef::End => break,
            Coef::Code(run, ..) | Coef::Escape(run, _) => run,
        };
        i += run;
        if i >= 64 {
            break;
        }
        blk[usize::from(scan[i])] = saturate(level(&c, i));
        i += 1;
        if b.past_end() {
            bail!("bitstream ended inside a block");
        }
    }
    Ok(())
}

/// An intra block into `blk` (natural order, dequantised).
fn intra_block(b: &mut Bits, p: &Params, q: i32, cc: usize, dc_pred: &mut [i32; 3], blk: &mut [i32; 64]) -> Result<()> {
    let t = tables();
    let size = if cc == 0 { t.dc_lum.read(b)? } else { t.dc_chroma.read(b)? } as u32;
    let diff = if size == 0 {
        0
    } else {
        let v = b.get(size) as i32;
        if v & (1 << (size - 1)) == 0 { v - ((1 << size) - 1) } else { v }
    };
    dc_pred[cc] += diff;
    *blk = [0; 64];
    blk[0] = dc_pred[cc] << (3 - p.idp);
    // IVF is always set for intra blocks: table B.15. The matrix is indexed
    // by scan position, as PCSX2's IPU does.
    block_codes(b, &t.b15, 1, false, p.scan(), blk, |c, i| {
        let w = i32::from(p.iq[i]);
        match *c {
            Coef::Code(_, level, neg) => {
                let v = (level * q * w) >> 4;
                if neg { -v } else { v }
            }
            Coef::Escape(_, level) => (level * q * w) >> 4,
            Coef::End => 0,
        }
    })
}

/// A non-intra (residual) block.
fn non_intra_block(b: &mut Bits, p: &Params, q: i32, blk: &mut [i32; 64]) -> Result<()> {
    *blk = [0; 64];
    block_codes(b, &tables().b14, 0, true, p.scan(), blk, |c, i| {
        let w = i32::from(p.niq[i]);
        match *c {
            Coef::Code(_, level, neg) => {
                let v = ((2 * level + 1) * q * w) >> 5;
                if neg { -v } else { v }
            }
            Coef::Escape(_, level) => {
                let sign = if level < 0 { -1 } else { 0 };
                ((2 * (level + sign) + 1) * q * w) >> 5
            }
            Coef::End => 0,
        }
    })
}

/// libmpeg2's C IDCT as in PCSX2 (`IDCT_Block`): its input is in the
/// permuted order ((j & 0x36) >> 1 | (j & 9) << 2).
fn idct(natural: &[i32; 64]) -> [i32; 64] {
    const W1: i32 = 2841;
    const W2: i32 = 2676;
    const W3: i32 = 2408;
    const W5: i32 = 1609;
    const W6: i32 = 1108;
    const W7: i32 = 565;
    fn bf(w0: i32, w1: i32, d0: i32, d1: i32) -> (i32, i32) {
        let tmp = w0 * (d0 + d1);
        (tmp + (w1 - w0) * d1, tmp - (w1 + w0) * d0)
    }
    let mut blk = [0i32; 64];
    for (j, &v) in natural.iter().enumerate() {
        blk[((j & 0x36) >> 1) | ((j & 0x09) << 2)] = v;
    }
    // The block is s16 in PCSX2.
    let s16 = |v: i32| i32::from(v as i16);
    for r in 0..8 {
        let row = &mut blk[r * 8..r * 8 + 8];
        if row[1..].iter().all(|&v| v == 0) {
            let v = s16(i32::from((row[0] << 3) as u16 as i16));
            row.fill(v);
            continue;
        }
        let d0 = (row[0] << 11) + 128;
        let d1 = row[1];
        let d2 = row[2] << 11;
        let d3 = row[3];
        let t0 = d0 + d2;
        let t1 = d0 - d2;
        let (t2, t3) = bf(W6, W2, d3, d1);
        let (a0, a1, a2, a3) = (t0 + t2, t1 + t3, t1 - t3, t0 - t2);
        let (t0, t1) = bf(W7, W1, row[7], row[4]);
        let (t2, t3) = bf(W3, W5, row[5], row[6]);
        let b0 = t0 + t2;
        let b3 = t1 + t3;
        let t0 = t0 - t2;
        let t1 = t1 - t3;
        let b1 = ((t0 + t1) * 181) >> 8;
        let b2 = ((t0 - t1) * 181) >> 8;
        row[0] = s16((a0 + b0) >> 8);
        row[1] = s16((a1 + b1) >> 8);
        row[2] = s16((a2 + b2) >> 8);
        row[3] = s16((a3 + b3) >> 8);
        row[4] = s16((a3 - b3) >> 8);
        row[5] = s16((a2 - b2) >> 8);
        row[6] = s16((a1 - b1) >> 8);
        row[7] = s16((a0 - b0) >> 8);
    }
    for c in 0..8 {
        let g = |k: usize| blk[k * 8 + c];
        let d0 = (g(0) << 11) + 65536;
        let d1 = g(1);
        let d2 = g(2) << 11;
        let d3 = g(3);
        let t0 = d0 + d2;
        let t1 = d0 - d2;
        let (t2, t3) = bf(W6, W2, d3, d1);
        let (a0, a1, a2, a3) = (t0 + t2, t1 + t3, t1 - t3, t0 - t2);
        let (t0, t1) = bf(W7, W1, g(7), g(4));
        let (t2, t3) = bf(W3, W5, g(5), g(6));
        let b0 = t0 + t2;
        let b3 = t1 + t3;
        let t0 = (t0 - t2) >> 8;
        let t1 = (t1 - t3) >> 8;
        let b1 = (t0 + t1) * 181;
        let b2 = (t0 - t1) * 181;
        let out = [a0 + b0, a1 + b1, a2 + b2, a3 + b3, a3 - b3, a2 - b2, a1 - b1, a0 - b0];
        for (k, v) in out.into_iter().enumerate() {
            blk[k * 8 + c] = s16(v >> 17);
        }
    }
    blk
}

/// The macroblock's six blocks: 4 luma (TL, TR, BL, BR), Cb, Cr.
type Mb = [[i32; 64]; 6];

fn decode_intra_mb(b: &mut Bits, p: &Params, q: i32, dc: &mut [i32; 3]) -> Result<Mb> {
    let mut mb = [[0i32; 64]; 6];
    let mut blk = [0i32; 64];
    for (n, out) in mb.iter_mut().enumerate() {
        let cc = if n < 4 { 0 } else { n - 3 };
        intra_block(b, p, q, cc, dc, &mut blk)?;
        *out = idct(&blk).map(|v| v.clamp(0, 255));
    }
    Ok(mb)
}

fn decode_residual_mb(b: &mut Bits, p: &Params, q: i32) -> Result<Mb> {
    let t = tables();
    let cbp = t.cbp.read(b)?;
    let mut mb = [[0i32; 64]; 6];
    let mut blk = [0i32; 64];
    for (n, out) in mb.iter_mut().enumerate() {
        if cbp & (0x20 >> n) != 0 {
            non_intra_block(b, p, q, &mut blk)?;
            *out = idct(&blk);
        }
    }
    Ok(mb)
}

fn put_mb(pic: &mut Picture, mbx: usize, mby: usize, mb: &Mb) {
    let w = pic.width;
    for n in 0..4 {
        let (ox, oy) = (mbx * 16 + (n & 1) * 8, mby * 16 + (n >> 1) * 8);
        for r in 0..8 {
            for c in 0..8 {
                pic.y[(oy + r) * w + ox + c] = mb[n][r * 8 + c].clamp(0, 255) as u8;
            }
        }
    }
    let cw = w / 2;
    for r in 0..8 {
        for c in 0..8 {
            let i = (mby * 8 + r) * cw + mbx * 8 + c;
            pic.cb[i] = mb[4][r * 8 + c].clamp(0, 255) as u8;
            pic.cr[i] = mb[5][r * 8 + c].clamp(0, 255) as u8;
        }
    }
}

/// The reference macroblock at pixel offset (x, y), as `IpuLoadMacroblock`
/// builds it; chroma moves by floor(x / 2). The PS2 clears the quarters of
/// the 2x2-macroblock window that lie outside the picture (green after the
/// colour conversion); a few vectors point there with no residual, so the
/// encoder must have seen real pixels: the edge is repeated instead.
fn predict(r: &Picture, mbx: usize, mby: usize, x: i32, y: i32) -> Mb {
    let mut mb = [[0i32; 64]; 6];
    let (w, h) = (r.width as i32, r.height as i32);
    let at = |plane: &[u8], pw: i32, ph: i32, px: i32, py: i32| {
        i32::from(plane[(py.clamp(0, ph - 1) * pw + px.clamp(0, pw - 1)) as usize])
    };
    for n in 0..4 {
        let ox = mbx as i32 * 16 + (n as i32 & 1) * 8 + x;
        let oy = mby as i32 * 16 + (n as i32 >> 1) * 8 + y;
        for rr in 0..8 {
            for c in 0..8 {
                mb[n][(rr * 8 + c) as usize] = at(&r.y, w, h, ox + c, oy + rr);
            }
        }
    }
    let (cx, cy) = (mbx as i32 * 8 + x.div_euclid(2), mby as i32 * 8 + y.div_euclid(2));
    // Whole chroma pixels: averaging odd vectors (MPEG half-pel) matches
    // the next intra picture worse (measured on sak).
    for rr in 0..8 {
        for c in 0..8 {
            mb[4][(rr * 8 + c) as usize] = at(&r.cb, w / 2, h / 2, cx + c, cy + rr);
            mb[5][(rr * 8 + c) as usize] = at(&r.cr, w / 2, h / 2, cx + c, cy + rr);
        }
    }
    mb
}

/// `MovieViewer_IntraDecode` (`0x0077b480`): per macroblock an address
/// increment and an intra `BDEC`; DC predictors reset at the first.
fn decode_intra(b: &mut Bits, p: &Params, mbx: usize, mby: usize, qsc: u8) -> Result<Picture> {
    let t = tables();
    let mut pic = Picture::new(mbx * 16, mby * 16);
    let q = p.quantiser(qsc);
    let reset = 128 << p.idp;
    let mut dc = [reset; 3];
    for i in 0..mbx * mby {
        loop {
            if t.mbai.read(b)? != 0x23 {
                break;
            }
        }
        let mb = decode_intra_mb(b, p, q, &mut dc).with_context(|| format!("intra macroblock {i}"))?;
        put_mb(&mut pic, i % mbx, i / mbx, &mb);
    }
    Ok(pic)
}

/// The predicted-picture decoder (`0x0077b630`). Per macroblock: a mode
/// coded with the address-increment table (1 motion + residual, 2 intra,
/// 3 motion only, 4 end of picture), a second address increment (escapes
/// skipped), for modes 1 and 3 four motion codes a, b, c, d (x = 16 a + c,
/// y = 16 b + d, whole pixels, from the last intra picture), then the blocks.
fn decode_predicted(
    b: &mut Bits,
    p: &Params,
    mbx: usize,
    mby: usize,
    q_intra: u8,
    q_non: u8,
    reference: &Picture,
) -> Result<Picture> {
    let t = tables();
    let mut pic = reference.clone();
    let qi = p.quantiser(q_intra);
    let qn = p.quantiser(q_non);
    let reset = 128 << p.idp;
    let mut dc = [reset; 3];
    let mut first_intra = true;
    let (mut x, mut y) = (0i32, 0i32);
    let mut mb_index = 0usize;
    loop {
        let mode = t.mbai.read(b)?;
        if mode == 4 {
            break;
        }
        loop {
            if t.mbai.read(b)? != 0x23 {
                break;
            }
        }
        ensure!(mb_index < mbx * mby, "more macroblocks than the picture has");
        let (cx, cy) = (mb_index % mbx, mb_index / mbx);
        if mode & 1 != 0 {
            let mut code = || -> Result<i32> {
                let m = i32::from(t.mv.read(b)?);
                Ok(if m != 0 && b.get(1) == 1 { -m } else { m })
            };
            let (a, bb, c, d) = (code()?, code()?, code()?, code()?);
            x = i32::from((a * 16 + c) as i16);
            y = i32::from((bb * 16 + d) as i16);
        }
        let mb = match mode {
            1 => {
                let res = decode_residual_mb(b, p, qn).with_context(|| format!("macroblock {mb_index}"))?;
                let mut pred = predict(reference, cx, cy, x, y);
                for (pb, rb) in pred.iter_mut().zip(res.iter()) {
                    for (pv, rv) in pb.iter_mut().zip(rb.iter()) {
                        *pv = (*pv + rv).clamp(0, 255);
                    }
                }
                pred
            }
            2 => {
                if first_intra {
                    dc = [reset; 3];
                    first_intra = false;
                }
                decode_intra_mb(b, p, qi, &mut dc).with_context(|| format!("macroblock {mb_index}"))?
            }
            3 => predict(reference, cx, cy, x, y),
            m => bail!("macroblock mode {m} at {mb_index}"),
        };
        put_mb(&mut pic, cx, cy, &mb);
        mb_index += 1;
        if b.past_end() {
            bail!("bitstream ended at macroblock {mb_index}");
        }
    }
    Ok(pic)
}

// ---------------------------------------------------------------- tables
// The MPEG-2 tables (ISO/IEC 13818-2 annex B) as (code, length).

const SCAN_NORM: [u8; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47,
    55, 62, 63,
];

const SCAN_ALT: [u8; 64] = [
    0, 8, 16, 24, 1, 9, 2, 10, 17, 25, 32, 40, 48, 56, 57, 49, 41, 33, 26, 18, 3, 11, 4, 12, 19, 27, 34, 42, 50, 58, 35,
    43, 51, 59, 20, 28, 5, 13, 6, 14, 21, 29, 36, 44, 52, 60, 37, 45, 53, 61, 22, 30, 7, 15, 23, 31, 38, 46, 54, 62, 39,
    47, 55, 63,
];

const NONLINEAR_Q: [i32; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 36, 40, 44, 48, 52, 56, 64, 72, 80, 88, 96, 104,
    112,
];

/// B.1, increments 1..=33.
const MBAI: [(u16, u8); 33] = [
    (0x1, 1), (0x3, 3), (0x2, 3), (0x3, 4), (0x2, 4), (0x3, 5), (0x2, 5), (0x7, 7), (0x6, 7), (0xb, 8), (0xa, 8),
    (0x9, 8), (0x8, 8), (0x7, 8), (0x6, 8), (0x17, 10), (0x16, 10), (0x15, 10), (0x14, 10), (0x13, 10), (0x12, 10),
    (0x23, 11), (0x22, 11), (0x21, 11), (0x20, 11), (0x1f, 11), (0x1e, 11), (0x1d, 11), (0x1c, 11), (0x1b, 11),
    (0x1a, 11), (0x19, 11), (0x18, 11),
];

/// B.9, by coded_block_pattern 0..=63.
const CBP: [(u16, u8); 64] = [
    (0x1, 9), (0xb, 5), (0x9, 5), (0xd, 6), (0xd, 4), (0x17, 7), (0x13, 7), (0x1f, 8), (0xc, 4), (0x16, 7), (0x12, 7),
    (0x1e, 8), (0x13, 5), (0x1b, 8), (0x17, 8), (0x13, 8), (0xb, 4), (0x15, 7), (0x11, 7), (0x1d, 8), (0x11, 5),
    (0x19, 8), (0x15, 8), (0x11, 8), (0xf, 6), (0xf, 8), (0xd, 8), (0x3, 9), (0xf, 5), (0xb, 8), (0x7, 8), (0x7, 9),
    (0xa, 4), (0x14, 7), (0x10, 7), (0x1c, 8), (0xe, 6), (0xe, 8), (0xc, 8), (0x2, 9), (0x10, 5), (0x18, 8), (0x14, 8),
    (0x10, 8), (0xe, 5), (0xa, 8), (0x6, 8), (0x6, 9), (0x12, 5), (0x1a, 8), (0x16, 8), (0x12, 8), (0xd, 5), (0x9, 8),
    (0x5, 8), (0x5, 9), (0xc, 5), (0x8, 8), (0x4, 8), (0x4, 9), (0x7, 3), (0xa, 5), (0x8, 5), (0xc, 6),
];

/// B.10, |motion_code| 0..=16 (the sign bit follows a non-zero code).
const MV: [(u16, u8); 17] = [
    (0x1, 1), (0x1, 2), (0x1, 3), (0x1, 4), (0x3, 6), (0x5, 7), (0x4, 7), (0x3, 7), (0xb, 9), (0xa, 9), (0x9, 9),
    (0x11, 10), (0x10, 10), (0xf, 10), (0xe, 10), (0xd, 10), (0xc, 10),
];

/// B.12 and B.13, by dct_dc_size 0..=11.
const DC_LUM: [(u16, u8); 12] = [
    (0x4, 3), (0x0, 2), (0x1, 2), (0x5, 3), (0x6, 3), (0xe, 4), (0x1e, 5), (0x3e, 6), (0x7e, 7), (0xfe, 8), (0x1fe, 9),
    (0x1ff, 9),
];
const DC_CHROMA: [(u16, u8); 12] = [
    (0x0, 2), (0x1, 2), (0x2, 2), (0x6, 3), (0xe, 4), (0x1e, 5), (0x3e, 6), (0x7e, 7), (0xfe, 8), (0x1fe, 9),
    (0x3fe, 10), (0x3ff, 10),
];

/// B.14 (`next` codes) and B.15, entries 0..111 (RUN/LEVEL), then escape
/// and end of block; the sign bit follows each.
const B14: [(u16, u8); 113] = [
    (0x3, 2), (0x4, 4), (0x5, 5), (0x6, 7), (0x26, 8), (0x21, 8), (0xa, 10), (0x1d, 12), (0x18, 12), (0x13, 12),
    (0x10, 12), (0x1a, 13), (0x19, 13), (0x18, 13), (0x17, 13), (0x1f, 14), (0x1e, 14), (0x1d, 14), (0x1c, 14),
    (0x1b, 14), (0x1a, 14), (0x19, 14), (0x18, 14), (0x17, 14), (0x16, 14), (0x15, 14), (0x14, 14), (0x13, 14),
    (0x12, 14), (0x11, 14), (0x10, 14), (0x18, 15), (0x17, 15), (0x16, 15), (0x15, 15), (0x14, 15), (0x13, 15),
    (0x12, 15), (0x11, 15), (0x10, 15), (0x3, 3), (0x6, 6), (0x25, 8), (0xc, 10), (0x1b, 12), (0x16, 13), (0x15, 13),
    (0x1f, 15), (0x1e, 15), (0x1d, 15), (0x1c, 15), (0x1b, 15), (0x1a, 15), (0x19, 15), (0x13, 16), (0x12, 16),
    (0x11, 16), (0x10, 16), (0x5, 4), (0x4, 7), (0xb, 10), (0x14, 12), (0x14, 13), (0x7, 5), (0x24, 8), (0x1c, 12),
    (0x13, 13), (0x6, 5), (0xf, 10), (0x12, 12), (0x7, 6), (0x9, 10), (0x12, 13), (0x5, 6), (0x1e, 12), (0x14, 16),
    (0x4, 6), (0x15, 12), (0x7, 7), (0x11, 12), (0x5, 7), (0x11, 13), (0x27, 8), (0x10, 13), (0x23, 8), (0x1a, 16),
    (0x22, 8), (0x19, 16), (0x20, 8), (0x18, 16), (0xe, 10), (0x17, 16), (0xd, 10), (0x16, 16), (0x8, 10), (0x15, 16),
    (0x1f, 12), (0x1a, 12), (0x19, 12), (0x17, 12), (0x16, 12), (0x1f, 13), (0x1e, 13), (0x1d, 13), (0x1c, 13),
    (0x1b, 13), (0x1f, 16), (0x1e, 16), (0x1d, 16), (0x1c, 16), (0x1b, 16), (0x1, 6), (0x2, 2),
];
const B15: [(u16, u8); 113] = [
    (0x02, 2), (0x06, 3), (0x07, 4), (0x1c, 5), (0x1d, 5), (0x05, 6), (0x04, 6), (0x7b, 7), (0x7c, 7), (0x23, 8),
    (0x22, 8), (0xfa, 8), (0xfb, 8), (0xfe, 8), (0xff, 8), (0x1f, 14), (0x1e, 14), (0x1d, 14), (0x1c, 14), (0x1b, 14),
    (0x1a, 14), (0x19, 14), (0x18, 14), (0x17, 14), (0x16, 14), (0x15, 14), (0x14, 14), (0x13, 14), (0x12, 14),
    (0x11, 14), (0x10, 14), (0x18, 15), (0x17, 15), (0x16, 15), (0x15, 15), (0x14, 15), (0x13, 15), (0x12, 15),
    (0x11, 15), (0x10, 15), (0x02, 3), (0x06, 5), (0x79, 7), (0x27, 8), (0x20, 8), (0x16, 13), (0x15, 13), (0x1f, 15),
    (0x1e, 15), (0x1d, 15), (0x1c, 15), (0x1b, 15), (0x1a, 15), (0x19, 15), (0x13, 16), (0x12, 16), (0x11, 16),
    (0x10, 16), (0x05, 5), (0x07, 7), (0xfc, 8), (0x0c, 10), (0x14, 13), (0x07, 5), (0x26, 8), (0x1c, 12), (0x13, 13),
    (0x06, 6), (0xfd, 8), (0x12, 12), (0x07, 6), (0x04, 9), (0x12, 13), (0x06, 7), (0x1e, 12), (0x14, 16), (0x04, 7),
    (0x15, 12), (0x05, 7), (0x11, 12), (0x78, 7), (0x11, 13), (0x7a, 7), (0x10, 13), (0x21, 8), (0x1a, 16), (0x25, 8),
    (0x19, 16), (0x24, 8), (0x18, 16), (0x05, 9), (0x17, 16), (0x07, 9), (0x16, 16), (0x0d, 10), (0x15, 16), (0x1f, 12),
    (0x1a, 12), (0x19, 12), (0x17, 12), (0x16, 12), (0x1f, 13), (0x1e, 13), (0x1d, 13), (0x1c, 13), (0x1b, 13),
    (0x1f, 16), (0x1e, 16), (0x1d, 16), (0x1c, 16), (0x1b, 16), (0x01, 6), (0x06, 4),
];

const LEVEL: [i8; 111] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 35, 36, 37, 38, 39, 40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 1, 2, 3, 4, 5, 1,
    2, 3, 4, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1,
];
const RUN: [i8; 111] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7,
    8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15, 16, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_prefix_free() {
        let t = tables();
        for v in [&t.mbai, &t.cbp, &t.mv, &t.dc_lum, &t.dc_chroma, &t.b14, &t.b15] {
            assert!(v.table.iter().any(|e| e.0 > 0));
        }
        // Each code fills exactly its share: no overlaps.
        let check = |bits: u32, codes: &[(u16, u8)]| {
            let mut seen = vec![false; 1 << bits];
            for &(c, l) in codes {
                let shift = bits - u32::from(l);
                for e in &mut seen[(usize::from(c) << shift)..((usize::from(c) + 1) << shift)] {
                    assert!(!*e, "overlap {c:#x}/{l}");
                    *e = true;
                }
            }
        };
        check(16, &B14);
        check(16, &B15);
        check(9, &CBP);
        check(10, &MV);
        check(9, &DC_LUM);
        check(10, &DC_CHROMA);
        check(11, &MBAI);
    }

    #[test]
    fn delta_codes() {
        // 0x81: +1; 0x7f: -1 (one byte, 7-bit signed); 0x01 0x00: +256.
        assert_eq!(delta(&[0x81, 0xff, 0x01, 0x00]), vec![1, 0, 256]);
    }

    #[test]
    fn idct_dc() {
        let mut b = [0i32; 64];
        b[0] = 1024; // DC 128 at 8-bit precision, << 3
        let out = idct(&b);
        assert!(out.iter().all(|&v| v == 128), "{out:?}");
    }
}
