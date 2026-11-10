//! `audio.music = "ps2"` (mod `ps2-soundtrack`): every track of the PC's "Original"
//! soundtrack bank (`sound_pc/music_ps2/ffx_music_ps2_bank00.fsb`) rendered
//! again from the PS2 disc's sequences and instruments by our driver
//! (`audio/sqseq.rs`) and SPU2 (`audio/spu2.rs`), and written as a replacement bank
//! with the same names, order and format (`formats/fsb5.rs`). `docs/music.md`.

use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::config::Config;
use crate::disc::Disc;
use crate::jobs::{Ctx, Job, Output};
use crate::formats::fsb5;
use crate::audio::sqseq::{self, Bgm, RenderPlan, Tables, Wd};

pub const BANK: &str = "ffx_data/gamedata/ps3data/sound_pc/music_ps2/ffx_music_ps2_bank00.fsb";

/// The PC bank's samples by the PS2 sequence that plays them: the game's
/// music id (the `musicNNN.bgm` number) for each event of
/// `ffx_music_ps2.fev`, from FFX.exe's id table (`0x00b4f1c8`, sound.md).
/// Where two ids share an event, the one listed is the one whose render
/// matches the PC's recording (`docs/music.md`).
pub const TRACKS: &[(&str, u16)] = &[
    ("1_02_zanarkand", 130),
    ("1_03_prelude", 48),
    ("1_04_tidus_s_theme", 161),
    ("1_05_otherworld", 49),
    ("1_06_run", 165),
    ("1_08_creep", 44),
    ("1_09_battle_theme", 16),
    ("1_10_victory_fanfare", 138),
    ("1_11_game_over_oneshot", 140),
    ("1_12_out_of_the_frying_pan", 18),
    ("1_13_leap_in_the_dark", 11),
    ("1_14_underwater_ruins", 31),
    ("1_15_oui_are_al_bhed", 163),
    ("1_16_enemy_attack", 12),
    ("1_17_the_blitzers", 24),
    ("1_18_besaid", 39),
    ("1_19_spira_unplugged", 22),
    ("1_20_hum_of_the_fayth", 160),
    ("1_21_phantoms", 17),
    ("1_22_the_trials", 23),
    ("1_23_hymn_valefor", 156),
    ("1_24_the_summoning", 13),
    ("1_25_braska_s_daughter", 32),
    ("1_26_good_night_oneshot", 143),
    ("2_01_yuna_s_theme", 136),
    ("2_02_sprouting", 128),
    ("2_04_silence_before_the_storm", 139),
    ("2_05_song_of_prayer_ifrit", 154),
    ("2_06_luca", 147),
    ("2_08_unwavering_decision", 10),
    ("2_09_the_splendid_performance", 129),
    ("2_10_confrontation", 35),
    ("2_11_blitz_off", 27),
    ("2_12_auron_s_theme", 146),
    ("2_13_mi_ihen_road", 19),
    ("2_14_brass_de_chocobo", 38),
    ("2_15_travel_company", 133),
    ("2_16_permitted_passage_oneshot", 33),
    ("2_17_seymour_s_theme", 34),
    ("2_18_twilight", 45),
    ("2_19_djose_temple", 132),
    ("2_20_song_of_prayer_ixion", 152),
    ("2_21_wanna_ride_the_ciparph", 20),
    ("2_22_rikku_s_theme", 30),
    ("2_23_guadosalam", 36),
    ("3_01_thunder_plain", 29),
    ("3_02_ject_s_theme", 51),
    ("3_03_macalania_forest", 14),
    ("3_04_sea_of_mists", 170),
    ("3_06_seymour_s_ambition", 26),
    ("3_07_song_of_prayer_shiva", 155),
    ("3_09_blazing_desert", 40),
    ("3_10_crisis", 28),
    ("3_11_the_truth_revealed", 41),
    ("3_12_start", 131),
    ("3_13_marriage_ceremony_oneshot", 167),
    ("3_14_raid", 166),
    ("3_15_tragedy_oneshot", 168),
    ("3_16_i_can_fly_oneshot", 169),
    ("3_17_path_of_repentance", 135),
    ("3_18_song_of_prayer_bahamut", 150),
    ("3_19_time_of_judgment", 174),
    ("3_20_my_father_s_murderer", 137),
    ("4_01_yuna_s_decision", 134),
    ("4_02_lulu_s_theme", 171),
    ("4_03_brave_advancement", 21),
    ("4_04_song_of_prayer_bodyguard", 153),
    ("4_05_people_of_the_north_pole", 37),
    ("4_06_song_of_prayer_the_lonzo", 158),
    ("4_07_wandering_flame", 15),
    ("4_08_someday_the_dream_will_end", 141),
    ("4_09_song_of_prayer_yunalesca", 159),
    ("4_10_challenge", 145),
    ("4_11_at_the_end_of_the_abyss", 162),
    ("4_12_darkness", 172),
    ("4_13_song_of_prayer_spira", 157),
    ("4_14_the_deceased_laugh", 25),
    ("4_15_seymour_battle", 42),
    ("4_16_song_of_prayer_anima", 151),
    ("4_17_summoned_beasts_battle", 176),
    ("4_18_decisive_battle", 50),
    ("m_01_inori_effect", 178),
    ("m_05_the_temple_band", 177),
    ("m_07_reception_for_great_sage_micah", 164),
    ("m_08_the_advancers", 43),
    ("m_56_illusion", 142),
    ("m_58_nostalgia", 144),
    ("m_69_hum_of_the_fayth_oneshot", 148),
    ("m_71_wakka_s_theme", 46),
];

