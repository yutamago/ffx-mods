//! The live engine: PS2 songs (`sqseq` on `spu2`) started, stopped,
//! paused and given a volume by the game's music events, mixed into one
//! 48 kHz stereo stream. Nothing here knows about Windows or FMOD
//! (`hook.rs` does): an event is a `u32` key (the game's FMOD `Event*`).
//!
//! Every event that plays gets its own song and its own SPU2 (the PC plays
//! a field song paused under the battle music and resumes it; on the PS2
//! the driver kept the field song too). The mix is the converter's output
//! stage (`music::Master`: +9 dB, limiter at -1 dBFS) over the sum of
//! the songs, each times its event's volume (the game's: fades, the music
//! volume setting) and the PC event's own volume (`EVENT_DB`), so a song
//! started and left alone is the offline bank's sample for sample.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use demaster::jobs::music::Master;
use demaster::audio::spu2::Spu2;
use demaster::audio::sqseq::{self, Bgm, Song, Tables, Wd};

/// FMOD event states as the game tests them (`audio-platform.md`).
pub const STATE_READY: u32 = 1;
/// PLAYING | CHANNELSACTIVE, what FMOD answered for a playing music event.
pub const STATE_PLAYING: u32 = 0x18;

/// Frames a volume change takes (one FMOD tick at 48 kHz): the game
/// moves a fade in 30 Hz steps; ramping each step avoids zipper noise.
pub const RAMP: u32 = 1600;

/// Tracks the live engine leaves to the bank: their guitar and vocals were
/// streamed from the disc next to the sequence (`docs/music.md`), so the
/// PC's recording (kept in the bank) is the whole song. Otherworld,
/// Confrontation, Jecht's theme.
pub const KEEP_BANK: &[u16] = &[49, 35, 51];

/// The volume (dB) of each music id's event in `ffx_music_ps2.fev`
/// (through FFX.exe's id table `0x00b4f1c8`): FMOD multiplies it into
/// everything the event plays. Read from the PC's data with ffx-rust's
/// `fev` reader (every value is a whole dB step).
pub const EVENT_DB: &[(u16, i8)] = &[
    (10, -11), (11, -11), (12, -9), (13, -9), (14, -13), (15, -13), (16, -13), (17, -12),
    (18, -10), (19, -11), (20, -9), (21, -11), (22, -10), (23, -10), (24, -13), (25, -10),
    (26, -12), (27, -9), (28, -13), (29, -10), (30, -8), (31, -3), (32, -13), (33, -13),
    (34, -8), (35, -11), (36, -9), (37, -11), (38, -10), (39, -11), (40, -10), (41, -12),
    (42, -7), (43, -9), (44, -5), (45, -11), (46, -3), (48, -12), (49, -9), (50, -8),
    (51, -13), (128, -10), (129, -9), (130, -8), (131, -10), (132, -9), (133, -4), (134, -8),
    (135, -4), (136, -8), (137, -14), (138, -14), (139, -8), (140, -6), (141, -10), (142, -3),
    (143, -5), (144, -3), (145, -11), (146, -8), (147, -12), (148, -3), (149, -13), (150, -14),
    (151, -11), (152, -9), (153, -10), (154, -13), (155, -11), (156, -10), (157, -11), (158, -9),
    (159, -13), (160, -13), (161, -10), (162, -4), (163, -6), (164, -10), (165, -10), (166, -7),
    (167, -4), (168, -8), (169, -3), (170, -11), (171, -7), (172, -7), (174, -12), (176, -8),
    (177, -7), (178, -10), (179, -11), (181, -7),
];

/// The event volume of music `id` as a gain (1 for ids without an event).
pub fn event_gain(id: u16) -> f32 {
    EVENT_DB
        .iter()
        .find(|e| e.0 == id)
        .map_or(1.0, |e| 10f32.powf(f32::from(e.1) / 20.0))
}

/// A sequence, its bank and the bank's samples as sound RAM.
pub type Loaded = (Arc<Bgm>, Arc<Wd>, Arc<Vec<u8>>);

/// The music files the converter extracted (`<mod>/music/`, job
/// `audio.live`): `iopsound.irx`, `libsd.irx`, `musicNNN.bgm`,
/// `waveNNNN.wd`. Loaded on first use and kept.
pub struct Library {
    dir: PathBuf,
    tables: Arc<Tables>,
    songs: BTreeMap<u16, Option<Loaded>>,
}

