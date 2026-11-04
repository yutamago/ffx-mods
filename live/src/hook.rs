//! `demaster_live.dll` inside FFX.exe (32-bit): the entry the C# mod calls,
//! the wrappers put into FFX.exe's FMOD import slots, and the user stream
//! that carries the engine's output into FMOD's mixer.
//!
//! **Which calls.** The game's `FmodMusic` (`0x007086b0..0x0070a0a0`)
//! keeps one FMOD event per music id (records of 0x3c bytes at `FmodMusic
//! +0x10`, `+0` the `Event*`) and drives them through eight `Event`
//! methods: `start`, `stop`, `getState`, `setCallback`, `setVolume`,
//! `getVolume`, `setPaused`, `getPaused` (`audio-platform.md`). Every one
//! goes through one import slot, so a wrapper in the slot sees every call
//! (the SFX's and the voices' too). A call whose `Event*` is the event of a
//! music record, while the PS2 soundtrack is chosen (`FmodMusic +0x2c` =
//! 1) and the engine has that music, goes to the engine; everything else
//! goes on to FMOD unchanged. The game's own logic (cross-fades, the
//! replay point that pauses the field music under a battle, fades, the
//! music volume setting, the BGM type switch) stays the game's: the engine
//! only replaces the sound the events would have made.
//!
//! **The end of a song.** FMOD reports an event's end through its callback
//! (`eventcallback` `0x00707a50`, which clears the record's play state):
//! the engine queues it and the `EventSystem::update` wrapper calls it on
//! the FMOD thread, after FMOD's own update, as FMOD would.
//!
//! **Tempo.** FMOD commands 0x1d/0x1e (`iMusicTempoTranslate/Direct`) are
//! empty on PC; their entries in the command handler table go to the
//! engine (the driver's tempo modifier).
//!
//! **The sound.** One FMOD user stream (`System::createSound` with
//! `OPENUSER | CREATESTREAM | LOOP_NORMAL` and a PCM read callback, as the
//! game's movie player makes one) on the game's `System`, 48 kHz stereo
//! 16-bit, started on the first music start and left running. Its read
//! callback renders the engine. The channel's volume stays 1: the game
//! multiplies the music volume setting into each event's volume.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::engine::{Engine, Library};

const IMAGE_BASE: u32 = 0x0040_0000;

/// `g_FmodManager` (`FmodManager*`): `+8` the `FfxFmod`.
const FMOD_MANAGER: u32 = 0x00ce_9010;
/// Music records (`FmodMusic +0x10`), 0x3c bytes each, by music id.
const RECORDS: u32 = 0xb6;

// FFX.exe's import slots (fmod_event.dll, fmodex.dll).
const SLOT_GET_PAUSED: u32 = 0x00b0_c73c;
const SLOT_SET_PAUSED: u32 = 0x00b0_c740;
const SLOT_GET_VOLUME: u32 = 0x00b0_c744;
const SLOT_SET_VOLUME: u32 = 0x00b0_c748;
const SLOT_SET_CALLBACK: u32 = 0x00b0_c74c;
const SLOT_GET_STATE: u32 = 0x00b0_c750;
const SLOT_STOP: u32 = 0x00b0_c754;
const SLOT_START: u32 = 0x00b0_c758;
const SLOT_UPDATE: u32 = 0x00b0_c768;
const SLOT_SET_PRIORITY: u32 = 0x00b0_c77c;
const SLOT_CREATE_SOUND: u32 = 0x00b0_c7a0;
const SLOT_PLAY_SOUND: u32 = 0x00b0_c7a4;

/// Where the DLL says it is installed: `LIVE` over 4 bytes of the DOS stub
/// of FFX.exe's header (never read at run time), for test scripts that
/// wait for it (`tools/inject`).
const MARK: u32 = 0x0040_0070;

/// The user stream's decode buffer in frames (85 ms; the movie's size).
const DECODE_FRAMES: u32 = 0x1000;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> usize;
    fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
}

static BASE: AtomicU32 = AtomicU32::new(0);