/// The PS2 files the renderer reads.
pub struct Sources {
    pub tables: Tables,
    sec: Vec<u8>,
    sound_dir: String,
}

fn find_name(disc: &Disc, suffix: &str) -> Option<String> {
    let mut names: Vec<&str> = disc.names().filter(|n| n.ends_with(suffix)).collect();
    names.sort_unstable();
    names.first().map(|s| s.to_string())
}

impl Sources {
    pub fn load(disc: &mut Disc) -> Result<Self> {
        let iop = disc.read("jppc/irx/iopsound.irx")?.context("iopsound.irx not on the disc")?;
        let libsd = disc.read("jppc/irx/libsd.irx")?.context("libsd.irx not on the disc")?;
        let tables = Tables::from_irx(&iop, &libsd)?;
        let sec_name = find_name(disc, "/wave/wave_wd.sec").context("wave_wd.sec not on the disc")?;
        let sound_dir = sec_name.trim_end_matches("wave/wave_wd.sec").to_string();
        let sec = disc.read(&sec_name)?.context("wave_wd.sec")?;
        Ok(Sources { tables, sec, sound_dir })
    }

    /// The bytes of `musicNNN.bgm`.
    pub fn sequence(&self, disc: &mut Disc, id: u16) -> Result<Vec<u8>> {
        let name = format!("{}music/data/music{id:03}.bgm", self.sound_dir);
        disc.read(&name)?.with_context(|| format!("{name} not on the disc"))
    }

    pub fn song(&self, disc: &mut Disc, id: u16) -> Result<(Bgm, Wd)> {
        let name = format!("{}music/data/music{id:03}.bgm", self.sound_dir);
        let b = self.sequence(disc, id)?;
        let bgm = Bgm::parse(&b).with_context(|| name.clone())?;
        let wd = self.bank(disc, bgm.wd).with_context(|| format!("music {id}"))?;
        Ok((bgm, wd))
    }

    /// Instrument bank `id` of `wave_wd.dat`.
    pub fn bank(&self, disc: &mut Disc, id: u16) -> Result<Wd> {
        let b = self.bank_bytes(disc, id)?;
        Wd::parse(&b).with_context(|| format!("WD {id}"))
    }

    /// The bytes of instrument bank `id` (the driver's `waveNNNN.wd`).
    pub fn bank_bytes(&self, disc: &mut Disc, id: u16) -> Result<Vec<u8>> {
        let dat = format!("{}wave/wave_wd.dat", self.sound_dir);
        let at = Wd::locate(&self.sec, id).with_context(|| format!("no WD {id}"))?;
        let head = disc.read_range(&dat, at, 0x800)?.context("wave_wd.dat")?;
        let size = Wd::size(&head).context("WD header")?;
        disc.read_range(&dat, at, size)?.context("wave_wd.dat")
    }

