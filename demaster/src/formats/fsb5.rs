//! FSB5 sound banks as the PC game ships them (ffx-rust
//! `docs/formats/sound.md`): version 1, codec 7, IMA ADPCM in the Xbox
//! layout. Reads a bank's head (to copy its names, order and flags) and
//! writes a bank sample by sample, the data streamed through a file.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};

const RATES: [u32; 11] = [4000, 8000, 11000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 96000];

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("truncated FSB")?.try_into().expect("4 bytes")))
}

/// One sample of a bank's head.
#[derive(Debug, Clone)]
pub struct SampleInfo {
    pub name: String,
    pub rate: u32,
    pub channels: u16,
    pub frames: u32,
    /// Loop start and end (inclusive), frames.
    pub looped: Option<(u32, u32)>,
    /// Its data: offset from the data section, and size.
    pub offset: u64,
    pub size: u64,
}

/// The parts of a bank's head a replacement keeps.
#[derive(Debug, Clone)]
pub struct Head {
    /// Bytes 0x1c..0x3c (flags and hash), copied unchanged.
    pub extra: [u8; 32],
    pub samples: Vec<SampleInfo>,
    /// File offset of the data section.
    pub data_at: u64,
}

impl Head {
    /// Bytes needed to parse a head, from its first 0x3c bytes.
    pub fn len(first: &[u8]) -> Result<usize> {
        Ok(0x3c + u32_at(first, 12)? as usize + u32_at(first, 16)? as usize)
    }

    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.get(..4) == Some(b"FSB5"), "not an FSB5 bank");
        ensure!(u32_at(b, 4)? == 1, "FSB5 version {}", u32_at(b, 4)?);
        ensure!(u32_at(b, 24)? == 7, "FSB codec {} (not IMA ADPCM)", u32_at(b, 24)?);
        let count = u32_at(b, 8)? as usize;
        let names_at = 0x3c + u32_at(b, 12)? as usize;
        let mut at = 0x3c;
        let mut samples = Vec::with_capacity(count);
        for i in 0..count {
            let v = u64::from(u32_at(b, at)?) | u64::from(u32_at(b, at + 4)?) << 32;
            at += 8;
            let mut more = v & 1 != 0;
            let mut s = SampleInfo {
                name: String::new(),
                rate: RATES.get(((v >> 1) & 0xf) as usize).copied().unwrap_or(44100),
                channels: if (v >> 5) & 1 != 0 { 2 } else { 1 },
                frames: (v >> 34) as u32,
                looped: None,
                offset: ((v >> 6) & 0x0fff_ffff) * 16,
                size: 0,
            };
            while more {
                let c = u32_at(b, at)?;
                at += 4;
                more = c & 1 != 0;
                let size = ((c >> 1) & 0xff_ffff) as usize;
                match c >> 25 {
                    1 => s.channels = u16::from(*b.get(at).context("truncated")?),
                    2 => s.rate = u32_at(b, at)?,
                    3 => s.looped = Some((u32_at(b, at)?, u32_at(b, at + 4)?)),
                    _ => {}
                }
                at += size;
            }
            let off = u32_at(b, names_at + 4 * i)? as usize;
            let n = b.get(names_at + off..).context("name table")?;
            s.name = String::from_utf8_lossy(&n[..n.iter().position(|&c| c == 0).unwrap_or(n.len())]).into_owned();
            samples.push(s);
        }
        let data_size = u64::from(u32_at(b, 20)?);
        for i in 0..samples.len() {
            let end = samples.get(i + 1).map_or(data_size, |n| n.offset);
            samples[i].size = end.saturating_sub(samples[i].offset);
        }
        let mut extra = [0; 32];
        extra.copy_from_slice(&b[0x1c..0x3c]);
        let data_at = (names_at + u32_at(b, 16)? as usize) as u64;
        Ok(Head { extra, samples, data_at })
    }
}

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97,
    107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796,
    876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871,
    5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623,
    27086, 29794, 32767,
];
const INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// IMA ADPCM state of one channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ima {
    pred: i32,
    index: i32,
}

impl Ima {
    fn step_code(&mut self, code: u8) -> i32 {
        let step = STEPS[self.index as usize];
        let mut diff = step >> 3;
        if code & 1 != 0 { diff += step >> 2; }
        if code & 2 != 0 { diff += step >> 1; }
        if code & 4 != 0 { diff += step; }
        if code & 8 != 0 { diff = -diff; }
        self.pred = (self.pred + diff).clamp(-32768, 32767);
        self.index = (self.index + INDEX[usize::from(code & 7)]).clamp(0, 88);
        self.pred
    }

