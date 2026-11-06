//! FFX's PS2 music driver: the Square sequencer of `iopsound.irx` (the IOP
//! sound module) playing a `musicNNN.bgm` with the instruments of a
//! `wave_wd.dat` bank on the SPU2 (`spu2.rs`). The commands, the 240 Hz
//! timer, the voice allocation, the volume, pan and pitch formulas follow
//! the module's code (`docs/music.md`, "The driver"); the tables it needs
//! (pan law, the pitch steps' random table, the reverb presets of
//! `libsd.irx`) are read from the player's disc.

use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};

use crate::spu2::{self, ReverbPreset, Spu2};

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

/// Tables of the sound modules on the disc.
#[derive(Clone)]
pub struct Tables {
    /// Pan law: gain x256 by pan position 0..255 (0x40 left, 0x80 centre, 0xc0 right).
    pan: Vec<i32>,
    /// The module's 256 random bytes (vibrato/volume randomisation).
    rnd: Vec<u8>,
    semi: [u32; 12],
    fine: Vec<u32>,
    sine: Vec<i32>,
    /// `libsd`'s reverb presets by mode (0 off .. 9).
    pub reverb: Vec<ReverbPreset>,
}

/// The `.data` section of an IOP module (IRX: ELF, relocatable at 0).
fn irx_data(irx: &[u8]) -> Result<&[u8]> {
    ensure!(irx.get(..4) == Some(b"\x7fELF"), "not an IRX");
    let shoff = u32_at(irx, 0x20) as usize;
    let shnum = usize::from(u16_at(irx, 0x30));
    let shstr = usize::from(u16_at(irx, 0x32));
    let sec = |i: usize| -> (u32, u32, usize, usize) {
        let o = shoff + 40 * i;
        (u32_at(irx, o), u32_at(irx, o + 4), u32_at(irx, o + 16) as usize, u32_at(irx, o + 20) as usize)
    };
    let (_, _, stroff, _) = sec(shstr);
    for i in 0..shnum {
        let (name, kind, off, size) = sec(i);
        let n = &irx[stroff + name as usize..];
        if kind == 1 && n.starts_with(b".data\0") {
            return irx.get(off..off + size).context("IRX .data out of range");
        }
    }
    bail!("IRX without .data")
}

impl Tables {
    /// Finds the tables in `iopsound.irx` and `libsd.irx`.
    pub fn from_irx(iopsound: &[u8], libsd: &[u8]) -> Result<Self> {
        let semi: [u32; 12] = std::array::from_fn(|k| (131072.0 * 2f64.powf(k as f64 / 12.0)) as u32);
        let fine: Vec<u32> = (0..256).map(|i| (65536.0 * 2f64.powf(i as f64 / 3072.0)) as u32).collect();
        let sine: Vec<i32> = (0..512)
            .map(|i| (65536.0 * (std::f64::consts::PI * i as f64 / 512.0).sin()).round() as i32)
            .collect();
        // The pitch table is computable; the pan law and the random bytes
        // sit at fixed distances after it in the module's data.
        let data = irx_data(iopsound)?;
        let key: Vec<u8> = semi.iter().flat_map(|v| v.to_le_bytes()).collect();
        let at = data
            .windows(key.len())
            .position(|w| w == key.as_slice())
            .context("iopsound.irx: pitch table not found (unknown driver version)")?;
        let rnd = data.get(at + 0x4e0..at + 0x5e0).context("iopsound.irx: random table")?.to_vec();
        let pan: Vec<i32> = data
            .get(at + 0x5e0..at + 0x7e0)
            .context("iopsound.irx: pan table")?
            .chunks(2)
            .map(|c| i32::from(i16::from_le_bytes([c[0], c[1]])))
            .collect();
        ensure!(pan[0x40] == 256 && pan[0x80] == pan[0x7f] && pan[0xff] == 0, "iopsound.irx: pan table looks wrong");

        // libsd: ten presets of a mask word and 32 registers, the work area
        // sizes (10 words) right before them. Mode 0 is off; modes 1-9 end
        // in the input volumes 0x8000, 0x8000.
        let ld = irx_data(libsd)?;
        let mut found = None;
        for p in (0x28..ld.len().saturating_sub(10 * 0x44)).step_by(4) {
            let ok = (1..10).all(|m| {
                let o = p + m * 0x44;
                u32_at(ld, o) == 0 && u16_at(ld, o + 0x40) == 0x8000 && u16_at(ld, o + 0x42) == 0x8000
            });
            if ok {
                found = Some(p);
                break;
            }
        }
        let p = found.context("libsd.irx: reverb presets not found")?;
        let reverb = (0..10)
            .map(|m| ReverbPreset {
                regs: std::array::from_fn(|r| u16_at(ld, p + m * 0x44 + 4 + 2 * r)),
                size: u32_at(ld, p - 0x28 + 4 * m),
            })
            .collect();
        Ok(Tables { pan, rnd, semi, fine, sine, reverb })
    }

    /// Tables without the disc, for tests: a constant-power pan law with
    /// the module's anchor points, zero random bytes, no reverb presets.
    pub fn generic() -> Self {
        let semi: [u32; 12] = std::array::from_fn(|k| (131072.0 * 2f64.powf(k as f64 / 12.0)) as u32);
        let fine: Vec<u32> = (0..256).map(|i| (65536.0 * 2f64.powf(i as f64 / 3072.0)) as u32).collect();
        let sine: Vec<i32> = (0..512)
            .map(|i| (65536.0 * (std::f64::consts::PI * i as f64 / 512.0).sin()).round() as i32)
            .collect();
        let pan = (0..256)
            .map(|p: i32| {
                let x = ((p - 0x40) as f64 / 128.0).clamp(0.0, 1.0);
                (256.0 * (x * std::f64::consts::FRAC_PI_2).cos()).round() as i32
            })
            .collect();
        Tables { pan, rnd: vec![0; 256], semi, fine, sine, reverb: vec![ReverbPreset::default(); 10] }
    }

    /// The LFO shapes by number (phase: 10 bits a period).
    fn wave(&self, w: u8, p: u32) -> i32 {
        let sine = |p: u32| {
            let v = self.sine[(p & 0x1ff) as usize];
            if p & 0x200 != 0 { -v } else { v }
        };
        let tri = |p: u32| {
            let f = (p & 0xff) as i32;
            match (p >> 8) & 3 {
                0 => f * 0x100,
                1 => 0x10000 - f * 0x100,
                2 => -f * 0x100,
                _ => f * 0x100 - 0x10000,
            }
        };
        let saw = |p: u32| (((p << 22) as i32) >> 24) << 8;
        let square = |p: u32| if p & 0x200 != 0 { -0x10000 } else { 0x10000 };
        let random = |p: u32| i32::from(self.rnd[((p >> 8) & 0xff) as usize] as i8) << 8;
        match w & 0xf {
            0 => sine(p),
            1 => tri(p),
            2 => saw(p),
            4 => random(p),
            8 => sine(p ^ 0x200),
            9 => tri(p ^ 0x200),
            10 => ((((!p) << 22) as i32) >> 24) << 8,
            12 => i32::from(self.rnd[(((p >> 8) & 0xff) ^ 0x40) as usize] as i8) << 8,
            11 | 13 | 14 | 15 => square(p ^ 0x200),
            _ => square(p),
        }
    }

    /// The SPU pitch register (0x1000: 48 kHz) a key plays a region at,
    /// without bend, detune or scale (FFX Music Studio's sample survey).
    pub fn key_pitch(&self, key: u8, tune: u32) -> u32 {
        self.pitch(i32::from(key as i8) << 20, 0, tune, 0)
    }

    /// The SPU pitch for a note (`key << 20` units), a fine offset (1/256
    /// semitone), the region's tuning word and the track's pitch scale.
    fn pitch(&self, note: i32, fine: i32, tune: u32, scale: i8) -> u32 {
        let mut u = (note >> 12) + fine + ((tune as i32) >> 16);
        let mut oct = 0i32;
        while u < 0 {
            u += 0xc00;
            oct -= 1;
        }
        let k = ((u >> 8) & 0x7f) as u32;
        let shift = (10 - (oct + (k / 12) as i32)) as u32 & 0x1f;
        let mut p = (self.semi[(k % 12) as usize] >> shift).wrapping_mul(self.fine[(u & 0xff) as usize]) >> 16;
        let s = i32::from(scale);
        if s != 0 {
            if s < 1 {
                p = ((p as i32 * (s & 0xff)) >> 8) as u32;
            } else {
                p = p.wrapping_add(((p as i32 * (s + 1)) >> 7) as u32);
            }
        }
        p
    }
}

/// One key/velocity region of an instrument (0x20 bytes in the WD).
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub flags: u32,
    /// Sample start, bytes from the bank's sample data.
    pub sample: u32,
    pub loop_off: u32,
    pub adsr: u32,
    pub tune: u32,
    pub key_hi: i8,
    pub vel_hi: u8,
    pub vol: u8,
    pub pan: u8,
    pub rev: u8,
}

/// A `wave_wd.dat` instrument bank ("WD").
#[derive(Clone)]
pub struct Wd {
    pub id: u16,
    /// First region of each instrument.
    pub instruments: Vec<usize>,
    pub regions: Vec<Region>,
    /// The ADPCM sample data, loaded at SPU address 0.
    pub samples: Vec<u8>,
}

impl Wd {
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.get(..2) == Some(b"WD"), "not a WD bank");
        let id = u16_at(b, 2);
        let size = u32_at(b, 4) as usize;
        let ninst = u32_at(b, 8) as usize;
        let nreg = u32_at(b, 12) as usize;
        let ptrs: Vec<usize> = (0..ninst).map(|i| u32_at(b, 0x20 + 4 * i) as usize).collect();
        let first = *ptrs.first().context("WD without instruments")?;
        let regions = (0..nreg)
            .map(|k| {
                let o = first + 0x20 * k;
                Region {
                    flags: u32_at(b, o),
                    sample: u32_at(b, o + 4),
                    loop_off: u32_at(b, o + 8),
                    adsr: u32_at(b, o + 12),
                    tune: u32_at(b, o + 16),
                    key_hi: b[o + 0x14] as i8,
                    vel_hi: b[o + 0x15],
                    vol: b[o + 0x16],
                    pan: b[o + 0x17],
                    rev: b[o + 0x18],
                }
            })
            .collect();
        let instruments = ptrs.iter().map(|&p| (p - first) / 0x20).collect();
        let sdata = first + nreg * 0x20;
        let samples = b.get(sdata..sdata + size).context("WD sample data truncated")?.to_vec();
        Ok(Wd { id, instruments, regions, samples })
    }

    /// The bank's bytes: `wave_wd.sec` gives each bank's sector in `wave_wd.dat`.
    pub fn locate(sec: &[u8], id: u16) -> Option<u64> {
        let o = usize::from(id) * 4;
        let s = i32::from_le_bytes(sec.get(o..o + 4)?.try_into().ok()?);
        (s >= 0).then_some(s as u64 * 0x800)
    }

    /// Total bytes of a bank starting at `b` (header, regions, samples).
    pub fn size(b: &[u8]) -> Option<usize> {
        let size = u32_at(b, 4) as usize;
        let nreg = u32_at(b, 12) as usize;
        let first = u32_at(b, 0x20) as usize;
        Some(first + nreg * 0x20 + size)
    }

    /// `FUN_00101c70`: the region instrument `inst` plays for a key and
    /// velocity (the first of a stereo pair, flag 1).
    pub fn region_for(&self, inst: usize, key: u8, vel: u8) -> usize {
        let rs = &self.regions;
        let mut r = self.instruments[inst];
        if rs[r].flags & 0x30000 != 0 {
            while rs[r].flags & 0x200 == 0 && rs[r].key_hi < key as i8 {
                if rs[r].flags & 1 != 0 {
                    r += 1;
                }
                r += 1;
            }
            let k = rs[r].key_hi;
            while rs[r].flags & 0x200 == 0 && rs[r].vel_hi < vel && rs[r].key_hi == k {
                if rs[r].flags & 1 != 0 {
                    r += 1;
                }
                r += 1;
            }
        }
        r.min(rs.len() - 1)
    }

    /// The regions of instrument `i` (up to the next instrument's first).
    pub fn instrument_regions(&self, i: usize) -> &[Region] {
        let Some(&first) = self.instruments.get(i) else { return &[] };
        let end = self.instruments.get(i + 1).copied().unwrap_or(self.regions.len());
        self.regions.get(first..end.max(first)).unwrap_or(&[])
    }

    /// Appends instrument `i` of `other` (with all of `other`'s sample data,
    /// once per bank: `base` is where an earlier call put it) and returns
    /// its number here and the sample base used.
    pub fn import(&mut self, other: &Wd, i: usize, base: Option<u32>) -> (usize, u32) {
        let base = base.unwrap_or_else(|| {
            let at = self.samples.len().next_multiple_of(16);
            self.samples.resize(at, 0);
            self.samples.extend_from_slice(&other.samples);
            at as u32
        });
        let n = self.instruments.len();
        self.instruments.push(self.regions.len());
        let mut rs: Vec<Region> = other.instrument_regions(i).to_vec();
        if rs.is_empty() {
            let first = other.instruments.get(i).copied().unwrap_or(0);
            rs.push(other.regions[first.min(other.regions.len() - 1)]);
        }
        let last = rs.len() - 1;
        rs[last].flags |= 0x200;
        for r in &mut rs {
            r.sample += base;
        }
        self.regions.extend(rs);
        (n, base)
    }
}

