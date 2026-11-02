# Music: the PS2 sound driver (`audio.music_engine`)

The PC's "Original" soundtrack (`sound_pc/music_ps2/ffx_music_ps2_bank00.fsb`,
89 FMOD samples) is a recording of the PS2 music, mastered again (every
track peak-normalised and limited) and IMA-ADPCM encoded. With
`audio.music_engine = "ps2"` the converter (job `audio.music`) renders the
tracks itself from the player's disc, with the PS2's own sequences and
instruments played by a port of the PS2 sound driver on an SPU2 model, and
writes a replacement bank in the same format. Choosing "Original" in the
game's menu (or `audio.soundtrack = "ps2"` for a new game) then plays it.

Phase 1 (this) renders offline. Phase 2 (a live engine) is planned below.

## The PS2 data

All under `_ffx/proj/sound/<region>/` (`us` on the German disc too):

| File | What |
|---|---|
| `music/data/musicNNN.bgm` | 114 sequences, `NNN` = the game's music id (`BGM ` magic) |
| `music/data/music.dat` + `.sec` | the same sequences in one file; `.sec`: `u32 sector, u32 size` per id |
| `wave/wave_wd.dat` + `.sec` | the instrument banks ("WD"); `.sec`: `i32` sector per bank id, -1 none |
| `jppc/irx/iopsound.irx` | the IOP sound driver that plays them (MIPS R3000) |
| `jppc/irx/libsd.irx` | Sony's SPU2 library (the reverb presets) |

**BGM header** (0x20 bytes): `BGM `, `u16` id, `u16` WD bank id, `u8`
tracks, `u8` reverb mode (libsd mode: 5 hall, 4 studio C, 0xff none),
`i16` reverb depth (EVOL; 0 keeps the current), `u8` volume (`& 0x7f`),
`u16` ppqn (48), `u32` size, `u32` flags (`0x10000`: notes key off one tick
before the next event). Then per track `u32` length and its events: a
variable-length delta (MIDI style), then `command, arguments, delta`
repeated. The command set is the driver's table at `0x17500` of the
module (128 handlers; `converter/src/sqseq.rs` lists the argument layout).

