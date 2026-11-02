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
| `build.mod_dir` | Where the files go. Empty: the folder `demaster.exe` is in, which is right when it sits in `mods\yuta.demaster`. |

Relative paths are relative to the folder of `demaster.toml`. On Windows,
write paths in single quotes (`iso = 'D:\Games\FFX.iso'`): inside double
quotes a backslash starts an escape sequence.

The command line overrides the file: `demaster.exe build --iso <file>
--game <folder> --mod-dir <folder> --config <file>`. More options:
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
| `maps.backgrounds` | 7 | the 4:3 PS2 paintings of fixed-camera rooms | the wider HD paintings | converter | planned |
| `maps.battle` | 8 | PS2 battle map textures | HD | converter (`maps.battle`) | done (`docs/maps.md`) |
| `effects.textures` | 9 | PS2 spell, summon and effect textures | HD (2x upscales) | converter | planned |
| `render.glow` | 10 | no glow/blow-out | the HD glow | mod | done (`docs/render.md`) |
| `render.lighting` | 10 | characters lit by the PS2 light colours and directions | the HD light levels | mod | light levels and directions done; the shader's softer falloff stays (`docs/render.md`) |
| `render.shadows` | 11 | ground shadows only, no shadow maps | shadow maps | mod | done (`docs/render.md`) |
| `render.aspect` | 14 | 4:3 with black bars and the PS2 projection | 16:9 | mod | planned |
| `ui.menus` | 12 | the PS2 pieces of the menu and battle atlases (the menu layout and the Flash screens stay HD, `docs/ui.md`) | HD | converter (`ui.menus`) | battle HUD markers only; the PS2 window look needs code |
| `ui.portraits` | 12 | PS2 portraits: menu faces, aeons, CTB list | HD | converter (`ui.menus`) | done (`docs/ui.md`) |
| `ui.font` | 13 | the PS2 font | the HD typeface | converter | planned (`docs/font.md`) |
| `ui.title` | 16 | the PS2 title screen and logos | "HD Remaster" logo | converter (`ui.title`) | done |
| `ui.loading` | 16 | no loading screen, no splash videos | HD | converter (`ui.loading`: loading screen textures transparent) and mod (splash videos) | loading screen textures done; splash videos planned |
| `ui.battle_subtitles` | 17 | subtitles for battle dialogue | none | mod | planned |
| `ui.battle_menu_delay` | 18 | the battle menu reacts at once | 0.2-0.45 s delays | mod | planned |
| `ui.button_glyphs` | 22 | PlayStation button glyphs | Xbox-style | converter (`ui.glyphs`) | pad prompts done (`docs/ui.md`) |
| `audio.soundtrack` | 20 | the original soundtrack by default | arranged | mod | planned |
| `movies` | 15, 21 | the 4:3 PS2 movies with their audio | the cropped HD movies | converter | planned |

`movies` is a top-level key: it goes above the first `[table]` in the file.

`render.texture_filter` (row 19) has other values: `"game"` (default) keeps
the filter of the game's graphics options; `"nearest"` shows unfiltered
texels. Applied by the mod; planned (`docs/render.md`).

## Fixes (default `true`)

| Key | `true` | Applied by | Status |
|---|---|---|---|
| `fixes.npc_mouths` | NPC mouths no longer hang open in some cutscenes | mod | planned |
| `fixes.farplane_video` | the Farplane background video plays | mod | planned |
| `fixes.lulu_fury` | Lulu's Fury spins register | mod | planned |

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