/// A `musicNNN.bgm` sequence.
#[derive(Clone)]
pub struct Bgm {
    pub id: u16,
    pub wd: u16,
    pub reverb_mode: u8,
    pub reverb_depth: i16,
    pub volume: u8,
    pub flags: u32,
    /// The whole file.
    pub data: Vec<u8>,
    /// Start offset of each track's events (after its length word).
    pub tracks: Vec<usize>,
}

impl Bgm {
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.get(..4) == Some(b"BGM "), "not a BGM");
        let n = usize::from(b[8]);
        let mut tracks = Vec::with_capacity(n);
        let mut p = 0x20;
        for _ in 0..n {
            let len = u32_at(b, p) as usize;
            tracks.push(p + 4);
            p += 4 + len;
            ensure!(p <= b.len(), "BGM track out of range");
        }
        Ok(Bgm {
            id: u16_at(b, 4),
            wd: u16_at(b, 6),
            reverb_mode: b[9],
            reverb_depth: u16_at(b, 10) as i16,
            volume: b[12],
            flags: u32_at(b, 0x14),
            data: b.to_vec(),
            tracks,
        })
    }

    /// Whether the song loops (a loop point in its second track).
    pub fn loops(&self) -> bool {
        // cheap scan: the loop point is command 0x02 of track 1
        self.tracks.len() > 1 && scan(&self.data, self.tracks[1]).contains(&0x02)
    }
}

/// The commands of one track, in order (for checks).
fn scan(d: &[u8], mut p: usize) -> Vec<u8> {
    let mut ops = Vec::new();
    let varlen = |d: &[u8], p: &mut usize| {
        let mut v = 0u32;
        loop {
            let Some(&b) = d.get(*p) else { return v };
            *p += 1;
            v = v << 7 | u32::from(b & 0x7f);
            if b & 0x80 == 0 {
                return v;
            }
        }
    };
    varlen(d, &mut p);
    while let Some(&op) = d.get(p) {
        p += 1;
        ops.push(op);
        let Some(args) = arg_bytes(op) else { break };
        for a in args {
            if *a == 0 {
                varlen(d, &mut p);
            } else {
                p += 1;
            }
        }
        if op == 0 {
            break;
        }
        varlen(d, &mut p);
    }
    ops
}

/// Argument layout per command: 1 a byte, 0 a variable-length number.
/// `None`: the driver ends the track.
pub fn arg_bytes(op: u8) -> Option<&'static [u8]> {
    const B: &[u8] = &[1];
    const BB: &[u8] = &[1, 1];
    const BBB: &[u8] = &[1, 1, 1];
    const VB: &[u8] = &[0, 1];
    const V: &[u8] = &[0];
    Some(match op {
        0x00..=0x04 | 0x06 | 0x10 | 0x18 | 0x29 | 0x2a | 0x2b | 0x2e | 0x2f | 0x30 | 0x41 | 0x49 | 0x51
        | 0x60 | 0x61 | 0x64 | 0x65 | 0x68 | 0x69 | 0x7f => &[],
        0x05 | 0x08 | 0x0a | 0x0d | 0x12 | 0x13 | 0x1a | 0x1b | 0x20 | 0x22 | 0x24 | 0x26 | 0x31..=0x38
        | 0x3c..=0x3f | 0x42 | 0x44 | 0x46 | 0x4a | 0x4c | 0x4e | 0x52 | 0x54 | 0x56 | 0x58..=0x5b | 0x5d
        | 0x6c | 0x70 | 0x71 | 0x79 => B,
        0x09 | 0x0b | 0x0c | 0x11 | 0x19 | 0x21 | 0x39 | 0x47 | 0x4f | 0x5c | 0x62 | 0x78 => BB,
        0x40 | 0x48 | 0x50 => BBB,
        0x23 | 0x25 | 0x27 | 0x2c | 0x43 | 0x45 | 0x4b | 0x4d | 0x53 | 0x55 => VB,
        0x28 => V,
        _ => return None,
    })
}

/// A value with a linear slide (`x << 12` fixed point).
#[derive(Debug, Clone, Copy, Default)]
struct Slide {
    v: i32,
    step: i32,
    n: i32,
}

impl Slide {
    fn set(&mut self, v: i32) {
        *self = Slide { v, step: 0, n: 0 };
    }
    /// Starts a slide to `target` (unshifted) over `n` ticks (0: 1).
    fn to(&mut self, target: i32, n: u32) {
        let mut n = if n == 0 { 1 } else { n as i32 };
        self.step = slide_step(self.v, target, &mut n);
        self.n = n;
    }
    fn run(&mut self, m: i32) -> bool {
        if self.n == 0 {
            return false;
        }
        let k = m.min(self.n);
        self.n -= k;
        self.v += self.step * k;
        true
    }
}

/// The driver's slide step: `((target - cur) << 12 | 0x800) / n`.
fn slide_step(cur: i32, target: i32, n: &mut i32) -> i32 {
    if target == cur >> 12 {
        *n = 0;
        return 0;
    }
    (((target - (cur >> 12)) * 0x1000) | 0x800) / *n
}

#[derive(Debug, Clone, Copy, Default)]
struct Lfo {
    wave: Option<u8>,
    rate: i32,
    rate_step: i32,
    rate_n: i32,
    depth: i32,
    depth_step: i32,
    depth_n: i32,
    delay: u16,
    fade: u16,
    // the pan LFO keeps its phase and output in the track
    phase: u32,
    out: i32,
}

impl Lfo {
    fn start(&mut self, depth: u8, rate: u8, wave: u8) {
        self.depth = i32::from(depth) << 12;
        self.rate = 0x100000 / if rate == 0 { 256 } else { i32::from(rate) };
        self.wave = Some(wave & 0xf);
        self.rate_n = 0;
        self.depth_n = 0;
    }
    fn slides(&mut self, m: i32) {
        if self.wave.is_none() {
            return;
        }
        if self.rate_n != 0 {
            let k = m.min(self.rate_n);
            self.rate_n -= k;
            self.rate += self.rate_step * k;
        }
        if self.depth_n != 0 {
            let k = m.min(self.depth_n);
            self.depth_n -= k;
            self.depth += self.depth_step * k;
        }
    }
    /// Fade-in length for a voice's LFO.
    fn fade_len(&self) -> i32 {
        let r = self.rate >> 12;
        let per = if r != 0 { 0x100 / r } else { 0x100 };
        if self.fade == 0 { 0 } else { per * i32::from(self.fade) * 4 }
    }
}

// Track flags (`+0xe8`).
const T_NOTE: u32 = 1;
const T_PEDAL: u32 = 4;
const T_MUTE: u32 = 8;
// Track flags 2 (`+0xf0`).
const T2_TIE: u32 = 0x80000;
const T2_KEYED: u32 = 0x100000;
const T2_NO_AUTO_OFF: u32 = 0x200000;

#[derive(Debug, Clone)]
struct Track {
    pos: Option<usize>,
    stack: [(usize, u16); 4],
    sp: usize,
    inst: Option<usize>,
    key: u8,
    vel: u8,
    tflags: u8,
    vol: Slide,
    expr: Slide,
    pan: Slide,
    master: i32,
    plfo: Lfo,
    vlfo: Lfo,
    panlfo: Lfo,
    adsr_ovr: [i8; 8],
    flags: u32,
    flags2: u32,
    wait: i32,
    porta_n: i32,
    porta_step: i32,
    porta_time: i32,
    porta_pitch: i32,
    delta_off: i32,
    bend_raw: i16,
    bend: i16,
    detune: i16,
    scale: i8,
    bend_range: i8,
    /// External input: the track has played `0x00` (no slides, no pan LFO)
    /// until its next command.
    ext_off: bool,
}

impl Track {
    fn new(pos: usize, header_flags: u32) -> Self {
        Track {
            pos: Some(pos),
            stack: [(0, 0); 4],
            sp: 0,
            inst: None,
            key: 0,
            vel: 0,
            tflags: if header_flags & 0x10000 != 0 { 0x10 } else { 0 },
            vol: Slide { v: 0x3fff000, ..Slide::default() },
            expr: Slide { v: 0x3fff000, ..Slide::default() },
            pan: Slide { v: 0x80000, ..Slide::default() },
            master: 0x7f000,
            plfo: Lfo::default(),
            vlfo: Lfo::default(),
            panlfo: Lfo::default(),
            adsr_ovr: [-1; 8],
            flags: 0x10 | 0xc00,
            flags2: if header_flags & 0x10000 == 0 { T2_NO_AUTO_OFF } else { 0 },
            wait: 0,
            porta_n: 0,
            porta_step: 0,
            porta_time: 0,
            porta_pitch: -1,
            delta_off: 0,
            bend_raw: 0,
            bend: 0,
            detune: 0,
            scale: 0,
            bend_range: 2,
            ext_off: false,
        }
    }
}

/// The driver's state of one SPU voice.
#[derive(Debug, Clone, Copy, Default)]
struct DVoice {
    track: Option<usize>,
    region: usize,
    key: u8,
    vel: u8,
    tflags: u8,
    lfo_phase: u32,
    lfo_fade: i32,
    lfo_fade_n: i32,
    lfo_delay: u16,
    vlfo_delay: u16,
    flags: u32,
    pitch: u32,
    note: i32,
    adsr: u32,
    dirty: u32,
    envx: i16,
}

#[derive(Debug, Clone, Copy)]
struct KeyOn {
    track: usize,
    key: u8,
    vel: u8,
    tflags: u8,
}

/// The loop point's snapshot (command 0x02 in track 1).
#[derive(Debug, Clone)]
struct Snapshot {
    tracks: Vec<(Option<usize>, i32, u32, (u8, u8, u8))>,
    active: i32,
    tempo: i32,
    tick: u32,
}

/// The whole sequencer state at the start of a tick (FFX Music Studio's
/// loop range).
#[derive(Debug, Clone)]
struct RangeSnap {
    tracks: Vec<Track>,
    active: i32,
    song_flags: u32,
    tempo: i32,
    snapshot: Option<Snapshot>,
}

/// FFX Music Studio's changes on top of the sequence (none by default: then
/// the driver plays exactly as `iopsound.irx`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mods {
    /// Per track (missing: unchanged).
    pub tracks: Vec<TrackMod>,
    /// Tempo factor (0 or 1: the song's).
    pub tempo: f64,
    /// Loop from tick `a` to tick `b` instead of the song's loop: at the
    /// start of tick `b` (or the song's end) the state from the start of
    /// tick `a` comes back, volumes and programs included.
    pub loop_range: Option<(u32, u32)>,
    /// Stop at the end instead of jumping back to the loop point.
    pub no_loop: bool,
}

/// One track's mixer settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackMod {
    /// Volume factor (0: muted).
    pub gain: f32,
    /// Added to the pan position (0x40 is hard left of centre 0x80).
    pub pan: i32,
    /// Semitones added to every new note.
    pub transpose: i32,
    /// Instrument for new notes instead of the track's program.
    pub program: Option<usize>,
}

impl Default for TrackMod {
    fn default() -> Self {
        TrackMod { gain: 1.0, pan: 0, transpose: 0, program: None }
    }
}

