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
- **Long samples in pieces** (`FUN_001012d8`): a region with WD flag
  `0x10` holds a sample cut into 0x2000-byte pieces, each ending in loop
  end + repeat; its key-on sets the loop start to the second piece, and
  every interrupt, once the voice plays inside the piece the loop start
  points to (`LSA <= NAX < LSA + 0x2000`), the driver moves the loop start
  to the next piece. The sample plays through once (banks 143 and 149, the
  only ones with such regions). Without this the second piece looped
  forever ("Track notes", 1).

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

- Of the 86 rendered tracks, 71 correlate >= 0.9 window by window (median
  0.97), 10 between 0.8 and 0.9, 5 below; the median spectral correlation
  is 0.986. Loop periods agree to a few ms in 60 tracks (battle theme 78.317
  s against 78.318). Reverb tails decay as in the recordings in every band.
- Where the loop period differs (Luca, Auron's theme, Darkness, ...), the
  render still matches the recording beyond the PC's loop end: the PC loops
  a shorter repeated section; ours loops where the sequence does.
- Below 0.8: Prelude (the PS2 plays a 48 s, 16 kHz sample loop; the PC's
  is another, longer recording), Game over (the recording carries the
  game's own fade and a slower tempo: script commands, see phase 2), The
  Summoning, the Marriage and Tragedy one-shots (timing drifts by a
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

## Track notes

Where the game plays each music id, and what the odd tracks are. Sources:
the scripts as decompiled by Atel# (`yutamago/ffx-atel-src`: every
`MusicLoad`/`MusicPlay`/`MusicCrossStart`/`MusicStart`/`MusicPhraseStart`/
`MusicMultiStart` with its id), the event headers' music words (the sound
table at `hdr + [hdr + 0x2c]`, word `+0x38`: a map's own music, started on
entry), the Luca sphere theater's list (`lchb1200`, shared data `0x2040`:
92 music ids, M01..M92, with titles in its text), FFX.exe (`SndMusicStart`
calls with constants: only 0x10 battle theme and 0x8a fanfare; the battle
end's 0x8c game over), the PC's music id table (`0x00b4f1c8`) and the
disc. Battle formations (`battle/btl/*`) carry no music id: boss music is
started by the field script before the battle, random battles play 0x10.
Scratch scripts: `~/ffx-work/demaster/scratch/mus/` (not in the repo).

### 1. Good Night (music143) and music149

| | |
|---|---|
| Symptom | the first piece of the sample plays, then the last ~0.3-0.6 s loops |
| Cause | a decoding gap in our driver, not the game: both banks hold one long sample cut into 0x2000-byte pieces, every piece ending in an ADPCM block with loop end + repeat (flags 3), the last one with end (1). Their regions have WD flag `0x10`, for which the key-on sets the loop start to SSA + 0x2000 (already ported) and the driver's per-interrupt `FUN_001012d8` moves the loop start one piece on whenever the voice plays inside the piece it points to (`LSA <= NAX < LSA + 0x2000` -> `LSA + 0x2000`; MIPS at `0x101350..0x1013a8`). Without that step the voice jumped back to piece 2 forever: 0x2000 bytes = 14336 samples, 0.36 s at 40 kHz (143), 0.64 s at 22.4 kHz (149) |
| Not the cause | sample offsets (`0xc`, `0x214fc`: the SPU ignores the low 4 bits; block 0 is the usual zero block), key splits (one instrument, one stereo pair / one region), the game (the inns, e.g. `djyt0200` W20, keep the playing music, `MusicPlay(143)`, then `MusicCrossStart(saved, 60)`: the one-shot plays whole, no fade) |
| Fix | `sqseq::Song::update_voices` does the driver's step (`Spu2::nax`/`lsa`); test `a_long_sample_in_pieces_plays_through` |
| Affected | only banks 143 and 149 (the only WD banks of 834 with flag-`0x10` regions); every other render is byte for byte unchanged |
| music143 now | 5.7 s stereo recording at 40 kHz, the whole piece; against the PC's Good Night: spectral correlation 0.569 -> 0.889, 10 s window correlation 0.53 -> 0.95, envelope 0.34 -> 0.93 (it moves from "below 0.8" to ">= 0.9") |
| music149 now | 39 s mono recording at 22.4 kHz, looped by the song: Hum of the Fayth, spectral correlation with the PC's 1_20 recording 0.76 -> 0.971 (music160, the bank's choice for that event: 0.972) |

### 2. Tracks whose bank is missing (music052, 190, 191, 193-199)

| | |
|---|---|
| How the driver picks a bank | by the WD id in the BGM header (`u16` at 4, usually the song's own number; 191 names 190). The game loads a song with `MusicLoad(id)`; the driver reads the sequence and the WD of that id (debug path `wave/wave%4.4u.wd`, on the disc `wave_wd.dat` at the sector `wave_wd.sec` gives). There is no other bank source for music: `iMusicDataSend` sends the same pair; the SE banks (`se000.seb`, the monster files) and `shout.dat` are separate; command `0x21` (other bank) is used by no sequence. A song whose bank is not loaded does not play at all (driver message `MUSIC PLAY : Wave-Data was not entry`) |
| The banks | not on the disc. `wave_wd.sec` has no entry for 52, 190, 193..199, and no WD header with these ids exists anywhere on the ISO: 1154 WD headers in all (any 16-byte boundary), 834 in `wave_wd.dat` (all indexed) and 320 SE banks (ids 1000+) inside `jppc/battle/mon/_mNNN/mNNN.bin` |
| Uses | none: no script, no map music word, not in the sphere theater, no constant in FFX.exe; the PC's music table has no event for 52, and 190+ are beyond its 182 entries |
| Music | none of them shares a melody with any other sequence (transposition-free 5-grams of note intervals: at most 0.11 with a named track); 190/191 are one piece (0.82-0.96), 193..199 a family of short pieces sharing 20-65% |
| Conclusion | leftovers from development whose instruments were not shipped. The studio can only play them with instruments from other banks (unchanged) |

### 3. Track names

The names come from the PC: FFX.exe's music id table (`0x00b4f1c8`, 182
entries) maps each music id to an event of `ffx_music_ps2.fev`, whose
sample in `ffx_music_ps2_bank00.fsb` is named after the soundtrack
("1_09_battle_theme", disc 1 track 9; "m_NN" for the sphere theater's
numbers); `musicjob::TRACKS` keeps one PS2 sequence per sample. Where two
ids share an event (149/160, 157/179, 166/181) only one carried the name.
The studio now shows a name for every sequence, the unofficial ones marked
"(proposed)" (`library::PROPOSED`).