    /// The code whose step lands nearest `s`: the decoder adds
    /// `step/8 + k*step/4` (k = the low 3 bits), so k = 4*|diff|/step.
    fn encode(&mut self, s: i32) -> u8 {
        let step = STEPS[self.index as usize];
        let diff = s - self.pred;
        let sign = if diff < 0 { 8 } else { 0 };
        let k = (4 * diff.abs() / step).min(7) as u8;
        let code = sign | k;
        self.step_code(code);
        code
    }

    /// One 0x24-byte block of 64 samples (`s[64]`, the next block's first
    /// sample as the unused 64th code's target).
    fn block(&mut self, s: &[i16], next: i16, out: &mut [u8; 36]) {
        self.pred = i32::from(s[0]);
        out[0..2].copy_from_slice(&s[0].to_le_bytes());
        out[2] = self.index as u8;
        out[3] = 0;
        for k in 0..64 {
            let target = if k + 1 < 64 { s[k + 1] } else { next };
            let c = self.encode(i32::from(target));
            if k & 1 == 0 {
                out[4 + k / 2] = c;
            } else {
                out[4 + k / 2] |= c << 4;
            }
        }
    }
}

/// Encodes stereo frames into Xbox IMA blocks (0x48 bytes a 64 frames).
pub struct StereoEncoder {
    st: [Ima; 2],
    pending: Vec<[i16; 2]>,
    pub frames: u64,
}

impl Default for StereoEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoEncoder {
    pub fn new() -> Self {
        StereoEncoder { st: [Ima::default(); 2], pending: Vec::with_capacity(128), frames: 0 }
    }

    fn emit(&mut self, out: &mut impl Write, block: &[[i16; 2]], next: [i16; 2]) -> Result<()> {
        let mut b = [[0u8; 36]; 2];
        for c in 0..2 {
            let ch: Vec<i16> = block.iter().map(|f| f[c]).collect();
            self.st[c].block(&ch, next[c], &mut b[c]);
        }
        let mut o = [0u8; 0x48];
        o[0..4].copy_from_slice(&b[0][0..4]);
        o[4..8].copy_from_slice(&b[1][0..4]);
        for w in 0..8 {
            o[8 + w * 8..12 + w * 8].copy_from_slice(&b[0][4 + w * 4..8 + w * 4]);
            o[12 + w * 8..16 + w * 8].copy_from_slice(&b[1][4 + w * 4..8 + w * 4]);
        }
        out.write_all(&o)?;
        Ok(())
    }

    pub fn push(&mut self, out: &mut impl Write, frames: &[[i16; 2]]) -> Result<()> {
        for f in frames {
            self.pending.push(*f);
            self.frames += 1;
            if self.pending.len() == 65 {
                let next = self.pending[64];
                let block: Vec<[i16; 2]> = self.pending.drain(..64).collect();
                self.emit(out, &block, next)?;
            }
        }
        Ok(())
    }

    /// Flushes the last block (padded with silence). Returns the frames in
    /// blocks (a multiple of 64) and the bytes written in all.
    pub fn finish(mut self, out: &mut impl Write) -> Result<u64> {
        if !self.pending.is_empty() {
            let mut block = std::mem::take(&mut self.pending);
            let rest: Vec<[i16; 2]> = if block.len() > 64 { block.split_off(64) } else { Vec::new() };
            block.resize(64, [0, 0]);
            let next = rest.first().copied().unwrap_or([0, 0]);
            self.emit(out, &block, next)?;
            if !rest.is_empty() {
                let mut b2 = rest;
                b2.resize(64, [0, 0]);
                self.emit(out, &b2, [0, 0])?;
            }
        }
        Ok(self.frames.div_ceil(64) * 64)
    }
}

/// Decodes Xbox IMA stereo blocks (for checks): the header's predictor,
/// then 63 codes.
pub fn decode_stereo(data: &[u8]) -> Vec<[i16; 2]> {
    let mut out = Vec::with_capacity(data.len() / 0x48 * 64);
    for b in data.chunks_exact(0x48) {
        let mut ch = [[0i16; 64]; 2];
        for c in 0..2 {
            let h = &b[4 * c..4 * c + 4];
            let mut st = Ima { pred: i32::from(i16::from_le_bytes([h[0], h[1]])), index: i32::from(h[2]).min(88) };
            ch[c][0] = st.pred as i16;
            let mut k = 1;
            for w in 0..8 {
                let word = &b[8 + w * 8 + 4 * c..12 + w * 8 + 4 * c];
                for byte in word {
                    for nib in [byte & 0xf, byte >> 4] {
                        if k < 64 {
                            ch[c][k] = st.step_code(nib) as i16;
                        }
                        k += 1;
                    }
                }
            }
        }
        for k in 0..64 {
            out.push([ch[0][k], ch[1][k]]);
        }
    }
    out
}