    /// The banks `wave_wd.sec` lists.
    pub fn bank_ids(&self) -> Vec<u16> {
        (0..self.sec.len() / 4)
            .filter(|&i| Wd::locate(&self.sec, i as u16).is_some())
            .map(|i| i as u16)
            .collect()
    }

    /// The music ids of the disc's `musicNNN.bgm` files, ascending.
    pub fn music_ids(&self, disc: &Disc) -> Vec<u16> {
        let dir = format!("{}music/data/music", self.sound_dir);
        let mut ids: Vec<u16> = disc
            .names()
            .filter_map(|n| n.strip_prefix(&dir)?.strip_suffix(".bgm")?.parse().ok())
            .collect();
        ids.sort_unstable();
        ids
    }
}

/// A 16-bit stereo WAV written as it goes.
pub struct WavWriter {
    f: std::io::BufWriter<std::fs::File>,
    frames: u32,
    rate: u32,
}

impl WavWriter {
    pub fn create(path: &Path, rate: u32) -> Result<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::io::BufWriter::new(std::fs::File::create(path).with_context(|| path.display().to_string())?);
        f.write_all(&[0; 44])?;
        Ok(WavWriter { f, frames: 0, rate })
    }
    pub fn write(&mut self, s: &[[i16; 2]]) -> Result<()> {
        for x in s {
            self.f.write_all(&x[0].to_le_bytes())?;
            self.f.write_all(&x[1].to_le_bytes())?;
        }
        self.frames += s.len() as u32;
        Ok(())
    }
    pub fn finish(mut self) -> Result<()> {
        let data = self.frames * 4;
        let mut h = Vec::with_capacity(44);
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&(36 + data).to_le_bytes());
        h.extend_from_slice(b"WAVEfmt ");
        h.extend_from_slice(&16u32.to_le_bytes());
        h.extend_from_slice(&1u16.to_le_bytes());
        h.extend_from_slice(&2u16.to_le_bytes());
        h.extend_from_slice(&self.rate.to_le_bytes());
        h.extend_from_slice(&(self.rate * 4).to_le_bytes());
        h.extend_from_slice(&4u16.to_le_bytes());
        h.extend_from_slice(&16u16.to_le_bytes());
        h.extend_from_slice(b"data");
        h.extend_from_slice(&data.to_le_bytes());
        self.f.seek(SeekFrom::Start(0))?;
        self.f.write_all(&h)?;
        self.f.flush()?;
        Ok(())
    }
}

/// The output stage: the gain ([`GAIN_DB`]) and a look-ahead peak limiter
/// at -1 dBFS for the rare peaks the gain pushes over (0.06% of the
/// samples on average). Its delay is compensated: frame n in is frame n out.
#[derive(Clone)]
pub struct Master {
    gain: f32,
    thr: f32,
    rel: f32,
    g: f32,
    delay: std::collections::VecDeque<([f32; 2], f32)>,
}

/// Look-ahead in frames (2 ms).
const LOOKAHEAD: usize = 96;

impl Default for Master {
    fn default() -> Self {
        Self::new()
    }
}

impl Master {
    pub fn new() -> Self {
        Master {
            gain: gain(),
            thr: 32767.0 * 0.891_250_9, // -1 dBFS
            rel: 1.0 / (0.08 * 48000.0),
            g: 1.0,
            delay: std::collections::VecDeque::with_capacity(LOOKAHEAD + 1),
        }
    }

    /// One frame at the chip's level (the gain comes here).
    pub fn frame(&mut self, x: [f32; 2], out: &mut Vec<[i16; 2]>) {
        let x = [x[0] * self.gain, x[1] * self.gain];
        let peak = x[0].abs().max(x[1].abs());
        let target = if peak > self.thr { self.thr / peak } else { 1.0 };
        self.delay.push_back((x, target));
        // the smallest gain needed within the look-ahead, approached
        // linearly so it is reached by the time that frame comes out
        let need = self.delay.iter().map(|d| d.1).fold(1.0f32, f32::min);
        if need < self.g {
            self.g -= (self.g - need) / LOOKAHEAD as f32 * 4.0;
            self.g = self.g.max(need);
        } else {
            self.g = (self.g + self.rel).min(need).min(1.0);
        }
        if self.delay.len() > LOOKAHEAD {
            let (y, t) = self.delay.pop_front().expect("delay");
            let g = self.g.min(t);
            out.push(y.map(|v| (v * g).round().clamp(-32768.0, 32767.0) as i16));
        }
    }