/// The original slot values, by wrapper.
static O_START: AtomicU32 = AtomicU32::new(0);
static O_STOP: AtomicU32 = AtomicU32::new(0);
static O_GET_STATE: AtomicU32 = AtomicU32::new(0);
static O_SET_CALLBACK: AtomicU32 = AtomicU32::new(0);
static O_SET_VOLUME: AtomicU32 = AtomicU32::new(0);
static O_GET_VOLUME: AtomicU32 = AtomicU32::new(0);
static O_SET_PAUSED: AtomicU32 = AtomicU32::new(0);
static O_GET_PAUSED: AtomicU32 = AtomicU32::new(0);
static O_UPDATE: AtomicU32 = AtomicU32::new(0);

struct State {
    engine: Engine,
    /// Music id per `Event*` the engine plays or holds settings for.
    owned: BTreeMap<u32, u16>,
    /// Music ids the engine plays (files there, not a kept track).
    handled: BTreeMap<u16, bool>,
    sound: u32,
    channel: u32,
    stream_failed: bool,
    log: Option<std::fs::File>,
    /// The install time: log lines carry the seconds since it and the
    /// stream's output time.
    t0: std::time::Instant,
    wav: Option<Tee>,
    buf: Vec<[i16; 2]>,
    /// The last volume logged per event.
    logged_volume: BTreeMap<u32, f32>,
    /// `EventSystem::update` calls and stream reads since the install (the
    /// heartbeat line every 300 updates, 10 s).
    updates: u64,
    reads: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state() -> MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

impl State {
    fn log(&mut self, line: &str) {
        let (t, s) = (self.t0.elapsed().as_secs_f64(), self.engine.frames as f64 / 48000.0);
        if let Some(f) = &mut self.log {
            let _ = writeln!(f, "[{t:8.3} s, stream {s:8.3} s] {line}");
        }
    }
}

/// A WAV of the engine's output (`DEMASTER_LIVE_WAV`), its header kept
/// right as it grows.
struct Tee {
    f: std::fs::File,
    frames: u32,
}

impl Tee {
    fn create(path: &str) -> Option<Tee> {
        let mut f = std::fs::File::create(path).ok()?;
        f.write_all(&[0; 44]).ok()?;
        Some(Tee { f, frames: 0 })
    }
    fn write(&mut self, s: &[[i16; 2]]) {
        let mut b = Vec::with_capacity(s.len() * 4);
        for x in s {
            b.extend_from_slice(&x[0].to_le_bytes());
            b.extend_from_slice(&x[1].to_le_bytes());
        }
        let _ = self.f.write_all(&b);
        self.frames += s.len() as u32;
        let data = self.frames * 4;
        let mut h = Vec::with_capacity(44);
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&(36 + data).to_le_bytes());
        h.extend_from_slice(b"WAVEfmt ");
        h.extend_from_slice(&16u32.to_le_bytes());
        h.extend_from_slice(&1u16.to_le_bytes());
        h.extend_from_slice(&2u16.to_le_bytes());
        h.extend_from_slice(&48000u32.to_le_bytes());
        h.extend_from_slice(&(48000u32 * 4).to_le_bytes());
        h.extend_from_slice(&4u16.to_le_bytes());
        h.extend_from_slice(&16u16.to_le_bytes());
        h.extend_from_slice(b"data");
        h.extend_from_slice(&data.to_le_bytes());
        let _ = self.f.seek(SeekFrom::Start(0));
        let _ = self.f.write_all(&h);
        let _ = self.f.seek(SeekFrom::End(0));
    }
}

/// An ffx.sym VA in this process.
fn rebase(va: u32) -> u32 {
    BASE.load(Ordering::Relaxed) + (va - IMAGE_BASE)
}

/// A word of the game's memory, 0 for a null address.
fn rd(addr: u32) -> u32 {
    if addr < 0x10000 {
        return 0;
    }
    // SAFETY: addresses of FFX.exe's FMOD objects, read the way the game
    // reads them (the FMOD thread, which these wrappers run on, owns them).
    unsafe { (addr as *const u32).read_unaligned() }
}