impl Library {
    pub fn open(dir: &Path) -> Result<Self> {
        let rd = |n: &str| std::fs::read(dir.join(n)).with_context(|| dir.join(n).display().to_string());
        let tables = Tables::from_irx(&rd("iopsound.irx")?, &rd("libsd.irx")?)?;
        Ok(Library { dir: dir.to_path_buf(), tables: Arc::new(tables), songs: BTreeMap::new() })
    }

    /// From tables and songs in memory (tests).
    pub fn from_parts(tables: Tables, songs: Vec<(u16, Bgm, Wd)>) -> Self {
        let songs = songs
            .into_iter()
            .map(|(id, b, w)| {
                let ram = Arc::new(w.samples.clone());
                (id, Some((Arc::new(b), Arc::new(w), ram)))
            })
            .collect();
        Library { dir: PathBuf::new(), tables: Arc::new(tables), songs }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Music `id` with its bank, or `None` when its files are missing.
    pub fn song(&mut self, id: u16) -> Option<Loaded> {
        if !self.songs.contains_key(&id) {
            let load = || -> Result<_> {
                let b = Bgm::parse(&std::fs::read(self.dir.join(format!("music{id:03}.bgm")))?)?;
                let w = Wd::parse(&std::fs::read(self.dir.join(format!("wave{:04}.wd", b.wd)))?)?;
                let ram = Arc::new(w.samples.clone());
                Ok((Arc::new(b), Arc::new(w), ram))
            };
            let v = load().ok();
            self.songs.insert(id, v);
        }
        self.songs[&id].clone()
    }

    pub fn has(&mut self, id: u16) -> bool {
        self.song(id).is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The sequence plays.
    Play,
    /// The sequence has ended (a one-shot) or was stopped: the chip plays
    /// its release and reverb tails out, then the slot goes.
    Tail { quiet: u32, stopped: bool },
}

struct Slot {
    event: u32,
    id: u16,
    song: Song,
    spu: Spu2,
    phase: Phase,
    paused: bool,
    /// Output of the last interrupt not yet mixed.
    buf: Vec<[i16; 2]>,
    at: usize,
    /// Event volume now, its target and step per frame.
    vol: f32,
    target: f32,
    step: f32,
    gain: f32,
    /// Seconds of tail after the song before the event counts as finished.
    tail_max: u32,
}

/// What the game set on an event, kept while no song plays for it.
#[derive(Debug, Clone, Copy)]
struct EventInfo {
    volume: f32,
    callback: Option<(u32, u32)>,
    paused: bool,
}

impl Default for EventInfo {
    fn default() -> Self {
        EventInfo { volume: 1.0, callback: None, paused: false }
    }
}

/// An event whose song ended by itself: the game's callback is due
/// (FMOD's `EVENTFINISHED`, on the FMOD thread).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finished {
    pub event: u32,
    pub callback: u32,
    pub userdata: u32,
}

pub struct Engine {
    pub lib: Library,
    slots: Vec<Slot>,
    events: BTreeMap<u32, EventInfo>,
    finished: Vec<Finished>,
    master: Master,
    mix: Vec<[f32; 2]>,
    out: Vec<[i16; 2]>,
    /// Output frames so far.
    pub frames: u64,
    /// Apply the PC events' own volumes ([`EVENT_DB`]); off for comparing
    /// with the converter's renders.
    pub event_volumes: bool,
    /// The driver's tempo modifier as the game last set it (the driver's,
    /// not a song's: songs started later play with it too).
    tempo: i8,
}

impl Engine {
    pub fn new(lib: Library) -> Self {
        Engine {
            lib,
            slots: Vec::new(),
            events: BTreeMap::new(),
            finished: Vec::new(),
            master: Master::new(),
            mix: Vec::new(),
            out: Vec::new(),
            frames: 0,
            event_volumes: true,
            tempo: 0,
        }
    }

    fn info(&mut self, event: u32) -> &mut EventInfo {
        self.events.entry(event).or_default()
    }

    /// The song playing for `event` (not its tail after a stop).
    fn live(&self, event: u32) -> Option<usize> {
        self.slots.iter().position(|s| s.event == event && !matches!(s.phase, Phase::Tail { stopped: true, .. }))
    }