    pub fn process(&mut self, input: &[[i16; 2]], out: &mut Vec<[i16; 2]>) {
        for f in input {
            self.frame([f32::from(f[0]), f32::from(f[1])], out);
        }
    }

    /// The last frames still in the look-ahead (the queue's output is the
    /// input delayed, so with these the output has the input's length).
    pub fn finish(&mut self, out: &mut Vec<[i16; 2]>) {
        for _ in 0..LOOKAHEAD {
            self.frame([0.0, 0.0], out);
        }
    }
}

/// How a track is rendered: looping songs play the loop twice and the
/// bank loops over the second pass (its start carries the first pass's
/// reverb and release tails, as the PS2 does when it jumps back).
pub fn plan(bgm: &Bgm) -> RenderPlan {
    RenderPlan { loops: if bgm.loops() { 2 } else { 0 }, max_seconds: 900.0, tail_seconds: 8.0 }
}

/// `demaster music <iso> <vbf> <out dir> all|<id|name>...`
pub fn debug(iso: &Path, vbf: &str, out: &Path, specs: &[String]) -> Result<()> {
    let pc = ffx_vbf::Archive::open(vbf)?;
    let mut disc = Disc::open(iso, &pc)?;
    let src = Sources::load(&mut disc)?;
    let mut list: Vec<(String, u16)> = Vec::new();
    for s in specs {
        if s == "all" {
            list.extend(TRACKS.iter().map(|(n, i)| (n.to_string(), *i)));
        } else if let Ok(id) = s.parse::<u16>() {
            let name = TRACKS.iter().find(|t| t.1 == id).map_or(format!("music{id:03}"), |t| t.0.to_string());
            list.push((name, id));
        } else if let Some(t) = TRACKS.iter().find(|t| t.0 == s) {
            list.push((t.0.to_string(), t.1));
        } else {
            bail!("unknown track {s}");
        }
    }
    for (name, id) in list {
        let t0 = std::time::Instant::now();
        let (bgm, wd) = src.song(&mut disc, id)?;
        let path: PathBuf = out.join(format!("{name}.wav"));
        let mut w = WavWriter::create(&path, crate::audio::spu2::RATE)?;
        let raw = crate::audio::spu2::knob("DEMASTER_RAW") != 0.0;
        let mut master = Master::new();
        let mut buf = Vec::new();
        let marks = sqseq::render(&src.tables, &bgm, &wd, plan(&bgm), |s| {
            if raw {
                return w.write(s);
            }
            buf.clear();
            master.process(s, &mut buf);
            w.write(&buf)
        })?;
        if !raw {
            buf.clear();
            master.finish(&mut buf);
            w.write(&buf)?;
        }
        let frames = w.frames;
        w.finish()?;
        println!(
            "{name}\tmusic{id:03}\twd {}\tframes {frames}\tloop {:?}\tjumps {:?}\tend {:?}\tdropped {}\tunsupported {:?}\t{:.1}s",
            bgm.wd,
            marks.loop_point,
            marks.loop_jumps,
            marks.end,
            marks.dropped,
            marks.unsupported,
            t0.elapsed().as_secs_f64()
        );
    }
    Ok(())
}

/// `demaster music-bank <bank.fsb> <out dir> [name...]`: the samples of a
/// written bank decoded to WAVs, with their loop points.
pub fn bank_debug(bank: &Path, out: &Path, names: &[String]) -> Result<()> {
    let b = std::fs::read(bank).with_context(|| bank.display().to_string())?;
    let head = fsb5::Head::parse(&b)?;
    let headers = u32::from_le_bytes(b[12..16].try_into().expect("4")) as usize;
    let nsize = u32::from_le_bytes(b[16..20].try_into().expect("4")) as usize;
    let data_at = 0x3c + headers + nsize;
    let offsets = fsb5::offsets(&b)?;
    for (i, s) in head.samples.iter().enumerate() {
        if !names.is_empty() && !names.contains(&s.name) {
            continue;
        }
        let start = data_at + offsets[i] as usize;
        let end = offsets.get(i + 1).map_or(b.len(), |&o| data_at + o as usize);
        let pcm = fsb5::decode_stereo(&b[start..end]);
        let pcm = &pcm[..(s.frames as usize).min(pcm.len())];
        let mut w = WavWriter::create(&out.join(format!("{}.wav", s.name)), s.rate)?;
        w.write(pcm)?;
        w.finish()?;
        println!("{}\t{} Hz\t{} frames\tloop {:?}", s.name, s.rate, s.frames, s.looped);
    }
    Ok(())
}