/// `FmodMusic*`.
fn fmod_music() -> u32 {
    let mgr = rd(rebase(FMOD_MANAGER));
    let fmod = rd(mgr.wrapping_add(8));
    if fmod == 0 { 0 } else { rd(fmod + 0xc) }
}

/// The music id whose record holds `event`, while the PS2 soundtrack is
/// chosen.
fn music_id(event: u32) -> Option<u16> {
    let m = fmod_music();
    if m == 0 || event == 0 || rd(m + 0x2c) != 1 {
        return None;
    }
    let rec = rd(m + 0x10);
    if rec == 0 {
        return None;
    }
    (0..RECORDS).find(|&i| rd(rec + i * 0x3c) == event).map(|i| i as u16)
}

/// Whether the record of `id` still holds `event` (the PS2 set chosen).
fn still(event: u32, id: u16) -> bool {
    let m = fmod_music();
    m != 0 && rd(m + 0x2c) == 1 && rd(rd(m + 0x10) + u32::from(id) * 0x3c) == event
}

/// The engine's state when `event` is a music event it handles, else
/// `None` (the call goes to FMOD).
fn ours(event: u32) -> Option<(MutexGuard<'static, Option<State>>, u16)> {
    let mut g = state();
    let st = g.as_mut()?;
    if let Some(&id) = st.owned.get(&event) {
        if still(event, id) {
            return Some((g, id));
        }
        // the pointer is no longer that music's event (cleared, freed, or
        // the arranged soundtrack chosen): drop it
        st.owned.remove(&event);
        st.engine.forget(event);
        st.log(&format!("event {event:08x} (music {id}) gone"));
    }
    let id = music_id(event)?;
    let handled = match st.handled.get(&id) {
        Some(&h) => h,
        None => {
            let h = !crate::engine::KEEP_BANK.contains(&id) && st.engine.lib.has(id);
            st.handled.insert(id, h);
            let what = if h { "live" } else { "left to the bank" };
            st.log(&format!("music {id}: {what}"));
            h
        }
    };
    if !handled {
        return None;
    }
    st.owned.insert(event, id);
    Some((g, id))
}

type Fn1 = unsafe extern "system" fn(u32) -> u32;
type Fn2 = unsafe extern "system" fn(u32, u32) -> u32;
type FnF = unsafe extern "system" fn(u32, f32) -> u32;
type Fn3 = unsafe extern "system" fn(u32, u32, u32) -> u32;

fn orig<T: Copy>(slot: &AtomicU32) -> T {
    let p = slot.load(Ordering::Relaxed) as usize;
    // SAFETY: the slot's original value, a function of that shape.
    unsafe { std::mem::transmute_copy::<usize, T>(&p) }
}

unsafe extern "system" fn w_start(ev: u32) -> u32 {
    let Some((mut g, id)) = ours(ev) else {
        return unsafe { orig::<Fn1>(&O_START)(ev) };
    };
    let st = g.as_mut().expect("state");
    let ok = st.engine.start(ev, id);
    let v = st.engine.volume(ev);
    st.log(&format!("start music {id} (event {ev:08x}, volume {v:.3}): {}", if ok { "playing" } else { "FAILED" }));
    let need_stream = st.sound == 0 && !st.stream_failed;
    drop(g);
    if need_stream {
        start_stream();
    }
    0
}

unsafe extern "system" fn w_stop(ev: u32, immediate: u32) -> u32 {
    let Some((mut g, id)) = ours(ev) else {
        return unsafe { orig::<Fn2>(&O_STOP)(ev, immediate) };
    };
    let st = g.as_mut().expect("state");
    let was = st.engine.playing(ev).is_some();
    st.engine.stop(ev);
    if was {
        st.log(&format!("stop music {id}{}", if immediate & 0xff != 0 { " (immediate)" } else { "" }));
    }
    0
}

