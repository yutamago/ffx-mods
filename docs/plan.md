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
| PS2-drawn menu textures, portraits, icons, windows | 12, 22 | wip | done: title, portraits, battle atlas, pad glyphs, loading, message/choice windows, battle command windows, PS2 cursor, item icons, battle words, Overdrive tab, main menu and its Items, Abilities, Status, Equip, Overdrive, Customize, Config screens, PS2 fonts M/S, the 16x12 font at the PS2 size, the PS2 menu sequence scenes (bands), the PAL menu layout (`docs/ui.md`; compared with the PS2 in PCSX2 through injected calls, `docs/compare.md`; not in Fahrenheit). the battle HUD (help bar, CTB list, party list), the battle sequence scenes ("EKSTASE!!" tab, "BEFEHL" footer, glow, lines), the command window's numbers in the 16x12 font, the title picture at the PS2 place (compared with the PS2: battle 26.9 -> 19.3, title 51.0 -> 18.2). The world map picture and its location plates and the sphere grid's PS2 texture pages (converted and hooked, not run in the game). the battle target info window and its help-bar line (compared: 29.9 -> 20.3), the message choice cursor (PS2 hand and ball, title), PS2 text on the pen line (no European 0.78-line offset) with 20-line line feeds, the menu scene lines on; the Overdrive mini-game windows (Tidus, Auron, Lulu, timer; code, not compared with the PS2), the world map picture and location plates, the sphere grid texture pages (not compared). Todo: the sphere grid's panels (PS2 `0x00260378`), Wakka's reels, Libra (Scan) window, enemy gauges |
| Config file + converter `build` command | all | done | `docs/config.md` |
| Fahrenheit mod package | | wip | mod `yuta.demaster` (F8 status window), `demaster.exe`; release zip (`tools/package.cs`: mod, `demaster.exe`, `demaster.toml`, players' `README.txt`, `config.md`, `LICENSES.txt`; built and checked locally, no game data); CI (mod, Linux tests, Windows `demaster.exe`, zip check) and tag releases (`docs/ci.md`). Open: first release not tagged (ffx-rust is public now: CI needs no token). The F8 window is seen in the real Fahrenheit install (2026-11-04: opens and closes on F8, "The PS2 files match demaster.toml (32744 files)") |

## Milestone 2: models

| Item | Rows | Status | Notes |
|---|---|---|---|
| PS2 mesh → Phyre model converter | 1-4 | done | 165 models: 32 party, 95 NPCs, 16 monsters, 2 aeons, 5 weapons, 15 objects (`docs/models.md`). Tidus face checked in game |
| Texture animation (eyes, mouths), cloth | 1 | done | 110 converted characters blink/talk, kept-mesh frames repainted (~2,900); PS2 cloth pieces simulated (55 characters). Static: c107, c041, n295, n333, n343, m124, m070-m076 (`docs/models.md`) |

## Milestone 3: interface

| Item | Rows | Status |
|---|---|---|
| PS2 font | 13 | done | EFIGS share one font: PS2 glyphs + widths, PS2 damage digits (`docs/font.md`); German umlauts seen in preview only |
| Title logo, loading screen, splash videos | 16 | done | title picture and menu at the PS2 place (`docs/ui.md`); splash: OpeningScreen.swf skipped by hook; not seen on screen; the PS2's "SQUARESOFT presents" boot screen back with `ui.boot_logo` (off by default; checked in the game against PCSX2) |
| Battle dialogue subtitles | 17 | dropped | the PC already subtitles scripted battle dialogue; no case found where the PS2 did and the PC doesn't (user, 2026-11-02). `ui.battle_subtitles` only forces the save's subtitle bit on |
| Battle menu input delay | 18 | done (static) | 8 delay branches removed (`docs/patches.md`); not timed in game |

## Milestone 4: movies and screen

| Item | Rows | Status |
|---|---|---|
| PS2 FMVs into the PC video format | 15, 21 | done | 49 of 53 (4 with 3D overlays stay HD); ffmpeg downloaded on first use (`docs/movies.md`) |
| 4:3 aspect ratio | 14 | done | final copy squeezed to 4:3 + PS2 camera aspect; movies 960x720; 105 rooms with PS2 paintings (`docs/screen.md`). Battle and field framing match the PS2 in PCSX2 (`docs/compare.md`, "The 3D framing"); cutscenes not compared. Mouse on PS2-style menus unchecked |
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
| PS2 music engine phase 2: live, driven by the game's iMusic* commands (runtime tempo/fades, streamed guitar/vocal layers) | 20 | wip | `audio.music_engine = "live"`: `demaster_live.dll` (the driver in the game through FMOD's event imports and a user stream), job `audio.live`; in the game (harness and Fahrenheit): starts, stops, loops, the battle's pause and resume of the field song, fades, fanfare, the volume setting, pause, the tempo commands, all as the game asks; output = the offline render sample for sample. Todo: the 3 rock tracks' streamed layers (stay the bank's recording), a comparison by ear with PCSX2. Default stays `"ps2"` (the bank) (`docs/music.md`, "Phase 2") |

## Timing

| Item | Rows | Status |
|---|---|---|
| Frame rates, waits, PAL vs NTSC vs PC | | done: measured in PCSX2 and read from both binaries (`docs/timing.md`) |
| Run speed | 23 | done (static), `timing.run_speed`, default PS2 (34; user, 2026-11-04); not run in the game |
| PAL speed (`timing.pal_speed`) | | dropped: the user chose NTSC timing as "the PS2 original" (2026-11-04); what it would take stays in `docs/timing.md` |
| Lip-sync lag (forum: audio 1/5 s behind) | | measured: the PC voice is heard 167-192 ms after its start frame (PS2 10-40 ms), 80-120 ms later against the picture than the PAL PS2, about 1/5 s after the mouth starts; FMOD's default 1024 x 4 output buffer is most of it. Fix proposed (smaller DSP buffer before `EventSystem::init`), not built: needs a Fahrenheit measurement (`docs/timing.md`, "Voice start") |
| PS2 slowdown timing outside the 3 sync scenes | | dropped for now: needs NTSC PS2 recordings |

## Fixes

| Item | Status |
|---|---|
| NPC mouths open in cutscenes | done (`fixes.npc_mouths`): the PC's `Ch_Mouth` sets the mouth texture frame inverted against the PS2, so an NPC (208 models with a texture mouth) keeps the open frame after talking; measured on both (agency clerk: PC 1/1/0, PS2 1/0/1); the patch (`jle` -> `jg` at `0x00834714`) not yet run in Fahrenheit (`docs/patches.md`) |
| Farplane video | done (static) | movie-init branch patched; particles during the video not checked |
| Lulu's Fury input | done (static) | skipped diagonal filled in; no real stick test |

## Changes wanted in ffx-rust

(none yet)