impl Mods {
    fn track(&self, ti: usize) -> TrackMod {
        self.tracks.get(ti).copied().unwrap_or_default()
    }
}

/// What happened during a render (for the FSB's loop points and the checks).
#[derive(Debug, Clone, Default)]
pub struct Marks {
    /// Output frame of the loop point (first 0x02).
    pub loop_point: Option<u64>,
    /// Output frames where the song jumped back to the loop point.
    pub loop_jumps: Vec<u64>,
    /// Output frame where the last track ended (non-looping songs).
    pub end: Option<u64>,
    /// Commands the renderer does not carry out, with counts.
    pub unsupported: std::collections::BTreeMap<u8, u32>,
    /// Key-ons dropped for want of a voice.
    pub dropped: u32,
}

/// What the sequencer's commands did (FFX Music Studio's MIDI export): see
/// [`Song::log`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeqEvent {
    /// A key-on command (`0x10`-`0x13`), queued for the next interrupt.
    NoteOn { key: u8, vel: u8 },
    /// A queued key-on that did not start: no voice was free (the driver
    /// then drops the rest of the interrupt's key-ons too). Logged at the
    /// tick of its `NoteOn`.
    Dropped { key: u8 },
    /// A key-off of the track's key that takes effect (not tied).
    NoteOff { key: u8 },
    /// Tie on (`0x2a`: the next key-ons reuse the voice) or off (`0x2b`).
    Tie(bool),
    /// Program change (`0x20`) to an instrument of the bank.
    Program(usize),
    /// Sustain pedal (`0x3c`).
    Pedal(bool),
    /// Pitch bend `-0x2000..0x1fff` (`0x5c`) and the bend range in
    /// semitones (`0x5d`).
    Bend(i16),
    BendRange(i8),
    /// The track's volume, expression (`0..=0x3fff`, linear) and pan
    /// position (`0x40` left, `0x80` centre, `0xc0` right) after the tick's
    /// commands and slides, when they changed.
    Volume(i32),
    Expression(i32),
    Pan(i32),
    /// The reverb (wet) send of the track's voices on or off (`0x60`-`0x62`).
    Wet(bool),
    /// Tempo in BPM (`0x08`).
    Tempo(u8),
    /// The reverb depth (`0x0a`, `0x0b` target): `x / 128` of full.
    Depth(i8),
    /// The song's loop point (`0x02` in track 1) and the jump back to it.
    LoopPoint,
    LoopJump,
    /// A counted loop: its start (`0x04`) and its end command (`0x05`) with
    /// the pass that just ended and the count.
    RepeatStart,
    RepeatEnd { pass: u16, count: u16 },
    /// The track's end (`0x00`).
    End,
    /// A command the typed events above do not cover, as stored: the
    /// envelope (`0x30`-`0x39`), portamento and pitch slide (`0x28`, `0x29`,
    /// `0x2c`), the voice flags (`0x3d`-`0x3f`), the LFOs (`0x40`-`0x56`),
    /// pitch scale and detune (`0x58`-`0x5b`), the mix switches (`0x60`-
    /// `0x62`, `0x64`-`0x69`) and the depth slide (`0x0b`). `args` holds its
    /// arguments in order (a variable-length one as its value), the rest 0.
    Raw { op: u8, args: [u32; 3] },
}

/// Commands logged as [`SeqEvent::Raw`].
fn raw_logged(op: u8) -> bool {
    matches!(op, 0x0b | 0x28 | 0x29 | 0x2c | 0x30..=0x39 | 0x3d..=0x3f | 0x40..=0x56 | 0x58..=0x5b | 0x60..=0x62 | 0x64..=0x69)
}

/// Pseudo commands of the external input ([`Song::ext_command`]): values the
/// sequence reaches only through slides, set at once. Volume and expression
/// `0..=0x3fff` (two bytes, little endian), the pan position (`0x80` centre;
/// two bytes, signed), the mix switches (one byte: bit 0 dry left, 1 dry
/// right, 2 wet left, 3 wet right, as `0x60`-`0x62` set them).
pub const EXT_VOLUME: u8 = 0x80;
pub const EXT_EXPRESSION: u8 = 0x81;
pub const EXT_PAN: u8 = 0x82;
pub const EXT_MIX: u8 = 0x83;
/// The song's end (what the last `0x00` of a song without loop does: every
/// voice keyed off and let go, the sustain pedal too).
pub const EXT_SONG_END: u8 = 0x84;

/// The header of an external song ([`Song::external`]): what a `.bgm`
/// header gives the driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtHeader {
    /// The song volume (`0..=0x7f`).
    pub volume: u8,
    /// Reverb mode (`libsd`: 0 off, 1-9).
    pub reverb_mode: u8,
    /// Reverb depth (0: full).
    pub reverb_depth: i16,
    /// Header flags (`0x10000`: automatic key-offs, voice flag `0x10`).
    pub flags: u32,
}

impl Default for ExtHeader {
    fn default() -> Self {
        ExtHeader { volume: 0x7f, reverb_mode: 0, reverb_depth: 0, flags: 0 }
    }
}

impl ExtHeader {
    pub fn of(bgm: &Bgm) -> Self {
        ExtHeader { volume: bgm.volume, reverb_mode: bgm.reverb_mode, reverb_depth: bgm.reverb_depth, flags: bgm.flags }
    }
}

/// Commands from outside waiting for their tick.
#[derive(Debug, Clone, Default)]
struct Ext {
    /// By tick: (track, command bytes), in the order given.
    queue: std::collections::BTreeMap<u64, Vec<(usize, Vec<u8>)>>,
    /// The commands of the tick being played.
    now: Vec<(usize, Vec<u8>)>,
}

/// One logged event: the tick (sequence ticks played since the start, not
/// jumping back with the loops: 48 a quarter note at the song's tempo) and
/// the track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogEntry {
    pub tick: u64,
    /// Output frame of the interrupt that played the tick (48 kHz; key-ons
    /// sound from the next interrupt, `FRAMES_PER_IRQ` later).
    pub frame: u64,
    pub track: usize,
    pub event: SeqEvent,
}

#[derive(Clone)]
pub struct Song {
    tab: Arc<Tables>,
    bgm: Arc<Bgm>,
    wd: Arc<Wd>,
    tracks: Vec<Track>,
    voices: [DVoice; spu2::VOICES],
    queue: Vec<KeyOn>,
    active: i32,
    song_flags: u32,
    tempo: i32,
    tempo_counter: i32,
    depth: Slide,
    depth_dirty: bool,
    voice_start: usize,
    snapshot: Option<Snapshot>,
    // per-track scratch of the tick (the driver's globals)
    dirty: u32,
    mix_dirty: bool,
    pub marks: Marks,
    frame: u64,
    voices_in_use: usize,
    /// The tick the next call of `tick` plays (counted from the song's
    /// start in sequence time; jumps back with the loops).
    pub tick: u32,
    mods: Mods,
    tempo_frac: f64,
    range_snap: Option<RangeSnap>,
    /// The driver's song tempo modifier (IRX `0x00222450`, x << 12) and
    /// its slide (step `0x00222454`, steps left `0x00222458`), set by the
    /// game's `MusicTempoDirect` / `MusicTempoTranslate` (live engine).
    tempo_mod: i32,
    tempo_mod_step: i32,
    tempo_mod_n: i32,
    /// Interrupts so far (the driver's 60 Hz work runs every 4th).
    irqs: u32,
    /// When `Some`, every command's effect is appended here (FFX Music
    /// Studio's MIDI export). Off by default; it changes nothing else.
    pub log: Option<Vec<LogEntry>>,
    /// Ticks played since the start (the log's clock).
    ticks_run: u64,
    /// Per track the volume, expression and pan last logged.
    log_mix: Vec<[i32; 3]>,
    /// External input (FFX PS2 Sound): see [`Song::external`].
    ext: Option<Box<Ext>>,
    /// The command being carried out from the external input, and where
    /// its next argument is.
    ext_src: Option<(Vec<u8>, usize)>,
}

/// Output frames per timer interrupt (48 kHz / 240 Hz).
pub const FRAMES_PER_IRQ: usize = 200;

impl Song {
    pub fn new(tab: Arc<Tables>, bgm: Arc<Bgm>, wd: Arc<Wd>, spu: &mut Spu2) -> Self {
        let mut tracks: Vec<Track> = bgm.tracks.iter().map(|&p| Track::new(p, bgm.flags)).collect();
        for t in &mut tracks {
            let mut p = t.pos.expect("start");
            let d = varlen(&bgm.data, &mut p);
            t.pos = Some(p);
            t.wait = d as i32 + 1;
        }
        let mode = usize::from(bgm.reverb_mode);
        if mode < tab.reverb.len() && mode != 0 {
            spu.set_reverb(0, tab.reverb[mode]);
            spu.set_reverb(1, tab.reverb[mode]);
        }
        let depth = if bgm.reverb_depth != 0 { i32::from(bgm.reverb_depth) << 4 } else { 0x7fff << 4 };
        let n = tracks.len() as i32;
        Song {
            tab,
            bgm,
            wd,
            tracks,
            voices: [DVoice::default(); spu2::VOICES],
            queue: Vec::new(),
            active: n,
            song_flags: 0,
            // the song starts at 1 BPM until its first tempo command
            tempo: 0x1000,
            tempo_counter: 1,
            depth: Slide { v: depth, ..Slide::default() },
            depth_dirty: true,
            voice_start: 0,
            snapshot: None,
            dirty: 0,
            mix_dirty: false,
            marks: Marks::default(),
            frame: 0,
            voices_in_use: 0,
            tick: 0,
            mods: Mods::default(),
            tempo_frac: 0.0,
            range_snap: None,
            tempo_mod: 0,
            tempo_mod_step: 0,
            tempo_mod_n: 0,
            irqs: 0,
            log: None,
            ticks_run: 0,
            log_mix: Vec::new(),
            ext: None,
            ext_src: None,
        }
    }

    /// A song played from outside (FFX PS2 Sound, the DAW plugin): `tracks`
    /// tracks without sequence data that never end on their own; their
    /// commands come from [`Song::ext_command`] and are carried out in the
    /// track's turn of their tick (after its slides), exactly as if they
    /// stood in the sequence there. The clock is the driver's own (tempo
    /// command `0x08`, 1 BPM before the first) or given per interrupt
    /// ([`Song::ext_irq`]).
    pub fn external(tab: Arc<Tables>, wd: Arc<Wd>, tracks: usize, header: ExtHeader, spu: &mut Spu2) -> Self {
        let tracks = tracks.clamp(1, 255);
        let mut b = vec![0u8; 0x20];
        b[..4].copy_from_slice(b"BGM ");
        b[6..8].copy_from_slice(&wd.id.to_le_bytes());
        b[8] = tracks as u8;
        b[9] = header.reverb_mode;
        b[10..12].copy_from_slice(&header.reverb_depth.to_le_bytes());
        b[12] = header.volume;
        b[0xe..0x10].copy_from_slice(&48u16.to_le_bytes());
        b[0x14..0x18].copy_from_slice(&header.flags.to_le_bytes());
        for _ in 0..tracks {
            // the longest wait (never reached), then the end
            let t = [0xffu8, 0xff, 0xff, 0x7f, 0x00];
            b.extend_from_slice(&(t.len() as u32).to_le_bytes());
            b.extend_from_slice(&t);
        }
        let n = b.len() as u32;
        b[0x10..0x14].copy_from_slice(&n.to_le_bytes());
        let bgm = Arc::new(Bgm::parse(&b).expect("external song header"));
        let mut s = Song::new(tab, bgm, wd, spu);
        s.ext = Some(Box::default());
        s
    }

    /// External input: `cmd` (a command as stored in a sequence, `op` and
    /// its arguments, or one of the `EXT_*` pseudo commands) for track `ti`
    /// at `tick` ([`Song::ticks_run`] when it plays). Commands of one tick
    /// and track run in the order given; a tick already played: the next.
    /// The loop commands (`0x02`-`0x06`) are ignored; `0x00` ends the track
    /// until its next command. No effect on a song from a sequence.
    pub fn ext_command(&mut self, tick: u64, ti: usize, cmd: &[u8]) {
        let Some(ext) = &mut self.ext else { return };
        if ti >= self.tracks.len() || cmd.is_empty() || (0x02..=0x06).contains(&cmd[0]) {
            return;
        }
        ext.queue.entry(tick).or_default().push((ti, cmd.to_vec()));
    }