unsafe extern "system" fn w_get_state(ev: u32, out: *mut u32) -> u32 {
    let Some((g, _)) = ours(ev) else {
        return unsafe { orig::<Fn2>(&O_GET_STATE)(ev, out as u32) };
    };
    let s = g.as_ref().expect("state").engine.state(ev);
    if !out.is_null() {
        // SAFETY: the caller's out parameter.
        unsafe { out.write(s) };
    }
    0
}

unsafe extern "system" fn w_set_callback(ev: u32, cb: u32, ud: u32) -> u32 {
    let Some((mut g, _)) = ours(ev) else {
        return unsafe { orig::<Fn3>(&O_SET_CALLBACK)(ev, cb, ud) };
    };
    g.as_mut().expect("state").engine.set_callback(ev, cb, ud);
    0
}

unsafe extern "system" fn w_set_volume(ev: u32, v: f32) -> u32 {
    let Some((mut g, id)) = ours(ev) else {
        return unsafe { orig::<FnF>(&O_SET_VOLUME)(ev, v) };
    };
    let st = g.as_mut().expect("state");
    st.engine.set_volume(ev, v);
    let last = st.logged_volume.get(&ev).copied().unwrap_or(-1.0);
    if (v - last).abs() >= 0.1 || (v == 0.0 && last != 0.0) || (v >= 0.999 && last < 0.999) {
        st.logged_volume.insert(ev, v);
        st.log(&format!("volume music {id}: {v:.3}"));
    }
    0
}

unsafe extern "system" fn w_get_volume(ev: u32, out: *mut f32) -> u32 {
    let Some((mut g, _)) = ours(ev) else {
        return unsafe { orig::<Fn2>(&O_GET_VOLUME)(ev, out as u32) };
    };
    let v = g.as_mut().expect("state").engine.volume(ev);
    if !out.is_null() {
        // SAFETY: the caller's out parameter.
        unsafe { out.write(v) };
    }
    0
}

unsafe extern "system" fn w_set_paused(ev: u32, paused: u32) -> u32 {
    let Some((mut g, id)) = ours(ev) else {
        return unsafe { orig::<Fn2>(&O_SET_PAUSED)(ev, paused) };
    };
    let st = g.as_mut().expect("state");
    let p = paused & 0xff != 0;
    if st.engine.paused(ev) != p {
        st.log(&format!("{} music {id}", if p { "pause" } else { "resume" }));
    }
    st.engine.set_paused(ev, p);
    0
}

unsafe extern "system" fn w_get_paused(ev: u32, out: *mut u8) -> u32 {
    let Some((mut g, _)) = ours(ev) else {
        return unsafe { orig::<Fn2>(&O_GET_PAUSED)(ev, out as u32) };
    };
    let p = g.as_mut().expect("state").engine.paused(ev);
    if !out.is_null() {
        // SAFETY: the caller's out parameter (a C++ bool).
        unsafe { out.write(u8::from(p)) };
    }
    0
}

/// `EventSystem::update`, then the end callbacks of the songs that ended.
unsafe extern "system" fn w_update(es: u32) -> u32 {
    let r = unsafe { orig::<Fn1>(&O_UPDATE)(es) };
    let done = {
        let mut g = state();
        let Some(st) = g.as_mut() else { return r };
        st.updates += 1;
        if st.updates % 300 == 0 {
            let (u, r, n) = (st.updates, st.reads, st.engine.sounding());
            st.log(&format!("{u} updates, {r} stream reads, {n} songs sounding"));
        }
        let done = st.engine.take_finished();
        for d in &done {
            let id = st.owned.get(&d.event).copied().unwrap_or(0xffff);
            st.log(&format!("music {id} ended (event {:08x})", d.event));
        }
        done
    };
    for d in done {
        type Cb = unsafe extern "system" fn(u32, u32, u32, u32, u32) -> u32;
        // SAFETY: the callback and userdata the game gave setCallback;
        // type 4 is FMOD_EVENT_CALLBACKTYPE_EVENTFINISHED.
        unsafe {
            let cb: Cb = std::mem::transmute(d.callback as usize);
            cb(d.event, 4, 0, 0, d.userdata);
        }
    }
    r
}