/// The output gain: the PC's recordings (and so the game's mix around
/// them) sit this far above the chip's own level (`docs/music.md`).
pub const GAIN_DB: f32 = 9.0;

pub fn gain() -> f32 {
    10f32.powf(GAIN_DB / 20.0)
}

struct MusicJob;

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(MusicJob)]
}

impl Job for MusicJob {
    fn name(&self) -> &'static str {
        "audio.music"
    }
    fn settings(&self) -> &'static [&'static str] {
        &["audio.music"]
    }
    fn enabled(&self, config: &Config) -> bool {
        config.audio.music.ps2()
    }
    /// 2: long samples in pieces play through (music143, music149).
    fn version(&self) -> u32 {
        2
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        build_bank(ctx, out)
    }
}

/// Bytes `start..start + len` of a file in the archive, reading only the
/// blocks that hold them (the PS2 music bank is 672 MB).
pub fn read_range(vbf: &Path, pc: &ffx_vbf::Archive, path: &str, start: u64, len: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let entry = pc.find(path).with_context(|| format!("{path} not in the archive"))?;
    let mut f = std::fs::File::open(vbf)?;
    let mut out = Vec::with_capacity(len as usize);
    let mut at = 0u64;
    let end = start + len;
    for block in pc.blocks(entry)? {
        let b0 = at;
        at += block.size as u64;
        if at <= start {
            continue;
        }
        if b0 >= end {
            break;
        }
        let mut stored = vec![0u8; block.stored];
        f.seek(SeekFrom::Start(block.offset))?;
        f.read_exact(&mut stored)?;
        let bytes = if block.is_compressed() {
            let mut d = Vec::with_capacity(block.size);
            flate2::read::ZlibDecoder::new(&stored[..]).read_to_end(&mut d)?;
            d
        } else {
            stored
        };
        let lo = start.saturating_sub(b0) as usize;
        let hi = (end - b0).min(bytes.len() as u64) as usize;
        out.extend_from_slice(&bytes[lo..hi]);
    }
    ensure!(out.len() as u64 == len, "{path}: short read");
    Ok(out)
}

/// The head of a bank in the archive.
pub fn read_head(vbf: &Path, pc: &ffx_vbf::Archive, path: &str) -> Result<fsb5::Head> {
    let first = read_range(vbf, pc, path, 0, 0x3c)?;
    let len = fsb5::Head::len(&first)? as u64;
    fsb5::Head::parse(&read_range(vbf, pc, path, 0, len)?)
}

/// Tracks the bank keeps as the PC recorded them: on the PS2 their guitar
/// and vocal parts were streamed from the disc alongside the sequence
/// (`BattleStreamingStart` in the driver), which this renderer does not
/// play; the sequence alone lacks them (`docs/music.md`).
pub const KEEP_PC: &[&str] = &["1_05_otherworld", "2_10_confrontation", "3_02_ject_s_theme"];

/// One track's IMA data in `part`.
struct Part {
    part: PathBuf,
    rate: u32,
    frames: u32,
    looped: Option<(u32, u32)>,
    bytes: u64,
}

/// A track copied from the PC bank.
fn pc_part(vbf: &Path, ctx: &Ctx, head: &fsb5::Head, s: &fsb5::SampleInfo, part: &Path) -> Result<Part> {
    let b = read_range(vbf, ctx.pc, BANK, head.data_at + s.offset, s.size)?;
    std::fs::write(part, &b)?;
    Ok(Part { part: part.to_path_buf(), rate: s.rate, frames: s.frames, looped: s.looped, bytes: b.len() as u64 })
}

