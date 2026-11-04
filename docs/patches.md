# Code patches

The mod's changes to `FFX.exe`'s code (`mod/GamePatches.cs`, keyed in
`mod/Patches.cs` by their `demaster.toml` setting). Addresses are VAs of
ffx-rust's `symbols/ffx.sym` (image base `0x00400000`; Fahrenheit's
`FhCallLocation` takes VA − `0x400000`, the file offset in `.text` is
VA − `0x400c00`). A byte patch is written only when the bytes there are the
original ones listed here, so another game build or another mod's patch at
the same place is left alone and the patch shows as *failed* in the log and
the F8 window.

Under Fahrenheit `FFX.exe` is relocated (in the 2026-11-02 runs its image
sat `0x670000` above `0x00400000`), so absolute addresses inside code bytes
move: the subtitle patch compares its operand against
`Native.reloc(0x0113079c)`, not the ffx.sym value. Fahrenheit also refuses
`FhCallHandle.fnptr` until every module's `init` is done, so hooks fetch the
functions they call on their first run.

How each was checked is at the end of its section; "static" means read from
the code, not seen in the game.

## `ui.battle_menu_delay` (row 18)

**What the PC does.** Each battle window has a per-frame input step
(`w+0x9c`, set by `TOBtlSetCallback` `0x008a0de0`). The PC's steps,
`TOWaitBtlWin*` at `0x008a87a0..0x008a8bd0`, read the pad only once the
window has been open long enough: `f32 w+0xe0` (open time, `graphicGetTime() -
w+0xdc`) against f32 0.3 (`0x00b3eb44`) or f64 0.45 (`0x00b43d50`, a widened
0.45f). Before that the window ignores every button.

**The patch.** The constants are shared with other code (0.3 has ~25
readers), so the branch that skips the pad read goes instead:

| VA | Function | Delay | Bytes |
|---|---|---|---|
| `0x008a87b7` | `TOWaitBtlWinChange` | 0.3 s | `74 09` → `90 90` |
| `0x008a8a57` | `TOWaitBtlWinLeft` | 0.45 s | `7b 09` → `90 90` |
| `0x008a8aa1` | `TOWaitBtlWinMain` (the command list) | 0.45 s | `7b 09` → `90 90` |
| `0x008a8ad7` | `TOWaitBtlWinPrepare` | 0.3 s | `74 09` → `90 90` |
| `0x008a8b07` | `TOWaitBtlWinRight` | 0.45 s | `7b 09` → `90 90` |
| `0x008a8b37` | `TOWaitBtlWinStd` (magic, items, skills, targets lists) | 0.3 s | `74 09` → `90 90` |
| `0x008a8b67` | `TOWaitBtlWinStdMode2` (summons) | 0.45 s | `7b 1d` → `90 90` |
| `0x008a8ba7` | `TOWaitBtlWinStdMode3` (weapons) | 0.45 s | `7a 04` → `eb 04` |

All eight are checked before any is written.

**The 0.2 s.** The PC's window opening is drawn over time too:
`TODrawWaitBtlWinMain`/`Right`/`Left`/`Change` show only the opening plate
while the open time is below 0.2 s (f64 `0x00b3eb60`, shared with ~20 other
functions), then the rows slide in 0.1 s apart. That is the HD window's
animation, not an input gate; the patch leaves it, and input works during it.
Whether Nexus mod 360 ("No more battle menu delay", WAZAAAAA) also shortens
it could not be read (its page answers 403 to the fetch tool).

**Checked:** static (the eight compares and their branch senses, x87 status
bits worked through: C0 for "below"; a NaN runs the pad read in every one).
In the Fahrenheit run (2026-11-02) all eight sites held the original bytes
and were patched, and the game ran on. Not measured in a battle: the harness
can't write code, and the Fahrenheit test install has no input driver.

## `ui.battle_subtitles` (row 17)

