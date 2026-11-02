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
| Texture animation (eyes, mouths), cloth | 1 | done | 110 converted characters blink/talk, kept-mesh frames repainted (~2,900); PS2 cloth pieces simulated (55 characters). Static: c107, c041, n295, n333, n343, m124, m070-m076 (`docs/models.md`) |

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
| PS2 FMVs into the PC video format | 15, 21 | done | 49 of 53 (4 with 3D overlays stay HD); ffmpeg downloaded on first use (`docs/movies.md`) |
| 4:3 aspect ratio | 14 | todo | plan in `docs/screen.md` |
| PS2 2D backgrounds | 7 | wip | 446 layers in 68 rooms with unchanged framing; 37 re-framed rooms need 4:3; animated parts not decoded; not seen in game |

## Milestone 5: rendering

| Item | Rows | Status |
|---|---|---|
| Glow / blow-out off | 10 | done | blur-and-add pass skipped (`docs/render.md`) |
| PS2-like lighting | 10 | wip | light levels/directions = PS2 script values at 1.0 (assumption); shader falloff still HD |
| Shadows | 11 | done | blob shadows only; check maps that normally use shadow maps for artefacts |
| Texture filtering option | 19 | todo | plan in `docs/render.md` |
| Soundtrack default "Original" | 20 | done (static) | new-game save gets Original; menu switch still works |
| PS2 music engine (offline render): our port of iopsound.irx + SPU2 model renders the 89 tracks into the `music_ps2` bank | 20 | done | 70/86 tracks correlate ≥0.9 with the PC Original recordings; 3 rock tracks copied (streamed layers); in-game capture matches our render (`docs/music.md`) |
| PS2 music engine phase 2: live, driven by the game's iMusic* commands (runtime tempo/fades, streamed guitar/vocal layers) | 20 | todo | plan in `docs/music.md` |

## Fixes

| Item | Status |
|---|---|
| NPC mouths open in cutscenes | open | no PC-specific cause found, no reproducible case known (`docs/patches.md`) |
| Farplane video | done (static) | movie-init branch patched; particles during the video not checked |
| Lulu's Fury input | done (static) | skipped diagonal filled in; no real stick test |

## Changes wanted in ffx-rust

(none yet)
