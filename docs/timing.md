# Timing: frame rates, waits and speeds, PS2 vs PC

The question: do some PS2/PC differences come from timing? The scripts
count `WaitFrames`, the PS2 ran at a fixed rate, the PC runs on a clock,
and PAL and NTSC are said to differ in speed.

Short answer:

| What | PS2 PAL (our disc) | PS2 NTSC / International | PC | Real difference? |
|---|---|---|---|---|
| Logic frames a second (field, battle, events) | 25 (2 of 50 vblanks; measured) | 29.97 (2 of 59.94 vblanks; same code) | 29.97 (a frame each 2 x 1/59.94 s) | PAL only: 17 % slower |
| Menus | 50 (1 vblank) | 59.94 | 59.94 | PAL only |
| When a frame takes longer (slowdown) | the game slows down | the game slows down | the PC runs extra logic steps to keep 29.97 a second | yes, in scenes where the PS2 dropped frames |
| `WaitFrames` counts | logic frames | logic frames | logic frames; movie frames in the 4 movies with a 3D scene | no (same unit) |
| Tidus's run speed | 34 (1.70 units a frame; 42.5 a second) | 34 (inferred, below) | 38 (1.90 a frame; 56.9 a second) | **yes**: `timing.run_speed` |
| Voice start | waits a length-dependent number of frames before the audio starts | starts when the stream is ready | as NTSC, plus per-voice delays for 29 lines | PAL only (and the PC's 29 lines) |
| Music, voices, movies | real time | real time | real time | no |
| Higher refresh rate, vsync off | - | - | no effect: the limiter runs on the clock | no |

Addresses: FFX.exe from `ffx-rust/symbols/ffx.sym`; PS2 from the German PAL
ELF (`jppc/prog/SLPM_675.13` on our disc = SCES_504.92's ELF), read with
capstone (scratch `ps2.py`) and in PCSX2 2.8.2 through PINE.

## The logic rate

**PS2.** `Sg_MainLoop` (`0x0011b2c0`) runs the update (`Sg_GetCurExecFrames`
times) and the draw, then calls `0x002d3ff8(2)` and `sceGsSwapDBuffDc`
(`0x002db648`) with `g_SgFrame` (`0x00307f2c`) + 1. `0x002d3ff8(n)` waits
(WaitSema on the vblank semaphore) until the vblank counter `0x0078b100`
reaches `n`, then for the next vblank, resets the counter to 1 and returns
the vblanks the frame took. So a frame is 2 vblanks, or more when the
frame's work took longer: the game slows down, it does not skip.

The vblank handler `0x0011ad90` adds 1 to `g_SgVCount` (`0x00307f30`) and
`0x00307f34` per vblank (the thread `docs/debugger.md` saw calling
`0x0011ad9c`). `Sg_MainCalcRate` (`0x0011b170`) turns the vblanks since the
last frame into the motion rate, `(vcount - last) << 7` (`0x00307f38`;
`0x100` for a 2-vblank frame) and its f32 (`0x00307f3c`), forced to
`0x100` when the exec frame count is not 1. Characters with keep-fps on
(`Ch_SetKeepFps`, and `Sg_SetKeepFps(1)` during battle effects) scale their
motion step by it, so their animation keeps its real-time speed through a
slowdown; everything else advances one step a frame.

The menu loop (`0x0021ff90`, next to `0x0021fe10`, which the main loop's
menu check `0x0011b0d0` calls) waits `0x002d3ff8(0)`: one vblank a frame.

**Measured** (PCSX2 2.8.2 at full speed, `field-kami0100` state, PINE reads
of `0x00307f2c`/`0x00307f30`/`0x00307f38` every 2 ms for 10.0 s): 250 logic
frames, 500 vblanks, rate `0x100` the whole time: **25.0 logic frames and
50 vblanks a second**. The NTSC International disc runs the same code at
59.94 Hz: 29.97 a second (not measured; we have no NTSC disc).

**PC.** `Game_Main`'s limiter (`0x0062ee50`) calls `App_Frame` once every
`g_FrameVsyncs` (`0x00c30e88`) x 0.016683 s (1/59.94) of
`Phyre_PTimerWin32_GetTime` and sleeps in between. `g_FrameVsyncs` is
written only by `SetFlipVSyncInterval` (`0x00625090`) through
`graphicSetFlipVsnc`: 2 in `Sg_MainInit`, 1 while the main menu runs
(`TkMenuMainLoop` `0x008a9ca0`), 2 in the sphere grid (`eiAbmStart`) and 1
again when it closes. So the PC runs the NTSC PS2's rates exactly: 29.97
logic frames a second, 59.94 in the menus. `g_SgVCount` gets + 2 a frame
(`Sg_FrameSync` `0x00821e80`), not real vblanks, so the motion rate is
always 1.0 (except in the sync scenes below).

**Late frames on the PC.** `Sg_FrameSync` caps the frame time at 0.3 s and,
when at least 2/60 s have built up and `Sg_CanSpeedUp` (`0x0081fd30`)
allows it, runs `g_VoiceHold` = (carry + dt) x 30 logic steps in that frame
(`updateFFX` `0x00822840` loops `Sg_MainLoop`). The logic keeps 29.97 steps
a second of real time; the PS2 slowed down instead. `Sg_CanSpeedUp` says no
for videos without the movie camera, some spells (magic `0x14a`, `0x14c`,
`0x17d` on map `0xd0`, `0x132`, `0xa9`, `0x1b5`, `0x1b6`, `0xcd`), some story
points and fades; there the PC slows down like the PS2. The same mechanism
runs the F1 speed-up (`SpdCtrl`).

## PAL: no speed compensation

The PAL ELF has one region switch: `0x002d9870` returns the byte
`0x00567eb1`, which is 1 in the ELF's data (read 1 in PCSX2) and has no
writer that is called. FFX.exe has the same function as `FFXGetVideoMode`
(`0x00887c40`, byte `0x01330831`), 0 unless the debug menu's item `0x2e`
sets it. Its nine PS2 callers:

| Caller | With PAL |
|---|---|
| `Sg_MainInit` `0x0011ade0` | GS mode 3 (PAL) instead of 2 for `sceGsResetGraph` (`0x002d98b0`) |
| `0x0011a5d0`, `0x00125e40` | display position (y offset `0x34` instead of `0x0e`), display mode |
| `0x00138aa8` | the play-time clock counts 50 vblanks a second instead of 60 |
| `0x001588f8`, `0x00158958` (movie function table `0x0030d4e4`) | the movie frame number for scripts and the movie camera: frames x 30 / 25 |
| `0x0015f800` | movie sound record of `0x7800` bytes instead of `0x6400` |
| `0x0015ddbc` | a movie (IPU) wait |
| `0x0012cca0` (`Native_VoiceStart` start) and `0x002d5ba8` (`SndCalcVoiceWait`) | the voice start wait (below) |
| `0x001ad140` | debug `host0:` set-up |

Nothing in movement, motion, waits or the frame rate. `Sg_MainCalcRate`
is the PC's (`0x80` per vblank, `0x100` for a normal frame), and the
player's speeds are compile-time constants (below). So the PAL release runs
every frame-counted thing at 25/29.97 = 83 % of the NTSC speed: walking and
running, animations, camera moves, fades, `WaitFrames`, battle animations.
Real time stays real time: the music (the IOP driver's 240 Hz timer,
`docs/music.md`), voices (streams), the 25 fps PAL movies, and the play
clock. The PAL port only kept those in step (movie frames x 30/25, play
clock, voice waits).

## Run speed

`Sg_PlayerInitDefault` (run at every event jump) sets the player's speeds
with `Sg_PlayerSetSpeed(kind, v)`: 1 walk, 2 run, `0x12` swim (`0x13` a
fourth).

| | walk | run | swim |
|---|---|---|---|
| PS2 PAL `0x001164b0` (`lui` constants) | 16 | **34** | 20 |
| PS2 PAL in memory (PCSX2, `0x0057f688..`, next to `g_Player` `0x0057f650`) | 16 | 34 | 20 |
| PC `0x0082d8d0` (`.rdata` `0x00b9231c`, `0x00b59830`, `0x00b4ae10`) | 16 | **38** | 20 |

`Sg_ControlPlayer` sets the character speed to `v x stick / 256`, and the
character moves `speed x 0.05` units a frame. **Measured** in PCSX2
(`field-kami-outside`, right held 180 frames, the player chr at
`*0x0057f650`, position at `+0x10`/`+0x18`, sampled every logic frame):
the run accelerates in steps of 0.2125 (0.638, 0.850, 1.062, 1.275, 1.488)
to **1.700 units a frame** (24 frames at exactly that), i.e. 34 x 0.05;
42.5 units a second at PAL's 25 frames. The PC's 38 gives 1.90 a frame,
56.9 a second at 29.97 (static: same code and constants; not measured in
the PC game).

**Which is original.** The PC's own event scripts (the same as the PS2's)
set the run speed back to **34** after their special runs: 21 calls
`PlayerSetSpeed(2, 34f)` in `bsil0100`, `mihn0000`-`0800`, `nagi0000`,
`lmyt0000`, `mtgz0800`, `dome0400`, `scene1` (and 102 for the chocobo
scenes, 16 for walking). They were written for a default of 34, so 34 is
the original's value and 38 is the remaster's. On the PC this also means
the run speed changes in the middle of the game: 38 after each map change,
34 in the scenes whose scripts reset it, until the next map change.

The encounter check runs on a walk timer (`g_BtlWalkTimer`,
`g_BtlWalkPeriod` 3.0, `MsBattleInitAtEvent` `0x00781c50`), apparently
time spent moving rather than distance (not traced further), so a faster
run would cover more ground between battles. That is why the setting
defaults to the PC's speed.

**Setting.** `timing.run_speed = "ps2"` (default `"hd"`): the PC's
`fld [0x00b59830]` at `0x0082d8e6` reads the `.rdata` 34.0 at `0x00b656c4`
instead (`mod/GamePatches.cs`). Checked: bytes and constants (static);
not yet run in the game.

## `WaitFrames` and the other waits

**PS2.** Native `0x0000` (table `0x00308bc0`, 16 bytes a native: start,
poll, -, int): start `0x00126d20` pops the count, poll `0x00126d50` is
`count -= 1; done when count < 1`, once per logic frame. No movie case.

**PC.** `Native_WaitFrames_Poll_0000` (`0x0085c6b0`): the same, except while
`g_WaitFramesByTime` (`0x0112a008`) is set and `movie_have_camera()`
(movies `0x20`, `0x2e`, `0x46`, `0x48`, the four with a 3D scene on top):
then it subtracts the movie frames played since the last call (`0x00645d90`:
`getCurrentFrame - +0x6e0`), so scripts stay on the video's frames.
`g_WaitFramesByTime` is set by `Native_b009_MovieCameraWait` and
`Native_b00a_MovieStartPicture` (the Farplane loop) and cleared by
`Native_b004_MovieClose` and `destroyMoviePicture`; it also gates the movie
player's pause, volume and skip prompt. The PC's video advances one frame
per update (`docs/movies.md`), so for normal movies both count the same.

The other waits (motion end, camera, move, message, voice) are polls that
run once per logic frame on both. **The unit is the same; the PS2 and PC
differ only where the PS2's frames took longer than 2 vblanks, and on PAL.**

## Slowdowns and the PC's sync data

The remaster knows the PS2's slowdowns mattered: three cutscenes with
voices and music replay the PS2's recorded frame timing.
`SyncManager` reads `GameData/PS3Data/SyncData/` (`SyncList.txt`,
`SyncDataList.txt`, `SyncVoiceList.txt`, `SyncData_<map>_...txt`);
`TkInitScene` loads the map's file (`iSyncDataChange(map)`). Each record is
(end time s, duration s, vblanks), the last `(1, 1, 999)`. While a file is
active (`g_SyncOn` `0x012fb8a0`), `Sg_FrameSync` steps the logic when the
real time passes each record's time (`Sg_SyncCutScene` `0x00821d60`) and
`Sg_MainCalcRate` takes the record's vblanks as the motion rate. Durations
are vblanks / 59.94: the recordings are of an NTSC PS2.

| Event (map id) | Steps | PS2 time | At a steady 30 fps | Frames that took 3+ vblanks |
|---|---|---|---|---|
| `mcyt0600` (80, Macalania spring) | 3,474 | 138.3 s | 115.9 s | 36 % |
| `dome0600` (270) | 3,673 | 131.1 s | 122.6 s | 14 % |
| `azit0300` (280) | 1,300 | 65.1 s | 43.4 s | 98 % (the scene ran at 20 fps) |

So in those scenes the NTSC PS2 took up to 50 % longer than 30 fps logic
would, and the voices and music were placed for that. The PC reproduces
these three. Everywhere else the PC runs steady 30 fps logic where the PS2
may have run slower: frame-driven parts (animations, camera, `WaitFrames`)
end sooner against real-time voices and music. Which other scenes dropped
frames on the PS2 is not known; finding them needs an NTSC PS2 run (a PAL
run measures PAL). Not a setting.

`SyncVoiceList.txt` adds per-voice delays (`VoiceTable_find(id, 1/2/3)`:
standby, start and after-start frames) for 29 voice lines of `mcyt0600` and
`azit0300`.

## Voice start (lip sync)

Scripts start a line with `VoiceStandby(id)` (stream read) and
`VoiceStart()`; mouths are script-driven, not voice-driven. `VoiceStart`'s
poll waits until the stream is ready (`SqsIopSoundSystemStatus` bit
`0x10000` clear, no movie), then counts down a number of frames; only its
end (`Native_VoiceStart_End` `0x0085de80`, `SndStartStream2`) starts the
audio. The countdown is `SndCalcVoiceWait()` (PS2 `0x002d5ba8`, PC
`0x00885d20`) when the script set none:
`(sectors x 2048 - 32) / 48000 x 16/28 x 2016/2048 x 4.97` frames, and 0
unless the video mode is PAL. So:

- **NTSC and PC**: the audio starts as soon as the stream is ready (plus the
  PC's per-voice delays for the 29 listed lines).
- **PAL**: the audio starts later, by a number of frames that grows with the
  line's length (about 5 frames a second of sound, the gap between 30 and
  25 frames; with the formula as compiled less: the `16/28` looks inverted).
  That lines up the end of a slowed PAL lip animation with the end of the
  line.

The forum reports that the HD Remaster's audio trails the lips by about
1/5 s. Nothing in the frame logic explains that: the PC starts the voice
on the same frame the NTSC PS2 does. A constant lag would have to come from
FMOD's stream start (buffering) after `SndStartStream2`. Not measured: it
needs an audio capture with frame times on the PC.

## Higher frame rates

The PC's limiter runs on the clock and `g_FrameVsyncs` is clamped to 1..2
(`SetFlipVSyncInterval`), so a 144 Hz display or vsync off changes nothing:
29.97 frames a second in the field and battle, 59.94 in the menus, and the
picture is drawn once per logic frame (no interpolation). Nothing runs
faster. The vblank emulation thread (`VsyncThreadImpl_run`) calls the
TkSyncV callbacks every 16 ms (62.5 a second, not 59.94); what those
drive on the PC was not checked.

## Which "PS2 original"?

- The game was made for NTSC: 29.97 logic frames, NTSC-timed movies,
  script waits and animation authored for 30 fps; the remaster's sync data
  are NTSC recordings.
- The PAL release is the same game slowed to 83 % (plus 50 Hz borders),
  with a few real-time fixes (movie frames, play clock, voice waits). It is
  what European players saw, and our disc and the user's comparison
  screenshots are PAL.

The defaults keep the PC's NTSC timing. Whether to offer PAL speed is the
user's call.

## What a `timing.pal_speed` would take

Not implemented. All of these, together:

1. **Frame interval**: `Game_Main`'s limiter uses `g_FrameVsyncs x
   0.016683333333333335` (an f64 in the code) and `1/59.94` at start; make
   a frame 0.04 s (menus 0.02 s).
2. **Catch-up**: `Sg_FrameSync`'s `x 60` / `x 30` (`due`, `g_VoiceHold`)
   would have to become 50 / 25, or slow frames would be run twice.
3. **Movies**: the PC video advances one frame per update, so the movies job
   would have to write 25 fps files (no duplication to 29.97), and the four
   camera movies (HD files at 29.97) would play slower than their sound.
4. **Voices**: `FFXGetVideoMode` = 1 (byte `0x01330831`) for the PAL voice
   wait; it also switches `SqsSetCycleMode(1)` (the sound driver) and the
   PS2 movie viewer's sound size, which need checking on PC.
5. **Sync scenes**: the three SyncData scenes replay NTSC wall-clock times;
   at 25 fps they would stall or run extra steps. Turn sync off for them.
6. **Speed-up and timers**: the F1 speed-up, the battle menu timers in
   seconds, the play clock (real time on both).

Risk: medium to high (movies and sync scenes); it would change no picture,
only speed.

## Not done

- A cutscene timed side by side (PS2 in PCSX2 vs PC): needs a voiced scene
  reachable from a save on both; the measurements above are the PS2 rate,
  the run, and static reading of both binaries.
- The PC run speed and logic rate were not measured in the PC game, only
  read from the code.
- The 1/5 s lip lag: needs an audio capture.

## Tools

Scratch (not in git): `~/ffx-work/demaster/scratch/timing/` `ps2.py`/`q.py`
(PS2 ELF disassembly: `fnl`, `range`, `callargs`, `jrefs`), `pine.py`
(`sample <s> <addr>[:f]...`: PINE reads every 2 ms), `run.sh` (hold a pad
key while sampling), `vbf.py` (VBF list/extract), `syncdata.py`.
