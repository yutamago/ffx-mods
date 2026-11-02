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
| Effect textures | 9 | wip | 7,327 converted (`docs/effects.md`); user: PS2 Energy Blast trails were white, ours purple — alpha/palette fix in progress |
| PS2-drawn menu textures, portraits, icons | 12, 22 | todo | named after PS2 VRAM pages |
| Config file + converter `build` command | all | done | `docs/config.md` |
| Fahrenheit mod package | | wip | mod `yuta.demaster` (F8 status window), `demaster.exe`, package script, CI. Open: F8 window not seen on screen; CI needs secret `FFX_RUST_TOKEN` |

## Milestone 2: models

| Item | Rows | Status | Notes |
|---|---|---|---|
| PS2 mesh → Phyre model converter | 1-4 | wip | party (32 pages), 95 NPCs, 27 monsters, aeons, weapons, objects |
| Texture animation (eyes, mouths) | 1 | todo | `.chr` animation block; HD `*_anim_*` textures |

## Milestone 3: interface

| Item | Rows | Status |
|---|---|---|
| PS2 font | 13 | todo |
| Title logo, loading screen, splash videos | 16 | todo |
| Battle dialogue subtitles | 17 | todo |
| Battle menu input delay | 18 | todo |

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
| Soundtrack default "Original" | 20 | todo |

## Fixes

| Item | Status |
|---|---|
| NPC mouths open in cutscenes | todo |
| Farplane video | todo |
| Lulu's Fury input | todo |

## Changes wanted in ffx-rust

(none yet)