    /// `Event::start`: starts music `id` for `event` from the beginning
    /// (a song already playing for it is stopped first, as FMOD restarts
    /// an event). False when the music's files are missing.
    pub fn start(&mut self, event: u32, id: u16) -> bool {
        let Some((bgm, wd, ram)) = self.lib.song(id) else {
            return false;
        };
        self.stop(event);
        let mut spu = Spu2::with_ram(ram);
        let mut song = Song::new(Arc::clone(&self.lib.tables), bgm, wd, &mut spu);
        if self.tempo != 0 {
            song.tempo_direct(self.tempo);
        }
        let info = *self.info(event);
        let plan = demaster::jobs::music::plan(&self.lib.song(id).expect("loaded").0);
        self.slots.push(Slot {
            event,
            id,
            song,
            spu,
            phase: Phase::Play,
            paused: info.paused,
            buf: Vec::with_capacity(sqseq::FRAMES_PER_IRQ),
            at: 0,
            vol: info.volume,
            target: info.volume,
            step: 0.0,
            gain: if self.event_volumes { event_gain(id) } else { 1.0 },
            tail_max: (plan.tail_seconds * f64::from(demaster::audio::spu2::RATE)) as u32,
        });
        true
    }

    /// `Event::stop`: the PS2 driver's stop: every voice keyed off, the
    /// release and reverb tails play out. The event is ready again at once.
    pub fn stop(&mut self, event: u32) {
        if let Some(i) = self.live(event) {
            let s = &mut self.slots[i];
            s.song.release_all(&mut s.spu);
            s.phase = Phase::Tail { quiet: 0, stopped: true };
            s.paused = false;
        }
        self.info(event).paused = false;
    }

    pub fn set_paused(&mut self, event: u32, paused: bool) {
        self.info(event).paused = paused;
        if let Some(i) = self.live(event) {
            self.slots[i].paused = paused;
        }
    }

    pub fn paused(&mut self, event: u32) -> bool {
        self.info(event).paused
    }

    pub fn set_volume(&mut self, event: u32, v: f32) {
        let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        self.info(event).volume = v;
        if let Some(i) = self.live(event) {
            let s = &mut self.slots[i];
            s.target = v;
            if s.song.frame() == 0 {
                // nothing played yet (the game sets the volume right
                // after the start): FMOD takes it at once
                s.vol = v;
                s.step = 0.0;
            } else {
                s.step = (v - s.vol) / RAMP as f32;
            }
        }
    }

    pub fn volume(&mut self, event: u32) -> f32 {
        self.info(event).volume
    }

    /// `MusicTempoDirect(v)`: the driver's tempo modifier
    /// (`Song::tempo_direct`) on every song playing and the ones to come.
    pub fn tempo_direct(&mut self, v: i8) {
        self.tempo = v;
        for s in self.slots.iter_mut().filter(|s| s.phase == Phase::Play) {
            s.song.tempo_direct(v);
        }
    }

    /// `MusicTempoTranslate(frames, v)`: the modifier slides to `v` over
    /// `frames` 60 Hz steps.
    pub fn tempo_translate(&mut self, frames: i32, v: i8) {
        self.tempo = v;
        for s in self.slots.iter_mut().filter(|s| s.phase == Phase::Play) {
            s.song.tempo_translate(frames, v);
        }
    }

    pub fn set_callback(&mut self, event: u32, callback: u32, userdata: u32) {
        self.info(event).callback = (callback != 0).then_some((callback, userdata));
    }

    /// `Event::getState`.
    pub fn state(&self, event: u32) -> u32 {
        if self.live(event).is_some() { STATE_PLAYING } else { STATE_READY }
    }

    /// The music id playing for `event`.
    pub fn playing(&self, event: u32) -> Option<u16> {
        self.live(event).map(|i| self.slots[i].id)
    }

    /// Forgets an event (its pointer no longer is a music event): its
    /// song stops at once, without a tail.
    pub fn forget(&mut self, event: u32) {
        self.slots.retain(|s| s.event != event);
        self.events.remove(&event);
    }

    /// Events whose songs ended by themselves since the last call.
    pub fn take_finished(&mut self) -> Vec<Finished> {
        std::mem::take(&mut self.finished)
    }

    /// Songs sounding (tails included).
    pub fn sounding(&self) -> usize {
        self.slots.len()
    }

