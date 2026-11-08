# Settings (`demaster.toml`)

`demaster.toml` sits next to `demaster.exe` in `mods/yuta.demaster/`. It says
where your PS2 disc image is and which parts of the game look like the PS2
original. Every row of [`differences.md`](differences.md) has a setting here.

**The rule for defaults: presentation from the PS2, logic from the PC.**
Presentation settings are `"ps2"` (the default) or `"hd"` (the HD
Remaster). Fixes are `true` (fixed, the default) or `false` (as on PC).
Game logic (balance, Eternal Calm, boosters, controls, PC text and voices,
30 fps) always stays the PC's.

**After changing a setting:** run `demaster.exe build` again, then restart
the game. Fahrenheit picks up the mod's files only when the game starts, and
the mod reads `demaster.toml` only then. In the game, F8 shows whether the
built files match the settings.

**Status.** Not every setting does something yet. `demaster.exe build` ends
with the list of settings it has no job for, and F8 lists the code patches
that don't exist yet. The *Status* column below is as of this version.

Who applies a setting:
- **converter**: `demaster.exe build` writes PS2 files into
  `mods/yuta.demaster/efl/x/`. With `"hd"` it writes none and removes those
  of an earlier build.
- **mod**: code in `yuta.demaster.dll`, applied when the game starts.

## Paths