**WD bank**: `WD`, `u16` id, `u32` sample data size, `u32` instruments,
`u32` regions, `u32` SPU address (set by the driver), then at `0x20`
`u32` offsets of each instrument's first region; regions are 0x20 bytes:
`u32` flags (bit 0: stereo pair with the next region, `0x100` first, `0x200`
last, `0x10000` key/velocity split), `u32` sample offset, `u32` loop offset,
`u16` ADSR1, `u16` ADSR2, `u16` (unused by the driver), `i16` tuning in
1/256 semitones (key 60 + tuning = the sample's native 48 kHz), `u8` key
high, `u8` velocity high, `u8` volume, `u8` pan (bit 7: use `(pan & 0x7f) +
0x40`), `u8` reverb override (1 wet only, else wet + dry). The 4-bit ADPCM
sample data follows the regions.

The music id table of FFX.exe (`0x00b4f1c8`, ffx-rust `sound.md`) maps the
game's music ids to the PC bank's events; the PS2 file number is the music
id, so `musicjob::TRACKS` lists the PS2 sequence of every PC sample. Where
two ids share an event, the one whose render matches the recording is used
(hum of the fayth: 160 rather than 149).

## The driver (`sqseq.rs`)

`iopsound.irx` was decompiled with Ghidra (relocated to `0x100000`, its
command and LFO tables added as functions); the port keeps its arithmetic:

- **Timing**: a hardware timer at 36.864 MHz / 153600 = 240 Hz. Each
  interrupt first sends key-offs and does the voice updates (key-ons queued
  by the last tick, volumes, pitches, mix switches, `KON`), then subtracts
  the tempo (BPM, starting at 1 until the first `0x08`) from a counter and
  runs one tick per 300 it falls below 1. 48 ppqn: one tick per 1/0.8 BPM s.
- **Loops**: `0x02` in track 1 snapshots every track (position, wait, flags,
  key) and the tempo; when all tracks have ended, or at `0x03`, the song
  restores the snapshot and ticks again at once. `0x04`/`0x05 n` are counted
  loops (4 deep). Volumes, pans and programs are *not* restored.
- **Voices**: 48 (24 a core); a note takes the first voice whose envelope
  (`ENVX`, read back each interrupt) is 0, else steals the quietest.
  Stereo regions take two voices, the first panned left.
- **Volume**: `(song volume + 1) * 511 >> 7`, times expression, velocity,
  track volume and region volume in the driver's fixed point; the pan law
  is the module's 256-entry table (read from the disc), 0x40 left, 0x80
  centre, 0xc0 right.
- **Pitch**: `key * 256 + detune + bend + tuning` through the module's
  semitone and 1/256-semitone tables (both `floor(2^(n/12))` exactly),
  vibrato from the pitch LFO (`0x40..0x47`, 10-bit phase, the module's
  waveforms: sine, triangle, saw, square, random).
- **Reverb**: commands `0x60`/`0x61` switch a track's voices between dry
  and dry + wet, `0x0a` sets the depth (EVOL = value << 8).

Not carried out (not used by FFX's music, counted in `Marks::unsupported`):
volume LFO (`0x48`), noise and pitch modulation (`0x64`, `0x68`),
random variation (`0x78`), other banks (`0x21`), the tempo slide (`0x09`,
which the driver itself never applies).

## The SPU2 (`spu2.rs`)

48 kHz, two cores: 4-bit ADPCM with the five filters and the block flags
(loop start, loop end, repeat; an end without repeat stops the voice), the
ADSR envelope (rates as the hardware steps them, exponential attack above
0x6000, exponential decay), 4-tap interpolation (a Gaussian kernel close to
the hardware's table, not its exact values), voice volumes in the fixed
register format, dry and wet sums per core, and the reverb: the PS1/SPU2
algorithm (same-side and cross IIR, four combs, two all-pass) at 24 kHz
through a 39-tap half-band resampler, with `libsd`'s preset registers and
work-area sizes read from the disc (addresses x4 on the SPU2). Core 1 mixes
core 0's output dry (`MMIX` 0xfc0 / 0xfcc as the driver sets them).

## The bank (`fsb5.rs`, `musicjob.rs`)

The job reads the PC bank's head (only its first blocks: the bank is 672
MB) for the names, order and the 32 bytes of flags/hash the `.fev` refers
to, renders the 89 tracks four at a time, each streamed into a file, and
writes FSB5 v1, codec 7 (IMA ADPCM, Xbox layout: per channel a header with
the first sample and the step index, then 63 codes), 48 kHz stereo,
32-byte aligned data, a loop chunk per sample. 886 MB, about 90 s.

- **Loops**: a looping song is rendered to its second jump back; the bank
  loops over the second pass, whose start carries the first pass's reverb
  and release tails, as the PS2's does when it jumps back. One-shots run
  until the reverb has died out.
- **Level**: the chip's output +9 dB (one gain for every track, so the
  PS2's balance between tracks stays) through a 2 ms look-ahead limiter at
  -1 dBFS (it touches 0.06% of the samples on average). The PC's tracks are
  normalised one by one, so ours sit 0-7 dB below them (median -2 dB).
- **Kept from the PC**: Otherworld, Confrontation and Jecht's theme. Their
  guitar and vocals were streamed from the disc next to the sequence
  (`BattleStreamingStart` in the driver); the sequences alone lack them, so
  the bank keeps the PC's samples for these three, byte for byte.

Debug: `demaster music <iso> <vbf> <out> all|<id|name>...` writes WAVs
(after the gain and limiter; `DEMASTER_RAW=1` without), with the loop
points; `demaster music-bank <bank.fsb> <out> [name...]` decodes a bank
(also the PC's: the decoder matches ffx-rust's `ffx sound wav` byte for
byte).

## How it was checked

Against the PC's recordings (`~/ffx-work/demaster/music/`, scripts in
`scripts/`, not in the repo): our bank decoded back to WAV and each track
aligned to the recording, then 64-band spectra compared in 10 s windows over
the whole recording, the loop periods compared, and spectrograms plotted
side by side (`cmp_bank/<track>.png`).

- Of the 86 rendered tracks, 70 correlate >= 0.9 window by window (median
  0.97), 10 between 0.8 and 0.9, 6 below; the median spectral correlation
  is 0.986. Loop periods agree to a few ms in 60 tracks (battle theme 78.317
  s against 78.318). Reverb tails decay as in the recordings in every band.
- Where the loop period differs (Luca, Auron's theme, Darkness, ...), the
  render still matches the recording beyond the PC's loop end: the PC loops
  a shorter repeated section; ours loops where the sequence does.
- Below 0.8: Prelude (the PS2 plays a 48 s, 16 kHz sample loop; the PC's
  is another, longer recording), Good night and Game over (the recordings
  carry the game's own fade and a slower tempo: script commands, see phase
  2), The Summoning, the Marriage and Tragedy one-shots (timing drifts by a
  few percent: rubato or script tempo). Some tracks differ in one narrow
  bass band by 5-19 dB (Wandering Flame at 62 Hz), not traced yet.
- The recordings' broadband hiss that follows each note is the PC bank's
  4-bit codec noise; our bank has the same codec.
- In the game (`tools/replays/music-battle.txt` through `tools/shotrun`,
  `FFX_HOOK_LOG_OPENS=1`, `FFX_AUDIO_WAV`, `FFX_TEST_VOLUME=100`): after
  switching the save to the original soundtrack, the battle's music is read
  as `mods: .../music_ps2/ffx_music_ps2_bank00.fsb`, and the captured mix
  matches our battle theme sample for sample (waveform correlation 0.87;
  93% of the captured energy explained in every 2 s window after the
  start; the rest is the battle's sound effects), while the PC recording
  explains under 15%. `music-besaid.txt` is no test: that part of the beach
  plays no music.

## Phase 2: a live engine (plan)

The PC game drives music through `iMusic*` commands (ffx-rust `sound.md`,
"FMOD command queue"); several are empty on PC (`musicTempoDirect/
Translate`, `musicPitchDirect/Translate`, `musicMuteSet`), and the
recordings bake in whatever the PS2 did with them. A live engine would play
the sequences as the PS2 did, commands included:

1. A Fahrenheit hook on the FMOD command handlers for the music commands
   (`0x14..0x2f`, `FfxFmod::music*`) in the C# mod, forwarding them to a
   native engine (the Rust `sqseq` + `spu2`, built as a DLL next to
   `demaster.exe`) instead of FMOD's events when the original soundtrack is
   on.
2. The engine streams its output into FMOD as one user PCM stream (as the
   movie player does: `createSound` with a `pcmreadcallback`), so the
   game's master/music volumes and FMOD's mixer stay; it runs on the FMOD
   tick (30 Hz) and renders 240 Hz interrupts ahead into a ring buffer.
3. Commands map to the driver's own: `iMusicRead/DataSend` load BGM + WD
   (from the converter's extracted copies, not the PC bank), start/stop,
   `VolumeTranslate/FadeOut` -> song volume slides, `TempoDirect/Translate`
   -> the tempo modifier (`DAT_00222450`), `PitchDirect` -> the global
   pitch (`DAT_00222440`), `ReplayPointSet` -> the loop-point snapshot,
   `CrossPlay` -> two songs at once (the driver has a second song slot).
   `iMusicPlayState`/`SyncStandby` answer from the engine's state.
4. The streamed layers of the three rock tracks: find the battle streams on
   the disc (the driver's `BattleStreaming`, VAG streams in the voice data)
   and mix them in; until then those three stay the PC's recording.
5. Check against the offline renders (same bytes for a plain start) and
   against captures of scenes that use tempo and fades (the game over, an
   inn's good night).
