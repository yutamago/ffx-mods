//! A PS2 sound chip (SPU2) for the music renderer (`docs/music.md`): 48
//! voices on two cores playing 4-bit ADPCM from sound RAM with the ADSR
//! envelope, the 4-tap interpolation, per-voice volumes and the reverb, at
//! 48 kHz. Written from the hardware's documented behaviour (PCSX2's SPU2
//! and the PS1 SPU it extends were the reference for the facts); the driver
//! (`sqseq.rs`) programs it through the same registers `libsd` would.
//!
//! What is left out because the music never uses it: noise, pitch
//! modulation, the IRQ, sound data input and the voice streams.

/// Voices per core.
pub const CORE_VOICES: usize = 24;
pub const VOICES: usize = 2 * CORE_VOICES;
pub const RATE: u32 = 48_000;

/// ADPCM filter coefficients (/64).
const FILTERS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];

/// The reverb registers of one preset, in the order `libsd` stores them
/// (the PS1 order): two all-pass delays, eight volumes, twenty addresses
/// (8-byte units, x4 halfwords on the SPU2), the two input volumes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReverbPreset {
    pub regs: [u16; 32],
    /// Work area size in 8-byte units.
    pub size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone)]
struct Voice {
    // registers
    vol_l: i16,
    vol_r: i16,
    pitch: u16,
    adsr1: u16,
    adsr2: u16,
    ssa: u32,
    lsa: u32,
    dry_l: bool,
    dry_r: bool,
    wet_l: bool,
    wet_r: bool,
    // state
    nax: u32,
    block: [i16; 28],
    block_flags: u8,
    pos: usize,
    hist: [i32; 2],
    interp: [i32; 4],
    counter: u32,
    phase: Phase,
    level: i32,
    wait: i32,
    endx: bool,
}

impl Default for Voice {
    fn default() -> Self {
        Voice {
            vol_l: 0,
            vol_r: 0,
            pitch: 0,
            adsr1: 0,
            adsr2: 0,
            ssa: 0,
            lsa: 0,
            dry_l: true,
            dry_r: true,
            wet_l: false,
            wet_r: false,
            nax: 0,
            block: [0; 28],
            block_flags: 0,
            pos: 0,
            hist: [0; 2],
            interp: [0; 4],
            counter: 0,
            phase: Phase::Off,
            level: 0,
            wait: 0,
            endx: false,
        }
    }
}

/// Debug switches from the environment (`DEMASTER_RAW=1`: `demaster music`
/// writes the chip's output without the gain and limiter), 0.0 when unset.
pub fn knob(name: &str) -> f32 {
    use std::sync::OnceLock;
    static K: OnceLock<std::collections::HashMap<String, f32>> = OnceLock::new();
    let k = K.get_or_init(|| {
        std::env::vars().filter(|(n, _)| n.starts_with("DEMASTER_")).filter_map(|(n, v)| Some((n, v.parse().ok()?))).collect()
    });
    k.get(name).copied().unwrap_or(0.0)
}

/// One step of the envelope: cycles to wait and the level change, from a
/// 7-bit rate (shift << 2 | step) as the hardware computes them.
fn env_step(rate: u32, decrease: bool, exponential: bool, level: i32) -> (i32, i32) {
    let shift = (rate >> 2) as i32;
    let step = (rate & 3) as i32;
    let mut cycles = 1i32 << (shift - 11).max(0);
    let base = if decrease { -8 + step } else { 7 - step };
    let mut delta = base << (11 - shift).max(0);
    if exponential && !decrease && level > 0x6000 {
        cycles *= 4;
    }
    if exponential && decrease {
        delta = delta * level >> 15;
    }
    (cycles, delta)
}

impl Voice {
    fn key_on(&mut self, ram: &[u8]) {
        self.nax = self.ssa;
        self.hist = [0; 2];
        self.interp = [0; 4];
        self.counter = 0;
        self.phase = Phase::Attack;
        self.level = 0;
        self.wait = 0;
        self.endx = false;
        self.decode(ram);
    }

    fn key_off(&mut self) {
        if self.phase != Phase::Off {
            self.phase = Phase::Release;
            self.wait = 0;
        }
    }

