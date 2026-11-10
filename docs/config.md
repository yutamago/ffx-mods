# Settings

Every difference between the HD Remaster and the PS2 original that the
PS2 mods restore is a setting, and so is every change of the other mods. **Each mod has its own settings file,
`settings.toml` in its folder (`mods/yuta.<name>/`), with only its
settings**, also in the PS2 bundle. The zips ship the defaults as
`settings.default.toml`, never `settings.toml`: `install.cmd` makes
`settings.toml` from it when there is none, and adds the settings a newer
version brought (with their comments and defaults) to an existing one
without changing its values, so unpacking a newer zip keeps the player's
settings. A key the file leaves out has the mod's default (the C#
`ModSettings.get`, the converter's `Config::parse_with_defaults`); a mod
without `settings.toml` (install.cmd not run yet) uses its defaults. Every row of
[`differences.md`](differences.md) has a setting here. (`demaster/demaster.toml`
holds every setting in one file, for `demaster build` without a mod: every
job into one folder, for development.)

| Mod | Settings |
|---|---|
| `yuta.ps2-music-engine` | `audio.music_engine` |
| `yuta.ps2-soundtrack` | `audio.music`, `audio.soundtrack` |
| `yuta.ps2-movies` | `movies` (reads `render.aspect` of `yuta.ps2-aspect-ratio`) |
| `yuta.ps2-characters` | `models.party`, `models.npc`, `models.monster`, `models.aeon`, `models.weapon`, `models.object`, `render.texture_filter`, `render.texture_quality` |
| `yuta.ps2-maps` | `maps.field`, `maps.battle`, `maps.backgrounds` (reads `render.aspect` of `yuta.ps2-aspect-ratio`) |
| `yuta.ps2-effects` | `effects.textures` |
| `yuta.ps2-lighting` | `render.lighting` |
| `yuta.no-hd-post-effects` | `render.glow`, `render.shadows` |
| `yuta.ps2-interface` | `ui.menus`, `ui.portraits`, `ui.font`, `ui.title`, `ui.loading`, `ui.button_glyphs`, `ui.boot_logo` |
| `yuta.ps2-aspect-ratio` | `render.aspect` |
| `yuta.ps2-game-feel` | `timing.run_speed`, `ui.battle_menu_delay`, `ui.battle_subtitles` |
| `yuta.bug-fixes` | `fixes.npc_mouths`, `fixes.farplane_video`, `fixes.lulu_fury` |
| `yuta.modern-minimap` | `minimap.style`, `minimap.size`, `minimap.corner` (not a PS2 part; see [Minimap](#minimap)) |
| `yuta.fps-unlock` | `frames.*` (not a PS2 part; see [FPS unlock](#fps-unlock)) |
| none yet | `maps.geometry` (nothing to build yet) |

**Settings of another mod.** No mod reads another mod's settings, with one
exception the converter handles: the PS2 movies and the 2D rooms'
backgrounds are built for the picture the game will show, so the `movies`
and `maps.backgrounds` jobs read `render.aspect` from PS2 aspect ratio's
`settings.toml` when that mod is installed (in `mods\loadorder`), and take
`"hd"` (16:9) when it isn't (`mods.rs`, `Foreign`). The build stamp records
the value; when PS2 aspect ratio is installed, removed or switched later,
the F8 window shows PS2 movies and PS2 maps as "rebuild needed".

The converter's list is `demaster/src/mods.rs`; each mod's own is its
`mods/<name>/Mod.cs` (a test keeps them, the shipped `settings.default.toml`
and this page in step). A key in a mod's file that isn't one of its settings
is ignored with a warning (the converter's output and the F8 window).

**The rule for defaults of the PS2 mods: presentation from the PS2, logic from the PC.**
Presentation settings are `"ps2"` (the default) or `"hd"` (the HD
Remaster). Fixes are `true` (fixed, the default) or `false` (as on PC).
Game logic (balance, Eternal Calm, boosters, controls, PC text and voices,
30 fps) always stays the PC's.

**After changing a setting:** for a mod with PS2 files (music engine,
soundtrack, movies, characters, maps, effects, lighting, interface), run
its `install.cmd` again (or, for every mod at once, `install.cmd` in
`mods\yuta.ffx-mods\`, in FFX Demaster) (it runs
`demaster.exe build`, which redoes only what changed); then restart the
game. Fahrenheit picks up the mods' files only when the game starts, and
the mods read their settings only then. In the game, F8 shows every mod
of the family and whether its built files match its settings.

**Status.** Not every setting does something yet; the
*Status* column below is as of this version.

Who applies a setting:
- **converter**: `demaster.exe build` (run by `install.cmd`) writes PS2
  files into the mod's `efl/x/`. With `"hd"` it writes none and removes
  those of an earlier build.
- **mod**: code in the mod's DLL, applied when the game starts.

## Paths

| Key | Meaning |
|---|---|
| `source.iso` | Your PS2 disc of Final Fantasy X, any region: an image file (`.iso`), or on Windows the drive with the disc in it (`'E:'`, read as the raw volume `\\.\E:`). Required for the mods that build PS2 files, except PS2 lighting (without it the event lights stay HD). `install.cmd` asks and writes it here. |
| `source.game` | The game folder, the one with `FFX.exe` and `data\FFX_Data.vbf`. Empty: the converter looks in the usual Steam folders (`steamapps\common\FINAL FANTASY FFX&FFX-2 HD Remaster` in every Steam library). |
| `source.ffmpeg` | ffmpeg, which converts the movies (not included). Empty: `ffmpeg.exe` next to `demaster.exe` or in `tools\`, else the one on your PATH; on Windows, if there is none, the converter downloads an LGPL build of FFmpeg 8.1.3 into `mods\yuta.demaster\tools\` (168 MB) and checks it (`docs/mods/movies.md`). |
| `build.mod_dir` | Where the files go. Empty: the folder `demaster.exe` is in, which is right when it sits in the mod's folder (`mods\yuta.<name>`). |

Relative paths are relative to the folder of the settings file. On Windows,
write paths in single quotes (`iso = 'D:\Games\FFX.iso'`): inside double
quotes a backslash starts an escape sequence.

The command line overrides the file: `demaster.exe build --mod <name>
--iso <file or drive> --game <folder> --mod-dir <folder> --ffmpeg <ffmpeg.exe>
--config <file>`. `--mod` picks the mod (without it: the mod whose manifest
is in the mod folder, else every job from `demaster.toml`). `--all`
builds every installed mod with PS2 files, each with its own settings,
from one pass over the disc (FFX Demaster's installer runs it). `--no-disc` builds
without the disc (PS2 lighting); `--save-source` writes `--iso`/`--game`
into the settings file (as `install.cmd` does). More options:
`--force` rebuilds jobs that are unchanged, `--only chr.monster,chr.npc`
runs just those jobs (`demaster.exe jobs` lists them), `--verbose` prints
every file.

## Presentation (default `"ps2"`)

| Key | Row | `"ps2"` | `"hd"` | Applied by | Status |
|---|---|---|---|---|---|
| `models.party` | 1 | PS2 party models | the remodelled party | converter (`chr.party`) | texture swap of kept meshes only (one model); mesh conversion planned |
| `models.npc` | 2 | PS2 NPC textures on the kept meshes, later the PS2 meshes | HD | converter (`chr.npc`) | textures: done |
| `models.monster` | 3 | PS2 monster textures (and meshes, later) | HD | converter (`chr.monster`) | textures: done |
| `models.aeon` | 4 | PS2 aeon textures | HD | converter (`chr.aeon`) | textures: done |
| `models.weapon` | 4 | PS2 weapon textures | HD | converter (`chr.weapon`) | textures: done |
| `models.object` | 4 | PS2 object textures (props, skeleton dummies) | HD | converter (`chr.object`) | textures: done |
| `maps.field` | 5 | PS2 field map textures (and texture-swap frames) | HD | converter (`maps.field`) | done (`docs/mods/maps.md`) |
| `maps.geometry` | 6 | PS2 geometry where the HD maps changed it | HD | converter | not planned yet |
| `maps.backgrounds` | 7 | the 4:3 PS2 paintings of fixed-camera rooms | the wider HD paintings | converter (`maps.backgrounds`; reads `render.aspect`) | with `render.aspect = "ps2"` all 105 rooms (the 37 the HD re-framed for 16:9 too); with `"hd"` the 68 whose HD painting keeps the PS2 framing. Animated parts stay HD except in re-framed rooms (`docs/mods/screen.md`) |
| `maps.battle` | 8 | PS2 battle map textures | HD | converter (`maps.battle`) | done (`docs/mods/maps.md`) |
| `effects.textures` | 9 | PS2 spell, summon and effect textures | HD (2x upscales) | converter (`fx.textures`) | done (`docs/mods/effects.md`) |
| `render.glow` | 10 | no glow/blow-out | the HD glow | mod | done (`docs/mods/render.md`) |
| `render.lighting` | 10 | characters lit by the PS2 formula: the PS2 light colours and directions at the PS2's level (254/128), `max(0, N.L)` falloff, no HD fill light, point lights only where the PS2 uses them (none in the opening) | the HD light levels and soft falloff | mod and converter (`render.lighting`: the patched character shaders; `render.lighting.events`: the PS2's `Light_Set` colours in the 6 event scripts the HD retuned) | done (`docs/mods/render.md`); Gouraud-like (interpolated normal not renormalised) but clamped per pixel, no 2x clamp of the PS2 vertex colour; the opening's Tidus close-up within 1% of the PS2 at the same pose |
| `render.shadows` | 11 | ground shadows only, no shadow maps | shadow maps | mod | done (`docs/mods/render.md`) |
| `render.aspect` | 14 | 4:3 with black bars and the PS2 projection; the PS2-placed menus, windows and battle HUD where the PS2 had them; the Flash screens (Esc menu, settings, save/load) squeezed into the 4:3; event sprites (the opening's staff credits) at the PS2 size | 16:9 | mod (and the `movies` and `maps.backgrounds` jobs) | done; mouse on the PS2 menus not checked (`docs/mods/screen.md`) |
| `render.texture_quality` | 19 | the PS2 textures (maps, effects, characters, menus) load at their full size at every texture quality of the game's graphics options; the HD ones follow the option | the option's `TQ_MEDIUM` and `TQ_LOW` (the `VQ_LOW` preset) drop the top one or two mip levels of the PS2 textures too: a quarter of the PS2 size at `TQ_LOW` | mod | done (`docs/mods/render.md`, "Texture quality") |
| `ui.menus` | 12 | the PS2 message windows (dialogue choices, yes/no questions: the PS2 plain window with its colours and pieces), the PS2 battle command windows (one blue-to-violet gradient box at the PS2 place and size, PS2 row layout; the change and gil windows stay HD), the PS2 main menu screen (blue-to-khaki background, brush-plate party rows, purple option plates, help bar, gil/time/location footer; the PS2 `meswin` written as `ps2win`), the PS2 Items and Abilities screens (two columns of gold/purple brush plates with count discs, the black sub-command bar, the target window, the ability screen's party plate), the PS2 hand-and-ball cursor where the game draws cursor kind 0, the PS2 item icons on their coloured tiles, the PS2 battle words (OVERKILL, IMMUNE, MISS, MP) at the PS2 size, the battle sequence's Overdrive tab, BEFEHL footer, glow and lines, the PS2 battle help bar, turn order (CTB) list and party list, the command windows' numbers in the 16x12 font, the PS2 `strtex` labels, the PAL disc's menu layout (`jppc/menu/menu_script.bin`), and the PS2 pieces of the menu and battle atlases (`ps2win`, `ps2btl`; Equip, Status, Overdrive, Customize, Config and the Flash screens stay HD, `docs/mods/interface.md`) | HD | converter (`ui.menus`) and mod | message windows done; battle windows, Overdrive tab, main menu, Items and Abilities checked through injected game calls, not in play; the other sub-menus not started |
| `ui.portraits` | 12 | PS2 portraits: menu faces, aeons, CTB list | HD | converter (`ui.menus`) | done (`docs/mods/interface.md`) |
| `ui.font` | 13 | the PS2 font and its letter widths in English, French, Italian, German and Spanish (dialogue, menus, battle text, help), and the PS2 damage digits; Japanese, Chinese and Korean keep the HD font | the HD typeface | converter (`ui.font`; the damage digits of the battle atlas by `ui.menus`) | done (`docs/mods/font.md`) |
| `ui.title` | 16 | the PS2 title screen and logos; the title picture laid out at the PS2 place (the PC showed it 1.24 times as big and higher) and the title menu's cursor moved with it; the still PS2 banner and water strip instead of the PC's animated water band | "HD Remaster" logo, HD placement | converter (`ui.title`) and mod | done; the cursor move not seen in play (the harness runs ported message code) |
| `ui.loading` | 16 | no loading screen, no splash videos, a black window at start (not white) | HD | converter (`ui.loading`: loading screen textures transparent) and mod (splash videos) | loading screen textures done; splash videos skipped, window backdrop black (`docs/mods/patches.md`) |
| `ui.battle_subtitles` | 17 | subtitles for battle dialogue | none | mod | scripted battle dialogue always subtitled (the PC already does this for its own saves); the missing "battle quotes" not found yet (`docs/mods/patches.md`) |
| `ui.battle_menu_delay` | 18 | the battle menu takes input at once (the HD window's 0.2 s opening animation stays) | 0.3-0.45 s input delays | mod | done (`docs/mods/patches.md`) |
| `ui.boot_logo` | 16 | the PS2's "SQUARESOFT® presents" screen at boot, before the FINAL FANTASY X PROJECT logo, with the PS2's fades and frame counts (`docs/mods/interface.md`, "Boot logo"); off (`"hd"`) by default | none (the PC removed it) | converter (`ui.boot_logo`: the disc's `titl02` logo layers and the PC's boot event `memochek` with the PS2's wait) | done |
| `ui.button_glyphs` | 22 | PlayStation button glyphs | Xbox-style | converter (`ui.glyphs`) | pad prompts done (`docs/mods/interface.md`) |
| `audio.soundtrack` | 20 | a New Game starts with the original soundtrack; loaded saves keep their own choice, and the game's menu still switches it | arranged | mod | done (`docs/mods/patches.md`) |
| `audio.music` | 20 | the original soundtrack rendered from the PS2 disc's music sequences and instruments by our PS2 sound driver and SPU2: the bank "Original" in the game's menu plays | the PC's recordings of the original soundtrack | converter (`audio.music`) | done (`docs/mods/music.md`) |
| `audio.music_engine` | 20 | `"live"` (its default; not `"ps2"`): the PS2 driver plays in the game, driven by the game's music commands (fades, pauses, tempo), while "Original" is chosen; what it can't play comes from the "Original" bank (the PC's, or the rendered one of `audio.music`) | `"hd"`: the bank alone | converter (`audio.live`: the disc's music files in `music/`) and mod (`demaster_live.dll`) | works in the game except the 3 tracks with streamed parts, which stay the bank's (`docs/mods/music.md`, "Phase 2") |
| `movies` | 15, 21 | the 4:3 PS2 movies with their audio | the cropped HD movies | converter (`movies`, needs ffmpeg; reads `render.aspect`) | 49 of 53, 960 x 720 with `render.aspect = "ps2"`, else with bars in 1280 x 720; the four with a 3D scene on top stay HD (cut to their middle 4:3 with `render.aspect = "ps2"`) (`docs/mods/movies.md`) |

`movies` is a top-level key: it goes above the first `[table]` in the file.

`render.texture_filter` (row 19) has other values: `"game"` (default) keeps
the filter of the game's graphics options (bilinear/trilinear or
anisotropic; the PS2 filtered bilinearly too); `"nearest"` shows unfiltered
texels on the 3D scene's models and maps (effects, 2D rooms, menus and
post-processing keep theirs). Applied by the mod (`docs/mods/render.md`).

`audio.music_engine` (row 20) is `"live"` or `"hd"`. FFX Demaster's
`demaster.toml` had a third value, `"ps2"`, for the rendered bank: that is
`audio.music = "ps2"` now (mod PS2 soundtrack), and `audio.music_engine`
no longer takes it.

PS2 textures of the 3D scene (maps, effects, kept-mesh characters, sphere
grid) whose HD texture has mip levels get a full mip chain
(`phyretex::mip_chain`), so surfaces seen from afar do not shimmer; the
game's texture quality (`TQ_LOW` drops the top two levels of every chain)
would shrink them, which `render.texture_quality` stops. Textures that are
never minified (menus, font, the 2D rooms' paintings) and the remodelled
characters' atlases (`models.*`) have one level, which no texture quality
shrinks (`docs/mods/render.md`, "Texture quality").

## Timing (default `"hd"`: the PC's speed)

The game's logic runs at the NTSC PS2's rate, 29.97 frames a second, as on
the PC: the user chose NTSC as "the PS2 original" (2026-11-04). The run speed
defaults to the PS2's.
Measurements and the reasons: [`timing.md`](mods/timing.md).

| Key | `"ps2"` (default) | `"hd"` | Applied by | Status |
|---|---|---|---|---|
| `timing.run_speed` | Tidus runs at the PS2's speed: 34 (1.70 units a frame, 51 a second at the PC's 29.97 frames a second) | 38 (1.90 a frame, 57 a second), 12 % faster; the PC's own scripts still set 34 in some scenes | mod | done, static (bytes checked); not run in the game |

A PAL speed option (`timing.pal_speed`, the whole game at 25 logic frames
a second) is not implemented; `timing.md` says what it would take.

## Fixes (default `true`)

| Key | `true` | Applied by | Status |
|---|---|---|---|
| `fixes.npc_mouths` | NPC mouth textures close after a line, as on the PS2 (the PC leaves the open frame) | mod | done, behaviour checked in the harness and PCSX2; the patch itself not yet run in Fahrenheit (`docs/mods/patches.md`) |
| `fixes.farplane_video` | the Farplane background video plays | mod | done, not yet seen in the game (`docs/mods/patches.md`) |
| `fixes.lulu_fury` | Lulu's Fury spins register: a diagonal the PC's once-a-frame stick reading skipped is filled in | mod | done, not yet tested with a stick (`docs/mods/patches.md`) |

Kimahri's Spirit Lance comes with `models.party`.

## Minimap

The Modern Minimap (`yuta.modern-minimap`) is not a part of the PS2 game: it
replaces the field minimap of both versions with a round one
([`minimap.md`](mods/modern-minimap.md)). All three settings are applied by the mod.

| Setting | Values | Default | Status |
|---|---|---|---|
| `minimap.style` | `"crt"`: round, centred on the player, turned with the camera, in the game's colours on a small CRT screen; `"plain"`: the same without the CRT screen; `"game"`: the game's own minimap | `"crt"` | done, seen in the game (Thunder Plains; `"plain"` and `"game"` too) |
| `minimap.size` | `"small"`, `"medium"`, `"large"`: 14, 18 or 24 % of the picture's height across | `"medium"` | done (`"large"` seen in the game) |
| `minimap.corner` | `"top_right"`, `"top_left"`, `"bottom_right"`, `"bottom_left"` of the 16:9 picture | `"top_right"` | done (`"bottom_left"` seen in the game) |

## FPS unlock

FPS unlock (`yuta.fps-unlock`) is not a part of the PS2 game: it draws the
game at the monitor's rate (or a fixed rate, or uncapped) while the world
keeps its 30 ticks a second, draws the camera and the characters between
ticks, and scales game time ([`fps-unlock.md`](mods/fps-unlock.md)). Every
setting is applied by the mod and read **live**: they are in the F8
window, a change applies at once and is written back into the mod's
`settings.toml` half a second later (only the lines that changed; a time
scale above 1.0 only after 5 s at it). The defaults are the mod's, not the
PS2's. Status: the frame timing is ffx-atel-sharp's `yuta.frames`, seen
in the game there (2026-11-07, 2026-11-08); as `yuta.fps-unlock`, with
`settings.toml` and the F8 window, not yet run in the game.

| Setting | Values | Default |
|---|---|---|
| `frames.enabled` | `true`; `false`: the game's own timing, limiter and vsync (the hooks stay in and pass everything through) | `true` |
| `frames.target` | `"game"` (the game's limiter, 30 fps), `"monitor"` (the window's monitor's refresh rate), `"fixed"` (`frames.fixed_fps`), `"unlimited"` | `"monitor"` |
| `frames.fixed_fps` | 10 to 1000 | `120.0` |
| `frames.vsync` | `"game"` (as the game has it), `"on"`, `"off"` (Alt+F9 switches on and off) | `"game"` |
| `frames.overlay` | the fps counter in the top-right corner | `true` |
| `frames.between_ticks` | `"redraw"`: a frame without a tick runs the game's draw with the world held; `"repeat"`: the last frame again | `"redraw"` |
| `frames.effects` | between ticks: `"replay"` (the last tick's effect draws again), `"run"` (too fast), `"hold"` (missing) | `"replay"` |
| `frames.speed.time_scale` | game time per real second, 0.05 to 100 | `1.0` |
| `frames.speed.fast` | the scale Ctrl+F9 switches to | `10.0` |
| `frames.speed.slow` | the scale Shift+F9 switches to | `0.25` |
| `frames.speed.mute_when_fast` | sound off above 1.0 | `true` |
| `frames.speed.booster_limits` | 1.0 where the PC booster won't speed up | `true` |
| `frames.speed.scene_time` | the scene's own animations (the shaders' `GlobalTime`: water, scrolling clouds, UV-animated surfaces; texture animations, the radial blur, light flash, character fog fade) follow game time and the pause; `false`: real time, as the game has them | `true` |
| `frames.interpolate.camera` | the camera between ticks | `true` |
| `frames.interpolate.characters` | the characters' place and facing between ticks | `true` |
| `frames.interpolate.bones` | their bones too | `true` |
| `frames.interpolate.cloth` | their cloth and hair too (needs `frames.hold.cloth`) | `true` |
| `frames.interpolate.objects` | the map's moving objects between ticks (field animated parts, objects the scripts move: lifts, gates, boats) | `true` |
| `frames.interpolate.repeats` | the camera on `"repeat"` frames | `true` |
| `frames.cuts.camera_distance` | a camera that moved further in one tick cut (world units) | `100.0` |
| `frames.cuts.camera_degrees` | a camera that turned more in one tick cut (degrees) | `45.0` |
| `frames.cuts.game_sets` | the game's instant camera sets are cuts when the step jumps | `true` |
| `frames.cuts.character_distance` | a character or map object that moved further in one tick warped (world units) | `50.0` |
| `frames.cuts.character_degrees` | a character or map object that turned more in one tick warped (degrees) | `120.0` |
| `frames.hold.characters` | redrawn frames hold `Ch_CalcMiddle` (bones, motion events) | `true` |
| `frames.hold.field_animation` | … `yiAnimMain` | `true` |
| `frames.hold.fades` | … the fade counters of `Sg_DrawFilter` | `true` |
| `frames.hold.cloth` | … the cloth step | `true` |
| `frames.budget.max_real_time` | the most real time one frame counts (s) | `0.3` |
| `frames.budget.max_ticks` | the most ticks one frame runs, 1 to 892 | `100` |
| `frames.budget.max_update_ms` | the most update time one frame may take (ms, 5 to 300) | `40.0` |
| `frames.budget.whole_updates` | whole game updates per frame before exec frames, 1 to 10 | `4` |

## Unknown keys

A key the converter doesn't know (a typo, or a setting of a newer version) is
ignored with a warning, from `demaster.exe` and in the game's log. A value
that isn't allowed (`monster = "psx"`) stops the build with the line number.

## Files the build writes

In the mod folder:

- `efl/x/...`: the PS2 files, at the game's own paths.
- `build/files.json`: every file of the last build, by job. The next build
  skips a job whose settings, inputs (disc image and `FFX_Data.vbf`, by size
  and date) and converter version are unchanged and whose files are all
  there, and removes the files of jobs that are now off.
- `build/stamp.json`: the settings the files were built with and the
  converter version. The mod compares it with its `settings.toml` at
  startup. It
  is deleted when a build starts, so an interrupted build reads as "not
  built".

Only files listed in `build/files.json` are ever removed.