    /// Renders `n` frames of the mix into `out` (appended).
    pub fn render(&mut self, n: usize, out: &mut Vec<[i16; 2]>) {
        self.mix.clear();
        self.mix.resize(n, [0.0; 2]);
        let mut done = Vec::new();
        for (k, s) in self.slots.iter_mut().enumerate() {
            if s.paused {
                continue;
            }
            for f in self.mix.iter_mut() {
                if s.at == s.buf.len() {
                    s.buf.clear();
                    s.at = 0;
                    match s.phase {
                        Phase::Play => {
                            s.song.interrupt(&mut s.spu, &mut s.buf);
                            if !s.song.playing() {
                                s.phase = Phase::Tail { quiet: 0, stopped: false };
                            }
                        }
                        Phase::Tail { quiet, stopped } => {
                            s.spu.render(sqseq::FRAMES_PER_IRQ, &mut s.buf);
                            let silent = s.buf.iter().all(|x| x[0].unsigned_abs() < 2 && x[1].unsigned_abs() < 2);
                            let quiet = if s.spu.busy() { 0 } else { quiet + s.buf.len() as u32 };
                            if (silent && !s.spu.busy()) || quiet >= s.tail_max {
                                done.push(k);
                                break;
                            }
                            s.phase = Phase::Tail { quiet, stopped };
                        }
                    }
                }
                let x = s.buf[s.at];
                s.at += 1;
                if s.step != 0.0 {
                    s.vol += s.step;
                    if (s.step > 0.0 && s.vol >= s.target) || (s.step < 0.0 && s.vol <= s.target) {
                        s.vol = s.target;
                        s.step = 0.0;
                    }
                }
                let g = s.vol * s.gain;
                f[0] += f32::from(x[0]) * g;
                f[1] += f32::from(x[1]) * g;
            }
        }
        for &k in done.iter().rev() {
            let s = self.slots.remove(k);
            if matches!(s.phase, Phase::Tail { stopped: false, .. }) {
                // a song that ended by itself: FMOD's EVENTFINISHED
                if let Some((callback, userdata)) = self.events.get(&s.event).and_then(|e| e.callback) {
                    self.finished.push(Finished { event: s.event, callback, userdata });
                }
            }
        }
        self.out.clear();
        for f in &self.mix {
            self.master.frame(*f, &mut self.out);
        }
        // the limiter's look-ahead: its first frames come out late; pad
        // with silence so the stream keeps its length
        while self.out.len() < n {
            self.out.insert(0, [0, 0]);
        }
        out.extend_from_slice(&self.out[..n]);
        self.frames += n as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank of one instrument: a square wave looping on one ADPCM block,
    /// instant attack, full sustain, a short release (FFX Music Studio's
    /// test bank).
    fn square_bank() -> Wd {
        let mut b = vec![0u8; 0x20];
        b[..2].copy_from_slice(b"WD");
        b[4..8].copy_from_slice(&16u32.to_le_bytes());
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0x30u32.to_le_bytes());
        b.resize(0x30, 0);
        let mut r = vec![0u8; 0x20];
        r[0..4].copy_from_slice(&0x300u32.to_le_bytes());
        r[12..16].copy_from_slice(&(0x1fc0_000fu32 | 0x0c << 16).to_le_bytes());
        r[0x14] = 127;
        r[0x15] = 127;
        r[0x16] = 127;
        b.extend(r);
        let mut blk = vec![0x00u8, 0x07];
        blk.extend([0x77u8; 7]);
        blk.extend([0x99u8; 7]);
        b.extend(blk);
        Wd::parse(&b).unwrap()
    }

    fn varlen(b: &mut Vec<u8>, v: u32) {
        let mut stack = vec![(v & 0x7f) as u8];
        let mut v = v >> 7;
        while v > 0 {
            stack.push((v & 0x7f) as u8 | 0x80);
            v >>= 7;
        }
        b.extend(stack.iter().rev());
    }

    /// A sequence of `(command, arguments, delta)` per track.
    fn bgm(tracks: &[Vec<(u8, Vec<u8>, u32)>]) -> Bgm {
        let mut b = vec![0u8; 0x20];
        b[..4].copy_from_slice(b"BGM ");
        b[8] = tracks.len() as u8;
        b[9] = 0xff;
        b[12] = 0x7f;
        b[0xe..0x10].copy_from_slice(&48u16.to_le_bytes());
        for evs in tracks {
            let mut body = Vec::new();
            varlen(&mut body, 0);
            for (op, args, d) in evs {
                body.push(*op);
                body.extend_from_slice(args);
                if *op != 0 {
                    varlen(&mut body, *d);
                }
            }
            b.extend_from_slice(&(body.len() as u32).to_le_bytes());
            b.extend_from_slice(&body);
        }
        Bgm::parse(&b).unwrap()
    }

