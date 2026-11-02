# PS2 vs. HD Remaster: what differs and what the mod does about it

Baseline: the PS2 PAL disc (2002, built on *FFX International*). Target: the
Steam HD Remaster (`FFX.exe` sha256 `0537b2a1…686d`). Sources and evidence:
[`research/differences.md`](research/differences.md) (catalogue with URLs),
[`research/hd-vs-ps2-assets.md`](research/hd-vs-ps2-assets.md) (measured mesh
and UV comparison), [`research/ps2-textures.md`](research/ps2-textures.md),
[`disc.md`](disc.md).

The rule for defaults: **presentation from the PS2, logic from the PC.**
Every row with a setting can be switched back to the HD version.

Effort: **S** a texture or data swap with known formats; **M** needs a
converter step that doesn't exist yet; **L** needs engine or shader work.

## Presentation (default: PS2)

| # | Difference | PS2 | HD | How the mod restores it | Setting | Effort |
|---|---|---|---|---|---|---|
| 1 | Party models (Tidus, Yuna, …; field `c0xx`, battle `c1xx`) | Original meshes and 256x256 8-bit textures | Every party member remodelled (2-3x the triangles, new 4-8x atlases) | Convert the PS2 `.chr` mesh and textures into a new `.dae.phyre`/`.dds.phyre` on the PS2 skeleton (bone *i* = `SkeletonNode_i`) | `models.party` | M |
| 2 | NPC models | Original | 129 kept, 95 remodelled (n008, n009, n026, n080-82, most of n143-n356) | Kept: texture swap. Remodelled: mesh conversion as in 1 | `models.npc` | S / M |
| 3 | Monster models | Original | 304 kept with upscaled textures, 27 remodelled (8 fully: m112, m124, m141, m213, m222, m248-m250) | As in 2 | `models.monster` | S / M |
| 4 | Aeons, weapons, objects | Original | Mostly kept; s013, s015, s016, s031, w031, w041, w080, w082, w083 and 21 objects remodelled | As in 2 | `models.aeon`, `models.weapon`, `models.object` | S / M |
| 5 | Field map textures | PS2 4/8-bit textures (`mapout.txa`) | Upscaled and repainted (~4x); HD texture names encode the PS2 VRAM region | Cut each named region out of the PS2 VRAM image and write it into the HD page | `maps.field` | S |
| 6 | Field map geometry | Original | ~92% kept; some maps extended or remodelled (sins02, parts of cdsp00, bsil00's sky/ground) | Not planned: geometry swap needs a map writer. PS2 textures on the HD geometry first | (`maps.geometry`, later) | L |
| 7 | 2D backgrounds of fixed-camera rooms | 4:3 paintings | Repainted wider for 16:9 | Swap in the PS2 image (with 4:3, see 14) | `maps.backgrounds` | M |
| 8 | Battle map textures | PS2 | Upscaled | As in 5 | `maps.battle` | S |
| 9 | Spell, summon and effect textures | PS2 | 95% exact 2x upscales, same names | Texture swap | `effects.textures` | S |
| 10 | Lighting and glow | PS2 vertex lighting, no glow | PhyreEngine relighting, strong glow/blow-out (Besaid, Mi'ihen, Bikanel) | Turn off the glow pass first (known: vertex shader `7e29e6ac`); PS2-like character shading later | `render.glow`, `render.lighting` | M / L |
| 11 | Shadows | Simple PS2 shadows | Shadow maps | Option to use the blob shadow only | `render.shadows` | M |
| 12 | Menus, cursor, windows, portraits, icons | PS2 blue windows | New UI; some submenus draw gradients from vertices | Texture swap for the PS2-drawn menus (texture names follow PS2 VRAM pages); the Flash screens (Esc menu, save/load, settings) have no PS2 counterpart and stay HD | `ui.menus`, `ui.portraits` | S / M |
| 13 | Font | PS2 font | New typeface; 962 per-event/per-battle font pages with their own glyph tables | Rebuild the font pages and glyph tables from the PS2 font | `ui.font` | M |
| 14 | Aspect ratio | 4:3 | 16:9 only | Pillarbox output and restore the PS2 projection; no mod has done this yet | `render.aspect` | L |
| 15 | FMVs | 4:3, PS2 MPEG quality | Cropped to 16:9, clearer | Convert the PS2 movies into the PC video format (with black bars, or with 14) | `movies` | M |
| 16 | Title logo, loading screen, splash videos | Plain logo, no loading screen | "HD Remaster" logo, Abes loading screen, Square Enix/Virtuos/CRI splashes | Swap logo; hide loading screen; skip splashes | `ui.title`, `ui.loading` | S |
| 17 | Battle dialogue subtitles | Shown (International) | Not shown | Turn the PS2 subtitles back on | `ui.battle_subtitles` | dropped (plan.md) |
| 18 | Battle menu input delay | None | 0.2-0.45 s delays | Remove the delays | `ui.battle_menu_delay` | S |
| 19 | Texture filtering | Bilinear at PS2 resolution | Anisotropic at HD resolution | Leave to the game's filter; option for nearest | `render.texture_filter` | S |
| 20 | Music | PS2 sequenced soundtrack | Arranged by default; in-game switch to "Original" | Set the game's own option to Original | `audio.soundtrack` | S |
| 21 | Movie audio | 5.1 in some movies | Stereo | Comes with 15 | `movies` | M |
| 22 | Button prompts | PS2 glyphs | Xbox-style | PS2 glyph textures | `ui.button_glyphs` | S |

## PC bugs (default: fixed)

| Bug | Setting |
|---|---|
| NPC mouths open in some cutscenes | `fixes.npc_mouths` |
| Farplane background video never plays | `fixes.farplane_video` |
| Kimahri's Spirit Lance missing parts from one side | comes with 1 |
| Lulu's Fury spin often not registered | `fixes.lulu_fury` |

## Logic and content (default: PC, untouched)

The HD Remaster's balance is the same International base as the PAL disc.
The mod leaves these alone:

- Eternal Calm, Last Mission, the -Will- audio drama;
- the boosters (F1-F5: speed-up, Supercharge, encounters, auto-battle, hide HUD) and the Esc-menu cheats;
- auto-save, PC controls and remapping, the graphics options;
- PC text (the PAL text is not used), English/Japanese voices;
- 30 fps logic. PAL's 50 Hz speed and black bars are not reproduced.

## Things with no PS2 counterpart (stay HD)

- HD-only party models: costumes c307, c801-c817;
- the Flash screens: Esc menu, save/load, settings, pause text;
- minimaps;
- Eternal Calm.
