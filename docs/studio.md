# FFX Music Studio (`studio/`, `ffx-music-studio.exe`)

A player and experimental editor for the PS2 music of your own FFX disc.
It plays the disc's `musicNNN.bgm` sequences through the converter's port
of the PS2 sound driver on its SPU2 model (`docs/music.md`), the same path
that renders the "Original" soundtrack bank, and lets you change them while
they play. Nothing is saved: edits live in memory until you pick another
track or press *Reset track*.

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

`--open <track> [--tab roll|mixer|chip|song] [--seek <tick>] [--paused]
[--master <x>]` starts with a track open (used for the screenshots).

On Windows the sound goes to the default output device (WASAPI, shared
mode, ~40 ms of buffer). The Linux build has no sound output unless built
with `--features alsa` (needs ALSA's development files); without it a clock
drains the audio at the real rate, so everything else works.

## The window

**Track list** (left): every `musicNNN.bgm` on the disc (112 on the German
disc). The 89 that the PC's "Original" soundtrack bank uses carry the bank's
names ("Battle Theme (1-09)", disc and track number of the soundtrack)
from `musicjob::TRACKS`, in the soundtrack's order; the others follow as
`musicNNN`. The filter matches names and ids. A click loads the track and
plays it.

**Transport** (top): play/pause (also Space), stop, *Loop* (jump back at
the end; off: play to the end and stop), the elapsed time (loops included),
bar.beat and tick of the sequence, a seek slider over the sequence's ticks,
*Tempo* (the sequencer's speed: the driver's tempo counter runs faster or
slower, the pitch stays), *Speed* (the output resampled: tempo and pitch
together), *Master* (before the limiter), *Reset track*, and the voices in
use out of 48. "re-sequence N ms" shows how long the last reload took.

**Piano roll**: every channel's notes on the sequence's timeline (48 ticks
a quarter note, bars of four beats), the active channel bright (brightness
by velocity), the others dim (*all channels* off hides them). Below, the
active channel's velocities. In the ruler, the loop markers A (green) and
B (red), and the playhead (yellow); the part after the song's end is
shaded. The view follows the playhead while playing (*follow*).

- click on empty space: a new note (length and velocity of the last note
  edited, 1/8 and 100 at first), snapped to the grid (*Snap*);
- drag a note: move it in time and pitch; drag its right end: length;
- right click: delete it (or select and press Delete);
- drag a bar in the velocity lane: its velocity;
- the selected note's key, start, length and velocity can also be typed
  in the toolbar;
- click in the ruler: seek; drag A or B: the loop range;
- wheel: up and down the keys; Shift+wheel (or a horizontal wheel): along
  the time; Ctrl+wheel: zoom.

An edit re-encodes the sequence and the player restarts it silently up to
the current tick ("re-sequence", typically 10-200 ms), so the change is
heard at once.

**Mixer & instruments**: per channel the voices the driver has given it
now, its current program (instrument), mute, solo, volume (x0..x2), pan
(added to the driver's pan position, ±64), transpose (semitones, new notes),
and the instrument. On the right, the instrument banks of `wave_wd.dat`
(the track's own first): pick a bank, then an instrument (its regions and
key ranges); *Play* previews it at the *preview key* for two beats through
the same driver (without reverb). *use selected* gives the instrument to a
channel; *x* goes back to the channel's own. An instrument from another
bank is appended to the track's bank in memory, with that bank's samples
(the sound RAM is a model, so there is no 2 MB limit).

**Sound chip**: the reverb preset (the song's, off, or one of libsd's nine:
room, studio A/B/C, hall, space, echo, delay, pipe, with the registers and
work-area sizes read from the disc's `libsd.irx`; switching clears the
work area), wet (the reverb return) and dry gains, the interpolation
(Gaussian is the SPU2's 4-tap; linear and nearest are for comparison), and
the master volume. The output stage after the chip is the converter's
(+9 dB, limiter at -1 dBFS).

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
- **The driver** (`sqseq::Song`) now owns its data (`Arc`), is `Clone`,
  counts the sequence tick it plays (`Song::tick`, which jumps back with
  the loops), and takes the studio's `Mods`: per track a volume factor
  (mute/solo), a pan offset, a transpose and a program, a tempo factor, a
  loop range and "no loop". With the default `Mods` it is the PS2 driver
  exactly: the converter's renders are byte for byte the same as before
  (checked on four tracks, 2-3 min each).
- **Loop range**: at the start of tick A the whole sequencer state is kept
  (all tracks with volumes, programs, LFOs; tempo; the song's own loop
  snapshot); at the start of tick B, or when the song ends first, it comes
  back and the notes sounding are released. That is not what the PS2 does
  at its loop point (it restores positions, keys and tempo only, `docs/
  music.md`), which is why the song's own loop is played by the driver's
  own logic when the markers are not moved.
- **The chip** (`spu2::Spu2`): shared sound RAM (`Arc`, so instruments can
  be appended while it plays), `Clone`, the interpolation mode, dry/wet
  gains (1.0 is the hardware's integer path), `reverb_off`, and `skip` (the
  voices' envelopes and sample positions move on without output or reverb,
  for seeking: about 7x faster than rendering; a seek to 100 s takes
  0.2 s).
- **The player** (`studio/src/player.rs`): driver and chip, the converter's
  output stage (`musicjob::Master`, gain and limiter), the playback speed
  (linear resampling) and the device's rate. Seeking restarts the song and
  skips to the tick; a new sequence (an edit) does the same at the current
  tick.
- **The audio thread** (`studio/src/audio.rs`): an engine thread owns the
  player and keeps a ~40 ms ring buffer filled; the device callback only
  copies from it (silence if it ever runs dry). The UI sends commands
  (load, play, seek, new sequence, new settings, new bank, preview) and
  reads a status (position, voices, programs) at ~30 Hz; it never waits for
  the audio.

## `--render` and `--list`

```
ffx-music-studio [--config <file>] --list
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

It renders through the same `Session` and `Player` as the window (48 kHz
WAV). Channels count from 0, as in `docs/music.md`. `<track>` is an id,
`musicNNN` or part of the name (`battle_theme`). `--raw` writes the chip's
output without the gain and limiter, like `DEMASTER_RAW=1 demaster music`;
`--normalize` re-encodes every track in normalized form first.

## Tests

`cargo test -p ffx-music-studio` (no disc needed): decoding and encoding,
note analysis, normalizing, note edits; a synthetic bank (a square wave) and
song played through the player: the round trip (disc bytes, re-encoded,
normalized: the same samples), mute/solo silence, transpose +12 doubles the
zero crossings, tempo x2 halves a note's start time, an added note sounds, a
deleted one is gone, half the velocity is half the level, a moved note sits
at its new time and pitch, the song's own loop period, the loop markers
(first jump at B, then every B - A, the music after a jump is the music at
A), no-loop ends, seeking, an instrument from another bank.

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
  `wave_wd.sec` does not list. They open with an empty bank: the notes are
  there, the sound only with instruments from other banks.
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