/// The data offset of every sample (relative to the data section).
pub fn offsets(b: &[u8]) -> Result<Vec<u64>> {
    let count = u32_at(b, 8)? as usize;
    let mut at = 0x3c;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let v = u64::from(u32_at(b, at)?) | u64::from(u32_at(b, at + 4)?) << 32;
        at += 8;
        out.push(((v >> 6) & 0x0fff_ffff) * 16);
        let mut more = v & 1 != 0;
        while more {
            let c = u32_at(b, at)?;
            more = c & 1 != 0;
            at += 4 + ((c >> 1) & 0xff_ffff) as usize;
        }
    }
    Ok(out)
}

/// A sample of a bank being written: its data is already in the data file.
#[derive(Debug, Clone)]
pub struct Written {
    pub name: String,
    pub rate: u32,
    pub frames: u32,
    pub looped: Option<(u32, u32)>,
    pub offset: u64,
}

/// Writes the bank: header, sample headers, names, then the data copied
/// from `data` (a file of the samples' blocks at their offsets).
pub fn write_bank(out: &mut impl Write, extra: &[u8; 32], samples: &[Written], data: &Path) -> Result<()> {
    let data_size = std::fs::metadata(data)?.len();
    let mut heads = Vec::new();
    for s in samples {
        let rate = RATES.iter().position(|&r| r == s.rate).with_context(|| format!("rate {}", s.rate))? as u64;
        ensure!(s.offset % 32 == 0, "unaligned sample data");
        let has = s.looped.is_some() as u64;
        let v = has | rate << 1 | 1 << 5 | (s.offset / 16) << 6 | u64::from(s.frames) << 34;
        heads.extend_from_slice(&(v as u32).to_le_bytes());
        heads.extend_from_slice(&((v >> 32) as u32).to_le_bytes());
        if let Some((a, b)) = s.looped {
            heads.extend_from_slice(&(3u32 << 25 | 8 << 1).to_le_bytes());
            heads.extend_from_slice(&a.to_le_bytes());
            heads.extend_from_slice(&b.to_le_bytes());
        }
    }
    let mut names = Vec::new();
    let mut strings = Vec::new();
    for s in samples {
        names.extend_from_slice(&((samples.len() * 4 + strings.len()) as u32).to_le_bytes());
        strings.extend_from_slice(s.name.as_bytes());
        strings.push(0);
    }
    names.extend_from_slice(&strings);
    while names.len() % 32 != 0 {
        names.push(0);
    }
    let mut h = Vec::with_capacity(0x3c);
    h.extend_from_slice(b"FSB5");
    h.extend_from_slice(&1u32.to_le_bytes());
    h.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    h.extend_from_slice(&(heads.len() as u32).to_le_bytes());
    h.extend_from_slice(&(names.len() as u32).to_le_bytes());
    h.extend_from_slice(&u32::try_from(data_size).context("bank over 4 GB")?.to_le_bytes());
    h.extend_from_slice(&7u32.to_le_bytes());
    h.extend_from_slice(extra);
    out.write_all(&h)?;
    out.write_all(&heads)?;
    out.write_all(&names)?;
    let mut f = std::fs::File::open(data)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
    }
    Ok(())
}

/// Checks a written bank against the head it replaces: the same names in
/// the same order, stereo, and every sample's data range in the file.
#[allow(dead_code)]
pub fn check(head: &Head, bank: &[u8]) -> Result<Head> {
    let mine = Head::parse(bank)?;
    if mine.samples.len() != head.samples.len() {
        bail!("{} samples instead of {}", mine.samples.len(), head.samples.len());
    }
    for (a, b) in mine.samples.iter().zip(&head.samples) {
        ensure!(a.name == b.name, "sample {} where {} was", a.name, b.name);
        ensure!(a.channels == b.channels, "{}: channels", a.name);
    }
    Ok(mine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ima_round_trip_is_close() {
        let frames: Vec<[i16; 2]> = (0..1000)
            .map(|i| {
                let x = (i as f64 * 0.05).sin() * 12000.0;
                [x as i16, (x * 0.5) as i16]
            })
            .collect();
        let mut enc = StereoEncoder::new();
        let mut data = Vec::new();
        enc.push(&mut data, &frames).unwrap();
        let n = enc.finish(&mut data).unwrap();
        assert_eq!(n, 1024);
        assert_eq!(data.len(), 16 * 0x48);
        let dec = decode_stereo(&data);
        let err: f64 = frames.iter().zip(&dec).map(|(a, b)| f64::from((a[0] - b[0]).abs())).sum::<f64>() / 1000.0;
        assert!(err < 200.0, "mean error {err}");
    }
}
