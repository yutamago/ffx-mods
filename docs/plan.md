# Plan and status

Numbers refer to the rows of [`differences.md`](differences.md). Status:
**done** (checked in the game), **wip**, **todo**, **dropped** (with why).

## Milestone 1: texture swaps

| Item | Rows | Status | Notes |
|---|---|---|---|
| Disc reader, names, unpacking | | done | `docs/disc.md` |
| ARGB8 texture writer | | done | loads in game (Djose battle) |
| Character textures (kept meshes) | 2-4 | done | 621 pages; 188 remodelled pages skipped (milestone 2) |
| Field map textures | 5 | done | 13,701 textures + swap frames; `docs/maps.md`. Close palette calls listed in maps.tsv |
| Battle map textures | 8 | done | 2,129 textures |
| Effect textures | 9 | done | 7,357 converted (`docs/effects.md`); brightness moved into alpha where HD does, so bright draws burn out white like the PS2 (Energy Blast confirmed by user). Field-map particles not seen in game |
| PS2-drawn menu textures, portraits, icons | 12, 22 | wip | title, portraits, battle atlas, pad glyphs, loading screen done (`docs/ui.md`); main menu look, message window, item icons need code |
| Config file + converter `build` command | all | done | `docs/config.md` |
| Fahrenheit mod package | | wip | mod `yuta.demaster` (F8 status window), `demaster.exe`, package script, CI. Open: F8 window not seen on screen; CI needs secret `FFX_RUST_TOKEN` |

## Milestone 2: models

| Item | Rows | Status | Notes |
|---|---|---|---|
| PS2 mesh → Phyre model converter | 1-4 | done | 165 models: 32 party, 95 NPCs, 16 monsters, 2 aeons, 5 weapons, 15 objects (`docs/models.md`). Tidus face checked in game |
| Texture animation (eyes, mouths) | 1 | todo | faces static; path traced in `docs/models.md`. Also: PS2 cloth without HD counterpart doesn't swing | `.chr` animation block; HD `*_anim_*` textures |

## Milestone 3: interface

| Item | Rows | Status |
|---|---|---|
| PS2 font | 13 | todo | texture + table swap for EFIGS, plan in `docs/font.md` (~2 days) |
| Title logo, loading screen, splash videos | 16 | done | splash: OpeningScreen.swf skipped by hook; not seen on screen |
| Battle dialogue subtitles | 17 | dropped | the PC already subtitles scripted battle dialogue; no case found where the PS2 did and the PC doesn't (user, 2026-11-02). `ui.battle_subtitles` only forces the save's subtitle bit on |
| Battle menu input delay | 18 | done (static) | 8 delay branches removed (`docs/patches.md`); not timed in game |

## Milestone 4: movies and screen

| Item | Rows | Status |
|---|---|---|
| PS2 FMVs into the PC video format | 15, 21 | todo |
| 4:3 aspect ratio | 14 | todo |
| PS2 2D backgrounds | 7 | todo |

## Milestone 5: rendering

| Item | Rows | Status |
|---|---|---|
| Glow / blow-out off | 10 | todo |
| PS2-like lighting | 10 | todo |
| Shadows | 11 | todo |
| Texture filtering option | 19 | todo |
| Soundtrack default "Original" | 20 | done (static) | new-game save gets Original; menu switch still works |

## Fixes

| Item | Status |
|---|---|
| NPC mouths open in cutscenes | open | no PC-specific cause found, no reproducible case known (`docs/patches.md`) |
| Farplane video | done (static) | movie-init branch patched; particles during the video not checked |
| Lulu's Fury input | done (static) | skipped diagonal filled in; no real stick test |

## Changes wanted in ffx-rust

(none yet)