| id | where the game plays it | what it is | proposed name |
|---|---|---|---|
| 149 | Besaid temple (`bsyt0000`, `bsyt0001`), Kilika temple (map music of `klyt0300`, `0500`, `0600`, `0700`, `0900`), Djose temple (`djyt0300`), Macalania temple (`mcyt0200`), Lake Macalania (`lmyt0200`), `kino0900`, Luca (`lchb0700`), the ending (`endg0100`); theater M91 | the Hum of the Fayth as a 39 s mono loop (PC event: 1_20) | Hum of the Fayth, temples (1-20) |
| 179 | `hiku1500` (Highbridge); theater M92 | the same two tracks as 157 (Song of Prayer: Spira) byte for byte, its own copy of the recording (PC event: 4_13) | Song of Prayer: Spira, second copy (4-13) |
| 181 | Bevelle (`bvyt0000`) | another arrangement of Raid (30 tracks, 95% of 166's melodic 5-grams, other bank; PC event: 3_14) | Raid, Bevelle (3-14) |
| 180 | only the sphere theater, M72, titled "Hum Of The Fayth (Yuna)" in the game's text | 34.4 s mono recording at 44.1 kHz: a single voice humming | Hum of the Fayth (Yuna) (the game's own title) |
| 173 | none | the whole Otherworld, guitar and vocals included, as two 12 kHz recordings (30 s intro, 116 s body, looped from the body); against the PC's Otherworld: spectral correlation 0.969, envelope 0.91 | Otherworld, whole recording at 12 kHz (1-05) |
| 126 | none | Out of the Frying Pan (music018) with its own bank: same notes (5-grams 1.00), 9 of 11 tracks re-sequenced, an almost identical spectrogram | Out of the Frying Pan, other mix (1-12) |
| 127 | none | Battle Theme (music016), same notes (0.99), 18 tracks against 20, own bank | Battle Theme, other mix (1-09) |
| 175 | none | Start (music131), same notes (1.00/0.98), 20 tracks against 19, own bank | Start, other mix (3-12) |
| 070 | none | Nostalgia's notes on bank 70's noise samples (5.) | Nostalgia's notes on a noise bank (unused) |
| 071 | none | the same 7 tracks as 070 byte for byte, bank 71 (one stereo multi-sampled instrument), header volume 0: practically silent (peak 76 of 32767) | Nostalgia's notes, volume 0 (unused) |
| 182, 183 | none | 8 notes each, a scale (A, B) of 0.5 s notes on music180's voice recording played at 74-83 kHz: a pitch test | Test: scale on Yuna's hum (A / B) |
| 240 | none | 6 s, 57 notes on 20 channels with the general bank 0 (33 instruments): a short swept effect, the only user of counted loops and automatic key-offs | 6 s jingle on bank 0 (unused) |
| 052, 190, 191, 193-199 | none | bank missing (2.) | Unused piece / short piece NNN, bank not on the disc |

None of the "(proposed)" names is an official title except 180's. The
match of the other mixes to their named tracks is by notes and
spectrograms, not by ear.

### 4. Wakka's Theme (music046, m_71)

| Where | Evidence |
|---|---|
| The Luca sphere theater only (`lchb1200`, the sphere theater in the Luca café): entry M71 "Wakka’s Theme" of its list; worker W06 loads and plays `V18[V46]`, the selected entry | theater list at shared data `0x2040` index 71 = 46; text `M71 : Wakka’s Theme` |
| No event, map or battle | no `MusicLoad`/`Play`/`CrossStart`/`Start` of 46 in any of the 758 scripts (events, battles, monsters), no map music word 46, no constant in FFX.exe |

So in the story it never plays; it can be heard in the sphere theater
(and on the PC, which maps 46 to its own event, the same way).

### 5. music070

| | |
|---|---|
| Uses | none (no script, map, theater entry or FFX.exe constant; the PC's table has no event for 70) |
| What it is | Nostalgia's notes (music144: the same melody, 5-grams 1.00/0.95; 7 tracks against 6, re-sequenced, header reverb 4 depth 15000, volume 127) pointed at bank 70, whose 11 instruments (27 regions) are long unpitched one-shot samples: the notes trigger broadband noise bursts (spectrogram: no harmonics, rms -14 dB against Nostalgia's -27 dB), which is why it sounds like random sounds. Bank 70 does not belong to this sequence (or the sequence is a placeholder copied over an older song 70); its twin 071 plays the same notes on a piano-like stereo instrument at volume 0 |
| Category | an unused leftover, not a sound-effect sequence, ambience or jingle the game plays |

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