    /// Decodes the block at `nax` into `block`.
    fn decode(&mut self, ram: &[u8]) {
        let a = self.nax as usize;
        let b = ram.get(a..a + 16).unwrap_or(&[0; 16]);
        let shift = (b[0] & 0xf) as i32;
        let filter = ((b[0] >> 4) as usize).min(4);
        let flags = b[1];
        if flags & 4 != 0 {
            self.lsa = self.nax;
        }
        self.block_flags = flags;
        let (f0, f1) = FILTERS[filter];
        let shift = if shift > 12 { 9 } else { shift };
        for i in 0..28 {
            let byte = b[2 + i / 2];
            let nib = if i & 1 == 0 { byte & 0xf } else { byte >> 4 };
            let s = ((i32::from(nib as i8) << 28) >> 28) << 12 >> shift;
            let v = s + ((self.hist[0] * f0 + self.hist[1] * f1 + 32) >> 6);
            let v = v.clamp(-0x8000, 0x7fff);
            self.hist[1] = self.hist[0];
            self.hist[0] = v;
            self.block[i] = v as i16;
        }
        self.pos = 0;
    }

    /// The next decoded sample into the interpolation history.
    fn advance(&mut self, ram: &[u8]) {
        let s = i32::from(self.block[self.pos]);
        self.interp = [self.interp[1], self.interp[2], self.interp[3], s];
        self.pos += 1;
        if self.pos == 28 {
            let flags = self.block_flags;
            if flags & 1 != 0 {
                self.endx = true;
                self.nax = self.lsa;
                if flags & 2 == 0 {
                    // End without repeat: the voice stops.
                    self.phase = Phase::Off;
                    self.level = 0;
                }
            } else {
                self.nax += 16;
            }
            self.decode(ram);
        }
    }

    fn envelope(&mut self) {
        let a1 = u32::from(self.adsr1);
        let a2 = u32::from(self.adsr2);
        match self.phase {
            Phase::Off => return,
            Phase::Attack => {
                if self.level >= 0x7fff {
                    self.phase = Phase::Decay;
                    self.wait = 0;
                }
            }
            Phase::Decay => {
                let sustain = ((a1 & 0xf) as i32 + 1) * 0x800;
                if self.level <= sustain {
                    self.phase = Phase::Sustain;
                    self.wait = 0;
                }
            }
            _ => {}
        }
        if self.wait > 1 {
            self.wait -= 1;
            return;
        }
        let (rate, decrease, exponential) = match self.phase {
            Phase::Attack => ((a1 >> 8) & 0x7f, false, a1 & 0x8000 != 0),
            Phase::Decay => (((a1 >> 4) & 0xf) << 2, true, true),
            Phase::Sustain => ((a2 >> 6) & 0x7f, a2 & 0x4000 != 0, a2 & 0x8000 != 0),
            Phase::Release => ((a2 & 0x1f) << 2, true, a2 & 0x20 != 0),
            Phase::Off => unreachable!(),
        };
        let (cycles, delta) = env_step(rate, decrease, exponential, self.level);
        self.wait = cycles;
        self.level = (self.level + delta).clamp(0, 0x7fff);
        if self.phase == Phase::Release && self.level == 0 {
            self.phase = Phase::Off;
        }
    }

    /// One output sample (before the voice volumes) and the step forward.
    fn sample(&mut self, ram: &[u8], gauss: &[i32; 512]) -> i32 {
        if self.phase == Phase::Off {
            return 0;
        }
        let i = ((self.counter >> 4) & 0xff) as usize;
        let s = (gauss[0xff - i] * self.interp[0]
            + gauss[0x1ff - i] * self.interp[1]
            + gauss[0x100 + i] * self.interp[2]
            + gauss[i] * self.interp[3])
            >> 15;
        let out = s * self.level >> 15;
        self.envelope();
        self.counter += u32::from(self.pitch.min(0x3fff));
        while self.counter >= 0x1000 {
            self.counter -= 0x1000;
            self.advance(ram);
            if self.phase == Phase::Off {
                break;
            }
        }
        out
    }
}

/// The 4-tap interpolation table: a Gaussian smoothing kernel (sigma 0.55
/// samples, sum 1.0 in 1.15 fixed point). The hardware's table has the same
/// shape; its exact values are not reproduced.
fn gauss_table() -> [i32; 512] {
    let mut t = [0i32; 512];
    for i in 0..256 {
        let f = i as f64 / 256.0;
        // taps at -1, 0, 1, 2 relative to the second-oldest sample
        let w: Vec<f64> = [-1.0 - f, -f, 1.0 - f, 2.0 - f]
            .iter()
            .map(|d: &f64| (-(d * d) / (2.0 * 0.55 * 0.55)).exp())
            .collect();
        let sum: f64 = w.iter().sum();
        t[0xff - i] = (w[0] / sum * 32768.0).round() as i32;
        t[0x1ff - i] = (w[1] / sum * 32768.0).round() as i32;
        t[0x100 + i] = (w[2] / sum * 32768.0).round() as i32;
        t[i] = (w[3] / sum * 32768.0).round() as i32;
    }
    t
}