| Key | Meaning |
|---|---|
| `source.iso` | Your PS2 disc image of Final Fantasy X (`.iso`, any region). Required. |
| `source.game` | The game folder, the one with `FFX.exe` and `data\FFX_Data.vbf`. Empty: the converter looks in the usual Steam folders (`steamapps\common\FINAL FANTASY FFX&FFX-2 HD Remaster` in every Steam library). |
| `source.ffmpeg` | ffmpeg, which converts the movies (not included). Empty: `ffmpeg.exe` next to `demaster.exe` or in `tools\`, else the one on your PATH; on Windows, if there is none, the converter downloads an LGPL build of FFmpeg 8.1.3 into `mods\yuta.demaster\tools\` (168 MB) and checks it (`docs/movies.md`). |
| `build.mod_dir` | Where the files go. Empty: the folder `demaster.exe` is in, which is right when it sits in `mods\yuta.demaster`. |

Relative paths are relative to the folder of `demaster.toml`. On Windows,
write paths in single quotes (`iso = 'D:\Games\FFX.iso'`): inside double
quotes a backslash starts an escape sequence.

The command line overrides the file: `demaster.exe build --iso <file>
--game <folder> --mod-dir <folder> --ffmpeg <ffmpeg.exe> --config <file>`. More options:
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
| `maps.field` | 5 | PS2 field map textures (and texture-swap frames) | HD | converter (`maps.field`) | done (`docs/maps.md`) |
| `maps.geometry` | 6 | PS2 geometry where the HD maps changed it | HD | converter | not planned yet |
| `maps.backgrounds` | 7 | the 4:3 PS2 paintings of fixed-camera rooms | the wider HD paintings | converter (`maps.backgrounds`; reads `render.aspect`) | with `render.aspect = "ps2"` all 105 rooms (the 37 the HD re-framed for 16:9 too); with `"hd"` the 68 whose HD painting keeps the PS2 framing. Animated parts stay HD except in re-framed rooms (`docs/screen.md`) |
| `maps.battle` | 8 | PS2 battle map textures | HD | converter (`maps.battle`) | done (`docs/maps.md`) |
| `effects.textures` | 9 | PS2 spell, summon and effect textures | HD (2x upscales) | converter | planned |
| `render.glow` | 10 | no glow/blow-out | the HD glow | mod | done (`docs/render.md`) |
| `render.lighting` | 10 | characters lit by the PS2 formula: the PS2 light colours and directions at the PS2's level (254/128), `max(0, N.L)` falloff, no HD fill light, point lights only where the PS2 uses them (none in the opening) | the HD light levels and soft falloff | mod and converter (`render.lighting`: the patched character shaders) | done (`docs/render.md`); Gouraud-like (interpolated normal not renormalised) but clamped per pixel, no 2x clamp of the PS2 vertex colour; lit parts of the opening's close-ups ~12% brighter than the PS2 |
| `render.shadows` | 11 | ground shadows only, no shadow maps | shadow maps | mod | done (`docs/render.md`) |
| `render.aspect` | 14 | 4:3 with black bars and the PS2 projection; the PS2-placed menus, windows and battle HUD where the PS2 had them; the Flash screens (Esc menu, settings, save/load) squeezed into the 4:3; event sprites (the opening's staff credits) at the PS2 size | 16:9 | mod (and the `movies` and `maps.backgrounds` jobs) | done; mouse on the PS2 menus not checked (`docs/screen.md`) |
| `render.texture_quality` | 19 | the PS2 textures (maps, effects, characters, menus) load at their full size at every texture quality of the game's graphics options; the HD ones follow the option | the option's `TQ_MEDIUM` and `TQ_LOW` (the `VQ_LOW` preset) drop the top one or two mip levels of the PS2 textures too: a quarter of the PS2 size at `TQ_LOW` | mod | done (`docs/render.md`, "Texture quality") |
| `ui.menus` | 12 | the PS2 message windows (dialogue choices, yes/no questions: the PS2 plain window with its colours and pieces), the PS2 battle command windows (one blue-to-violet gradient box at the PS2 place and size, PS2 row layout; the change and gil windows stay HD), the PS2 main menu screen (blue-to-khaki background, brush-plate party rows, purple option plates, help bar, gil/time/location footer; the PS2 `meswin` written as `ps2win`), the PS2 Items and Abilities screens (two columns of gold/purple brush plates with count discs, the black sub-command bar, the target window, the ability screen's party plate), the PS2 hand-and-ball cursor where the game draws cursor kind 0, the PS2 item icons on their coloured tiles, the PS2 battle words (OVERKILL, IMMUNE, MISS, MP) at the PS2 size, the battle sequence's Overdrive tab, BEFEHL footer, glow and lines, the PS2 battle help bar, turn order (CTB) list and party list, the command windows' numbers in the 16x12 font, the PS2 `strtex` labels, the PAL disc's menu layout (`jppc/menu/menu_script.bin`), and the PS2 pieces of the menu and battle atlases (`ps2win`, `ps2btl`; Equip, Status, Overdrive, Customize, Config and the Flash screens stay HD, `docs/ui.md`) | HD | converter (`ui.menus`) and mod | message windows done; battle windows, Overdrive tab, main menu, Items and Abilities checked through injected game calls, not in play; the other sub-menus not started |
| `ui.portraits` | 12 | PS2 portraits: menu faces, aeons, CTB list | HD | converter (`ui.menus`) | done (`docs/ui.md`) |
| `ui.font` | 13 | the PS2 font and its letter widths in English, French, Italian, German and Spanish (dialogue, menus, battle text, help), and the PS2 damage digits; Japanese, Chinese and Korean keep the HD font | the HD typeface | converter (`ui.font`; the damage digits of the battle atlas by `ui.menus`) | done (`docs/font.md`) |
| `ui.title` | 16 | the PS2 title screen and logos; the title picture laid out at the PS2 place (the PC showed it 1.24 times as big and higher) and the title menu's cursor moved with it; the still PS2 banner and water strip instead of the PC's animated water band | "HD Remaster" logo, HD placement | converter (`ui.title`) and mod | done; the cursor move not seen in play (the harness runs ported message code) |
| `ui.loading` | 16 | no loading screen, no splash videos | HD | converter (`ui.loading`: loading screen textures transparent) and mod (splash videos) | loading screen textures done; splash videos skipped (`docs/patches.md`) |
| `ui.battle_subtitles` | 17 | subtitles for battle dialogue | none | mod | scripted battle dialogue always subtitled (the PC already does this for its own saves); the missing "battle quotes" not found yet (`docs/patches.md`) |
| `ui.battle_menu_delay` | 18 | the battle menu takes input at once (the HD window's 0.2 s opening animation stays) | 0.3-0.45 s input delays | mod | done (`docs/patches.md`) |
| `ui.boot_logo` | 16 | the PS2's "SQUARESOFT® presents" screen at boot, before the FINAL FANTASY X PROJECT logo, with the PS2's fades and frame counts (`docs/ui.md`, "Boot logo"); off (`"hd"`) by default | none (the PC removed it) | converter (`ui.boot_logo`: the disc's `titl02` logo layers and the PC's boot event `memochek` with the PS2's wait) | done |
| `ui.button_glyphs` | 22 | PlayStation button glyphs | Xbox-style | converter (`ui.glyphs`) | pad prompts done (`docs/ui.md`) |
| `audio.soundtrack` | 20 | a New Game starts with the original soundtrack; loaded saves keep their own choice, and the game's menu still switches it | arranged | mod | done (`docs/patches.md`) |
| `audio.music_engine` | 20 | the original soundtrack rendered from the PS2 disc's music sequences and instruments by our PS2 sound driver and SPU2 (what "Original" in the game's menu plays); `"live"`: that driver plays in the game, driven by the game's music commands (fades, pauses, tempo), with the rendered bank as the fallback | the PC's recordings of the original soundtrack | converter (`audio.music`; with `"live"` also `audio.live`: the disc's music files in `music/`) and mod (`"live"`: `demaster_live.dll`) | `"ps2"` done; `"live"` works in the game except the 3 tracks with streamed parts, which stay the bank's (`docs/music.md`, "Phase 2") |
| `movies` | 15, 21 | the 4:3 PS2 movies with their audio | the cropped HD movies | converter (`movies`, needs ffmpeg; reads `render.aspect`) | 49 of 53, 960 x 720 with `render.aspect = "ps2"`, else with bars in 1280 x 720; the four with a 3D scene on top stay HD (cut to their middle 4:3 with `render.aspect = "ps2"`) (`docs/movies.md`) |

`movies` is a top-level key: it goes above the first `[table]` in the file.

`render.texture_filter` (row 19) has other values: `"game"` (default) keeps
the filter of the game's graphics options (bilinear/trilinear or
anisotropic; the PS2 filtered bilinearly too); `"nearest"` shows unfiltered
texels on the 3D scene's models and maps (effects, 2D rooms, menus and
post-processing keep theirs). Applied by the mod (`docs/render.md`).

`audio.music_engine` (row 20) has three values: `"ps2"` (default) renders
the original soundtrack bank from the disc; `"hd"` keeps the PC's
recordings; `"live"` builds the bank too and copies the disc's music files
into the mod's `music/` folder, and the mod loads `demaster_live.dll`,
which plays them in the game while "Original" is chosen. Switching between
`"ps2"` and `"live"` renders the bank again once (its build key holds the
value).

PS2 textures of the 3D scene (maps, effects, kept-mesh characters, sphere
grid) whose HD texture has mip levels get a full mip chain
(`phyretex::mip_chain`), so surfaces seen from afar do not shimmer; the
game's texture quality (`TQ_LOW` drops the top two levels of every chain)
would shrink them, which `render.texture_quality` stops. Textures that are
never minified (menus, font, the 2D rooms' paintings) and the remodelled
characters' atlases (`models.*`) have one level, which no texture quality
shrinks (`docs/render.md`, "Texture quality").

## Timing (default `"hd"`: the PC's speed)

The game's logic runs at the NTSC PS2's rate, 29.97 frames a second, as on
the PC: the user chose NTSC as "the PS2 original" (2026-11-04). The run speed
defaults to the PS2's.
Measurements and the reasons: [`timing.md`](timing.md).

| Key | `"ps2"` (default) | `"hd"` | Applied by | Status |
|---|---|---|---|---|
| `timing.run_speed` | Tidus runs at the PS2's speed: 34 (1.70 units a frame, 51 a second at the PC's 29.97 frames a second) | 38 (1.90 a frame, 57 a second), 12 % faster; the PC's own scripts still set 34 in some scenes | mod | done, static (bytes checked); not run in the game |

A PAL speed option (`timing.pal_speed`, the whole game at 25 logic frames
a second) is not implemented; `timing.md` says what it would take.

## Fixes (default `true`)

| Key | `true` | Applied by | Status |
|---|---|---|---|
| `fixes.npc_mouths` | NPC mouth textures close after a line, as on the PS2 (the PC leaves the open frame) | mod | done, behaviour checked in the harness and PCSX2; the patch itself not yet run in Fahrenheit (`docs/patches.md`) |
| `fixes.farplane_video` | the Farplane background video plays | mod | done, not yet seen in the game (`docs/patches.md`) |
| `fixes.lulu_fury` | Lulu's Fury spins register: a diagonal the PC's once-a-frame stick reading skipped is filled in | mod | done, not yet tested with a stick (`docs/patches.md`) |

Kimahri's Spirit Lance comes with `models.party`.

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
  converter version. The mod compares it with `demaster.toml` at startup. It
  is deleted when a build starts, so an interrupted build reads as "not
  built".

Only files listed in `build/files.json` are ever removed.