    /// Tempo 120; track 1: key 60 for `len` ticks, a rest, the end (or the
    /// loop back to its start with `looped`).
    fn song(looped: bool, len: u32) -> Bgm {
        let mut t1 = vec![(0x20u8, vec![0u8], 0u32)];
        if looped {
            t1.push((0x02, vec![], 0));
        }
        t1.push((0x11, vec![60, 100], len));
        t1.push((0x1a, vec![60], 48));
        t1.push((0x00, vec![], 0));
        bgm(&[vec![(0x08, vec![120], 0), (0x00, vec![], 0)], t1])
    }

    fn engine(looped: bool) -> Engine {
        Engine::new(Library::from_parts(Tables::generic(), vec![(7, song(looped, 24), square_bank())]))
    }

    fn rms(x: &[[i16; 2]]) -> f64 {
        (x.iter().map(|s| f64::from(s[0]).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    }

    #[test]
    fn missing_music_is_not_started() {
        let mut e = engine(false);
        assert!(!e.start(1, 99));
        assert_eq!(e.state(1), STATE_READY);
    }

    #[test]
    fn start_plays_and_one_shot_finishes_with_callback() {
        let mut e = engine(false);
        e.set_callback(5, 0x1234, 0x5678);
        assert!(e.start(5, 7));
        assert_eq!(e.state(5), STATE_PLAYING);
        let mut out = Vec::new();
        e.render(48000, &mut out);
        assert!(rms(&out) > 10.0, "the note sounds");
        for _ in 0..20 {
            e.render(48000, &mut out);
        }
        assert_eq!(e.state(5), STATE_READY);
        assert_eq!(e.take_finished(), vec![Finished { event: 5, callback: 0x1234, userdata: 0x5678 }]);
    }

    #[test]
    fn stop_releases_and_is_ready_at_once_without_callback() {
        let mut e = engine(true);
        e.set_callback(5, 1, 2);
        e.start(5, 7);
        let mut out = Vec::new();
        e.render(4800, &mut out);
        e.stop(5);
        assert_eq!(e.state(5), STATE_READY);
        for _ in 0..20 {
            e.render(48000, &mut out);
        }
        assert_eq!(e.sounding(), 0, "the tail ended");
        assert!(e.take_finished().is_empty());
    }

    #[test]
    fn looping_song_keeps_playing() {
        let mut e = engine(true);
        e.start(5, 7);
        let mut out = Vec::new();
        for _ in 0..10 {
            out.clear();
            e.render(48000, &mut out);
        }
        assert_eq!(e.state(5), STATE_PLAYING);
        assert!(rms(&out) > 10.0);
    }

    #[test]
    fn pause_freezes_and_resume_goes_on() {
        let mut e = engine(true);
        e.start(5, 7);
        let mut out = Vec::new();
        e.render(4800, &mut out);
        e.set_paused(5, true);
        out.clear();
        e.render(4800, &mut out);
        // the limiter's look-ahead still flushes 96 frames
        assert!(out[200..].iter().all(|s| s == &[0, 0]));
        assert_eq!(e.state(5), STATE_PLAYING);
        e.set_paused(5, false);
        out.clear();
        e.render(4800, &mut out);
        assert!(rms(&out) > 10.0);
    }

    /// Sequence ticks in one second at tempo 120 (96 a second) with the
    /// tempo modifier set, or slid over 60 frames (1 s), once the song's
    /// tempo command has run. (Set before the start, -128 would hold the
    /// song at its first tick for good: it starts at 1 BPM, and 1 x 0x8000
    /// >> 16 is 0, in the driver too.)
    fn ticks_in_a_second(direct: i8, slide: Option<i8>) -> u32 {
        // one note held for 10000 ticks: no loop, no end within a second
        let mut e = engine(false);
        e.lib = Library::from_parts(Tables::generic(), vec![(7, song(false, 10000), square_bank())]);
        e.start(5, 7);
        let mut out = Vec::new();
        e.render(sqseq::FRAMES_PER_IRQ * 4, &mut out);
        let t0 = e.slots[0].song.tick;
        e.tempo_direct(direct);
        if let Some(v) = slide {
            e.tempo_translate(60, v);
        }
        e.render(48000, &mut out);
        e.slots[0].song.tick - t0
    }

    #[test]
    fn tempo_modifier_as_the_driver() {
        assert_eq!(ticks_in_a_second(0, None), 96);
        // -128: BPM x 0x8000 >> 16 = 60; 127: 120 + (120 x 0x8000 >> 15) = 240
        assert_eq!(ticks_in_a_second(-128, None), 48);
        assert_eq!(ticks_in_a_second(127, None), 192);
        // a slide from 0 to -128 over a second: in between
        let s = ticks_in_a_second(0, Some(-128));
        assert!((60..90).contains(&s), "{s}");
        // a later song plays with the modifier: +127 doubles from the start
        let mut e = engine(false);
        e.lib = Library::from_parts(Tables::generic(), vec![(7, song(false, 10000), square_bank())]);
        e.tempo_direct(127);
        e.start(5, 7);
        let mut out = Vec::new();
        e.render(48000, &mut out);
        assert!((190..=194).contains(&e.slots[0].song.tick), "{}", e.slots[0].song.tick);
    }

    #[test]
    fn volume_scales_and_ramps() {
        let mut a = engine(true);
        let mut b = engine(true);
        a.start(5, 7);
        b.set_volume(5, 0.5);
        b.start(5, 7);
        let (mut x, mut y) = (Vec::new(), Vec::new());
        a.render(9600, &mut x);
        b.render(9600, &mut y);
        let r = rms(&y) / rms(&x);
        assert!((r - 0.5).abs() < 0.02, "{r}");
        // a change ramps over RAMP frames
        b.set_volume(5, 0.0);
        y.clear();
        b.render(RAMP as usize + 200, &mut y);
        assert!(y[RAMP as usize + 150..].iter().all(|s| s[0].abs() < 2));
        // a volume set after the start, before any output, holds at once
        let mut c = engine(true);
        c.start(5, 7);
        c.set_volume(5, 0.0);
        let mut z = Vec::new();
        c.render(4800, &mut z);
        assert!(z.iter().all(|s| s == &[0, 0]));
    }

    #[test]
    fn plain_start_is_the_offline_render() {
        // volume 1 and event gain 1 (id 7 has no event): the stream is
        // musicjob's render through the same Master
        let tab = Tables::generic();
        let bgm = song(true, 24);
        let wd = square_bank();
        let mut want = Vec::new();
        let mut m = Master::new();
        let mut raw = Vec::new();
        sqseq::render(&tab, &bgm, &wd, sqseq::RenderPlan { loops: 0, max_seconds: 2.0, tail_seconds: 1.0 }, |s| {
            raw.extend_from_slice(s);
            Ok(())
        })
        .unwrap();
        m.process(&raw, &mut want);
        let mut e = engine(true);
        e.start(5, 7);
        let mut got = Vec::new();
        e.render(raw.len(), &mut got);
        // Master's output is delayed by its look-ahead in both
        assert_eq!(&got[96..96 + want.len()], &want[..]);
    }

    /// With the disc's files (`DEMASTER_LIVE_MUSIC=<mod>/music`, else it
    /// says so and passes): music 16 (battle theme) and 140 (game over)
    /// started and left alone are the converter's renders sample for
    /// sample, through the loop jump.
    #[test]
    fn disc_songs_are_the_offline_renders() {
        let Ok(dir) = std::env::var("DEMASTER_LIVE_MUSIC") else {
            eprintln!("DEMASTER_LIVE_MUSIC not set: skipped");
            return;
        };
        let mut lib = Library::open(Path::new(&dir)).unwrap();
        for (id, seconds) in [(16u16, 130.0), (140, 30.0)] {
            let (bgm, wd, _) = lib.song(id).unwrap();
            let frames = (seconds * 48000.0) as usize;
            let mut raw = Vec::new();
            sqseq::render(&lib.tables, &bgm, &wd, sqseq::RenderPlan { loops: 0, max_seconds: seconds, tail_seconds: 1.0 }, |s| {
                raw.extend_from_slice(s);
                Ok(())
            })
            .unwrap();
            let mut want = Vec::new();
            Master::new().process(&raw, &mut want);
            let mut e = Engine::new(Library::open(Path::new(&dir)).unwrap());
            e.event_volumes = false;
            e.start(1, id);
            let mut got = Vec::new();
            while got.len() < frames {
                e.render(4096, &mut got);
            }
            let n = want.len().min(got.len() - 96);
            assert!(n > frames / 2);
            assert!(got[96..96 + n] == want[..n], "music {id}");
            if id == 16 {
                assert!(e.slots[0].song.marks.loop_jumps.len() == 1, "the battle theme jumped back once");
            }
        }
    }
}