/// A half-band low-pass for the reverb's 2:1 down- and up-sampling.
fn halfband() -> Vec<f32> {
    const N: usize = 39;
    let mid = (N / 2) as f64;
    let mut h: Vec<f64> = (0..N)
        .map(|n| {
            let x = n as f64 - mid;
            let sinc = if x == 0.0 { 0.5 } else { (std::f64::consts::PI * x / 2.0).sin() / (std::f64::consts::PI * x) };
            let w = 0.42 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / (N - 1) as f64).cos()
                + 0.08 * (4.0 * std::f64::consts::PI * n as f64 / (N - 1) as f64).cos();
            sinc * w
        })
        .collect();
    let sum: f64 = h.iter().sum();
    for v in &mut h {
        *v /= sum;
    }
    h.into_iter().map(|v| v as f32).collect()
}

#[derive(Debug, Clone, Default)]
struct Reverb {
    on: bool,
    preset: ReverbPreset,
    buf: Vec<i16>,
    cur: usize,
    // 48 kHz input history and 24 kHz output history for the resampler
    in_hist: Vec<[f32; 2]>,
    out_hist: Vec<[f32; 2]>,
    phase: bool,
}

fn sat(v: i32) -> i16 {
    v.clamp(-0x8000, 0x7fff) as i16
}

impl Reverb {
    fn set(&mut self, preset: ReverbPreset) {
        self.on = true;
        self.preset = preset;
        let len = (preset.size as usize * 8).saturating_sub(1).max(1);
        self.buf = vec![0; len];
        self.cur = 0;
        self.in_hist = vec![[0.0; 2]; 39];
        self.out_hist = vec![[0.0; 2]; 39];
    }

    fn at(&self, off: u16) -> usize {
        (self.cur + usize::from(off) * 4) % self.buf.len()
    }

    fn rd(&self, off: u16) -> i32 {
        i32::from(self.buf[self.at(off)])
    }

    fn rd_prev(&self, off: u16) -> i32 {
        let n = self.buf.len();
        i32::from(self.buf[(self.at(off) + n - 1) % n])
    }

    fn wr(&mut self, off: u16, v: i32) {
        let a = self.at(off);
        self.buf[a] = sat(v);
    }

    /// One 24 kHz step: input L/R, output L/R.
    fn step(&mut self, l: i32, r: i32) -> (i32, i32) {
        let g = self.preset.regs;
        let v = |i: usize| i32::from(g[i] as i16);
        let (d_apf1, d_apf2) = (g[0], g[1]);
        let (v_iir, v_comb1, v_comb2, v_comb3, v_comb4, v_wall, v_apf1, v_apf2) =
            (v(2), v(3), v(4), v(5), v(6), v(7), v(8), v(9));
        let (m_lsame, m_rsame, m_lcomb1, m_rcomb1, m_lcomb2, m_rcomb2, d_lsame, d_rsame) =
            (g[10], g[11], g[12], g[13], g[14], g[15], g[16], g[17]);
        let (m_ldiff, m_rdiff, m_lcomb3, m_rcomb3, m_lcomb4, m_rcomb4, d_ldiff, d_rdiff) =
            (g[18], g[19], g[20], g[21], g[22], g[23], g[24], g[25]);
        let (m_lapf1, m_rapf1, m_lapf2, m_rapf2) = (g[26], g[27], g[28], g[29]);
        let (v_lin, v_rin) = (v(30), v(31));

        let lin = l * v_lin >> 15;
        let rin = r * v_rin >> 15;
        let iir = |inp: i32, wall: i32, prev: i32| ((inp + (wall * v_wall >> 15) - prev) * v_iir >> 15) + prev;
        let ls = iir(lin, self.rd(d_lsame), self.rd_prev(m_lsame));
        let rs = iir(rin, self.rd(d_rsame), self.rd_prev(m_rsame));
        let ld = iir(lin, self.rd(d_rdiff), self.rd_prev(m_ldiff));
        let rd = iir(rin, self.rd(d_ldiff), self.rd_prev(m_rdiff));
        self.wr(m_lsame, ls);
        self.wr(m_rsame, rs);
        self.wr(m_ldiff, ld);
        self.wr(m_rdiff, rd);
        let mut lo = (v_comb1 * self.rd(m_lcomb1)
            + v_comb2 * self.rd(m_lcomb2)
            + v_comb3 * self.rd(m_lcomb3)
            + v_comb4 * self.rd(m_lcomb4))
            >> 15;
        let mut ro = (v_comb1 * self.rd(m_rcomb1)
            + v_comb2 * self.rd(m_rcomb2)
            + v_comb3 * self.rd(m_rcomb3)
            + v_comb4 * self.rd(m_rcomb4))
            >> 15;
        let n = self.buf.len();
        let back = |s: &Self, m: u16, d: u16| -> i32 {
            let a = (s.at(m) + n - (usize::from(d) * 4) % n) % n;
            i32::from(s.buf[a])
        };
        for (m_l, m_r, d, va) in [(m_lapf1, m_rapf1, d_apf1, v_apf1), (m_lapf2, m_rapf2, d_apf2, v_apf2)] {
            let bl = back(self, m_l, d);
            let br = back(self, m_r, d);
            lo -= va * bl >> 15;
            ro -= va * br >> 15;
            self.wr(m_l, lo);
            self.wr(m_r, ro);
            lo = (i32::from(sat(lo)) * va >> 15) + bl;
            ro = (i32::from(sat(ro)) * va >> 15) + br;
        }
        self.cur = (self.cur + 1) % n;
        (i32::from(sat(lo)), i32::from(sat(ro)))
    }