/// The stream's PCM read callback (FMOD's stream thread, or its update).
unsafe extern "system" fn pcm_read(_sound: u32, data: *mut u8, len: u32) -> u32 {
    let frames = (len / 4) as usize;
    let mut g = state();
    let Some(st) = g.as_mut() else {
        // SAFETY: FMOD's buffer of len bytes.
        unsafe { std::ptr::write_bytes(data, 0, len as usize) };
        return 0;
    };
    let mut buf = std::mem::take(&mut st.buf);
    buf.clear();
    st.reads += 1;
    st.engine.render(frames, &mut buf);
    // SAFETY: FMOD's buffer holds len bytes; frames * 4 <= len.
    unsafe { std::ptr::copy_nonoverlapping(buf.as_ptr() as *const u8, data, frames * 4) };
    if let Some(w) = &mut st.wav {
        w.write(&buf);
    }
    st.buf = buf;
    0
}

/// Creates and starts the user stream on the game's `System` (on the FMOD
/// thread, from the first music start).
fn start_stream() {
    let mgr = rd(rebase(FMOD_MANAGER));
    let fmod = rd(mgr.wrapping_add(8));
    let system = rd(fmod.wrapping_add(8));
    if system == 0 {
        return;
    }
    let mut ex = [0u32; 0x22];
    ex[0] = 0x88; // cbsize
    ex[1] = 48000 * 4 * 10; // length in bytes (looped)
    ex[3] = 2; // numchannels
    ex[4] = 48000; // defaultfrequency
    ex[5] = 2; // FMOD_SOUND_FORMAT_PCM16
    ex[6] = DECODE_FRAMES; // decodebuffersize
    ex[11] = pcm_read as *const () as usize as u32; // pcmreadcallback
    // LOOP_NORMAL | 2D | OPENUSER | CREATESTREAM | SOFTWARE
    let mode = 0x4c8 | 0x2;
    type Create = unsafe extern "system" fn(u32, u32, u32, *mut u32, *mut u32) -> u32;
    type Play = unsafe extern "system" fn(u32, i32, u32, u32, *mut u32) -> u32;
    type Prio = unsafe extern "system" fn(u32, i32) -> u32;
    let mut sound = 0u32;
    let mut channel = 0u32;
    // SAFETY: the game's own fmodex imports, called as the movie player
    // calls them; the state lock is not held (createSound may read).
    let (r1, r2) = unsafe {
        let create: Create = std::mem::transmute(rd(rebase(SLOT_CREATE_SOUND)) as usize);
        let r1 = create(system, 0, mode, ex.as_mut_ptr(), &mut sound);
        let mut r2 = u32::MAX;
        if r1 == 0 {
            let play: Play = std::mem::transmute(rd(rebase(SLOT_PLAY_SOUND)) as usize);
            r2 = play(system, -1, sound, 0, &mut channel);
            if r2 == 0 {
                let prio: Prio = std::mem::transmute(rd(rebase(SLOT_SET_PRIORITY)) as usize);
                prio(channel, 0);
            }
        }
        (r1, r2)
    };
    let mut g = state();
    let Some(st) = g.as_mut() else { return };
    if r1 == 0 && r2 == 0 {
        st.sound = sound;
        st.channel = channel;
        st.log(&format!("stream on: sound {sound:08x}, channel {channel:08x}"));
    } else {
        st.stream_failed = true;
        st.log(&format!("stream FAILED: createSound {r1}, playSound {r2}"));
    }
}

/// `g_FmodCommandHandlers`: the FMOD thread's handler per queued command,
/// `cdecl(FfxFmod*, u32 args[4])`.
const HANDLERS: u32 = 0x00c3_a368;
/// Commands 0x1d `iMusicTempoTranslate(frames, tempo)` and 0x1e
/// `iMusicTempoDirect(tempo)`: their handlers (`0x00707f10`, `0x00707f70`)
/// call `FfxFmod` methods that are empty on PC.
const CMD_TEMPO: [(u32, u32); 2] = [(0x1d, 0x0070_7f10), (0x1e, 0x0070_7f70)];

