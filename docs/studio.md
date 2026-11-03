# FFX Music Studio (`studio/`, `ffx-music-studio.exe`)

A player and experimental editor for the PS2 music of your own FFX disc.
It plays the disc's `musicNNN.bgm` sequences through the converter's port
of the PS2 sound driver on its SPU2 model (`docs/music.md`), the same path
that renders the "Original" soundtrack bank, and lets you change them while
they play. Track edits are not saved: they live in memory until you pick
another track or press *Reset track* (*Export WAV* writes the edited track
as a sound file). The window's own settings are kept (below).

## Running it

It reads the same `demaster.toml` as `demaster.exe` and has no file
dialogs. It looks for the file:

1. `--config <file>` if given;
2. next to `ffx-music-studio.exe`;
3. the mod folder layout: `demaster.toml`, `mods/yuta.demaster/demaster.toml`
   or `yuta.demaster/demaster.toml` up to three folders above the exe (so the
   exe can sit in `mods/yuta.demaster/` next to `demaster.exe`, or in a
   folder beside it);
4. the current folder.

From that file it takes `source.iso` (the PS2 disc image) and `source.game`
(the PC game folder, or the Steam libraries when empty): the disc's file
names come from the PC's tables in `FFX_Data.vbf`, as for the converter.
If the file or the disc is missing, the window says which files it looked
at and what to set.

```bash
cargo run --release -p ffx-music-studio -- --config my.toml        # Linux, for development
tools/build-studio-windows.sh                                       # ffx-music-studio.exe (cargo-xwin)
```

`--open <track> [--tab rack|roll|mixer|chip|song] [--seek <tick>]
[--paused] [--master <x>] [--output <x>] [--channel <ch>] [--hold-key
<key>]` starts with a track open: `--master` is the track's volume,
`--output` the output volume for this run (not saved), `--channel` the
selected channel (with `--tab roll`: shown alone), `--hold-key` holds a key
on the piano roll's keyboard (MIDI number; used for the screenshots).