/// Renders one track into a file of IMA blocks.
fn render_part(src: &Sources, ctx: &Ctx, id: u16, part: &Path) -> Result<Part> {
    let (bgm, wd) = ctx.disc(|d| src.song(d, id))?;
    let f = std::fs::File::create(part).with_context(|| part.display().to_string())?;
    let mut w = std::io::BufWriter::new(f);
    let mut enc = fsb5::StereoEncoder::new();
    let mut master = Master::new();
    let mut buf = Vec::new();
    let marks = sqseq::render(&src.tables, &bgm, &wd, plan(&bgm), |s| {
        buf.clear();
        master.process(s, &mut buf);
        enc.push(&mut w, &buf)
    })?;
    buf.clear();
    master.finish(&mut buf);
    enc.push(&mut w, &buf)?;
    let real = enc.frames;
    let frames = enc.finish(&mut w)?;
    w.flush()?;
    let looped = match (marks.loop_jumps.first(), marks.loop_jumps.get(1)) {
        (Some(&a), Some(&b)) => (a as u32, (b.min(real) - 1) as u32),
        _ => (0, real.saturating_sub(1) as u32),
    };
    Ok(Part {
        part: part.to_path_buf(),
        rate: crate::audio::spu2::RATE,
        frames: u32::try_from(frames)?,
        looped: Some(looped),
        bytes: std::fs::metadata(part)?.len(),
    })
}

fn build_bank(ctx: &Ctx, out: &Output) -> Result<String> {
    use rayon::prelude::*;
    let vbf = ctx.vbf.as_ref().context("no FFX_Data.vbf path")?;
    let head = read_head(vbf, ctx.pc, BANK)?;
    for s in &head.samples {
        if !TRACKS.iter().any(|t| t.0 == s.name) {
            bail!("the PC bank has a track this converter does not know: {}", s.name);
        }
    }
    let src = ctx.disc(Sources::load)?;
    let tmp = out.temp_path(BANK)?;
    let parts_dir = tmp.with_extension("parts");
    std::fs::create_dir_all(&parts_dir)?;
    // four renders at a time, each streaming into its own file
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build()?;
    let parts: Vec<Result<Part>> = pool.install(|| {
        head.samples
            .par_iter()
            .enumerate()
            .map(|(i, s)| {
                let id = TRACKS.iter().find(|t| t.0 == s.name).expect("checked").1;
                let part = parts_dir.join(format!("{i:02}.ima"));
                let r = if KEEP_PC.contains(&s.name.as_str()) {
                    pc_part(vbf, ctx, &head, s, &part).with_context(|| format!("{} (PC)", s.name))
                } else {
                    render_part(&src, ctx, id, &part).with_context(|| format!("{} (music{id:03})", s.name))
                };
                if let Ok(p) = &r {
                    ctx.note(format!(
                        "{}: {:.1} s, loop {:?}",
                        s.name,
                        f64::from(p.frames) / f64::from(p.rate),
                        p.looped
                    ));
                }
                r
            })
            .collect()
    });
    // the data: every part at a 32-byte boundary, in the bank's order
    let data_path = parts_dir.join("data");
    let mut data = std::io::BufWriter::new(std::fs::File::create(&data_path)?);
    let mut written = Vec::new();
    let mut at = 0u64;
    for (s, p) in head.samples.iter().zip(parts) {
        let p = p?;
        let mut f = std::fs::File::open(&p.part)?;
        std::io::copy(&mut f, &mut data)?;
        written.push(fsb5::Written { name: s.name.clone(), rate: p.rate, frames: p.frames, looped: p.looped, offset: at });
        at += p.bytes;
        let pad = (32 - at % 32) % 32;
        data.write_all(&vec![0u8; pad as usize])?;
        at += pad;
        std::fs::remove_file(&p.part)?;
    }
    data.flush()?;
    drop(data);
    {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        fsb5::write_bank(&mut f, &head.extra, &written, &data_path)?;
        f.flush()?;
    }
    std::fs::remove_dir_all(&parts_dir)?;
    let size = std::fs::metadata(&tmp)?.len();
    out.put_file(BANK, &tmp)?;
    let seconds: f64 = written.iter().map(|w| f64::from(w.frames) / f64::from(w.rate)).sum();
    Ok(format!(
        "{} tracks rendered, {} kept from the PC ({:.0} min, {} MB)",
        written.len() - KEEP_PC.len(),
        KEEP_PC.len(),
        seconds / 60.0,
        size >> 20
    ))
}