    /// One 48 kHz sample through the 2:1 resampler.
    fn process(&mut self, l: i32, r: i32, h: &[f32]) -> (i32, i32) {
        if !self.on {
            return (0, 0);
        }
        self.in_hist.rotate_left(1);
        *self.in_hist.last_mut().expect("history") = [l as f32, r as f32];
        self.phase = !self.phase;
        let mut up = [0.0f32; 2];
        if self.phase {
            let mut acc = [0.0f32; 2];
            for (x, c) in self.in_hist.iter().zip(h) {
                acc[0] += x[0] * c;
                acc[1] += x[1] * c;
            }
            let (ol, or) = self.step(acc[0].round() as i32, acc[1].round() as i32);
            self.out_hist.rotate_left(1);
            *self.out_hist.last_mut().expect("history") = [ol as f32 * 2.0, or as f32 * 2.0];
        } else {
            self.out_hist.rotate_left(1);
            *self.out_hist.last_mut().expect("history") = [0.0, 0.0];
        }
        for (x, c) in self.out_hist.iter().zip(h) {
            up[0] += x[0] * c;
            up[1] += x[1] * c;
        }
        (up[0].round() as i32, up[1].round() as i32)
    }
}

#[derive(Debug, Clone, Default)]
struct Core {
    evol_l: i16,
    evol_r: i16,
    reverb: Reverb,
}

/// The chip. Sound RAM holds the sample data at byte addresses.
pub struct Spu2 {
    ram: Vec<u8>,
    voices: Vec<Voice>,
    cores: [Core; 2],
    gauss: [i32; 512],
    halfband: Vec<f32>,
    /// Gain applied at the output (1.0: the chip's own level).
    pub gain: f32,
}

impl Spu2 {
    pub fn new(ram: Vec<u8>) -> Self {
        Spu2 {
            ram,
            voices: vec![Voice::default(); VOICES],
            cores: [Core { evol_l: 0x7fff, evol_r: 0x7fff, ..Core::default() }, Core { evol_l: 0x7fff, evol_r: 0x7fff, ..Core::default() }],
            gauss: gauss_table(),
            halfband: halfband(),
            gain: 1.0,
        }
    }