**Why the PC shows none.** Battle scripts call native `0x70de` (handler
`0x007a79b0`, a jump to `0x00785210`) before each voiced line; when it answers
1 they draw the line with `0x7072` (`TODrawAMES_EVJIMAKU` `0x0089b230`), wait
with `0x70a9` and close it with `0x7044`; when it answers 0 only the voice
plays. 76 of the 863 battle scripts do this (`lchb02_00` "Whoa! What are
they?", `cdsp02_00`, `klyt00_*`, `mihn04_*`, `bsil05/07_*`, `kino02_00`).
`0x00785210` (the getter twin of `MsSetSaveConfigSubTitle`):

```
cmp [0x01133354], 0   ; g_BtlMuteSubtitle: the PC's speed-up booster in battle
je  +3
xor eax, eax / ret
mov eax, [0x0113079c] ; g_SaveMenu
sar eax, 9 / and eax, 1 / ret
```

Bit 9 of `g_SaveMenu` is the PS2's subtitle option (Fahrenheit's
`savedata.cs`: `config_subtitles`). `g_SaveMenu` lies inside `g_SaveData`,
so it comes from the save. Its setter `MsSetSaveConfigSubTitle`
(`0x00785f80`) has no caller and the PC config menu has no subtitle entry,
**but the bit is set anyway**: the new-game save the game builds at start-up
has `g_SaveMenu` = `0x00002f61` (Fahrenheit run, 2026-11-02), a PC save
(harness slot 5) `0x00006763`, the developer save 22 `0x0001a763`; bit 9
(`0x200`) is set in all three. So on the stock PC this scripted battle
dialogue *is* subtitled, except while the speed-up booster mutes it.

**The patch.** `0x0078521c`: `a1 9c 07 13 01` (`mov eax, [g_SaveMenu]`) →
`b8 00 02 00 00` (`mov eax, 0x200`): the getter answers 1 unless the
speed-up mutes it, as the PC intends. With the measured saves this changes
nothing; it only covers a save whose bit 9 is clear. Writing the save bit
instead would end up in the player's saves and be reset by every load.

**Open: what row 17 means.** The research row cites only the Final Fantasy
Wiki ("battle quotes not displayed"). The scripted lines above are not it.
The likely candidates are the battle system's own shouts (FMOD command 6,
`FfxFmod_shoutStart` `0x00706fd0` → `FmodVoice::shoutStart`: kill and
overdrive quotes), which have no text path found yet, or lines the speed-up
mutes. Needs a concrete line that International subtitles and the PC
doesn't, then its play path.