On Windows the sound goes to the default output device (WASAPI, shared
mode, ~40 ms of buffer). The Linux build has no sound output (and no MIDI)
unless built with `--features alsa` (needs ALSA's development files);
without it a clock drains the audio at the real rate, so everything else
works.

**Window settings** (`ffx-music-studio.toml` next to the exe, written when
they change): the output volume, the metronome, the MIDI input, the piano
roll's zoom and key height, the rack's row height and the snap. They stay
the same from track to track and from one start to the next. Track edits
(including the track volume) are never written.

## The window

**Track list** (left): every `musicNNN.bgm` on the disc (112 on the German
disc). The 89 that the PC's "Original" soundtrack bank uses carry the bank's
names ("Battle Theme (1-09)", disc and track number of the soundtrack)
from `musicjob::TRACKS`; the other 23 carry a name marked "(proposed)"
(`library::PROPOSED`, evidence in `docs/music.md`, "Track notes"); all
sorted alphabetically. The filter matches names and ids. A click loads the
track and plays it.

**Transport** (top): the track's name, *Undo* / *Redo* (also Ctrl+Z,
Ctrl+Y or Ctrl+Shift+Z: every edit, mixer and chip settings included; a
slider's drag is one step), *Export WAV*; play/pause (also Space), stop,
*Play A-B* (from marker A, looping between the markers), *Loop* (jump back
at the end; off: play to the end and stop), *Metronome* (a click on every
beat, higher on the bar), *Rec* (below), the elapsed time (loops included),
bar.beat and tick of the sequence, a seek slider; *Tempo* (the sequencer's
speed: the driver's tempo counter runs faster or slower, the pitch stays),
*Speed* (the output resampled: tempo and pitch together), *Output* (the
studio's output volume, after everything, kept), *Reset track*, the MIDI
input, and the voices in use out of 48. "re-sequence N ms" shows how long
the last reload took; "fading out" that a paused song is still sounding.

- **Pause** releases every sounding note: each voice runs its own SPU2
  release (its ADSR), the chip plays on with the reverb tail until it is
  silent, the sequence stands still. Play goes on from there. **Stop**
  does the same with a copy of the chip and goes back to the start; a
  second Stop (or a seek, an edit, another track) cuts at once.
- **Export WAV**: the track as edited (notes, mixer, instruments, chip,
  tempo, speed; without the metronome and the output volume), once through
  (the song's loop not taken), then 2 s of release and reverb, 48 kHz
  16-bit stereo, to `exports/musicNNN-<name>-<time>.wav` next to the exe.
  It renders in the background; the message bar says where it went.

**Channel rack** (the first tab; after FL Studio's playlist): one row per
channel. The header: the channel's colour, an activity LED and level meter
(lit while its voices sound, from the driver's voices and the chip's
envelopes), its number, its instrument (`<bank> #<program>`, or the bank
and instrument given in the mixer), its notes count, and M (mute),
S (solo) and a volume slider. To the right, the channel's notes as a mini
piano roll (its own key range over the row's height, brightness by
velocity) across the whole song, under one ruler with bar numbers, the
playhead and the loop markers A and B; the song's end is shaded.

- click a row: selects the channel (the piano keys and the MIDI keyboard
  play it); **double click**: its piano roll, the channel alone;
- ruler: click or drag to play from there; drag A or B: the loop range;
- wheel: rows up and down; Shift+wheel (or a horizontal wheel): along the
  time; Ctrl+wheel: zoom the time; Alt+wheel: row height (both around the
  pointer). A new track fits the whole song across.

**Piano roll**: the notes on the sequence's timeline (48 ticks a quarter
note, bars of four beats), the active channel bright (brightness by
velocity), the others dim (*all channels* off hides them). Below, the
active channel's velocities. In the ruler, the loop markers and the
playhead. The view follows the playhead while playing (*follow*).

- click on empty space: a new note (length and velocity of the last note
  touched, 1/8 and 100 at first), snapped to the grid (*Snap*), and heard;
- click a note: select it; Ctrl/Shift+click: add to or take from the
  selection; drag on empty space: a selection box (Ctrl/Shift: added);
  Ctrl+A: all; Escape: none;
- drag a note: move the selection in time and pitch; drag a note's right
  end: the selection's length; right click: delete a note; Delete or
  Backspace: the selection;
- arrow keys: nudge the selection (left/right one grid step, up/down a
  semitone, Shift+up/down an octave);
- Ctrl+C / Ctrl+X / Ctrl+V: copy, cut, paste (at the playhead, snapped
  down to the grid; the clipboard holds the notes as text, so they also go
  from one track or window to another);
- hover a note: its key, velocity, bar and length;
- drag a bar in the velocity lane: its velocity;
- one selected note's key, start, length and velocity can be typed in the
  toolbar;
- **the keyboard strip** on the left: click or hold a key to hear it on
  the channel's instrument (note-off on release; slide for a glissando); it
  places no note. Keys sounding (strip or MIDI) light up;
- click in the ruler: play from there; drag A or B: the loop range;
- wheel: up and down the keys; Shift+wheel (or a horizontal wheel): along
  the time; Ctrl+wheel: zoom the time; Alt+wheel: key height (around the
  pointer).

An edit re-encodes the sequence and the player restarts it silently up to
the current tick ("re-sequence", typically 10-200 ms), so the change is
heard at once.

**MIDI keyboard** (Windows; `midir`): *MIDI in* in the transport lists the
input devices; the saved one, else the first, is opened at the start
(*none* switches it off, the arrow looks again). Notes play live with their
velocity on the selected channel (rack) or the piano roll's channel, with
that channel's instrument now (its program at the playhead, or the one
given in the mixer), volume, pan and transpose; mute and solo do not
silence it. The sustain pedal (CC 64) holds notes, pitch bend moves them
(the channel's bend range, 2 semitones), CC 120/123 end them all. Every
MIDI channel plays. **Rec** records while the song plays: what you play
(MIDI or the strip) is shown red in the piano roll and goes into the
channel's notes, start and end snapped to the grid, when you press Rec
again or the song stops (one undo step).

**Mixer & instruments**: per channel its level (LED and meter), the voices
the driver has given it now, its current program (instrument), mute, solo,
volume (x0..x2), pan (added to the driver's pan position, ±64), transpose
(semitones, new notes), and the instrument. On the right, the instrument
banks of `wave_wd.dat` (the track's own first): pick a bank, then an
instrument (its regions and key ranges); *Play* previews it at the
*preview key* for two beats through the same driver (without reverb).
*use selected* gives the instrument to a channel; *x* goes back to the
channel's own. An instrument from another bank is appended to the track's
bank in memory, with that bank's samples (the sound RAM is a model, so
there is no 2 MB limit).

**Sound chip**: the reverb preset (the song's, off, or one of libsd's nine:
room, studio A/B/C, hall, space, echo, delay, pipe, with the registers and
work-area sizes read from the disc's `libsd.irx`; switching clears the
work area), wet (the reverb return) and dry gains, the interpolation
(Gaussian is the SPU2's 4-tap; linear and nearest are for comparison);
**Track volume** (the track's level before the converter's output stage,
+9 dB and the limiter at -1 dBFS; a track edit, 1.00 again with *Reset
track* or another track) and **Output volume** (the studio's, after
everything; the same slider as in the transport).

**Song: loop & tempo**: the header (bank, reverb mode and depth, volume,
length, the song's loop), the loop markers as numbers (and *song's loop* to
go back to it), and the tempo commands (`0x08`) of every channel with their
BPM, editable.

## How it plays

`converter` is a library as well as the `demaster` binary
(`converter/src/lib.rs`); the studio uses its `disc`, `config`, `build`
(`find_vbf`), `musicjob` (`Sources`, `TRACKS`, the output stage), `sqseq`
and `spu2`.

- **The sequence as events** (`studio/src/seq.rs`): each track is decoded
  into a list of commands at absolute ticks (stream order, arguments as
  stored). Encoding writes the deltas between the ticks again; an unedited
  sequence encodes to the disc's bytes exactly (tested for every sequence).
  Notes are derived from the key-ons and key-offs, as the driver pairs them
  (a key-off ends the voices of its key; with header flag `0x10000` the
  driver's automatic key-off one tick before the next command, which only
  `music240` uses).
- **Normalizing** a track before its first note edit: the driver keeps the
  current key and velocity between commands (`0x10` plays the last ones),
  so every key-on becomes `0x11 key vel` and every key-off `0x1a key`, each
  note gets its own key-off event, and automatic key-offs become explicit
  (with `0x2e` at the start). A note's two events carry its id, so a note
  can be moved, resized or deleted alone. New notes go after the other
  commands of their tick (key-offs before them) and never past the track's
  end command.
- **Edits and undo** (`session.rs`, `edit.rs`): the session keeps the
  state of all edits (sequence, bank, mixer, chip, tempo, speed, track
  volume, loop) as of the last checkpoint; the window checkpoints every
  frame without a mouse button down, so each finished gesture is one undo
  step (300 kept). Selections, box select, group moves, nudges and the
  clipboard text are in `edit.rs`.
- **The driver** (`sqseq::Song`) owns its data (`Arc`), is `Clone`,
  counts the sequence tick it plays (`Song::tick`, which jumps back with
  the loops), and takes the studio's `Mods`: per track a volume factor
  (mute/solo), a pan offset, a transpose and a program, a tempo factor, a
  loop range and "no loop". With the default `Mods` it is the PS2 driver
  exactly: the converter's renders are byte for byte the same as before
  (checked on four tracks, 2-3 min each). For the studio it also has
  `release_all` (key-off of every voice: pause), `live_key_on` /
  `live_key_off` / `live_bend` (notes from outside the sequence) and
  `voice_tracks` (which voice plays for which track: the meters).
- **Loop range**: at the start of tick A the whole sequencer state is kept
  (all tracks with volumes, programs, LFOs; tempo; the song's own loop
  snapshot); at the start of tick B, or when the song ends first, it comes
  back and the notes sounding are released. That is not what the PS2 does
  at its loop point (it restores positions, keys and tempo only, `docs/
  music.md`), which is why the song's own loop is played by the driver's
  own logic when the markers are not moved.
- **The chip** (`spu2::Spu2`): shared sound RAM (`Arc`, so instruments can
  be appended while it plays), `Clone`, the interpolation mode, dry/wet
  gains (1.0 is the hardware's integer path), `reverb_off`, `skip` (the
  voices' envelopes and sample positions move on without output or reverb,
  for seeking: about 7x faster than rendering; a seek to 100 s takes
  0.2 s), and `voice_level` (envelope times volume: the meters).
- **The player** (`studio/src/player.rs`): driver and chip, the converter's
  output stage (`musicjob::Master`, gain and limiter), the playback speed
  (linear resampling) and the device's rate. Seeking restarts the song and
  skips to the tick; a new sequence (an edit) does the same at the current
  tick. Paused, it renders the chip alone (no interrupts of the sequencer)
  until a third of a second is silent (at most 20 s). The metronome is a
  25 ms decaying sine mixed in after the output stage when the sequence's
  tick enters a new beat. Per interrupt it keeps the loudest level of each
  track's voices for the meters.
- **Live notes** (`player::Live`): a second driver on its own chip (same
  sound RAM, no reverb) playing a sequence of one silent, never-ending
  track per channel; a key is a key-on of the channel's track with the
  channel's instrument, volume, pan and transpose as `TrackMod`s, so it
  sounds exactly as the channel's own notes do (velocity, ADSR, regions),
  apart from the channel's volume commands and LFOs. Its output is mixed in
  at the song's level (the +9 dB, not the limiter).
- **MIDI** (`midi.rs`): `midir` calls back on its own thread; the message
  is parsed, goes through the keyboard state (sustain pedal: keys let go
  are held until the pedal is up; a key struck again under the pedal is
  ended first; each sounding key remembers its channel) and the engine's
  commands are sent from there, so a note does not wait for the window. The
  window only reads what was played (lit keys, recording).
- **The audio thread** (`studio/src/audio.rs`): an engine thread owns the
  player and keeps a ~40 ms ring buffer filled; the device callback only
  copies from it (silence if it ever runs dry). Its output is the sum of
  the song (playing or fading), a stopped song's fading copy, the live
  notes and an instrument preview, times the output volume. The UI sends
  commands (load, play, pause, stop, seek, new sequence, new settings, new
  bank, preview, live notes, metronome, output volume) and reads a status
  (position, voices, programs, levels) at ~30 Hz; it never waits for the
  audio.

## `--render`, `--list`, `--survey`

```
ffx-music-studio [--config <file>] --list
ffx-music-studio [--config <file>] --survey [<track>]
ffx-music-studio [--config <file>] --render <track> <seconds> <out.wav> [edits...]
  --mute <ch>  --solo <ch>  --gain <ch> <x>  --pan <ch> <offset>
  --transpose <semitones>  --transpose-ch <ch> <semitones>
  --instrument <ch> <bank>:<instrument>
  --tempo <factor>  --speed <factor>  --master <x>
  --reverb <0-9|name>  --wet <x>  --dry <x>  --interp gaussian|linear|nearest
  --loop <tick> <tick>  --no-loop  --start <tick>
  --delete-notes <ch>  --move-notes <ch> <ticks> <semitones>
  --add-note <ch> <tick> <length> <key> <velocity>
  --normalize   --raw
```

`--render` plays through the same `Session` and `Player` as the window
(48 kHz WAV; `--master` is the track volume). Channels count from 0, as in
`docs/music.md`. `<track>` is an id, `musicNNN` or part of the name
(`battle_theme`). `--raw` writes the chip's output without the gain and
limiter, like `DEMASTER_RAW=1 demaster music`; `--normalize` re-encodes
every track in normalized form first.

`--survey` lists per track the channels with notes, the notes, the length,
the bank and the longest samples its notes start (length at the played
pitch, the rate it plays at, whether it loops); `--survey <track>` every
note and region of one track.

## The tracks with almost no notes

Some tracks show one or two channels with one or two notes. They are
sequences like the others, but their music is a recording: one note starts
a long sample of the track's own bank (usually a stereo pair, two regions
on two voices). From `--survey` on the German disc (channel 0 holds the
tempo; "2 channels" in the rack are channel 0 and the one note):

| track | notes | what plays |
|---|---|---|
| Prelude (1-03), music048 | 1 | a 48.9 s stereo sample at 16 kHz, looped by the song |
| Hum of the Fayth (1-20), music160 | 1 | 38.1 s stereo recording at 23 kHz |
| Hum of the Fayth one-shot (m-69), music148 | 1 | 23.9 s at 35 kHz |
| Hymn Valefor (1-23), music156 | 1 | 37.8 s at 23 kHz |
| Song of Prayer: Ifrit, Ixion, Shiva, Bahamut, Bodyguard, the Lonzo, Yunalesca, Spira, Anima (music150-159) | 1 each | one recording each, 37-42 s at 20.7-23.5 kHz |
| Inori Effect (m-01), music178 | 1 | 41.0 s at 21 kHz |
| music179 | 1 | 39.6 s at 21 kHz |
| music180 | 1 | 34.4 s mono at 44.1 kHz |
| music173 | 2 | two recordings at 12 kHz: a 30.0 s intro (C4) and a 115.8 s body (D4), looped from the second |
| music149 | 1 | the Hum of the Fayth, a 39 s mono recording at 22 kHz in 0x2000-byte pieces ("long samples in pieces", `docs/music.md`), looped by the song |
| Good Night one-shot (1-26), music143 | 1 | a 5.7 s stereo recording at 40 kHz, in pieces like music149 |
| music182, music183 | 8 | scales of 0.5 s notes (A and B major) on music180's recording, played at 74-83 kHz |
| Zanarkand (1-02), Game Over (1-11), Twilight (2-18) | 406, 126, 138 | real note sequences on two channels: solo piano (one multi-sampled piano, 1.2-2.4 s looped samples) |

None of these is one of the three tracks with streamed parts (Otherworld,
Confrontation, Jecht's theme: 6-35 channels, 665-3156 notes; their guitar
and vocals came from a separate disc stream) or of the ten whose banks are
missing (music052, 190, 191, 193..199: 4-18 channels, 345-3365 notes).

## Tests

`cargo test -p ffx-music-studio` (no disc needed): decoding and encoding,
note analysis, normalizing, note edits; a synthetic bank (a square wave) and
song played through the player: the round trip (disc bytes, re-encoded,
normalized: the same samples), mute/solo silence, transpose +12 doubles the
zero crossings, tempo x2 halves a note's start time, an added note sounds, a
deleted one is gone, half the velocity is half the level, a moved note sits
at its new time and pitch, the song's own loop period, the loop markers
(first jump at B, then every B - A, the music after a jump is the music at
A), no-loop ends, seeking, an instrument from another bank. And: pause
(the first 5 ms after it as loud as before, quieter every 20 ms, silent
within 0.5 s, the tick unchanged, playing on from it), stop's fading copy,
the per-channel levels, live notes (sound, velocity, key-off to silence,
transpose), the metronome's clicks on the beats, the export's length;
undo/redo (note edits, a slider drag as one step, reset as one step, redo
dropped by a new edit), box select, group moves kept in range, copy and
paste through the clipboard text, snapping, the wheel's modifiers and the
zoom around the pointer (rows and keys), MIDI messages (note on/off,
velocity 0, sustain, bend, all off) through the keyboard state (pedal,
re-strike, channel change while a key is down), the window settings file
(round trip, bad values), the alphabetical track order, the proposed
names, a long sample in 0x2000-byte pieces playing through (WD flag
`0x10`).

With the disc (`FFX_STUDIO_CONFIG=<demaster.toml>`): every sequence encodes
to its own bytes and keeps its notes when normalized; five tracks render
the same samples three ways (8 s); `--include-ignored` adds every track for
30 s.

Checked by hand: `ffx-music-studio --render <track> 120 x.wav --raw` (and
with `--normalize`) is byte for byte the first 120 s of `DEMASTER_RAW=1
demaster music` (battle theme, through its loop jump; game over); without
`--raw` it matches `demaster music` except the limiter's last 96 frames.

## Limits

- Ten sequences (`music052`, `190`, `191`, `193`..`199`) name banks that
  `wave_wd.sec` does not list and that are nowhere on the disc (`docs/
  music.md`, "Track notes", 2; the game never plays them). They open with
  an empty bank: the notes are there, the sound only with instruments from
  other banks.
- The three tracks whose guitar and vocals were streamed from the disc
  (Otherworld, Confrontation, Jecht's theme) play without them, as in the
  converter (`docs/music.md`).
- Counted loops (`0x04`/`0x05`, only in `music240`) are shown once on the
  timeline; the playhead follows the driver's tick count.
- Two notes of the same key overlapping in one channel end together (the
  driver releases every voice of a key); a note moved onto another of the
  same key can cut it short.
- Editing notes at the tick of the loop point puts them after the `0x02`
  (they play on every pass).
- Transpose and instrument changes take effect from the next note; volume
  and pan at once. Reverb changes clear the work area (a short dropout of
  the tail).
- Seeking replays the song silently from the start (fast, but a very long
  seek on a slow machine can take a fraction of a second); the reverb
  starts empty after a seek.
- The time display counts output time (loops included); bars assume 4/4.
- A preview while the song plays is mixed in at 48 kHz without resampling
  for other device rates.
- Live notes (strip, MIDI) come out after the ~40 ms output buffer plus the
  device's own latency; they use the channel's instrument, volume, pan and
  transpose but not its volume/expression commands or LFOs. Recording
  stamps notes with the playhead as the window last saw it (~30 Hz), then
  snaps them; recorded notes go into the sequence when the recording
  ends, not while it runs (each edit re-sequences).
- Resuming after a pause does not restart the notes that were sounding:
  the sequence goes on with its next commands.