    /// External input: a new header (song volume, reverb mode and depth,
    /// flags), as if the song had started with it; a reverb mode that
    /// changes clears the work area.
    pub fn ext_set_header(&mut self, spu: &mut Spu2, h: ExtHeader) {
        if self.ext.is_none() || ExtHeader::of(&self.bgm) == h {
            return;
        }
        let old = ExtHeader::of(&self.bgm);
        let b = Arc::make_mut(&mut self.bgm);
        b.volume = h.volume;
        b.reverb_mode = h.reverb_mode;
        b.reverb_depth = h.reverb_depth;
        b.flags = h.flags;
        if h.reverb_mode != old.reverb_mode {
            let mode = usize::from(h.reverb_mode);
            if mode != 0 && mode < self.tab.reverb.len() {
                spu.set_reverb(0, self.tab.reverb[mode]);
                spu.set_reverb(1, self.tab.reverb[mode]);
            } else {
                spu.reverb_off();
            }
        }
        if h.reverb_depth != old.reverb_depth {
            let d = if h.reverb_depth != 0 { i32::from(h.reverb_depth) << 4 } else { 0x7fff << 4 };
            self.depth = Slide { v: d, ..Slide::default() };
            self.depth_dirty = true;
        }
        if h.flags & 0x10000 != old.flags & 0x10000 {
            for t in &mut self.tracks {
                if h.flags & 0x10000 != 0 {
                    t.tflags |= 0x10;
                    t.flags2 &= !T2_NO_AUTO_OFF;
                } else {
                    t.tflags &= !0x10;
                    t.flags2 |= T2_NO_AUTO_OFF;
                }
            }
        }
    }

    /// External input: commands waiting for their tick.
    pub fn ext_pending(&self) -> usize {
        self.ext.as_ref().map_or(0, |e| e.queue.values().map(Vec::len).sum())
    }

    /// External input: drops the commands waiting.
    pub fn ext_clear(&mut self) {
        if let Some(e) = &mut self.ext {
            e.queue.clear();
        }
    }

    /// One timer interrupt without output (the caller renders the chip,
    /// `FRAMES_PER_IRQ` frames to the next one). `ticks`: `None` runs the
    /// driver's own tempo counter, `Some(n)` plays exactly `n` ticks.
    pub fn ext_irq(&mut self, spu: &mut Spu2, ticks: Option<u32>) {
        match ticks {
            None => self.timer(spu),
            Some(n) => {
                if self.voices_in_use != 0 || !self.queue.is_empty() {
                    self.flush_key_offs(spu);
                    self.update_voices(spu);
                }
                self.queue.clear();
                if self.active != 0 {
                    for _ in 0..n {
                        self.ticks();
                    }
                }
                self.irqs = self.irqs.wrapping_add(1);
            }
        }
        self.frame += FRAMES_PER_IRQ as u64;
    }