**Checked:** the mechanism in the game through the ffx-rust harness
(`tools/replays/battle-subtitles.txt`, which sets the same bit through data,
since the harness can't write code): developer save 22, the Luca docks'
battle `lchb02_00`. With bit 9 cleared the opening's voiced lines play with no
text; with it set, "Tidus: Whoa! What are they?", "Lulu: Ancient machina
salvaged by the Al Bhed." and "Lulu: They are mostly vulnerable to
lightning." show at the bottom, then the command window comes as usual. The
developer save itself has bit 9 set (`g_SaveMenu` `0x0001a763`, a PS2-era
save). The code patch itself: bytes matched and written in the Fahrenheit
run; not watched in a battle there.

## `audio.soundtrack` (row 20)

**Where the choice lives.** In the save: `g_SaveData` (`0x0112ca90`,
`MsGetSaveEventAddress`) byte `+0x5ec`, 0 = arranged, 1 = original.
`iMusicRefreshBgmType` (`0x008cc170`) queues FMOD command `0x2e`
(`FmodMusic_switchBGM`), and set 1 is the second event project,
`Music_PS2/ffx_music_PS2.fev` (`FmodMusic_initData` loads `Music/ffx_music.fev`
into `+0` and `Music_PS2/...` into `+4`; `FmodMusic_read` takes `+4` when
`+0x2c` is 1). Setters: the in-game config (`MenuConfig_SetBgmType`
`0x008cb0b0`) and script natives `0x0264`/`0x0267` (`SndSetBgmTypeSave`,
used by `lchb1200`'s choice). Loading a save refreshes it
(`TkMenuJumpToLoadedScene`).

**The patch (apply once, don't force).** A hook on `MsSetSaveStartGame`
(`0x00786b00`, cdecl `void()`), which builds the new-game save (called by
`MsBattleInit` from `Sg_MainInit` at start-up): after it, byte `+0x5ec` = 1
and `iMusicRefreshBgmType()` (only when `g_FmodManager` `0x00ce9010` exists;
`iSoundInit` runs before `MsBattleInit`). So a New Game starts with the
original soundtrack, a loaded save keeps the choice it was saved with, and
the player can switch in the game's menu at any time; the choice is saved.
Forcing it on every load was rejected: it would fight the player's menu
choice.

**Checked:** in the Fahrenheit run the hook ran at start-up (3 s after the
logos were skipped) with `g_FmodManager` already set, so the switch was
queued. The music itself not heard, and a New Game not started (no input in
the test install).

## `ui.loading`: splash videos (row 16)

**What plays.** The Square Enix / Virtuos / CRI / Dolby logos are one Flash
movie, `/FFX_Data/GameData/PS3Data/flash/OpeningScreen.swf`, opened by
`PhyreScene_beginRenderOpeningScreen` (`0x006579d0`; it sets
`g_PhyreSceneOpeningPlaying` `0x00ccb9c2`, cleared by the movie's `Finish`
callback `0x00652400`). `graphicInitScene` (`0x006416c0`) loops
`PInput_Update` + `PhyreScene_renderOpeningScreen` while
`PhyreScene_isOpeningScreenPlaying` (`0x00660400`, `mov al, [0x00ccb9c2];
ret`) says so, then deletes the player (`0x0065b750`) and waits for the
loading (the loading screen, whose textures the converter makes
transparent).

**The patch.** A hook on `PhyreScene_isOpeningScreenPlaying` (its only
caller is that loop) returning 0: the loop never runs; the player is created
and deleted unseen. ffx-rust's harness ends the logos the same way (it
clears the flag after each flip, `docs/host.md`), so the rest of the
start-up is known to cope.

**Checked:** in the Fahrenheit runs the hook answered (log line 7 s after
the mod's init) and the start-up went on: the new-game save was built 2 s
later and the game ran until the 60 s timeout without errors. Not looked at
on screen (no window capture of the Fahrenheit game).

## `timing.run_speed` (row 23)

`Sg_PlayerInitDefault` (`0x0082d8d0`, at every event jump) sets the run
speed from the `.rdata` 38.0 at `0x00b59830` (`d9 05` at `0x0082d8e6`); the
PS2's is 34 (`docs/timing.md`, "Run speed"). With `"ps2"` the operand at
`0x0082d8e8` points at the `.rdata` 34.0 at `0x00b656c4` (relocated
addresses, as `ui.battle_subtitles`); both constants are checked first.
Default `"hd"`.

**Checked:** static (bytes and constants in `FFX.exe` sha256 `0537b2a1…`).
Not run in the game.

## `fixes.farplane_video`

**Why it never plays.** The Farplane (`ikai0800`, worker 25, entry 0) starts
its background loop with native `0xb00a` (`MovieStartPicture`, handler
`0x0076eb80`): movie 104 (`FAL_jp.webm`), flags `0x480` (`0x80` = loop). The
only other users are the dev maps `test01`/`test11`. The handler:

```
76eba9: call graphicInitFMVPlayer(movie, flags & 0x80)   ; 0x00641690
76ebb2: test eax, eax
76ebb4: je 0x76ebde                                      ; 74 28
76ebb6: graphicPlayVideo(0); g_WaitFramesByTime 0x0112a008 = 1,
        0x00c40e2c = 1, g_MoviePictureOn 0x0112a00c = 1
```

`graphicInitFMVPlayer` returns `PhyFMVPlayerManager_initialize`'s eax, and
every exit of that (`0x006d9c80`) passes `0x006da184: xor eax, eax`: always 0.
The movie is loaded and never played. Normal FMVs (`Native_b000`) ignore the
result, which is why they work.

**The patch.** `0x0076ebb4`: `74 28` → `90 90`. Nexus mod 355 fixed it with
an exe patch too. Its v1.1 also mentions Farplane particles missing while
the video plays; not looked at.

**Checked:** static (byte-level); the bytes matched in the Fahrenheit run.
Not seen in the game (needs a save at the Farplane).

## `fixes.lulu_fury`

**How spins are counted.** `TOBtlCtrlLuluLimitWindow` (`0x00891b80`),
state 4 (i16 `0x0133d708`) with the stick (u32 `0x00c58ef0` != 0; else the
keyboard path `setLuluODKeyboardInput`): each call reads the right stick
twice, `TkGetControlerAnalogHist(0, 0, axis, hist)` (`0x00888c20`, a pure
read) with hist −1 and 0, and turns each reading into a diagonal code: 4
(x ≥ 0xa0, y < 0x60), 5 (x ≥ 0xa0, y ≥ 0xa0), 6 (x < 0x60, y ≥ 0xa0), 7
(x < 0x60, y < 0x60). A code other than the last one in the buffer
(`0x0133d721`, count i8 `0x0133d731`) is appended. With 5 codes the first 5
are compared with a circle either way (from any start), and the buffer is
emptied either way; a circle raises the counter `0x0133d71e` and the level
`0x0133d71f` (table `0x0133f798`).

**Why the PC loses spins.** `TOControlBtlWindow` calls the window twice a
frame while Fury runs, meant for 60 Hz samples from the async input thread,
but that thread never reads XInput (`PInput::Update`, once per frame, is the
only path to it), so both samples are one reading or neutral. About one real
reading a frame: a diagonal zone is ~61° wide, so any spin faster than about
5 turns a second skips a zone (4 → 6), and one skipped zone fails the whole
5-code circle.

**The patch.** A hook on `TOBtlCtrlLuluLimitWindow` (cdecl `void()`). In
state 4 with the stick it reads the same two samples the original will read,
works out which codes the original will append, and when one is two zones
from the previous code and the spin's direction is known (from earlier
neighbouring codes), puts the skipped diagonal into the buffer: before the
original runs for the first new code, right after it (in front of the
appended code) for the second. Everything else, matching, counting, levels,
gauge, timer, stays the game's. Nothing is counted that the stick did not
pass through: a jump of two zones is only filled in the direction the stick
was already turning. The log reports how many diagonals were filled per
Fury.

**Checked:** the hook installs in the Fahrenheit run. The logic against a
Python model of the original's state-4 code (`0x00891cd2..0x00891f60`: two
samples per call, two calls per frame, one stick reading per frame), 300
frames, speed jittered ±20 %, 20 runs each, circles counted:

| °/frame (turns/s at 30 fps) | 30 (2.5) | 60 (5.0) | 70 (5.8) | 90 (7.5) | 110 (9.2) | 120 (10) | 150 (12.5) |
|---|---|---|---|---|---|---|---|
| original | 24.4 | 39.5 | 25.8 | 33.9 | 11.4 | 0.6 | 0.2 |
| with the fill | 24.4 | 48.9 | 50.8 | 45.6 | 18.6 | 12.1 | 42.1 |

It never counts fewer than the original. Not tested with a real stick: the
harness has no analog input; a test needs the stick faked (e.g. hooking
`inputGetCurrDeviceStickInfo` `0x00630c90` to turn a fixed angle per frame)
in a battle with Lulu's gauge full. Open: the PS2's own sampling rate for
this window (and so how fast a spin it accepted) is not known.

## `fixes.npc_mouths`: not patched

**How mouths work.** Scripts switch talking with natives `0x5076`, `0x507e`,
`0x7110`/`0x7122` (on) and `0x7111` (off): `chr+0x4d4`. `Ch_Mouth`
(`0x008344f0`, from `Ch_CalcMiddle_Batch`) moves the jaw target `+0x4de`
(random while talking, else 0), steps the angle `+0x4dc` towards it, rotates
bone 13 from the rest pose at `*(chr+0x1c4)+0x44` by `+0x4dc/10`°, and for
models with mouth textures (`chr+0x6fc`) sets the texture-animation frame to
open when the angle is above `g_ChrMouthTh` (`0x00c49718`, 10 = 1°). Closing
always ends with angle 0 and frame 0. Nothing PC-specific was found in this
path.

**Source.** The only report is one uncited line in the Final Fantasy Wiki's
"Final Fantasy X version differences" ("Several NPCs erroneously have their
mouths open during certain cutscenes"); forum threads only discuss lip-sync
timing (checked 2026-11-04).

**Why not patched.** No reproducible case is known (which NPC, which
event); none of the bug reports found names one. Two candidates, both
unproven: (1) remodelled HD NPCs whose jaw bone's rest pose differs from the
PS2 rest the code writes, which would leave the HD jaw open after the first
line; (2) the HD `_anim_` mouth material whose frame 0 is an open mouth. With
`models.npc = "ps2"` the converted NPCs use the PS2 skeleton and textures,
which would avoid (1). Next step: a case, then a trace of `chr+0x4d4`,
`+0x4dc`, `+0x4de` and the texture-animation frame during it.
