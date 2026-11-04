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
| PS2-drawn menu textures, portraits, icons, windows | 12, 22 | wip | done: title, portraits, battle atlas, pad glyphs, loading, message/choice windows, battle command windows, PS2 cursor, item icons, battle words, Overdrive tab, main menu and its Items, Abilities, Status, Equip, Overdrive, Customize, Config screens, PS2 fonts M/S, the 16x12 font at the PS2 size, the PS2 menu sequence scenes (bands), the PAL menu layout (`docs/ui.md`; compared with the PS2 in PCSX2 through injected calls, `docs/compare.md`; not in Fahrenheit). Todo: sphere grid, battle HUD (help bar, CTB list, party list), the battle command window's "EKSTASE!!" tab and "BEFEHL" label, title picture placement, worldmap |
| Config file + converter `build` command | all | done | `docs/config.md` |
| Fahrenheit mod package | | wip | mod `yuta.demaster` (F8 status window), `demaster.exe`; release zip (`tools/package.cs`: mod, `demaster.exe`, `demaster.toml`, players' `README.txt`, `config.md`, `LICENSES.txt`; built and checked locally, no game data); CI (mod, Linux tests, Windows `demaster.exe` + `ffx-music-studio.exe`, zip check) and tag releases (`docs/ci.md`). Open: CI has never passed, it needs the secret `FFX_RUST_TOKEN` (fine-grained PAT, `yutamago/ffx-rust` Contents read-only, `docs/ci.md`); first release not tagged; F8 window not seen on screen |

## Milestone 2: models

| Item | Rows | Status | Notes |
|---|---|---|---|
| PS2 mesh → Phyre model converter | 1-4 | done | 165 models: 32 party, 95 NPCs, 16 monsters, 2 aeons, 5 weapons, 15 objects (`docs/models.md`). Tidus face checked in game |
| Texture animation (eyes, mouths), cloth | 1 | done | 110 converted characters blink/talk, kept-mesh frames repainted (~2,900); PS2 cloth pieces simulated (55 characters). Static: c107, c041, n295, n333, n343, m124, m070-m076 (`docs/models.md`) |

## Milestone 3: interface

| Item | Rows | Status |
|---|---|---|
| PS2 font | 13 | done | EFIGS share one font: PS2 glyphs + widths, PS2 damage digits (`docs/font.md`); German umlauts seen in preview only |
| Title logo, loading screen, splash videos | 16 | done | splash: OpeningScreen.swf skipped by hook; not seen on screen |
| Battle dialogue subtitles | 17 | dropped | the PC already subtitles scripted battle dialogue; no case found where the PS2 did and the PC doesn't (user, 2026-11-02). `ui.battle_subtitles` only forces the save's subtitle bit on |
| Battle menu input delay | 18 | done (static) | 8 delay branches removed (`docs/patches.md`); not timed in game |

## Milestone 4: movies and screen

| Item | Rows | Status |
|---|---|---|
| PS2 FMVs into the PC video format | 15, 21 | done | 49 of 53 (4 with 3D overlays stay HD); ffmpeg downloaded on first use (`docs/movies.md`) |
| 4:3 aspect ratio | 14 | done | final copy squeezed to 4:3 + PS2 camera aspect; movies 960x720; 105 rooms with PS2 paintings (`docs/screen.md`). Mouse on PS2-style menus unchecked |
| PS2 2D backgrounds | 7 | done | 68 rooms at 16:9, 105 with 4:3; animated parts show one PS2 frame |

## Milestone 5: rendering

| Item | Rows | Status |
|---|---|---|
| Glow / blow-out off | 10 | done | blur-and-add pass skipped (`docs/render.md`) |
| PS2-like lighting | 10 | done | PS2 formula from the VU1 program: level 254/128, max(0,N·L) via 1,854 patched shaders, HD fill light off (`docs/render.md`) |
| Shadows | 11 | done | blob shadows only; check maps that normally use shadow maps for artefacts |
| Texture filtering option | 19 | done | nearest for model/map samplers only; all converted textures now have mip chains |
| Soundtrack default "Original" | 20 | done (static) | new-game save gets Original; menu switch still works |
| PS2 music engine (offline render): our port of iopsound.irx + SPU2 model renders the 89 tracks into the `music_ps2` bank | 20 | done | 70/86 tracks correlate ≥0.9 with the PC Original recordings; 3 rock tracks copied (streamed layers); in-game capture matches our render (`docs/music.md`) |
| PS2 music engine phase 2: live, driven by the game's iMusic* commands (runtime tempo/fades, streamed guitar/vocal layers) | 20 | todo | plan in `docs/music.md` |

## Timing

| Item | Rows | Status |
|---|---|---|
| Frame rates, waits, PAL vs NTSC vs PC | | done: measured in PCSX2 and read from both binaries (`docs/timing.md`) |
| Run speed | 23 | done (static), `timing.run_speed`, default PC; not run in the game |
| PAL speed (`timing.pal_speed`) | | open: user decision; what it would take in `docs/timing.md` |
| Lip-sync lag (forum: audio 1/5 s behind) | | open: not explained by the frame logic; needs an audio capture on PC |
| PS2 slowdown timing outside the 3 sync scenes | | dropped for now: needs NTSC PS2 recordings |

## Fixes

| Item | Status |
|---|---|
| NPC mouths open in cutscenes | unconfirmed | only source: one uncited line in the FF Wiki's version differences; forums only report lip-sync timing. No PC-specific cause in the jaw code; converted PS2 NPCs avoid both suspected causes. Next: compare cutscenes PS2 vs Demaster (`docs/compare.md`, `docs/debugger.md`); drop `fixes.npc_mouths` if nothing shows (`docs/patches.md`) |
| Farplane video | done (static) | movie-init branch patched; particles during the video not checked |
| Lulu's Fury input | done (static) | skipped diagonal filled in; no real stick test |

## Changes wanted in ffx-rust

(none yet)