    /// The external commands of track `ti` due now.
    fn ext_run(&mut self, ti: usize) {
        let Some(ext) = &mut self.ext else { return };
        if !ext.now.iter().any(|c| c.0 == ti) {
            return;
        }
        let mine: Vec<Vec<u8>> = {
            let (m, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut ext.now).into_iter().partition(|c| c.0 == ti);
            ext.now = rest;
            m.into_iter().map(|c| c.1).collect()
        };
        for cmd in mine {
            let op = cmd[0];
            let arg16 = |i: usize| i32::from(i16::from_le_bytes([cmd.get(i).copied().unwrap_or(0), cmd.get(i + 1).copied().unwrap_or(0)]));
            if op != 0 {
                self.tracks[ti].ext_off = false;
            }
            match op {
                EXT_VOLUME | EXT_EXPRESSION => {
                    let v = arg16(1).clamp(0, 0x3fff) << 12;
                    let t = &mut self.tracks[ti];
                    if op == EXT_VOLUME { t.vol.set(v) } else { t.expr.set(v) }
                    self.dirty |= 2;
                }
                EXT_PAN => {
                    let t = &mut self.tracks[ti];
                    t.pan.set(arg16(1) << 12);
                    if t.panlfo.wave.is_none() {
                        t.panlfo.out = 0;
                    }
                    self.dirty |= 2;
                }
                EXT_MIX => {
                    let b = u32::from(cmd.get(1).copied().unwrap_or(0) & 0xf);
                    let t = &mut self.tracks[ti];
                    t.flags = t.flags & !0x3c00 | b << 10;
                    self.mix_dirty = true;
                }
                EXT_SONG_END => {
                    for v in self.voices.iter_mut().filter(|v| v.track.is_some()) {
                        v.flags = v.flags & !0xd | 2;
                        v.track = None;
                    }
                    self.marks.end = Some(self.frame);
                }
                0x00 => {
                    self.tracks[ti].flags2 = 0;
                    self.key_off(ti);
                    for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
                        v.tflags &= 0xee;
                    }
                    self.tracks[ti].ext_off = true;
                }
                0x01..=0x7f => {
                    self.ext_src = Some((cmd, 1));
                    self.command(ti, op, 1);
                    self.ext_src = None;
                }
                _ => {}
            }
        }
    }

    /// Sequence ticks played since the start (loops included).
    pub fn ticks_run(&self) -> u64 {
        self.ticks_run
    }

    fn note(&mut self, track: usize, event: SeqEvent) {
        if let Some(l) = &mut self.log {
            l.push(LogEntry { tick: self.ticks_run, frame: self.frame, track, event });
        }
    }

    /// Output frames so far.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn mods(&self) -> &Mods {
        &self.mods
    }

    /// Changes the studio settings; sounding voices take the new volume and
    /// pan at the next interrupt (transpose and program: new notes).
    pub fn set_mods(&mut self, mods: Mods) {
        if mods.loop_range != self.mods.loop_range {
            self.range_snap = None;
        }
        self.mods = mods;
        for v in self.voices.iter_mut().filter(|v| v.track.is_some()) {
            v.dirty |= 2;
        }
    }

    /// A bank that holds the current one at the same place, with more
    /// instruments (studio: an instrument from another bank).
    pub fn set_wd(&mut self, wd: Arc<Wd>) {
        self.wd = wd;
    }

    pub fn wd(&self) -> &Arc<Wd> {
        &self.wd
    }

    /// Voices per track that the driver has assigned.
    pub fn voices_per_track(&self) -> Vec<u32> {
        let mut n = vec![0; self.tracks.len()];
        for v in &self.voices {
            if let Some(t) = v.track {
                n[t] += 1;
            }
        }
        n
    }

    /// The track each voice plays for (`None`: free or released to 0).
    pub fn voice_tracks(&self) -> [Option<usize>; spu2::VOICES] {
        std::array::from_fn(|i| self.voices[i].track)
    }

    /// FFX Music Studio: a note on track `ti` from outside the sequence (a
    /// key played live), started at the next interrupt like the sequence's
    /// own key-ons.
    pub fn live_key_on(&mut self, ti: usize, key: u8, vel: u8) {
        if ti < self.tracks.len() && self.queue.len() < 48 {
            let tflags = self.tracks[ti].tflags;
            self.queue.push(KeyOn { track: ti, key: key.min(127), vel: vel.min(127), tflags });
        }
    }

    /// FFX Music Studio: the key-off of a live note (every voice of the key
    /// on the track).
    pub fn live_key_off(&mut self, ti: usize, key: u8) {
        self.queue.retain(|k| !(k.track == ti && k.key == key));
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti) && v.key == key) {
            v.flags |= 2;
        }
    }

    /// FFX Music Studio: pitch bend of track `ti` as command `0x5c` sets it
    /// (`raw` -0x2000..0x1fff, the track's bend range).
    pub fn live_bend(&mut self, ti: usize, raw: i32) {
        if ti >= self.tracks.len() {
            return;
        }
        let t = &mut self.tracks[ti];
        let raw = raw.clamp(-0x2000, 0x1fff);
        t.bend_raw = raw as i16;
        t.bend = (raw * i32::from(t.bend_range) >> 5) as i16;
        self.bend(ti);
    }

    /// FFX Music Studio: key-off of every sounding voice now (pause: each
    /// note goes into its own release); the sequence stays where it is.
    pub fn release_all(&mut self, spu: &mut Spu2) {
        self.queue.clear();
        for v in &mut self.voices {
            v.flags &= !2;
        }
        spu.key_off((1u64 << spu2::VOICES) - 1);
    }

    /// Each track's current program (instrument).
    pub fn programs(&self) -> Vec<Option<usize>> {
        self.tracks.iter().map(|t| t.inst).collect()
    }

    /// The song's tempo now (BPM).
    pub fn bpm(&self) -> i32 {
        self.tempo >> 12
    }

    /// `MusicTempoDirect(v)` (IRX `0x00109120`): the song's tempo times
    /// `1 + v / 256` for `v < 0` (-128: half), `1 + (v + 1) / 128` for
    /// `v > 0` (127: double), 0 the song's own; any slide stops.
    pub fn tempo_direct(&mut self, v: i8) {
        self.tempo_mod = Self::tempo_mod_value(v) << 12;
        self.tempo_mod_n = 0;
    }

    /// `MusicTempoTranslate(frames, v)` (IRX `0x00109148`): the modifier
    /// slides to `v` over `frames` 60 Hz steps (NTSC cycle mode).
    pub fn tempo_translate(&mut self, frames: i32, v: i8) {
        if frames <= 0 {
            self.tempo_direct(v);
            return;
        }
        let target = (Self::tempo_mod_value(v) << 12) | 0x800;
        self.tempo_mod_n = frames;
        self.tempo_mod_step = target.wrapping_sub(self.tempo_mod) / frames;
    }

    fn tempo_mod_value(v: i8) -> i32 {
        let v = i32::from(v) * 0x100;
        if v > 0 { v + 0xff } else { v }
    }

    /// The tempo step of one interrupt with the modifier applied (the
    /// driver's integer arithmetic, IRX `0x00100cd0`).
    fn modded_tempo(&self, bpm: i32) -> i32 {
        let u = (self.tempo_mod >> 12) & 0xffff;
        if u == 0 {
            bpm
        } else if self.tempo_mod < 0 {
            ((bpm as u32).wrapping_mul(u as u32) as i32) >> 16
        } else {
            (((bpm as u32).wrapping_mul(u as u32 + 1) as i32) >> 15) + bpm
        }
    }

    /// Whether the sequence still plays.
    pub fn playing(&self) -> bool {
        self.active > 0
    }

    /// One 240 Hz timer interrupt, then `FRAMES_PER_IRQ` frames of output.
    pub fn interrupt(&mut self, spu: &mut Spu2, out: &mut Vec<[i16; 2]>) {
        self.timer(spu);
        spu.render(FRAMES_PER_IRQ, out);
        self.frame += FRAMES_PER_IRQ as u64;
    }

    /// One interrupt without output (seeking: see [`Spu2::skip`]).
    pub fn interrupt_skip(&mut self, spu: &mut Spu2) {
        self.timer(spu);
        spu.skip(FRAMES_PER_IRQ);
        self.frame += FRAMES_PER_IRQ as u64;
    }

    fn timer(&mut self, spu: &mut Spu2) {
        if self.voices_in_use != 0 || !self.queue.is_empty() {
            self.flush_key_offs(spu);
            self.update_voices(spu);
        }
        self.queue.clear();
        if self.active != 0 {
            let step = self.modded_tempo(self.tempo >> 12);
            if self.mods.tempo == 0.0 || self.mods.tempo == 1.0 {
                self.tempo_counter -= step;
            } else {
                self.tempo_frac += f64::from(step) * self.mods.tempo;
                let whole = self.tempo_frac.floor();
                self.tempo_frac -= whole;
                self.tempo_counter -= whole as i32;
            }
            if self.tempo_counter < 1 {
                loop {
                    self.tempo_counter += 300;
                    self.ticks();
                    if self.tempo_counter >= 1 {
                        break;
                    }
                }
            }
        }
        // every 4th interrupt (60 Hz, IRX 0x001003e0): the modifier's slide
        self.irqs = self.irqs.wrapping_add(1);
        if self.irqs & 3 == 0 && self.tempo_mod_n != 0 {
            self.tempo_mod = self.tempo_mod.wrapping_add(self.tempo_mod_step);
            self.tempo_mod_n -= 1;
        }
    }

    /// `FUN_00100824`: one tick; at the end of the song with a loop point,
    /// back to the loop point and the tick again. (The studio's loop range
    /// and no-loop setting change where it jumps.)
    fn ticks(&mut self) {
        loop {
            if let Some((a, b)) = self.mods.loop_range {
                if self.tick == b
                    && let Some(r) = self.range_snap.clone()
                {
                    self.restore_range(r, a);
                }
                if self.tick == a {
                    self.range_snap = Some(RangeSnap {
                        tracks: self.tracks.clone(),
                        active: self.active,
                        song_flags: self.song_flags,
                        tempo: self.tempo,
                        snapshot: self.snapshot.clone(),
                    });
                }
            }
            let alive = self.tick(1);
            self.tick += 1;
            if alive != 0 {
                self.ticks_run += 1;
                break;
            }
            if self.mods.no_loop {
                self.ticks_run += 1;
                if self.song_flags & 1 != 0 {
                    // what `end_track` does when a song without loop ends
                    for v in self.voices.iter_mut().filter(|v| v.track.is_some()) {
                        v.flags = v.flags & !0xd | 2;
                        v.track = None;
                    }
                    self.song_flags &= !1;
                    self.marks.end = Some(self.frame);
                }
                break;
            }
            if let Some((a, _)) = self.mods.loop_range
                && let Some(r) = self.range_snap.clone()
            {
                self.restore_range(r, a);
                continue;
            }
            if self.song_flags & 1 == 0 {
                self.ticks_run += 1;
                break;
            }
            self.note(1, SeqEvent::LoopJump);
            let s = self.snapshot.clone().expect("loop point");
            self.active = s.active;
            self.tempo = s.tempo;
            self.tick = s.tick;
            for (t, &(pos, wait, f2, (k, v, tf))) in self.tracks.iter_mut().zip(&s.tracks) {
                t.pos = pos;
                t.wait = wait;
                t.flags2 = f2;
                t.key = k;
                t.vel = v;
                t.tflags = tf;
            }
            self.marks.loop_jumps.push(self.frame);
        }
    }

    /// Back to the studio loop range's start.
    fn restore_range(&mut self, r: RangeSnap, a: u32) {
        self.tracks = r.tracks;
        self.active = r.active;
        self.song_flags = r.song_flags;
        self.tempo = r.tempo;
        self.snapshot = r.snapshot;
        self.tick = a;
        // the notes sounding at the jump are released
        for v in self.voices.iter_mut().filter(|v| v.track.is_some()) {
            v.flags |= 2;
        }
        self.marks.loop_jumps.push(self.frame);
    }

    fn tick(&mut self, n: i32) -> i32 {
        if self.depth.run(n) {
            self.depth_dirty = true;
        }
        if let Some(ext) = &mut self.ext {
            let now = self.ticks_run;
            while let Some(e) = ext.queue.first_entry() {
                if *e.key() > now {
                    break;
                }
                let v = e.remove();
                ext.now.extend(v);
            }
        }
        let mut left = self.active;
        let mut ti = 0;
        while left != 0 && ti < self.tracks.len() {
            if self.tracks[ti].pos.is_some() {
                self.dirty = 0;
                self.mix_dirty = false;
                let w = self.tracks[ti].wait;
                let m = w.min(n);
                self.tracks[ti].wait = w - n;
                if !self.tracks[ti].ext_off {
                    self.slides(ti, m);
                }
                if self.tracks[ti].flags2 & T2_NO_AUTO_OFF == 0 && self.tracks[ti].wait == 1 {
                    self.key_off(ti);
                }
                while self.tracks[ti].pos.is_some() && self.tracks[ti].wait < 1 {
                    let mut p = self.tracks[ti].pos.expect("pos");
                    let mut op = self.bgm.data.get(p).copied().unwrap_or(0);
                    p += 1;
                    if op & 0x80 != 0 {
                        op = 0;
                    }
                    self.tracks[ti].pos = Some(p);
                    self.command(ti, op, n);
                    if self.log.is_some() && raw_logged(op) {
                        let mut q = p;
                        let mut args = [0u32; 3];
                        for (k, a) in arg_bytes(op).unwrap_or(&[]).iter().enumerate() {
                            args[k] = if *a == 0 {
                                varlen(&self.bgm.data, &mut q)
                            } else {
                                q += 1;
                                u32::from(self.bgm.data.get(q - 1).copied().unwrap_or(0))
                            };
                        }
                        self.note(ti, SeqEvent::Raw { op, args });
                    }
                    let Some(mut p) = self.tracks[ti].pos else { break };
                    let mut d = varlen(&self.bgm.data, &mut p) as i32;
                    self.tracks[ti].pos = Some(p);
                    if d != 0 {
                        d += self.tracks[ti].delta_off;
                        if d < 1 {
                            d = 1;
                        }
                    }
                    let w = self.tracks[ti].wait;
                    if w < -1 {
                        let x = if w + d > 0 { -w } else { d };
                        self.slides(ti, x);
                    }
                    self.tracks[ti].wait += d;
                }
                if self.ext.is_some() {
                    self.ext_run(ti);
                }
                if self.log.is_some() {
                    self.log_mix(ti);
                }
                if self.dirty != 0 || self.mix_dirty {
                    let tflags = self.tracks[ti].flags;
                    for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
                        v.dirty |= self.dirty;
                        v.flags = v.flags & 3 | tflags & !3;
                    }
                }
                left -= 1;
            }
            ti += 1;
        }
        if let Some(ext) = &mut self.ext {
            ext.now.clear();
        }
        if self.song_flags & 2 != 0 {
            self.song_flags &= !2;
            if self.song_flags & 1 != 0 {
                self.active = 0;
            }
        }
        self.active
    }

    /// Logs the track's volume, expression and pan when they changed.
    fn log_mix(&mut self, ti: usize) {
        if self.log_mix.len() < self.tracks.len() {
            self.log_mix.resize(self.tracks.len(), [-1; 3]);
        }
        let t = &self.tracks[ti];
        // the pan as the voices add it up (it can leave 0..=0xff before the
        // pan LFO and the clamp)
        let now = [t.vol.v >> 12, t.expr.v >> 12, t.pan.v >> 12];
        let last = self.log_mix[ti];
        self.log_mix[ti] = now;
        let ev = [SeqEvent::Volume(now[0]), SeqEvent::Expression(now[1]), SeqEvent::Pan(now[2])];
        for k in 0..3 {
            if now[k] != last[k] {
                self.note(ti, ev[k]);
            }
        }
    }

    /// `FUN_001057a0`: slides and LFO delays over `m` ticks.
    fn slides(&mut self, ti: usize, m: i32) {
        let t = &mut self.tracks[ti];
        if t.vol.run(m) {
            self.dirty |= 2;
        }
        if t.expr.run(m) {
            self.dirty |= 2;
        }
        if t.pan.run(m) {
            self.dirty |= 2;
        }
        if t.porta_n != 0 {
            let k = m.min(t.porta_n);
            let delta = k * t.porta_step;
            t.porta_n -= k;
            self.dirty |= 1;
            t.porta_pitch += delta;
            let fine = i32::from(t.detune) + i32::from(t.bend);
            let scale = t.scale;
            for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
                v.note += delta;
                v.pitch = self.tab.pitch(v.note, fine, self.wd.regions[v.region].tune, scale);
            }
        }
        let t = &mut self.tracks[ti];
        t.plfo.slides(m);
        t.vlfo.slides(m);
        t.panlfo.slides(m);
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
            v.lfo_delay = v.lfo_delay.saturating_sub(m.clamp(0, 0xffff) as u16);
            v.vlfo_delay = v.vlfo_delay.saturating_sub(m.clamp(0, 0xffff) as u16);
        }
    }

    fn arg(&mut self, ti: usize) -> u8 {
        if let Some((c, at)) = &mut self.ext_src {
            *at += 1;
            return c.get(*at - 1).copied().unwrap_or(0);
        }
        let p = self.tracks[ti].pos.expect("pos");
        self.tracks[ti].pos = Some(p + 1);
        self.bgm.data.get(p).copied().unwrap_or(0)
    }

    fn arg_varlen(&mut self, ti: usize) -> u32 {
        if let Some((c, at)) = &mut self.ext_src {
            return varlen(c, at);
        }
        let mut p = self.tracks[ti].pos.expect("pos");
        let v = varlen(&self.bgm.data, &mut p);
        self.tracks[ti].pos = Some(p);
        v
    }

    fn for_voices(&mut self, ti: usize, mut f: impl FnMut(&mut DVoice)) {
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
            f(v);
        }
    }

    fn command(&mut self, ti: usize, op: u8, ticks: i32) {
        match op {
            0x00 => self.end_track(ti),
            0x01 => {}
            0x02 => self.loop_point(ti, ticks),
            0x03 => self.song_flags |= 2,
            0x04 => {
                let t = &mut self.tracks[ti];
                t.sp = (t.sp + 1) & 3;
                t.stack[t.sp] = (t.pos.expect("pos"), 0);
                self.note(ti, SeqEvent::RepeatStart);
            }
            0x05 => {
                let n = self.arg(ti);
                let n = if n == 0 { 256 } else { u16::from(n) };
                let pass = self.tracks[ti].stack[self.tracks[ti].sp].1 + 1;
                self.note(ti, SeqEvent::RepeatEnd { pass, count: n });
                let t = &mut self.tracks[ti];
                t.stack[t.sp].1 += 1;
                if t.stack[t.sp].1 != n {
                    t.pos = Some(t.stack[t.sp].0);
                } else {
                    t.sp = (t.sp + 3) & 3;
                }
            }
            0x06 => {
                let t = &mut self.tracks[ti];
                t.pos = Some(t.stack[t.sp].0);
            }
            0x08 => {
                let b = self.arg(ti);
                self.tempo = i32::from(b) << 12;
                self.note(ti, SeqEvent::Tempo(b));
            }
            0x09 => {
                // tempo slide: the driver keeps it but never applies it
                self.arg(ti);
                self.arg(ti);
            }
            0x0a => {
                let b = self.arg(ti) as i8;
                self.depth.set(i32::from(b) << 12);
                self.depth_dirty = true;
                self.note(ti, SeqEvent::Depth(b));
            }
            0x0b => {
                let n = self.arg(ti);
                let b = self.arg(ti) as i8;
                self.depth.to(i32::from(b), if n == 0 { 256 } else { u32::from(n) });
                self.note(ti, SeqEvent::Depth(b));
            }
            0x0c => {
                self.arg(ti);
                self.arg(ti);
            }
            0x0d => {
                self.arg(ti);
            }
            0x10 => self.key_on(ti),
            0x11 => {
                self.tracks[ti].key = self.arg(ti);
                self.tracks[ti].vel = self.arg(ti);
                self.key_on(ti);
            }
            0x12 => {
                self.tracks[ti].key = self.arg(ti);
                self.key_on(ti);
            }
            0x13 => {
                self.tracks[ti].vel = self.arg(ti);
                self.key_on(ti);
            }
            0x18 => self.key_off(ti),
            0x19 => {
                self.tracks[ti].key = self.arg(ti);
                self.arg(ti);
                self.key_off(ti);
            }
            0x1a => {
                self.tracks[ti].key = self.arg(ti);
                self.key_off(ti);
            }
            0x1b => {
                self.arg(ti);
                self.key_off(ti);
            }
            0x20 => {
                let p = usize::from(self.arg(ti));
                let t = &mut self.tracks[ti];
                if p < self.wd.instruments.len() {
                    t.inst = Some(p);
                    t.adsr_ovr = [-1; 8];
                    self.note(ti, SeqEvent::Program(p));
                } else {
                    t.inst = None;
                }
            }
            0x21 => {
                self.arg(ti);
                self.arg(ti);
                *self.marks.unsupported.entry(op).or_default() += 1;
            }
            0x22 | 0x24 => {
                let b = self.arg(ti) as i8;
                let mut v = i32::from(b) * 0x80000;
                if v > 0 {
                    v += 0x7f000;
                }
                let t = &mut self.tracks[ti];
                if op == 0x22 { t.vol.set(v) } else { t.expr.set(v) }
                self.dirty |= 2;
            }
            0x23 | 0x25 => {
                let n = self.arg_varlen(ti);
                let b = self.arg(ti) as i8;
                let mut target = i32::from(b) * 0x80;
                if target > 0 {
                    target += 0x7f;
                }
                let t = &mut self.tracks[ti];
                if op == 0x23 { t.vol.to(target, n) } else { t.expr.to(target, n) }
            }
            0x26 => {
                let b = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                t.pan.set((i32::from(b) + 0x40) * 0x1000);
                if t.panlfo.wave.is_none() {
                    t.panlfo.out = 0;
                }
                self.dirty |= 2;
            }
            0x27 => {
                let n = self.arg_varlen(ti);
                let b = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                if t.panlfo.wave.is_none() {
                    t.pan.v += t.panlfo.out * 0x1000;
                    t.panlfo.out = 0;
                }
                t.pan.to(i32::from(b) + 0x40, n);
            }
            0x28 => {
                let n = self.arg_varlen(ti);
                self.tracks[ti].porta_time = n as i32;
            }
            0x29 => {
                let t = &mut self.tracks[ti];
                t.porta_time = 0;
                t.porta_pitch = -1;
            }
            0x2a => {
                let t = &mut self.tracks[ti];
                t.flags2 = t.flags2 & !T2_KEYED | T2_TIE;
                self.note(ti, SeqEvent::Tie(true));
            }
            0x2b => {
                self.tracks[ti].flags2 &= !T2_TIE;
                self.note(ti, SeqEvent::Tie(false));
            }
            0x2c => {
                let n = self.arg_varlen(ti);
                let b = self.arg(ti) as i8;
                let mut k = if n == 0 { 1 } else { n as i32 };
                let step = slide_step(0, i32::from(b) << 8, &mut k);
                let t = &mut self.tracks[ti];
                t.porta_step = step;
                t.porta_n = k;
                t.porta_pitch &= !0xfff;
                self.for_voices(ti, |v| v.note &= !0xfff);
            }
            0x2e => self.tracks[ti].flags2 |= T2_NO_AUTO_OFF,
            0x2f => self.tracks[ti].flags2 &= !T2_NO_AUTO_OFF,
            0x30 => {
                self.tracks[ti].adsr_ovr = [-1; 8];
                let regions = &self.wd.regions;
                for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
                    v.adsr = regions[v.region].adsr;
                }
                self.write_adsr(ti, true, true);
            }
            0x31..=0x38 => {
                let b = self.arg(ti);
                let i = usize::from(op - 0x31);
                self.tracks[ti].adsr_ovr[i] = b as i8;
                let b = u32::from(b);
                let (mask, val, a1) = match op {
                    0x31 => (0xffff_80ff, b << 8, true),
                    0x32 => (0xffff_ff0f, b << 4, true),
                    0x33 => (0xffff_fff0, b, true),
                    0x34 => (0xe03f_ffff, b << 22, false),
                    0x35 => (0xffe0_ffff, b << 16, false),
                    0x36 => (0xffff_7fff, (b & 4) << 13, true),
                    0x37 => (0x1fff_ffff, (b & 6) << 29, false),
                    _ => (0xffdf_ffff, (b & 4) << 19, false),
                };
                self.for_voices(ti, |v| v.adsr = v.adsr & mask | val);
                self.write_adsr(ti, a1, !a1);
            }
            0x39 => {
                let a = self.arg(ti);
                let b = self.arg(ti);
                self.tracks[ti].adsr_ovr[1] = a as i8;
                self.tracks[ti].adsr_ovr[2] = b as i8;
                let val = u32::from(a) << 4 | u32::from(b);
                self.for_voices(ti, |v| v.adsr = v.adsr & 0xffff_ff00 | val);
                self.write_adsr(ti, true, false);
            }
            0x3c => {
                let b = self.arg(ti);
                let on = b != 0;
                self.note(ti, SeqEvent::Pedal(on));
                let t = &mut self.tracks[ti];
                if on { t.flags |= T_PEDAL } else { t.flags &= !T_PEDAL }
                self.for_voices(ti, |v| if on { v.flags |= T_PEDAL } else { v.flags &= !T_PEDAL });
            }
            0x3d => self.voice_start = usize::from(self.arg(ti)),
            0x3e | 0x3f => {
                let b = self.arg(ti);
                let bit = if op == 0x3e { 0x20 } else { 0x10 };
                let t = &mut self.tracks[ti];
                if b != 0 { t.tflags |= bit } else { t.tflags &= !bit }
            }
            0x40 | 0x48 | 0x50 => {
                let d = self.arg(ti);
                let r = self.arg(ti);
                let w = self.arg(ti);
                let t = &mut self.tracks[ti];
                let lfo = match op {
                    0x40 => &mut t.plfo,
                    0x48 => &mut t.vlfo,
                    _ => &mut t.panlfo,
                };
                lfo.start(d, r, w);
                if op == 0x50 {
                    lfo.out = 0;
                    lfo.phase = 0;
                } else {
                    let (delay, fade) = (lfo.delay, lfo.fade_len());
                    let pitch = op == 0x40;
                    self.for_voices(ti, |v| {
                        if pitch {
                            v.lfo_delay = delay;
                            v.lfo_fade = fade;
                            v.lfo_fade_n = 0;
                            v.lfo_phase = 0;
                        } else {
                            v.vlfo_delay = delay;
                        }
                    });
                }
                if op == 0x48 {
                    *self.marks.unsupported.entry(op).or_default() += 1;
                }
            }
            0x41 | 0x49 | 0x51 => self.lfo(ti, op).wave = None,
            0x42 | 0x4a | 0x52 => {
                let d = self.arg(ti);
                let l = self.lfo(ti, op);
                l.depth = i32::from(d) << 12;
                l.depth_n = 0;
            }
            0x43 | 0x4b | 0x53 => {
                let n = self.arg_varlen(ti);
                let d = self.arg(ti);
                let l = self.lfo(ti, op);
                let mut k = if n == 0 { 1 } else { n as i32 };
                l.depth_step = slide_step(l.depth, i32::from(d), &mut k);
                l.depth_n = k;
            }
            0x44 | 0x4c | 0x54 => {
                let r = self.arg(ti);
                let l = self.lfo(ti, op);
                l.rate = 0x100000 / if r == 0 { 256 } else { i32::from(r) };
                l.rate_n = 0;
            }
            0x45 | 0x4d | 0x55 => {
                let n = self.arg_varlen(ti);
                let r = self.arg(ti);
                let l = self.lfo(ti, op);
                let mut k = if n == 0 { 1 } else { n as i32 };
                l.rate_step = slide_step(l.rate, 0x100 / if r == 0 { 256 } else { i32::from(r) }, &mut k);
                l.rate_n = k;
            }
            0x46 | 0x4e | 0x56 => {
                let w = self.arg(ti);
                self.lfo(ti, op).wave = Some(w & 0xf);
            }
            0x47 | 0x4f => {
                let a = self.arg(ti);
                let b = self.arg(ti);
                let l = self.lfo(ti, op);
                l.delay = u16::from(a);
                l.fade = u16::from(b);
            }
            0x58 => {
                self.tracks[ti].scale = self.arg(ti) as i8;
                self.dirty |= 1;
            }
            0x59 => {
                let b = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                t.scale = t.scale.wrapping_add(b);
                self.dirty |= 1;
            }
            0x5a => {
                let b = self.arg(ti);
                self.tracks[ti].detune = (u16::from(b) << 8) as i16;
            }
            0x5b => {
                let b = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                t.detune = t.detune.wrapping_add(i16::from(b) << 8);
            }
            0x5c => {
                let lsb = i32::from(self.arg(ti));
                let msb = i32::from(self.arg(ti));
                let t = &mut self.tracks[ti];
                let raw = lsb + msb * 0x80 - 0x2000;
                t.bend_raw = raw as i16;
                t.bend = (raw * i32::from(t.bend_range) >> 5) as i16;
                self.bend(ti);
                self.note(ti, SeqEvent::Bend(raw as i16));
            }
            0x5d => {
                let r = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                t.bend_range = r;
                t.bend = (i32::from(t.bend_raw) * i32::from(r) >> 5) as i16;
                self.bend(ti);
                self.note(ti, SeqEvent::BendRange(r));
            }
            0x60 => {
                self.tracks[ti].flags |= 0x3c00;
                self.mix_dirty = true;
                self.note(ti, SeqEvent::Wet(true));
            }
            0x61 => {
                let t = &mut self.tracks[ti];
                t.flags = t.flags & !0x3000 | 0xc00;
                self.mix_dirty = true;
                self.note(ti, SeqEvent::Wet(false));
            }
            0x62 => {
                let a = self.arg(ti);
                let b = self.arg(ti);
                let t = &mut self.tracks[ti];
                let mut f = t.flags & !0x3c00;
                f |= match a {
                    1 => 0x1000,
                    2 => 0x1400,
                    _ => 0x400,
                };
                f |= match b {
                    1 => 0x2000,
                    2 => 0x2800,
                    _ => 0x800,
                };
                t.flags = f;
                self.mix_dirty = true;
                self.note(ti, SeqEvent::Wet(f & 0x3000 != 0));
            }
            0x64 | 0x65 | 0x68 | 0x69 => {
                let bit = if op < 0x68 { 0x200 } else { 0x100 };
                let t = &mut self.tracks[ti];
                if op & 1 == 0 { t.flags |= bit } else { t.flags &= !bit }
                self.mix_dirty = true;
                if op & 1 == 0 {
                    *self.marks.unsupported.entry(op).or_default() += 1;
                }
            }
            0x6c => {
                self.arg(ti);
            }
            0x70 => {
                let b = self.arg(ti) as i8;
                let t = &mut self.tracks[ti];
                t.delta_off = if b == 0 { 0 } else { (t.delta_off + i32::from(b)).clamp(-9999, 9999) };
            }
            0x71 | 0x79 => {
                self.arg(ti);
            }
            0x78 => {
                self.arg(ti);
                self.arg(ti);
                *self.marks.unsupported.entry(op).or_default() += 1;
            }
            0x7f => {}
            _ => {
                if arg_bytes(op).is_some() {
                    // known layout, no effect here
                    for a in arg_bytes(op).expect("layout") {
                        if *a == 0 { self.arg_varlen(ti); } else { self.arg(ti); }
                    }
                    *self.marks.unsupported.entry(op).or_default() += 1;
                } else {
                    // the driver ends the track on unknown commands
                    self.end_track(ti);
                }
            }
        }
    }

    fn lfo(&mut self, ti: usize, op: u8) -> &mut Lfo {
        let t = &mut self.tracks[ti];
        match op & 0xf8 {
            0x40 => &mut t.plfo,
            0x48 => &mut t.vlfo,
            _ => &mut t.panlfo,
        }
    }

    fn write_adsr(&mut self, _ti: usize, _a1: bool, _a2: bool) {
        // ADSR registers are written in `update_voices` from `v.adsr`
        // (the chip reads them every sample, so the order is the same).
        for v in &mut self.voices {
            v.dirty |= 4;
        }
    }

    fn bend(&mut self, ti: usize) {
        let t = &self.tracks[ti];
        let fine = i32::from(t.detune) + i32::from(t.bend);
        let scale = t.scale;
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
            v.pitch = self.tab.pitch(v.note, fine, self.wd.regions[v.region].tune, scale);
            v.dirty |= 1;
        }
    }

    fn end_track(&mut self, ti: usize) {
        self.tracks[ti].flags2 = 0;
        self.key_off(ti);
        self.note(ti, SeqEvent::End);
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti)) {
            v.tflags &= 0xee;
        }
        self.tracks[ti].pos = None;
        self.active -= 1;
        if self.active == 0 && self.song_flags & 1 == 0 {
            for v in self.voices.iter_mut().filter(|v| v.track.is_some()) {
                v.flags = v.flags & !0xd | 2;
                v.track = None;
            }
            self.marks.end = Some(self.frame);
        }
    }

    fn loop_point(&mut self, ti: usize, ticks: i32) {
        if ti != 1 {
            return;
        }
        self.song_flags |= 1;
        self.note(1, SeqEvent::LoopPoint);
        let adj = 1 - self.tracks[1].wait;
        let mut p = self.tracks[1].pos.expect("pos");
        let d = varlen(&self.bgm.data, &mut p) as i32;
        let tracks = self
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let snap = (t.key, t.vel, t.tflags);
                match i {
                    0 => (t.pos, t.wait + adj, t.flags2, snap),
                    1 => (Some(p), t.wait + d + adj, t.flags2, snap),
                    _ => (t.pos, t.wait - ticks + adj, t.flags2, snap),
                }
            })
            .collect();
        self.snapshot = Some(Snapshot { tracks, active: self.active, tempo: self.tempo, tick: self.tick });
        if self.marks.loop_point.is_none() {
            self.marks.loop_point = Some(self.frame);
        }
    }

    /// `FUN_00102d50`: queues a key-on.
    fn key_on(&mut self, ti: usize) {
        let (key, vel) = (self.tracks[ti].key, self.tracks[ti].vel);
        self.note(ti, SeqEvent::NoteOn { key, vel });
        let t = &mut self.tracks[ti];
        if self.queue.len() < 48 {
            self.queue.push(KeyOn { track: ti, key: t.key, vel: t.vel, tflags: t.tflags });
        }
        t.flags |= T_NOTE;
    }

    /// `FUN_00102e70`: key-off of the track's current key.
    fn key_off(&mut self, ti: usize) {
        let t = &mut self.tracks[ti];
        if t.flags2 & T2_TIE != 0 {
            return;
        }
        let key = t.key;
        t.porta_n = 0;
        self.note(ti, SeqEvent::NoteOff { key });
        self.queue.retain(|k| !(k.track == ti && k.key == key));
        for v in self.voices.iter_mut().filter(|v| v.track == Some(ti) && v.key == key) {
            v.flags |= 2;
        }
        self.tracks[ti].flags &= !T_NOTE;
    }

    /// `FUN_00102bec`: key-offs of voices marked and not held by the pedal.
    fn flush_key_offs(&mut self, spu: &mut Spu2) {
        let mut mask = 0u64;
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.flags & 6 == 2 {
                v.flags &= !2;
                mask |= 1 << i;
            }
        }
        if mask != 0 {
            spu.key_off(mask);
        }
    }

    /// `FUN_00101140`: a free voice, or the quietest.
    fn alloc(&mut self, ti: usize) -> Option<usize> {
        let tflags = self.tracks[ti].tflags;
        let start = if tflags & 0x20 != 0 { 0 } else { self.voice_start.min(spu2::VOICES - 1) };
        let mut best = 0x7fffi16;
        let mut best_i = 0;
        let mut i = start;
        if self.voices[i].envx > 0 {
            loop {
                if self.voices[i].envx < best {
                    best = self.voices[i].envx;
                    best_i = i;
                }
                i += 1;
                if i >= spu2::VOICES || self.voices[i].envx <= 0 {
                    break;
                }
            }
        }
        let pick = if i >= spu2::VOICES {
            if best != 0x7fff { Some(best_i) } else { None }
        } else {
            Some(i)
        };
        if let Some(p) = pick {
            self.voices[p].envx = 0x7fff;
        }
        pick
    }

    /// `FUN_00101c70`: the region for a key and velocity.
    fn region(&self, inst: usize, key: u8, vel: u8) -> usize {
        self.wd.region_for(inst, key, vel)
    }

    /// `FUN_00102468` + `FUN_00101d20`: starts a queued note.
    fn start_note(&mut self, spu: &mut Spu2, k: KeyOn, kon: &mut u64) -> bool {
        let ti = k.track;
        let m = self.mods.track(ti);
        let Some(inst) = m.program.filter(|&p| p < self.wd.instruments.len()).or(self.tracks[ti].inst) else {
            return true;
        };
        let mut vi = None;
        if self.tracks[ti].flags2 & T2_TIE != 0 {
            vi = self.voices.iter().position(|v| v.track == Some(ti));
        }
        let vi = match vi.or_else(|| self.alloc(ti)) {
            Some(v) => v,
            None => {
                self.marks.dropped += 1;
                return false;
            }
        };
        let r = self.region(inst, transpose(k.key, m.transpose), k.vel);
        self.voices[vi].region = r;
        self.setup(spu, ti, vi, k, kon);
        if self.wd.regions[r].flags & 1 != 0 && k.tflags & 0x11 == 0 {
            self.voices[vi].flags |= 0x40;
            if let Some(v2) = self.alloc(ti) {
                self.voices[v2].region = (r + 1).min(self.wd.regions.len() - 1);
                self.voices[v2].flags |= 0x80;
                self.setup(spu, ti, v2, k, kon);
            }
        }
        true
    }

    fn setup(&mut self, spu: &mut Spu2, ti: usize, vi: usize, k: KeyOn, kon: &mut u64) {
        let reg = self.wd.regions[self.voices[vi].region];
        let pk = transpose(k.key, self.mods.track(ti).transpose);
        let t = &mut self.tracks[ti];
        let v = &mut self.voices[vi];
        v.track = Some(ti);
        if t.porta_time == 0 || t.porta_pitch < 0 {
            t.porta_pitch = i32::from(pk as i8) << 20;
            v.note = if reg.flags & 0x20000 == 0 { i32::from(pk as i8) << 20 } else { i32::from(reg.key_hi) << 20 };
        } else {
            t.porta_pitch &= !0xfff;
            v.note = t.porta_pitch;
            t.porta_n = t.porta_time;
            let mut n = t.porta_n.max(1);
            t.porta_step = slide_step(0, i32::from(pk as i8) * 0x100 - (t.porta_pitch >> 12), &mut n);
            t.porta_n = n;
        }
        v.key = k.key;
        v.vel = k.vel;
        v.tflags = k.tflags;
        let mut flags = t.flags;
        if reg.rev != 0 {
            flags = flags & !0x3c00 | if reg.rev == 1 { 0x3000 } else { 0x3c00 };
        }
        v.flags = flags;
        let fine = i32::from(t.detune) + i32::from(t.bend);
        v.pitch = self.tab.pitch(v.note, fine, reg.tune, t.scale);
        let keep_lfo = t.flags2 & T2_KEYED != 0
            && (if k.tflags & 0x11 == 0 { t.flags2 & T2_TIE } else { t.flags2 & 0x280000 }) != 0;
        if !keep_lfo {
            if t.plfo.wave.is_some() {
                v.lfo_delay = t.plfo.delay;
                v.lfo_fade = t.plfo.fade_len();
                v.lfo_fade_n = 0;
                v.lfo_phase = 0;
            }
            if t.vlfo.wave.is_some() {
                v.vlfo_delay = t.vlfo.delay;
            }
        }
        let ssa = reg.sample;
        let lsa = if reg.flags & 0x10 == 0 { ssa + reg.loop_off } else { ssa + 0x2000 };
        let mut adsr = reg.adsr;
        let o = t.adsr_ovr;
        let b = |i: usize| u32::from(o[i] as u8);
        if o[0] >= 0 { adsr = adsr & 0xffff_80ff | b(0) << 8; }
        if o[1] >= 0 { adsr = adsr & 0xffff_ff0f | b(1) << 4; }
        if o[2] >= 0 { adsr = adsr & 0xffff_fff0 | b(2); }
        if o[3] >= 0 { adsr = adsr & 0xe03f_ffff | b(3) << 22; }
        if o[4] >= 0 { adsr = adsr & 0xffe0_ffff | b(4) << 16; }
        if o[5] >= 0 { adsr = adsr & 0xffff_7fff | (b(5) & 4) << 13; }
        if o[6] >= 0 { adsr = adsr & 0x1fff_ffff | (b(6) & 6) << 29; }
        if o[7] >= 0 { adsr = adsr & 0xffdf_ffff | (b(7) & 4) << 19; }
        v.adsr = adsr;
        spu.set_ssa(vi, ssa);
        spu.set_lsa(vi, lsa);
        spu.set_adsr(vi, adsr as u16, (adsr >> 16) as u16);
        if t.flags2 & (T2_TIE | T2_KEYED) != T2_TIE | T2_KEYED {
            t.flags2 |= T2_KEYED;
            *kon |= 1 << vi;
        }
        v.dirty |= 3;
    }

    /// `FUN_001025e8`: envelopes read back, key-ons, the pan LFO, volumes
    /// and pitches, the mix switches, the reverb depth and the key-on.
    fn update_voices(&mut self, spu: &mut Spu2) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.tflags & 0x13 == 0 {
                if v.envx > 0 {
                    let e = spu.envx(i);
                    v.envx = e;
                    if e <= 0 {
                        v.track = None;
                    } else if self.wd.regions.get(v.region).is_some_and(|r| r.flags & 0x10 != 0) {
                        // `FUN_001012d8`: a long sample cut into 0x2000-byte
                        // pieces, each ending in loop end + repeat. Once the
                        // voice is inside the piece the loop start points to,
                        // the loop start moves on to the next piece, so the
                        // sample plays through (banks 143 and 149).
                        let (nax, lsa) = (spu.nax(i), spu.lsa(i));
                        if lsa <= nax && nax < lsa + 0x2000 {
                            spu.set_lsa(i, lsa + 0x2000);
                        }
                    }
                }
            } else {
                v.envx = 0x7fff;
            }
        }
        let mut kon = 0u64;
        let queue = std::mem::take(&mut self.queue);
        for (i, k) in queue.iter().enumerate() {
            if !self.start_note(spu, *k, &mut kon) {
                if self.log.is_some() {
                    // queued in the tick before this interrupt
                    let at = self.ticks_run.saturating_sub(1);
                    for k in &queue[i..] {
                        if let Some(l) = &mut self.log {
                            l.push(LogEntry { tick: at, frame: self.frame, track: k.track, event: SeqEvent::Dropped { key: k.key } });
                        }
                    }
                }
                break;
            }
        }
        if self.active != 0 {
            for t in self.tracks.iter_mut().filter(|t| t.pos.is_some() && !t.ext_off) {
                if let Some(w) = t.panlfo.wave {
                    let x = self.tab.wave(w, t.panlfo.phase >> 12);
                    t.panlfo.phase = t.panlfo.phase.wrapping_add(t.panlfo.rate as u32);
                    t.panlfo.out = ((t.panlfo.depth >> 12) + 1) * x >> 16;
                }
            }
        }
        let sv = (i32::from(self.bgm.volume & 0x7f) + 1) * 0x1ff >> 7;
        let mut used = 0;
        for i in 0..spu2::VOICES {
            let Some(ti) = self.voices[i].track else { continue };
            used += 1;
            if self.tracks[ti].flags & T_MUTE != 0 {
                continue;
            }
            let t = &self.tracks[ti];
            if self.voices[i].dirty & 4 != 0 {
                let a = self.voices[i].adsr;
                spu.set_adsr(i, a as u16, (a >> 16) as u16);
            }
            if self.voices[i].dirty & 2 != 0 || t.vlfo.wave.is_some() || t.panlfo.wave.is_some() {
                let (l, r) = self.volume(i, sv);
                spu.set_volume(i, l, r);
            }
            if self.voices[i].dirty & 1 != 0 || t.plfo.wave.is_some() {
                let p = self.voice_pitch(i);
                spu.set_pitch(i, p);
            }
            self.voices[i].dirty = 0;
        }
        self.voices_in_use = used;
        // the mix switches (VMIXL/R, VMIXEL/R) from the voice flags
        for (i, v) in self.voices.iter().enumerate() {
            if v.track.is_none() && kon >> i & 1 == 0 {
                continue;
            }
            let f = v.flags;
            if f & 0x28 == 0 {
                spu.set_mix(i, f & 0x400 != 0, f & 0x800 != 0, f & 0x1000 != 0, f & 0x2000 != 0);
            } else {
                spu.set_mix(i, false, false, false, false);
            }
        }
        if self.depth_dirty {
            let d = (self.depth.v >> 4) as i16;
            spu.set_evol(0, d, d);
            spu.set_evol(1, d, d);
            self.depth_dirty = false;
        }
        if kon != 0 {
            spu.key_on(kon);
        }
    }

    /// `FUN_001014b4`: the voice's volume registers.
    fn volume(&self, i: usize, sv: i32) -> (u16, u16) {
        let v = &self.voices[i];
        let t = &self.tracks[v.track.expect("track")];
        let reg = &self.wd.regions[v.region];
        let mut a = if sv != 0 { sv + 1 } else { 0 };
        a = a * ((t.expr.v >> 12) + 1) >> 9;
        if v.vel != 0 {
            a = a * (i32::from(v.vel as i8) + 1) >> 7;
        }
        let mut m = t.master >> 12;
        if m != 0 {
            m += 1;
        }
        let mut rv = i32::from(reg.vol & 0x7f);
        if rv != 0 {
            rv += 1;
        }
        a = ((((a * m) >> 7) * (t.vol.v >> 12) >> 14) * rv) >> 7;
        let tm = self.mods.track(v.track.expect("track"));
        if tm.gain != 1.0 {
            a = ((a as f32 * tm.gain.max(0.0)) as i32).min(0x7fff);
        }
        let pan = if v.flags & 0xc0 == 0 {
            let mut p = if reg.pan & 0x80 == 0 {
                t.pan.v >> 12
            } else {
                let x = i32::from(reg.pan & 0x7f) + 0x40;
                if x == 0 { 0x80 } else { x }
            };
            p += t.panlfo.out + tm.pan;
            p.clamp(0, 0xff)
        } else if v.flags & 0x40 != 0 {
            0x40
        } else {
            0xbf
        };
        let l = a * self.tab.pan[pan as usize] >> 8;
        let r = a * self.tab.pan[(!pan & 0xff) as usize] >> 8;
        let reg = |x: i32| -> u16 { (((x & 0x7fff) >> 1) | (x & 0x4000)) as u16 };
        (reg(l), reg(r))
    }

    /// `FUN_00101810`: the pitch register with the vibrato.
    fn voice_pitch(&mut self, i: usize) -> u16 {
        let v = self.voices[i];
        let t = &self.tracks[v.track.expect("track")];
        let mut lfo = 0i32;
        if let Some(w) = t.plfo.wave {
            if v.lfo_delay == 0 {
                let d = (t.plfo.depth >> 12) as u32;
                let amt = if d < 0x80 { (d as i32 + 1) * 2 } else { ((d & 0x7f) as i32 + 1) * 0x18 };
                let fine = amt + i32::from(t.detune) + i32::from(t.bend);
                let p2 = self.tab.pitch(v.note, fine, self.wd.regions[v.region].tune, t.scale) as i32;
                let x = self.tab.wave(w, v.lfo_phase >> 12);
                lfo = (p2 - v.pitch as i32) * x >> 16;
                let vm = &mut self.voices[i];
                if vm.lfo_fade != 0 {
                    lfo = lfo * vm.lfo_fade_n / vm.lfo_fade;
                    vm.lfo_fade_n += 1;
                    if vm.lfo_fade_n >= vm.lfo_fade {
                        vm.lfo_fade = 0;
                    }
                }
                if lfo < 0 {
                    lfo >>= 1;
                }
                vm.lfo_phase = vm.lfo_phase.wrapping_add(t.plfo.rate as u32);
            }
        }
        ((lfo + v.pitch as i32) & 0x3fff) as u16
    }
}