/// Command 0x1d: `(frames, tempo & 0xff)` as the script gave them (the PC's
/// handler halves the frames for its 30 Hz; the driver counts 60 Hz).
unsafe extern "C" fn c_tempo_translate(_fmod: u32, args: *const u32) -> u32 {
    // SAFETY: the queue entry's four argument words.
    let (frames, v) = unsafe { (*args as i32, *args.add(1) as u8 as i8) };
    if let Some(st) = state().as_mut() {
        st.engine.tempo_translate(frames, v);
        st.log(&format!("tempo translate {v} over {frames} frames"));
    }
    0
}

/// Command 0x1e: `(tempo & 0xff)`.
unsafe extern "C" fn c_tempo_direct(_fmod: u32, args: *const u32) -> u32 {
    // SAFETY: the queue entry's four argument words.
    let v = unsafe { *args as u8 as i8 };
    if let Some(st) = state().as_mut() {
        st.engine.tempo_direct(v);
        st.log(&format!("tempo direct {v}"));
    }
    0
}

/// Writes `value` into the word at `addr` (an import slot or the mark),
/// returning the old value.
fn poke(addr: u32, value: u32) -> u32 {
    let p = addr as *mut u32;
    let mut old = 0u32;
    // SAFETY: a word of FFX.exe's image; made writable for the write.
    unsafe {
        VirtualProtect(p as *mut c_void, 4, 0x04, &mut old);
        let was = p.read_unaligned();
        p.write_unaligned(value);
        VirtualProtect(p as *mut c_void, 4, old, &mut old);
        was
    }
}

fn wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut n = 0;
    // SAFETY: a NUL-terminated UTF-16 string from the caller.
    unsafe {
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
}

/// Installs the live engine: `mod_dir` (UTF-16, NUL-terminated) is the mod
/// folder, whose `music/` the converter filled. Returns 0 when installed
/// (or already), 1 when the music files are missing or unreadable (nothing
/// changed: the bank plays), 2 when FFX.exe is not the expected build.
#[unsafe(no_mangle)]
pub extern "system" fn demaster_live_install(mod_dir: *const u16) -> u32 {
    let r = std::panic::catch_unwind(|| install(&wide(mod_dir)));
    r.unwrap_or(3)
}