    /// Voice volume registers (fixed mode: bits 0-14 are half the volume).
    pub fn set_volume(&mut self, v: usize, l: u16, r: u16) {
        let conv = |x: u16| -> i16 { (((x << 1) as i16) >> 1).wrapping_mul(2) };
        self.voices[v].vol_l = conv(l);
        self.voices[v].vol_r = conv(r);
    }
    pub fn set_pitch(&mut self, v: usize, p: u16) {
        self.voices[v].pitch = p;
    }
    pub fn set_adsr(&mut self, v: usize, a1: u16, a2: u16) {
        self.voices[v].adsr1 = a1;
        self.voices[v].adsr2 = a2;
    }
    pub fn set_ssa(&mut self, v: usize, a: u32) {
        self.voices[v].ssa = a & !15;
    }
    pub fn set_lsa(&mut self, v: usize, a: u32) {
        self.voices[v].lsa = a & !15;
    }
    pub fn set_mix(&mut self, v: usize, dry_l: bool, dry_r: bool, wet_l: bool, wet_r: bool) {
        let x = &mut self.voices[v];
        x.dry_l = dry_l;
        x.dry_r = dry_r;
        x.wet_l = wet_l;
        x.wet_r = wet_r;
    }
    pub fn key_on(&mut self, mask: u64) {
        for v in 0..VOICES {
            if mask >> v & 1 != 0 {
                let ram = std::mem::take(&mut self.ram);
                self.voices[v].key_on(&ram);
                self.ram = ram;
            }
        }
    }
    pub fn key_off(&mut self, mask: u64) {
        for v in 0..VOICES {
            if mask >> v & 1 != 0 {
                self.voices[v].key_off();
            }
        }
    }
    /// The envelope level the driver reads back (`ENVX`).
    pub fn envx(&self, v: usize) -> i16 {
        if self.voices[v].phase == Phase::Off {
            0
        } else {
            self.voices[v].level as i16
        }
    }
    pub fn set_evol(&mut self, core: usize, l: i16, r: i16) {
        self.cores[core].evol_l = l;
        self.cores[core].evol_r = r;
    }
    pub fn set_reverb(&mut self, core: usize, preset: ReverbPreset) {
        self.cores[core].reverb.set(preset);
    }

    /// Renders `n` stereo frames into `out`.
    pub fn render(&mut self, n: usize, out: &mut Vec<[i16; 2]>) {
        let ram = std::mem::take(&mut self.ram);
        for _ in 0..n {
            let mut dry = [[0i32; 2]; 2];
            let mut wet = [[0i32; 2]; 2];
            for (i, v) in self.voices.iter_mut().enumerate() {
                if v.phase == Phase::Off {
                    continue;
                }
                let s = v.sample(&ram, &self.gauss);
                let l = s * i32::from(v.vol_l) >> 15;
                let r = s * i32::from(v.vol_r) >> 15;
                let c = i / CORE_VOICES;
                if v.dry_l {
                    dry[c][0] += l;
                }
                if v.dry_r {
                    dry[c][1] += r;
                }
                if v.wet_l {
                    wet[c][0] += l;
                }
                if v.wet_r {
                    wet[c][1] += r;
                }
            }
            let mut mix = [0i32; 2];
            for c in 0..2 {
                let core = &mut self.cores[c];
                let (rl, rr) = core.reverb.process(wet[c][0].clamp(-0x8000, 0x7fff), wet[c][1].clamp(-0x8000, 0x7fff), &self.halfband);
                mix[0] += dry[c][0] + (rl * i32::from(core.evol_l) >> 15);
                mix[1] += dry[c][1] + (rr * i32::from(core.evol_r) >> 15);
            }
            out.push([
                (mix[0] as f32 * self.gain).round().clamp(-32768.0, 32767.0) as i16,
                (mix[1] as f32 * self.gain).round().clamp(-32768.0, 32767.0) as i16,
            ]);
        }
        self.ram = ram;
    }

    /// Whether any voice still sounds (a voice whose envelope reached 0
    /// in its sustain stays keyed on but is silent).
    pub fn busy(&self) -> bool {
        self.voices.iter().any(|v| v.phase != Phase::Off && v.level > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauss_sums_to_one() {
        let t = gauss_table();
        for i in 0..256 {
            let s = t[0xff - i] + t[0x1ff - i] + t[0x100 + i] + t[i];
            assert!((s - 32768).abs() <= 2, "{i}: {s}");
        }
    }

    #[test]
    fn adpcm_block_and_end() {
        // One block: filter 0, shift 12 -> nibble values; flags 1 (end, no repeat).
        let mut ram = vec![0u8; 32];
        ram[0] = 0x0c;
        ram[1] = 1;
        for b in &mut ram[2..16] {
            *b = 0x11;
        }
        let mut spu = Spu2::new(ram);
        spu.set_pitch(0, 0x1000);
        spu.set_adsr(0, 0x000f, 0x1fc0);
        spu.set_volume(0, 0x3fff, 0x3fff);
        spu.key_on(1);
        let mut out = Vec::new();
        spu.render(64, &mut out);
        assert!(!spu.busy(), "voice stops after an end block without repeat");
    }
}