/// A key moved by `semitones` (studio transpose), kept in 0..=127.
fn transpose(key: u8, semitones: i32) -> u8 {
    if semitones == 0 { key } else { (i32::from(key) + semitones).clamp(0, 127) as u8 }
}

/// A MIDI-style variable-length number at `p` (moves `p` past it).
pub fn varlen(d: &[u8], p: &mut usize) -> u32 {
    let mut v = 0u32;
    loop {
        let Some(&b) = d.get(*p) else { return v };
        *p += 1;
        v = v << 7 | u32::from(b & 0x7f);
        if b & 0x80 == 0 {
            return v;
        }
    }
}

/// How a render ends.
#[derive(Debug, Clone, Copy)]
pub struct RenderPlan {
    /// Loops of the looping part to play (looping songs).
    pub loops: usize,
    /// Upper bound in seconds.
    pub max_seconds: f64,
    /// Seconds of silence after a non-looping song that end it.
    pub tail_seconds: f64,
}

/// Renders a song, handing blocks of output to `sink` as it goes.
pub fn render(
    tab: &Tables,
    bgm: &Bgm,
    wd: &Wd,
    plan: RenderPlan,
    mut sink: impl FnMut(&[[i16; 2]]) -> Result<()>,
) -> Result<Marks> {
    let mut spu = Spu2::new(wd.samples.clone());
    let mut song = Song::new(Arc::new(tab.clone()), Arc::new(bgm.clone()), Arc::new(wd.clone()), &mut spu);
    let max = (plan.max_seconds * f64::from(spu2::RATE)) as u64;
    let mut buf = Vec::with_capacity(FRAMES_PER_IRQ * 64);
    let mut quiet = 0u64;
    let mut frames = 0u64;
    loop {
        buf.clear();
        for _ in 0..64 {
            song.interrupt(&mut spu, &mut buf);
        }
        let n = buf.len() as u64;
        let silent = buf.iter().all(|s| s[0].unsigned_abs() < 2 && s[1].unsigned_abs() < 2);
        // a looping song ends exactly at its `loops`-th jump back
        if song.marks.loop_jumps.len() >= plan.loops && plan.loops > 0 {
            let end = song.marks.loop_jumps[plan.loops - 1];
            let keep = end.saturating_sub(frames) as usize;
            sink(&buf[..keep.min(buf.len())])?;
            break;
        }
        sink(&buf)?;
        frames += n;
        if !song.playing() && !spu.busy() {
            // the reverb rings on after the last voice
            quiet += n;
            if silent || quiet as f64 >= plan.tail_seconds * f64::from(spu2::RATE) {
                break;
            }
        }
        if frames >= max {
            break;
        }
    }
    Ok(song.marks)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank of one instrument: one region, a looped square wave.
    fn square_wd() -> Wd {
        let mut block = vec![0x00u8, 0x07];
        block.extend([0x77; 7]);
        block.extend([0x88; 7]);
        let mut b = vec![0u8; 0x24];
        b[..2].copy_from_slice(b"WD");
        b[4..8].copy_from_slice(&(block.len() as u32).to_le_bytes());
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&0x30u32.to_le_bytes());
        b.resize(0x30, 0);
        let mut r = [0u8; 0x20];
        r[..4].copy_from_slice(&0x200u32.to_le_bytes());
        r[12..16].copy_from_slice(&0x1fc6_80ffu32.to_le_bytes());
        r[0x14] = 127;
        r[0x15] = 127;
        r[0x16] = 127;
        b.extend(r);
        b.extend(block);
        Wd::parse(&b).expect("wd")
    }

    fn bgm(tracks: &[Vec<u8>]) -> Bgm {
        let mut b = vec![0u8; 0x20];
        b[..4].copy_from_slice(b"BGM ");
        b[8] = tracks.len() as u8;
        b[9] = 0xff;
        b[12] = 0x60;
        for t in tracks {
            b.extend_from_slice(&(t.len() as u32).to_le_bytes());
            b.extend_from_slice(t);
        }
        Bgm::parse(&b).expect("bgm")
    }

    fn push_varlen(c: &mut Vec<u8>, mut x: u32) {
        let mut v = vec![(x & 0x7f) as u8];
        x >>= 7;
        while x != 0 {
            v.insert(0, (x & 0x7f) as u8 | 0x80);
            x >>= 7;
        }
        c.extend(v);
    }

    /// The log of a song as external commands, in the order the driver
    /// carried them out.
    fn as_ext(log: &[LogEntry]) -> Vec<(u64, usize, Vec<u8>)> {
        let mut out = Vec::new();
        let last_end = log.iter().rposition(|e| e.event == SeqEvent::End);
        for (i, e) in log.iter().enumerate() {
            let c: Vec<u8> = match e.event {
                SeqEvent::NoteOn { key, vel } => vec![0x11, key, vel],
                SeqEvent::NoteOff { key } => vec![0x1a, key],
                SeqEvent::Tie(on) => vec![if on { 0x2a } else { 0x2b }],
                SeqEvent::Program(p) => vec![0x20, p as u8],
                SeqEvent::Pedal(on) => vec![0x3c, u8::from(on)],
                SeqEvent::Bend(raw) => {
                    let v = (i32::from(raw) + 0x2000) as u16;
                    vec![0x5c, (v & 0x7f) as u8, (v >> 7) as u8]
                }
                SeqEvent::BendRange(r) => vec![0x5d, r as u8],
                SeqEvent::Volume(v) => [vec![EXT_VOLUME], (v as i16).to_le_bytes().to_vec()].concat(),
                SeqEvent::Expression(v) => [vec![EXT_EXPRESSION], (v as i16).to_le_bytes().to_vec()].concat(),
                SeqEvent::Pan(v) => [vec![EXT_PAN], (v as i16).to_le_bytes().to_vec()].concat(),
                SeqEvent::Tempo(b) => vec![0x08, b],
                SeqEvent::Depth(d) => vec![0x0a, d as u8],
                SeqEvent::Raw { op, args } => {
                    let mut c = vec![op];
                    for (k, a) in arg_bytes(op).unwrap_or(&[]).iter().enumerate() {
                        if *a == 0 { push_varlen(&mut c, args[k]) } else { c.push(args[k] as u8) }
                    }
                    c
                }
                SeqEvent::End => {
                    out.push((e.tick, e.track, vec![0x00]));
                    if Some(i) == last_end {
                        out.push((e.tick, e.track, vec![EXT_SONG_END]));
                    }
                    continue;
                }
                _ => continue,
            };
            out.push((e.tick, e.track, c));
        }
        out
    }

    #[test]
    fn external_input_plays_like_the_sequence() {
        let tab = Arc::new(Tables::generic());
        let wd = Arc::new(square_wd());
        let tracks = vec![
            vec![0, 0x08, 120, 0, 0x0a, 64, 96, 0x08, 90, 0, 0x00],
            vec![
                0, 0x20, 0, 0, 0x22, 100, 0, 0x26, 32, 0, 0x60, 0, 0x11, 60, 100, 24, 0x23, 48, 20, 0, 0x12, 64, 24, 0x1a,
                60, 0, 0x47, 4, 2, 0, 0x40, 30, 10, 0, 0, 0x5c, 0, 70, 24, 0x18, 0, 0x13, 90, 48, 0x5d, 12, 0, 0x5c, 0,
                50, 0, 0x5a, 3, 0, 0x58, 16, 12, 0x18, 0, 0x28, 20, 0, 0x11, 50, 90, 10, 0x18, 0, 0x11, 57, 90, 30, 0x18,
                0, 0x29, 0, 0x00,
            ],
            vec![
                6, 0x20, 0, 0, 0x50, 60, 20, 1, 0, 0x3c, 127, 0, 0x11, 48, 80, 0, 0x2a, 30, 0x12, 50, 0, 0x2b, 30, 0x18,
                0, 0x3c, 0, 0, 0x35, 8, 0, 0x31, 40, 0, 0x11, 55, 127, 0, 0x2c, 20, 4, 40, 0x1a, 55, 0, 0x61, 0, 0x11,
                55, 60, 20, 0x00,
            ],
        ];
        let bgm = Arc::new(bgm(&tracks));
        let irqs = 240 * 8;

        let mut spu = Spu2::new(wd.samples.clone());
        let mut song = Song::new(tab.clone(), bgm.clone(), wd.clone(), &mut spu);
        song.log = Some(Vec::new());
        let mut want = Vec::new();
        for _ in 0..irqs {
            song.interrupt(&mut spu, &mut want);
        }
        let log = song.log.take().expect("log");
        assert!(want.iter().any(|s| s[0] != 0), "the song sounds");
        assert!(log.iter().any(|e| matches!(e.event, SeqEvent::Raw { op: 0x40, args: [30, 10, 0] })));

        let mut spu = Spu2::new(wd.samples.clone());
        let mut ext = Song::external(tab, wd.clone(), 3, ExtHeader::of(&bgm), &mut spu);
        for (tick, ti, c) in as_ext(&log) {
            ext.ext_command(tick, ti, &c);
        }
        let mut got = Vec::new();
        for _ in 0..irqs {
            ext.ext_irq(&mut spu, None);
            spu.render(FRAMES_PER_IRQ, &mut got);
        }
        let first = want.iter().zip(&got).position(|(a, b)| a != b);
        assert_eq!(first, None, "first difference at frame {first:?}");
        assert_eq!(ext.ext_pending(), 0);
    }
}