fn install(mod_dir: &str) -> u32 {
    let mut g = state();
    if g.is_some() {
        return 0;
    }
    let dir = PathBuf::from(mod_dir).join("music");
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("live.log")).ok();
    let mut say = |s: &str| {
        if let Some(f) = &mut log {
            let _ = writeln!(f, "{s}");
        }
    };
    say(&format!("demaster_live {}: installing from {}", env!("CARGO_PKG_VERSION"), dir.display()));
    // SAFETY: the process's own module.
    let base = unsafe { GetModuleHandleW(std::ptr::null()) } as u32;
    BASE.store(base, Ordering::Relaxed);
    // the expected FFX.exe: the slots hold fmod_event.dll addresses (not
    // in FFX.exe's image) and the start slot's neighbours too
    let slots = [
        (SLOT_START, &O_START, w_start as *const () as usize),
        (SLOT_STOP, &O_STOP, w_stop as *const () as usize),
        (SLOT_GET_STATE, &O_GET_STATE, w_get_state as *const () as usize),
        (SLOT_SET_CALLBACK, &O_SET_CALLBACK, w_set_callback as *const () as usize),
        (SLOT_SET_VOLUME, &O_SET_VOLUME, w_set_volume as *const () as usize),
        (SLOT_GET_VOLUME, &O_GET_VOLUME, w_get_volume as *const () as usize),
        (SLOT_SET_PAUSED, &O_SET_PAUSED, w_set_paused as *const () as usize),
        (SLOT_GET_PAUSED, &O_GET_PAUSED, w_get_paused as *const () as usize),
        (SLOT_UPDATE, &O_UPDATE, w_update as *const () as usize),
    ];
    let image = base..base + 0x0200_0000;
    for (slot, _, _) in &slots {
        let v = rd(rebase(*slot));
        if v == 0 || image.contains(&v) {
            say(&format!("slot {slot:08x} holds {v:08x}: not the FFX.exe this was written for; not installed"));
            return 2;
        }
    }
    for (cmd, f) in CMD_TEMPO {
        let v = rd(rebase(HANDLERS + 4 * cmd));
        if v != rebase(f) {
            say(&format!("FMOD command {cmd:#x}'s handler is {v:08x}, not {:08x}: not installed", rebase(f)));
            return 2;
        }
    }
    let lib = match Library::open(&dir) {
        Ok(l) => l,
        Err(e) => {
            say(&format!("music files: {e:#}; not installed (the bank plays)"));
            return 1;
        }
    };
    let wav = std::env::var("DEMASTER_LIVE_WAV").ok().filter(|s| !s.is_empty()).and_then(|p| Tee::create(&p));
    if wav.is_some() {
        say("writing the engine's output to DEMASTER_LIVE_WAV");
    }
    *g = Some(State {
        engine: Engine::new(lib),
        owned: BTreeMap::new(),
        handled: BTreeMap::new(),
        sound: 0,
        channel: 0,
        stream_failed: false,
        log,
        t0: std::time::Instant::now(),
        wav,
        buf: Vec::new(),
        logged_volume: BTreeMap::new(),
        updates: 0,
        reads: 0,
    });
    drop(g);
    for (slot, keep, w) in &slots {
        let old = poke(rebase(*slot), *w as u32);
        keep.store(old, Ordering::Relaxed);
    }
    poke(rebase(HANDLERS + 4 * 0x1d), c_tempo_translate as *const () as u32);
    poke(rebase(HANDLERS + 4 * 0x1e), c_tempo_direct as *const () as u32);
    poke(rebase(MARK), u32::from_le_bytes(*b"LIVE"));
    let debug = if let Some(st) = state().as_mut() {
        st.log(&format!("installed: FFX.exe at {base:08x}, 9 import slots and 2 FMOD command handlers wrapped"));
        st.wav.is_some()
    } else {
        false
    };
    if debug {
        // with DEMASTER_LIVE_WAV: a line every 5 s from a thread of its
        // own, so the log shows whether FMOD's thread and stream still run
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let mut g = state();
                let Some(st) = g.as_mut() else { return };
                let (u, r, n) = (st.updates, st.reads, st.engine.sounding());
                st.log(&format!("watchdog: {u} updates, {r} stream reads, {n} songs"));
            }
        });
    }
    if let Some(id) = std::env::var("DEMASTER_LIVE_TEST").ok().and_then(|s| s.parse::<u32>().ok()) {
        std::thread::spawn(move || self_test(id));
    }
    0
}

/// `DEMASTER_LIVE_TEST=<music id>` (a test without a scene that plays
/// music): once FMOD runs, the game's own `iMuiscSetBgmType(1)`,
/// `iMusicRead(id)` and `iMusicStart(id)` (`0x00679fe0`, `0x0067a170`,
/// `0x0067a1e0`: FMOD commands 0x2e, 0x28, 0x27, the queue the scripts'
/// music commands use), and 20 s later `iMusicStop(id)` (`0x0067a210`).
fn self_test(id: u32) {
    type F = unsafe extern "C" fn(u32) -> u32;
    let call = |va: u32, a: u32| {
        // SAFETY: FFX.exe's cdecl(int) queue functions (Fmod_PushCommand
        // takes the queue's mutex: any thread may queue).
        unsafe { std::mem::transmute::<usize, F>(rebase(va) as usize)(a) };
    };
    let updates = || state().as_ref().map_or(0, |s| s.updates);
    while updates() < 100 {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if let Some(st) = state().as_mut() {
        st.log(&format!("self-test: original soundtrack, music {id}"));
    }
    call(0x0067_9fe0, 1);
    call(0x0067_a170, id);
    call(0x0067_a1e0, id);
    std::thread::sleep(std::time::Duration::from_secs(20));
    if let Some(st) = state().as_mut() {
        st.log(&format!("self-test: stop music {id}"));
    }
    call(0x0067_a210, id);
}
