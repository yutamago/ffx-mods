# The interface: menus, portraits, title, loading screen, button glyphs

Rows 12, 16 and 22 of [`differences.md`](differences.md). Converter:
`converter/src/uijob.rs`. The font (row 13) is in [`font.md`](font.md).

```bash
./target/release/demaster ui <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out>
```

writes `<out>/efl/x/<HD path>` and a preview per file
(`<out>/preview/<folder>_<name>.png`, HD left, result right). In
`demaster build` the jobs are `ui.menus` (settings `ui.menus` and
`ui.portraits`), `ui.title`, `ui.loading`, `ui.glyphs`
(`ui.button_glyphs`) and `ui.font` (`fontjob.rs`, [`font.md`](font.md)).

## Summary (German disc, 2026-11-02)

| Part | Files | In the game |
|---|---|---|
| Title screen (`map/titl/titl00/2d`): PS2 logo, menu, bar at the PS2 place; the menu cursor moved with it (`mod/TitlePatches.cs`) | 12 layers (EN x2, DE, FR, IT, ES, 6 "Install" labels) | checked (`boot.txt`, English); compared with the PS2 (`compare-title`) |
| "FINAL FANTASY X PROJECT" logo (`titl02`), "Fine" screen pieces (`titl01`) | 4 (the HD kept the PS2 pictures) | logo checked |
| "HD Remaster" under the opening's logo (`magic_0662/logo_hdremaster`) | 1, transparent | not reached by a replay |
| Loading screen (`menu/loading`: `ld00..24`, `loadingbg`) | 26, transparent | not seen: the harness loads hide it |
| Battle atlas (`menu_de`, `menu_us` `battle`): CTB faces, markers, letters; the damage digits with `ui.font` (also into `menu_fr`, `menu_it`, `menu_es`, `font.md`) | 2 (15 and 16 PS2 elements each), 5 with `ui.font` | checked (`battle-attack.txt`): PS2 CTB faces |
| Menu faces `face_ply`, `face_smn`, `sface_ply` | 14 (every language folder) | checked (`tools/replays/ui-status.txt`): PS2 face on the Status page |
| Button glyphs (`menu/d3d11/pad_icon`) | 1 | not checked (the harness pad shows no prompt in these replays) |
| Message window atlas `meswin`: the PS2 window pieces (below, "The PS2 windows") | 8 (every language folder) | checked (`tools/replays/ui-dialogue.txt`, `ui-dialogue-field.txt`): PS2 message windows |
| Battle command windows (code only: `mod/BattlePatches.cs`, below, "The battle command windows") | 0 | checked through injected game calls (`tools/replays/ui-battle-main.txt`, `ui-battle-item.txt`): PS2 gradient box and layout; hooks seen installing in Fahrenheit |
| Main menu screen: background, party rows, options, help bar, footer (`ps2win` texture + `mod/MainMenuPatches.cs`, below, "The main menu") | 8 (`ps2win`, every menu folder) | checked through injected game calls (`tools/replays/ui-mainmenu.txt`); hooks seen installing in Fahrenheit |
| The PS2 cursor (code: `TOMakePktShapeCursor` kind 0, below) | 0 | checked through injected game calls (battle windows) |
| Item icons, battle words, labels (`icon`, `strtex`; below, "Item icons, battle words, labels") | 8 `icon`, 2 `strtex` | checked through injected game calls (`tools/replays/ui-items.txt`, `ui-battle-overdrive.txt`) |
| The battle Overdrive tab (`ps2btl` + `mod/BattleSeqPatches.cs`, below) | 8 (`ps2btl`) | checked through injected game calls (`tools/replays/ui-battle-overdrive.txt`) |
| Items and Abilities screens (`mod/ItemMenuPatches.cs`, below) | 0 | checked through injected game calls (`tools/replays/ui-items.txt`, `ui-abilities.txt`); hooks seen installing in Fahrenheit |
| The PS2 menu fonts M and S (`ps2fnt` + `mod/Ps2Menu.cs`, below) | 8 (`ps2fnt`) | checked through injected game calls (`ui-mainmenu.txt`, `ui-items.txt`, `ui-abilities.txt`) |
| Status, Equip, Overdrive, Customize, Config screens (`mod/*MenuPatches.cs`, below) | 0 | checked through injected game calls (`ui-status-ps2.txt`, `ui-equip.txt`, `ui-overdrive.txt`, `ui-customize.txt`, `ui-config.txt`); not run in Fahrenheit |
| The 16x12 font at the PS2's size, the menu sequence scenes (`mod/Ps2Menu.cs`, below) | 0 | compared with the PS2 in PCSX2 (`docs/compare.md`) through injected game calls |
| The PAL menu layout (`jppc/menu/menu_script.bin`, below, "The menu layout data") | 1 | compared with the PS2 (Config) |
| The battle HUD: help bar, CTB list, party list (`mod/BattleHudPatches.cs`), the battle sequence scenes (BEFEHL footer, glow, lines; `mod/BattleSeqPatches.cs`), command window numbers in the 16x12 font (below, "The battle HUD", "The battle sequence scenes") | 0 | compared with the PS2 (`compare-battle-kami`) through injected game calls |

68 files in all; 20 HD textures have no PS2 counterpart (listed by
`demaster ui`). Previews and screenshots: `~/ffx-work/demaster/research/ui/`
(`preview/`, `ingame-*.png`).

## What a texture swap can and cannot do here

The PC kept the PS2 menu code (the `TO*`/`TkMn*` functions) but replaced the
GS packet leaves with `graphicDrawUIElement` / `graphicFontDraw`
(ffx-rust `docs/formats/ui-draw.md`) and **rebuilt the menu look**: the main
menu, its plates and backgrounds come from HD-only sheets (`menu_new`,
`battle_kuang`, `stonetexture`, `texture`, `ffx_bg`, 1920x1080 coordinates,
UVs such as `1879, 542` that exist only in those sheets). The PS2's blue
gradient windows were drawn from vertex colours (`NoTexture` quads on the
PC too), with the PC's colours. So **the PS2 menu look is code, not
textures**: the main menu (`ingame-mainmenu.png`) needs the PS2 draws
ported into the mod, with the PS2 `meswin` as a texture of its own (done for
the main menu screen, below, "The main menu"; the sub-menus stay HD). The texture swap
restores the pieces that are still PS2 art at PS2 places: faces, CTB list,
title, markers.

## PS2 menu textures

The PS2 menu images are raw indexed images, the palettes in one file:

| PS2 (`jppc/menu/`) | Format | HD (`menu*/d3d11/`) | Scale | VRAM name (`TOGetShapTextureName`) | Palette |
|---|---|---|---|---|---|
| `meswin.fmt` | 256x256, 8 bit | `meswin` 1024x1024 | 4 | `0x3e00` | 0 |
| `battle.fmt` | 256x256, 8 bit | `battle` 1024x1024 | 4 | `0x3f00` | 1 |
| `icon.fmt` | 256x128, 4 bit | `icon` 1024x512 | 4 | `0x3dc0` | per icon: a 16-colour row of CLUT 8-15 (below, "Item icons") |
| `strtex.fmt` | 512x128, 4 bit | `strtex` 2048x512 | 4 | `0x2ecc` (the PC's name; on the PS2 it sits at `0x3f00` in battle) | 13, row 7 |
| `face_ply.fmt` | 512x256, 8 bit | `face_ply` 1024x512 | 2, other layout | `0x2ed0` | 10 |
| `face_smn.fmt` | 512x256, 8 bit | `face_smn` 1024x512 | 2, other layout | `0x2ed4` | 11 |
| `sface_ply.fmt` | 256x128, 8 bit | `menu_*/sface_ply` 256x128 | 1, same pixels | | 10 |
| `worldmap.fmt` | 512x416, 8 bit | `worldmap` 1920x1080 | other layout | `0x2eac` | not done |
| `xfont1208.fmt`, `subfont.fmt` | 128x256, 4 bit (two 2-bit pages) | `menu*/xfont1208`, `subfont` | 1 | `0x3d80` | font (`font.md`) |
| `menu.clp` | 64x64 PSMCT32: 16 CLUTs of 256 colours, each a 16x16 CSM1 rectangle (CLUT *p* at x `16 (p % 4)`, y `16 (p / 4)`) | | | | |

`TOGetImageWH` (`0x008ac400`) returns the **PS2** size for these names
(256x256 for `0x3e00`/`0x3f00`, 256x128 for `0x3dc0`, 512x128 for
`0x2ecc`) and `TOMkpShapeXYWHUV` divides the texel UVs by it, so the HD
art is the PS2 layout at 4x and a PS2-size image fits the same UVs.

Palettes were picked by rendering each image with all 16 CLUTs and
comparing with the HD art by eye (the HD colours were repainted, so a
colour fit picks wrong CLUTs: the black CLUT 2 wins on dark sheets).
`icon` and `strtex` use 16-colour rows of the CLUTs (each icon its own
row), so no whole CLUT fits them; below, "Item icons, battle words,
labels".

The German disc holds `jppc/menu` in German and `uspc/menu` in English
(byte-identical to the PC's `uspc` copies). Text-bearing atlases go only
to the matching folder: `jppc` to `menu_de` (`menu_us`, `menu_fr`, ... for
other regions), `uspc` to `menu_us`. Faces have no text and go to every
folder.

### Atlases: element by element

`meswin` and `battle` were redrawn in part: the HD moved some pieces (the
feather pen to the top left; its PS2 place holds a black silhouette the PC
draws as its shadow), replaced others (the `SEKUNDÄR`/`PRIMÄR` tabs where
the PS2 had arrow bars), and repainted text in another typeface. The job
therefore decides per HD element (a connected region of HD alpha, at PS2
size): the PS2 texels in the element's box replace it only if

- the PS2 and HD alpha masks (dilated by one texel) overlap with IoU >=
  0.65 in the box, grown by the PS2 regions the element touches that lie
  mostly inside it;
- no large PS2 region the element touches reaches far out of it, and no
  PS2 texel lies within 3 texels left or right of the box (a longer PS2
  word: cutting it would show part of it);
- the HD element is not dark where the PS2 one is bright (a shadow);
- the element is in the portrait region (battle x >= 190) and
  `ui.portraits` is on, or elsewhere and `ui.menus` is on.

The result is written at the HD size: HD elements stay at full
resolution, PS2 elements are scaled up bilinear (what the game's filter
would do with the PS2-size image). Result: `battle` 15/16 PS2 elements
(CTB faces, `mon` markers, B markers, `BOSS`, A-H letters, ATB dots,
arrows); `meswin` 0. With looser rules (IoU 0.5, no cut check) more
elements pass, and the previews show the problems: HD labels replaced by
PS2 bars, `BEFEHL` cut to `BEFE`.

### Portraits

The HD face sheets have a new layout: `face_ply` 4x2 cells of 200x200 HD
pixels against the PS2's 4x2 cells of 91x98 texels (and new crops of the
art, so the luminance correlation between a cell and its PS2 twin is near
0). The job keeps the PS2 order, resamples each PS2 cell into the HD cell's
place at PS2 resolution (512x256), and swaps two cells only when that
raises the correlation by 0.4: the European HD sheets swap Cindy and Sandy
(`face_smn` cells 7 and 8), the Japanese one does not. Aspect: the PS2
cell is 91x98 texels on a 4:3 screen (about 1.08 wide), the HD cell is
square; the PS2 face is stretched to the square, 8% narrower than on PS2.
`sface_ply` is the PS2 sheet unchanged in the HD (correlation 1.0 per
cell), so it is written as is. Faces are made opaque (the PS2 alpha is
0x80 everywhere).

The CTB list's faces are in `battle` and follow `ui.portraits`.

## Title screen and logos

The title is a 2D map, `map/titl/titl00/2d/`: one full-screen quad per
layer (`titl00.dae.phyre`, UV 0..1), `0_<layer>_<w>_<h>_0.dds.phyre`. The
PS2's are the PSMT8 transfers of `jppc/map/titl/titl00/bin/mapout.vpa`
section 0 (`YNSC`), each with the 16x16 CT32 CLUT that follows it
(`uijob::bg_layers`). Same order, same sizes for layers 0-12:

| Layer | PS2 | HD |
|---|---|---|
| 0 | English, upper case (International) | the same, with "HD Remaster" |
| 3 | English (PAL) | Traditional Chinese (kept HD) |
| 5, 7, 9, 11 | Italian, French, German, Spanish | the same languages |
| 13 | | English (gets PS2 layer 3) |
| 14, 15 | | Japanese/Chinese, Korean (kept HD) |
| 2, 4, 6, 8, 10, 12 | "Install" labels | the same |

The PS2 image is the whole screen (512x448). Its top holds the PAL
web-site banner and the water strip; in the game the PC draws its own
water band (`menu/titl/water_NN`, Tk) over that part, so the banner is
covered (`ingame-title.png`). The PS2 shows the banner (PCSX2, the
`title` state, `docs/compare.md`); here it stays covered (intended).

**Placement** (`uijob::title_placed`, measured with `tools/compare`). The
PS2 shows rows 16..432 of the image 1:1 on its 512 x 416 screen. The PC
draws the layer as a 4:3 quad as wide as the screen and bottom-aligned:
texture row v at screen line `v * 960 / 448 - 240` of 720 (16:9 and 4:3
alike; fitted on harness shots), so it showed rows 112..448, 1.24 times
as big: the logo's top 60 PS2 lines higher, the bottom bar 25. The
converter now writes each 512 x 448 `titl00` layer laid out for that quad
(at twice the PS2 size, 1024 x 896, so the 0.8 scale keeps the PS2
detail): row v holds the PS2 row the PS2 showed at that screen line,
`16 + (v * 960 / 448 - 240) * 416 / 720`; the rows above the water strip
(the banner, PS2 rows 0..83) are black, so the PC's translucent water band
lies over black as before. In 16:9 the vertical framing is the PS2's too,
the picture stretched sideways as before.

**The menu** ("Neues Spiel" / "Laden" are part of the picture; the cursor
is the title menu message's, `(0x1c7, 0xc7)` on map 0x17): the PC lays the
message out with HD numbers, its rectangle from the message's f32 `+0x34`
(y 211; `TkMenuMessageLayout_008de370` also pins x to 0x160, 0x154 in
language 3) and the second choice 16 + 21.5 lines below the first
(`TOLayoutMessageChoices_008b9a60`, f64 `0x00b655e8`, read only there).
`mod/TitlePatches.cs` (`ui.title`, so that setting is now converter and
mod) runs the layout with `+0x34` 62 lines lower and the constant at 6.0:
the cursor sits at the PS2 picture's lines (280 and 302, 22 apart).
Checked in the harness on the game's own code (`FFX_HOOK_DISABLE=
replacements`: ffx-rust ports the message code) with call detours that
do the same (`tools/replays/compare-title.txt`), both choices. The other
`titl01`/`titl02` screens (the "Fine" and project logos) were not
compared and keep the PC's quad placement.

`titl02` (the "FINAL FANTASY X PROJECT 1999-2002" logo) and three
`titl01` layers are the PS2 pictures in the HD too (correlation 1.0): they
are written as PS2 for consistency. The other `titl01` layers ("Fine" at
the end) have a different HD layout and stay HD.

`magic_0662/tex/d3d11/logo_hdremaster` is the "HD Remaster" strip under
the logo in the opening: written transparent (the effects job leaves it
HD).

## Loading screen

`LoadingScreen_loadTextures` (`0x006e4770`) loads `menu/loading/d3d11/ld00`
.. `ld19` (spinner frames; `ld20`-`ld24` exist too) and `loadingbg` (the
Abes picture). The `ui.loading` job writes them as 8x8 transparent
textures, so the loading screen draws nothing (black). Not seen in the
harness (its loads are instant). The autosave icon (`autosaving`) and the
cloud-save screens stay. Hiding the screen entirely (not drawing its
black frame) and skipping the splash videos need code (`mod/Patches.cs`,
still planned); so `ui.loading` stays a mod setting, and changing it needs
a converter build too for the textures.

## Button glyphs

The PC draws every button prompt through `RemapButtonIcon_remapButtonIcon`
from `menu/d3d11/pad_icon` (pad) or `keyboard_icon`: a 12x8 grid of 34x34
cells, u = column / 12, v = row / 8 (`RemapButtonIcon_getJoypadUV`). Row 0
is X A B Y LB RB LT RT Back Start LS RS. The PS2 glyphs exist on the disc
(`icon.fmt`: △ ✕ ○ □, L1 L2 R1 R2, SELECT, START, grey: coloured at run
time), and the HD `icon` atlas has them too in colour (the PS4 version's,
in PS2 order but with ✕ and ○ swapped, at 4x). The job writes `pad_icon`
with: X → □, A → ✕, B → ○, Y → △, LB → L1, RB → R1, LT → L2, RT → R2
from the game's own `icon` atlas, and Back → SELECT, Start → START from
the PS2 `icon.fmt` (grey ramp). The sticks (LS, RS) and the d-pad arrows
stay. `keyboard_icon` stays (no PS2 counterpart).

Uncertain: the glyph colours (PS2 green △, red ○, blue ✕, pink □ as in the
HD icon) were not checked against PS2 footage; the PS2 prompts may have
been grey with coloured symbols as drawn by the vertex colour.

## No PS2 counterpart (stay HD)

- Iggy/Flash screens (`ps3data/flash/*.swf`): `escmenu` (Esc menu and its
  settings pages), `saveload`, `pausemenu`, `openingscreen`, `box` (tips),
  `autobattle`, `autobattle_disable`, `chocobo`, `enemy_0`/`enemy_1`,
  `freecamera`, `invincible`, `speed_2`/`speed_4`, `time_stop` (booster
  pop-ups).
- HD-only menu sheets: `menu_new`, `battle_kuang`, `stonetexture`,
  `texture`, `ffx_bg`, `summonbg`, `guide_circle`, `pause`, `skip_*`,
  `copyright_nagi`, `keyboard_icon`, `cloudsavetex/*`, `menu/titl/water_NN`,
  `loading/autosaving`, `savedataicons/*`.
- Title layers 3 (Chinese), 14, 15 and `titl01`'s "Fine" layers.

## The PS2 windows (`ui.menus`: texture and code)

**What the PS2 drew.** The PS2's message windows come from one routine,
`TOMkpWindowPlain` (FFX.exe `0x008f9a40`, SLPM_675.13 `0x001fa550`). It
stretches four meswin shapes, 200-203, over the window. They are FFX.exe's
meswin layout table (`0x00c5b460`) entries 0-3, in PS2 texels: (160, 56)
16x24 top-left corner, (176, 56) 8x24 top edge, (160, 80) 16x8 left edge,
(176, 80) 8x8 body. The corner goes at (x, y), the top edge stretches to
`w - 16`, the left edge to `h - 24`, and the body covers the rest. The
pieces are white, and their alpha gives the window its brushed top-left
edge. The top edge and the body take a horizontal gradient from
`col_mssgwind0` on the left to `col_mssgwind1` on the right. The corner and
the left edge take `col_mssgwind0`. These colours come from the PS2 menu
sequence data: `TOSeqInitParam` (`0x008ae6f0`) reads them into
`0x025d16b4` / `0x025d16b0`, and the PC still does so at run time. Values
read in the game (harness), bytes R G B A:

| Name (global) | RGBA |
|---|---|
| `col_mssgwind0` (`0x025d16b4`) | `19 00 31 6b` |
| `col_mssgwind1` (`0x025d16b0`) | `34 00 66 6b` |
| `col_complt0/1` (`0x025d163c/38`) | `4f 28 78 80` / `58 4a 82 80` |
| `col_nameplt0/1` (`0x025d166c/68`) | `24 3e 53 80` / `11 18 40 80` |
| `col_helpplt0/1` (`0x025d1614/10`) | `09 09 09 80` both |
| `col_placeplt*`, `col_helpsubplt*` | `7b 66 28 80` |
| `col_bsoplt0/1` (`0x025d16a0/9c`) | `3a 3a 3a 80` / `27 27 27 80` |
| `col_bitemplt0/1` (`0x025d1698/94`) | `51 42 23 80` / `3c 3f 12 80` |
| `col_babplt0/1` (`0x025d1690/8c`) | `4a 1e 55 80` / `42 0d 39 80` |
| `col_lmtplt0/1` (`0x025d1688/84`) | `1f 38 55 80` / `14 1c 38 80` |
| `col_menubg`, `col_menubg0`, `col_menubg1` (bytes at `0x025d1680`, `167c`, `1678`) | `80 be be be`, `80 aa cd 6b`, `80 c3 87 ba` (the byte order of `TOSeqGetColorRGBA` not worked out) |

So a message window is a translucent (alpha `0x6b`, 84 %) violet-blue
gradient, darker on the left, with no separate border: the PS2's kind-0
code draws nothing else. On screen (harness, 1280x720, over black) the left
end is (42, 0, 82) and the right end (86, 0, 169). That is exactly the
GS's modulate (`texel * colour >> 7`) with alpha `0x6b / 0x80`, so the PC's
menu shader reproduces the PS2 arithmetic for these vertex colours.

**Who used it on the PS2.** In the window kinds' handler table (SLPM_675.13
`0x00323710`, 6 words a kind, like FFX.exe's `g_YiWindowHandlers`
`0x00c5d788`), the draws of kind 0 (`0x00208758`) and kinds 3-9 call
`TOMkpWindowPlain` with the window rectangle, then set the text scissor
(x0 + 12, y0 + 4, x1 - 12, y1 - 4). Kinds 1 and 2 (`0x00208870`) draw no
plate, only the scissor, on the PS2 as on the PC. Other callers are the
yes/no window (`0x002362c0`, centred: the PC's `TOMenuDrawYesNoWindow`)
and fixed-place windows at `0x00223880`, `0x002238e0`, `0x002239b0` and
`0x0023d758` (not identified). `0x001fa0f0`, `0x001fa210` and `0x001fa448`
read the same colours but have no caller.

**What the remaster changed.** FFX.exe keeps `TOMkpWindowPlain` and the
draw of kinds 3-9, `yiWindowDrawPlate` (`0x00907390`), unchanged. Kind 0's
draw, `yiWindowDrawMenu` (`0x00907060`), instead draws
`TODrawMenuPlateXYWHType(x0 + 4, y0, w - 8, h, 10)`: the HD message box
(a shadow, the `stonetexture` body through shape -4, a gradient
`0x608f2c45` / `0x60501826`, the HD `meswin` frame). The HD `meswin` atlas
is empty where the PS2 pieces were, so the plain window drew nothing
visible.

**The restoration.**

- Converter (`ui.menus` job, `uijob::paste_window_pieces`): the PS2 texels
  of entries 0-3 (the box at (160, 56), 24x32) go into every
  `menu*/meswin` (8 folders; no text in them), 4x, texel for texel, and
  only where the HD atlas is empty.
- Mod (`mod/MenuPatches.cs`): a hook on `yiWindowDrawMenu` runs
  `yiWindowDrawPlate`, which is exactly the PS2's kind-0 draw. A hook on
  `TODrawMenuPlateXYWHType` draws type 10 (the remaster's message-box
  plate, also used by the yes/no questions and the battle-end notice) as
  `TOMkpWindowPlain(x, y, w, h)`. The other plate types stay HD.

So `ui.menus` is applied by both (`AppliedBy::Both` in `config.rs` and
`mod/Settings.cs`): changing it needs a `demaster build` and a restart.

**Checked.** In the ffx-rust harness, which can't run the mod's hooks: a
call to `TOSetWindowKind` (`0x00906e60`) sets the window's kind to 3, so
the game itself runs `yiWindowDrawPlate` (the function the hook runs) with
the converter's `meswin`. Two replays: `tools/replays/ui-dialogue.txt`
(New Game's sphere grid choice, both windows kind 0) and
`tools/replays/ui-dialogue-field.txt` (Rin's agency: the spoken line is
kind 1 and stays without a plate; the choice is kind 0). Each is shot at
16:9 and again after `render.aspect`'s 4:3 write. Screenshots in
`~/ffx-work/demaster/research/ui/menus/`: `choice-hd.png`,
`choice-ps2.png`, `choice-ps2-43.png`, `field-line.png`,
`field-choice-hd.png`, `field-choice-ps2.png`, `field-choice-ps2-43.png`,
`meswin-preview.png`. In the PS2 shots of the field choice the cursor is a
small arrow because the cursor depends on the window kind, which the
harness changed; with the hook the kind stays 0 and the cursor is the
game's. In the Fahrenheit install (2026-11-02) both hooks installed (log:
`ui.menus = ps2: PS2 message windows`) and the 8 `meswin` files were
served. No window came on screen there (that install has no input). The
type-10 hook was not seen drawing (the harness can't run it in place).

**Uncertain.** These shots were not compared with PS2 screenshots; the
look follows the PS2 code and data. This path draws no light border; if
PS2 shots show one, another routine draws it. The pieces are upscaled 4x
by nearest neighbour, so the stretched body's edges fade over a quarter
of a PS2 texel instead of the whole texel the PS2's bilinear filter would
blend.

## The battle command windows (`ui.menus`: code)

**What the PS2 drew.** SLPM_675.13's battle window table (`TOBtlSetCallback`,
PS2 `0x00210398`, jump table `0x00570ad0`, draw at window `+0x80`; the PC's is
`+0x90`) gives the main window (kind 0) the draw `0x00210a40` and every other
command window (kinds 1-7, 0xc-0xf, 0x11, 0x14) a draw that calls
`0x00211e88`. Both are one box and the rows, nothing else:

1. `0x001f0330(x0, y0, w, h, c0, c1)`: one untextured, alpha-blended
   gouraud quad (GIF PRIM `0x4c`), `c0` on the left vertices and `c1` on
   the right: a horizontal gradient from `dum91` to `dum92`, the battle
   sequence colours (PS2 `0x00324304`/`0x00324308`; the PC's
   `TOSeqInitParam` still reads them into `0x025d0af4`/`0x025d0af0`). Read
   in the game: `dum91` = `42 88 cb 3c` (light blue, 47 %), `dum92` =
   `42 00 85 6e` (violet, 86 %).
2. A scroll bar `0x001fbfc0` at `x1 - 10`, 4 wide, when there are more lines
   than rows.
3. The text scissor, the window's `+0x78` hook (a no-op for the main
   window), then the rows: 20 pixels apart, one name per cell, grey when
   `TOCheckBtlCommandUse < 0`, hidden when it is -4. The main window
   centres each name in its column; the others draw the parts of `+0x2c`
   left to right (kind 1 command icon +20, 2 item count, 4 MP cost, 5/6
   HP/MP, 7 weapon icons, 0x10 the name +widest name; numbers in cells of
   12 per digit after 5 pixels).
4. `TOBtlSetCurPos(0, x, y)` at the cursor's cell (the text x for the main
   window, the cell's left for the others).

The rectangle (`0x002105d8`, which the PC rewrote with fixed HD numbers): with
`dum79` = (45, 339) (`0x025d0afa`/`0x025d0af8`), the column width `colw` =
the widest shown name plus the parts (`0x00215bc0`), `W = cols * colw +
(cols - 1) * 16`: box x 0 .. max(45 + W + 32, `dumm79`) (`0x023cc090`, 130),
y 339 .. 339 + 20 rows + 2; text from (61, 340). So the PS2 window starts at
the screen's left edge, as wide as its longest command.

`col_bsoplt`, `col_bitemplt`, `col_babplt`, `col_lmtplt` (the four colour
pairs of the table above) are **not** used by the PS2 battle windows: their
only reader in SLPM_675.13 is `TOAdpShapeByKind` (`0x001f4c60`, the PC's
`0x008f1b50`), and its only caller (`0x00226d90`) is called from the menu
module (`0x00227218`..`0x0024ef3c`). They tint menu plates (the main menu's
ability and item lists, presumably), not the battle rows. The earlier note
here said otherwise.

**The restoration** (`mod/BattlePatches.cs`, from `MenuPatches.menus`):
hooks on the PC's seven command-window draws replace them with the PS2 draw
made of FFX.exe's primitives: `TODrawBoxXYWHC2` (`0x008f3db0`, untextured,
PS2 units, `c0` left), `DrawCrossMenuScrollParts`, `TOMakePktScissor`,
`ToMakeBtlEasyEdgeFont` at the PC's battle size (0.78, 1.0) for the names,
`ToGetBtlEasyFontWidth` for the column width, `DrawCrossMenuIconXYWHRGBA`
(12 x 12 PS2 units), `ToMakeBtlEasyDigitRight`, `TOBtlSetCurPos`:

| PC draw | Kinds | PS2 draw | Extras kept |
|---|---|---|---|
| `TODrawWaitBtlWinMain` `0x008a49b0` | 0 | `0x00210a40` (centred) | |
| `TODrawWaitBtlWinSub` `0x008a7520` | 1-4, 6, 0xe, 0x11 | `0x00211cc0` | |
| `TODrawWaitBtlWinLeft` `0x008a3680` | 0xc | `0x00210da0` | |
| `TODrawWaitBtlWinRight` `0x008a6a60` | 0xd | `0x00211528` | |
| `TODrawWaitBtlWinPrepare` `0x008a6870` | 0x14 | `0x002126c8` | the picked cells blink (as the PC) |
| `TODrawWaitBtlWinSummon` `0x008a75f0` | 5 | `0x002123c0` | `TOBtlDrawSummonPrim` at the PS2 place (x `dum21`, y 180 + n `dum22`) |
| `TODrawWaitBtlWinWeapon` `0x008a7dc0` | 7, 0xf | `0x00212580` | `TOBtlDrawWeaponAbilityWindow` at (x1 + 20, 365) |

The command lists, input, help and targeting stay the PC's, so PC-only
commands appear as rows like any other. The hooks keep the window's open
time (`+0xe0`) updated, which the PC's input steps read, and keep the PC's
early return while `0x0133d6e0` is 1 (main, sub). The PC added a command
icon to kinds 1-4 and 0xe (magic and skill lists); the PS2 had none there,
so it is skipped. Not hooked (stay HD): the change window (kind 10) and the
gil window (0x15, 0x16). Dropped with the HD draw: the plate rows, ripples,
rings, the "Commands"/"SUB" footer and the HD **Overdrive tab** of the main
window. The PS2 main draw has no Overdrive tab either (its `+0x78` is a
bare `jr ra`); on the PS2 the battle sequence scene (`TOSeqDraw(2, ...)`,
"bnormalll"/"blimitlay" layouts) may draw it, which the PC still runs but
which was not checked. If the tab is missing in the game, that is where to
look. Looked at again (2026-11-03): the PS2 `meswin` holds the label
("EKSTASE" on the German disc, (0, 224) 136 x 16: layout entry 59, and entry
64 = a dark bar (120, 88) 136 x 24 at (-16, 5) with the label at (28, 3)).
No SLPM_675.13 code passes 59, 64, 259 or 264 to a shape draw as an
immediate, and `battle_script.bin` (the battle sequence data, scenes
`blimitlay`, `bnormallay`, ...) holds the u16 `0x0103` (259) once, at
`0x6a8c`, in records not decoded here. So the tab is most likely a sequence
primitive (`TOSeqDraw(2, ...)` -> `TOMakePktSeqPrimLayout` `0x008fe0b0`,
which draws meswin shapes through `TOCopyWindowLayout`), still run by the
PC; with the HD `meswin` it would draw HD art from those cells. Not
restored: the scene record and the state that shows it were not found, and
no battle with a full Overdrive gauge was reached in the harness.
Found and restored since (2026-11-03): below, "The Overdrive tab".

**Checked.** The harness can't run C# hooks. `tools/calllist` (a shotrun
with three extra commands: the image's load delta, `VirtualAlloc` through
FFX.exe's import table, and `@ps2btl`) puts a 0x60-byte call-list thunk
(`tools/calllist/thunk.s`) into RWX memory, points the open window's draw
callback at it, and fills the list with the calls `BattlePatches` makes for
that window (the same layout code, `tools/calllist/src/btl.rs`, with the
game's `MsGetComData`, `ToGetBtlEasyFontWidth`, `TOCheckBtlCommandUse`,
`MsGetSaveItemNum` called at the frame boundary). The game then makes those
calls inside its own update. Replays `tools/replays/ui-battle-main.txt`
(Rikku's Attack/Special/Item) and `ui-battle-item.txt` (the item list,
two columns, icons and counts), each at 16:9 and after `render.aspect`'s
4:3 write. Screenshots in `~/ffx-work/demaster/research/ui/battle/`:
`main-hd.png`, `main-ps2.png`, `main-ps2-43.png`, `item-hd.png`,
`item-ps2.png`, `item-ps2-43.png`. Measured on screen over the battle
background: the box's left end (42, 82, 121) over (21, 35, 48) and right end
(60, 7, 121) are exactly the GS blend `bg + (c - bg) * a / 128` of `dum91`
and `dum92`, so the NoTexture menu shader reproduces the PS2 arithmetic.
In the Fahrenheit install (2026-11-02) the nine `ui.menus` hooks installed
(log `ui.menus = ps2: PS2 message and battle command windows`); no battle
was reached there (no input), so the hooks' own first-draw log lines were
not seen. (The first item shots were taken with the counts' right edge at the
cell's edge, where the scissor clipped the last digit's outline; the mod
and `btl.rs` end them 2 pixels earlier. Re-shot 2026-11-03 with the PS2
cursor: `btl-1-hd.png`, `btl-2-ps2.png`, `btl-3-ps2-43.png`,
`item-1-hd.png`, `item-2-ps2.png`, `item-3-ps2-43.png` in the same folder:
the "3" of Phoenix Down is whole.)

**Uncertain.** Not compared with PS2 screenshots: the look follows the PS2
code and the run-time colours. The names are the 16x12 font at the PS2's
size since 2026-11-03 (`TOMkpBaseMesFont` inside `Ps2Menu.ps2_sized`, the
PS2's 1612 draw `0x001f3810`; below, "The 16x12 font's size"), and the
column width follows from their PS2 widths; before, they were the PC's
battle typeface (`ToMakeBtlEasyEdgeFont`, 0.78), about half the PS2's size.
The numbers (item counts, MP costs, HP, MP) are the PS2's since 2026-11-04:
`0x00211e88` draws them with `0x001f3588` -> `0x001f3f30`, the number as
text (`0x00207650`, FFX.exe `TOIntToMes(value, out, digits, 0)`: `digits`
characters, left-padded with the font's digit-wide blank) in the 16x12
font, not font M; the mod does the same (`BattlePatches.number`,
`research/ui/battle/` not updated; checked with `ui-battle-item.txt`). The
box's size matches the PS2's (x 0..133, y 339..401 in `compare-battle-kami`). The scroll bar is the PC's `DrawCrossMenuScrollParts` (grey track,
pink thumb) at the PS2's place, not the PS2's own bar. The cursor is the PS2
one since 2026-11-03 (below, "The PS2 cursor"). Icons are the HD `icon` art at a
12 x 12 PS2 size.

## The main menu (`ui.menus`: texture and code)

**The atlas problem and its answer.** The PS2 menus draw brush plates,
labels, the S.LV badge and the cursor from `meswin` cells that the HD atlas
reuses for other art (the feather sits on the party plate's cells, "gil" on
the Overdrive label's). FFX.exe still holds the PS2 layout table of
`meswin` (`0x00c5b460`, 85 entries) and it is byte for byte the PS2's
(SLPM_675.13 `0x00321328`; `battle` `0x00322190` = `0x00c5c2c8` and `icon`
`0x00323440` = `0x00c5d578` too; checked entry by entry), so the PS2 shapes
only need the PS2 pixels under their UVs. The converter writes the whole PS2
`meswin` (256x256, CLUT 0, PS2 size) as a texture of its own,
`menu*/d3d11/ps2win.dds.phyre`, one per menu folder (the text cells in the
folder's language where the disc has it, else the disc's own), made from the
HD `meswin` as template with the texture id `ps2win.dds`. The mod hooks
`TOGetShapTextureName` (`0x008ac8c0`): while a ported PS2 draw runs, sheet
`0x3e00` answers `.../menu_xx/D3D11/ps2win.dds.phyre` instead of `meswin`,
and the first such answer loads it (`PhyreScene_loadMenuTexture`
`0x006651d0`, asynchronous: until it is in, the menu manager skips those
draws). Every other draw keeps the HD `meswin`. Fahrenheit's file loader and
the ffx-rust harness both serve a path the archive does not have.

**What the PS2 drew** (SLPM_675.13, the main menu's slot record `0x00326d50`:
`+4` draw `0x0024a4a8`; the PC's is `0x00c5b2b4` -> `TkMenuDrawMain`):

- Background (`TkMenuDrawAll`, PS2 `0x00243f98`): one full-screen untextured
  quad (`0x00245a88`: GIF PRIM `0x14c`, c0 left, c1 right) from
  `col_menubg0` to `col_menubg1` (black in menu mode `0x0040xxxx`). Read in
  the game: `6b aa cd 80` (light steel blue) to `ba c3 87 80` (khaki). The
  PC passes the same two words to `TODrawMenuBGC2` (`0x008f5c60`), which
  keeps only their alpha over the HD `ffx_bg`, and the screens then draw
  `TODrawMenuBG` (`0x008f5c10`, `ffx_bg` again). The PS2 has no second
  backdrop.
- Party rows (`0x0024a4a8`), for each listed member whose slide
  (`g_TkMenuMainRowSlide`) is not 0, at x = `rect_nameplt.x` - cos(slide) *
  1024 / 0x3000, y = `rect_nameplt.y` + 44 i: for the front line (i <
  `TkMenuGetPlayerPartyMask`) a box `TODrawBoxXYWHC2(x + pos_noactive, w h
  of rect_noactive, col_noactive0/1)` (`TkMn2GetMainPos`; `dc e1 e0 80`,
  345 x 42 at (0, -4): the light band behind the stats); the plate: shape 4
  stretched over `rect_nameplt.w` - 40 in `col_nameplt0` -> `col_nameplt1`
  and shape 5 (the brushed end) in `col_nameplt1` (`0x001ff2b8`, the PC's
  `0x008f5e90`); shape 0x16 (a black line, `pos_nmblkline`,
  `rect_nmblkline.w`); the name (16x12 font, colour
  `MenuPlayer_DamageClass`); shapes 0x21/0x22 (HP, MP) and
  `"%5d/%5d"`/`"%4d/%4d"` in colour 0x25 (black digits, as the HD's); shape
  0x3d (S.LV label, the round black badge); the sphere level right-aligned
  in 12-pixel cells ending at `pos_slvshape + pos_spstr + 2`; the cursors
  (`g_TkMenuMainChoice`, the swap pair `0x0187155c`/`0x01871560`, the
  second blinking on `TkMenuGetTimer() / 2`).
- Footer: gil right-aligned at `pos_gilstr` + 2, the play time at
  `pos_timestr` (`TOMenuDrawTimer`), shapes 0xd5 (bar) and 0xd6 ("GIL") at
  `pos_gilshape` (+ `pos_gilshape2`, which the sequence data does not have:
  0, 0), the location plates 0xd2/0xd3 (`rect_placesubplt`, gold) and
  0xd8/0xd9 (`rect_placeplt`, black: `0x001ff1e0` = the PC's `0x008f5e20`,
  left piece fixed, right stretched), shape 0x27 ("Location") and the
  location name.
- Options (`0x0024b148`, the PC's `MenuMain_DrawCmdWindow` `0x008e20f0`;
  nothing while `TkMenuMainGetMenuActiveMask() & 0x80800`): every option
  but the cursor's, then the cursor's (`0x0024af70`): y = ((`rect_complt.y`
  + 32 i) slide + 3 (0x1000 - slide)) / 4096, alpha slide / 32; a black box
  at +4, +4; the box `col_complt0` -> `col_complt1` (purple, untextured,
  `rect_complt` (378, 44 + 32 i, 134 x 23)); the name in the 16x12 font at
  +15, +2, grey when disabled; the cursor there at slide 0x1000.
- Help bar (`0x00243888`; the PC's is `0x008a9350`, a main-menu window's
  `+0x10`, installed at `0x008a8e8e`): shapes 0x45/0x46 over
  `rect_helpsubplt` (gold stroke), 6/7 over `rect_helpplt` (black brush),
  shape 0x3e ("HELP") at `rect_helpsubplt` + (0x50, 0), the message
  (`g_TkMenuHelpMessage`) at `rect_helpplt + pos_helpstr`. FFX.exe's
  `yiWindowDrawBar` (`0x00907c10`) is the same draw for kind-10 windows; no
  window uses it in the main menu.

The layout words come from `TOSeqInitParam` (PS2 `0x00209bc8`, PC
`0x008ae6f0`; the same 92 reads in the same order). The PC stores them
mirrored: a point is (y, x) from its lower address, a rectangle (h, w, y, x);
the mapping used (PS2 -> PC) is in `mod/MainMenuPatches.cs`.

**The restoration** (`mod/MainMenuPatches.cs`, from `MenuPatches.menus`):
hooks on `TkMenuDrawMain`, `MenuMain_DrawCmdWindow` and the help draw
replace them with the PS2 draws above, made of FFX.exe's primitives (the
same calls the PS2 made where the PC kept them): `TODrawBoxXYWHC2`,
`TOMakePktBoxXYWHRGBA`, `0x008f5e90`/`0x008f5e20`, `TOMkpShapeXY`,
`TOMkpShapeXYWHC2`, `TOMkpShapePartIndexXYWHC2`, the 16x12 font
`TOMkpBaseMesFont` `0x00901cb0` (and `0x00901ce0` with alpha, the PS2's
`0x001f3810`/`0x001f3858`), and the PS2 fonts M and S for the numbers (HP/MP strings, sphere level, gil,
play time; below, "The PS2 menu fonts"). A hook on `TODrawMenuBGC2` draws the
PS2 gradient with the colours it is given, and `TODrawMenuBG` draws nothing
while `TkMenuDrawAll` runs (hooked to know). Menu logic, the option list
(PC-only entries included) and the input stay the PC's.

**Checked.** `tools/replays/ui-mainmenu.txt` through `tools/calllist` (the
converter's `ui` output as the mod): it loads `ps2win`, points the main
menu's slot draw, the option window's and the help window's `+0x10` at
call-list thunks holding the calls `MainMenuPatches` makes (the same code in
Rust: `tools/calllist/src/menu.rs`; the atlas swap is the thunk writing the
`ps2win` name into the `TOGetShapTextureName` table slot around the draws),
and patches `TODrawMenuBGC2` to jump to the gradient and `TODrawMenuBG` to
return (`@protect`: `VirtualProtect` through `GetProcAddress`). Screenshots
in `~/ffx-work/demaster/research/ui/mainmenu/`: `main-1-hd.png`,
`main-2-ps2.png`, `main-3-ps2-43.png` (after `render.aspect`'s 4:3 write),
`main-4-items.png` (Items opened: the HD list over the PS2 background; the
thunks there are still the static main-menu lists, which the hooks would not
draw), `ps2win-atlas-grid.png` (the PS2 atlas with a 16-texel grid). In the
Fahrenheit install (2026-11-03) the hooks installed (log `ui.menus = ps2:
PS2 message, battle command windows and main menu`) and the file loader
indexed the `ps2win` files (copied in by hand for that run); the menu was not
opened there (no input).

**The help bar's label.** `rect_helpsubplt` is (0, -6, 468): the gold
stroke (shapes 0x45/0x46, dy 2) runs from y -4 and shape 0x3e sits at
(0x50, -6). Shape 0x3e has 17 parts: 16 pieces of a black brush strip
(meswin (0, 104) 8 x 16, at dx -80 .. 40, the last 88 wide: x 0 .. 208,
y -6 .. 10) and the "HELP" text ((168, 240) 88 x 8 at -67, 7: x 13, y 1).
The PC's `TOAdpEasyShapeXYWHC2` (`0x008ed380`) skips every part of 0x3e
but the last (`param_2 != 0x3e || n - 1 <= k`), so on the PC the label
floats on the gradient over the gold stroke. The port draws all 17 parts
(`MainMenuPatches.help_label`, each part through
`TOMkpShapePartIndexXYWHC2` with w = h = 0): the label now sits on the
black strip that joins the help bar, as on the PS2. Its place is the
PS2's: the PS2 frame buffer is 512 x 416 (`0x001182e0`: `0x200` x `0x1a0`)
and the menu's vertices are `(x + 0x700, y + 0x730) << 4` (`0x001fbbb8`),
`0x730` = 2048 - 416 / 2, so menu y 0 is the frame's first line and the
PS2 showed nothing above it either. The strip's top 6 lines and the gold
stroke's top 4 are cut off on both.

**Uncertain.** Compared with the PS2 code and data, not with PS2
screenshots. The 16x12 names and options are the PC's port of the PS2 font
code. Every main-menu screen but the sphere grid is ported (below).

## The PS2 menu fonts M and S (`ui.menus`: texture and code)

**What the PS2 drew.** Besides the 16x12 font, the PS2 menus draw numbers
and short strings in two small monospace fonts from `xfont1208.fmt`
(= `subfont.fmt`, sheet `0x3d80`: 128 x 256, 4 bit, two 2-bit pages in
bits 0-1 and 2-3). Glyph `i = c - 0x20` is on page `i & 1`:

| Font | Draw (SLPM_675.13) | Cells | Grid | Advance |
|---|---|---|---|---|
| M | text `0x001f5ff8` -> glyph `0x001f6110`; `"%d"` right `0x001f5e88` / `0x001f5f58` | 10 x 12 from v 144 | `((i % 24) / 2 * 10, 144 + i / 24 * 12)` | 12 |
| S | text `0x001f6500` / `0x001f6560` -> `0x001f6630` -> glyph `0x001f66f0`; `"%d"` right `0x001f63c0` / `0x001f6408` | 8 x 10 from v 192 | `((i % 32) / 2 * 8, 192 + i / 32 * 10)` | 10 |

A right-aligned value ends its last cell at x + 2. Each glyph is two
sprites: the shadow at (x + 1, y + 1) with `menu.clp` CLUT 7 (CBP `0x2ebc`)
row `2 (colour >> 4) + page`, then the face at (x, y) with CLUT 6
(`0x2eb8`) row `2 (colour & 15) + page` (a PSMT8 `TEX0` loads the CLUT, a
PSMT4 one picks the row with CSA, as for the item icons). Faces 0..7:
white ramp, grey, yellow, red, light ramp, black, pink, cyan; shadows 0..7:
black, dark grey, light grey, green, blue, purple, violet, teal. The
colours used: HP/MP strings (`"%5d/%5d"`, `"%4d/%4d"`), gil, AP, the target
window's HP in font S `0x25` (black, light grey shadow); the sphere level
and the item counts / MP costs in font M `0` (white, black shadow) or `1`
(grey, unusable). The play time (`0x0024ad28`, t in seconds): `"%02d"`
minutes at x - 18, `":"` at x - 28 (colour `0x25`, 1 on odd seconds),
`"%3d"` hours (at most 999) at x - 58.

**Why the PC draws nothing with them.** The PC's copies
(`TOMkpValueFontMClutRight` `0x009042e0`, `0x00904310`) name the sheet
`menu/subfont/D3D11/{font,shadow}_0_{0,1}.dds.phyre`, which the archive
does not have; the PC's own menus use the HD font instead. Serving those
pages was tried (2026-11-03) and the functions still drew nothing.

**The restoration.** The converter renders both fonts in every colour into
`menu*/d3d11/ps2fnt.dds.phyre` (`uijob::PS2_FONT`, 1024 x 1024, from the
first disc source's `xfont1208.fmt` and `menu.clp`: no text, every folder):
tile `k` (256 x 256 at `(k % 4, k / 4)`) holds face colour `k` (0..7) or
shadow colour `k - 8`; in a tile glyph `i` of font M sits at (1, 1) in the
16 x 16 cell `(i % 16, i / 16)`, font S in `(i % 16, 6 + i / 16)`. The empty
border keeps the bilinear filter from pulling in the neighbouring glyphs
(with the glyphs packed as on the PS2 sheet, the colon of the time showed
lines from its neighbours). `mod/Ps2Menu.cs` (`font_m`, `font_s`,
`value_m`, `value_s`, `play_time`) draws each glyph as two meswin shape-0
quads with explicit UVs (`TOMkpShapeXYWHUVC2` `0x00903ee0`: ids below 199
take the sheet `0x3e00` and UVs 0..1 as given) while
`TOGetShapTextureName(0x3e00)` answers `ps2fnt`
(`MainMenuPatches.font_scoped`; the first answer loads the texture).

**Checked.** `tools/replays/ui-mainmenu.txt`, `ui-items.txt`,
`ui-abilities.txt` (calllist: `menu::font_m` ... make the same calls):
`~/ffx-work/demaster/research/ui/fonts/main-2-ps2.png` (HP/MP, sphere
level, gil, time, the HELP strip), `items-3-ps2-43.png` (counts, grey for
unusable items), `abil-3-ps2-43.png` (the plate, "11 AP").

## The 16x12 font's size (`ui.menus`: code)

Names, options, help lines, list entries and labels in the ported screens
(and the battle command names) are the 16x12 font, drawn with the PC's
`TOMkpBaseMesFont` (`0x00901cb0`, PS2 `0x001f3810`). Compared with the PS2
(`docs/compare.md`) they came out half as wide and about 0.7 as high, from
the right start point, at 16:9 and at 4:3 alike (help line "Ausrüstung
wechseln.": PS2 166 x 16; PC 84 x 11.6 PS2 units at 1280x720 and in the
4:3 view). The cause is the PC's size rules, not the 4:3 squeeze:

- The PS2 (`0x001f43c8`; glyphs `0x001f0ec8` face, `0x001f11e8` shadow)
  draws each glyph 1:1: a sprite of `w` x 18 at the pen (`w` the width
  table's, y offset 0), the shadow at (+2, +2), the pen moving `w` + 2.
- The PC (`TOAddPktBaseMesFontLClutTypeRGBA` `0x008e92a0` ->
  `TOMakePktFont1612Font` `0x008fcad0` / `...Shadow` `0x008fcca0`) scales
  the glyph by 0.78 (x1 = x + 0.78 `w` `[0x00c5d778]`, y from y + 0.78 to
  y + 0.78 (18 + 1) `[0x00c5d77c]` for the European languages), and
  `graphicFontDraw` (`0x00640660`) maps the start by 1920 / 512 but the
  extent by 1080 / 416 (square screen pixels): a glyph is 0.54 `w` PS2
  columns wide. The pen moves by `TOAddFontWidthSpace` (`0x008e8ed0`):
  round(`w` 0.78 x 1080 / 416) / 3.75 + the letter space 0.8. The shadow
  is 2 screen pixels off (`graphicFontDraw`'s third and fourth arguments)
  plus `0x018716e0` / `0x018716e4` (0).

**The fix** (`Ps2Menu.ps2_sized`, used by `Ps2Menu.font`, `font_a` and
`text_width`; every ported screen and `BattlePatches` draw through them):
around each call the mod sets the words those rules read, and restores
them after (the shape draws read `0x00c5d778` / `0x00c5d77c` too):

| Word | PC | During a PS2 text |
|---|---|---|
| `0x00c5d778` glyph x factor | 1.0 | 3.75 / 2.596 / 0.78 = 1.852: `w` PS2 columns |
| `0x00c5d77c` glyph y factor | 1.0 | (18 + 0.78 yoff) / (0.78 (18 + yoff)): 18 lines |
| `0x018716a8`, `0x018716ac` (width units) | 3.75, 2.596 | 1, 1 / 0.78: `TOGetFontWidthInPiexl` answers `w` |
| `0x018716b4`, `0x018716b8` (letter space, cached) | 0.8 | 2 |
| `0x018716e0`, `0x018716e4` (shadow) | 0 | 2 - 2 / 3.75, 2 - 2 / 2.596: (+2, +2) PS2 units |

`yoff` is the languages' y offset (1 for languages 1..8, -1 for 10, else
0, from `TOGetFFXLang`); the glyph's top stays 0.78 lines below the PS2's
(the integer y cannot take it back). `TkMn2StringWidth` (`0x008c2cb0`)
inside the same words gives the PS2 width (sum of `w` + 2), so everything
sized from the text follows: the Items sub-command bar (now 496 wide with
the PS2's spacing, the PS2's bar), the Config choices' centring, the
Overdrive title, Customize's cost, the battle command window's column.
Line feeds inside a string still advance the PC's 16 lines, not 20.
`tools/calllist` does the same with store records around each
`TOMkpBaseMesFont` call (`menu::expand_shapes`, `btl::ps2btl`) and
`Ctx::text_width`.

## The menu sequence scenes (`ui.menus`: code)

The PS2's sub-screens start menu sequence (1) scenes when they open and
stop them when they close (`TkMn2SeqStartScene` / `StopScene`, PS2
`0x00227488` / `0x00227498`, from the records' open / close functions):
the dark blue bands that slide in and stay (the main menu's vertical band
left of the options, the bands under the help bar or party plate, the
column between two lists, Config's right-edge band). Read in PCSX2 (PS2
table `0x0068f540`: 8 slots of 0x20, +0 state, +8 name):

| Screen | Scenes |
|---|---|
| Main menu | scene26 |
| Items (and key items) | scene0, scene9 |
| Abilities | scene0, scene10 |
| Overdrive | scene0, scene14 |
| Status | scene0, scene11 |
| Config | scene4, scene17 |
| Equip | scene0, scene11, scene20 |
| Customize | scene0, scene21; its ability list also scene23 |

The PC starts Equip's only (`0x008cf830`); its sequence data has the
others and draws them the PS2 way once started. **The restoration**:
each ported screen's draw asks for its scenes (`Ps2Menu.scenes`); a scene
that is not running (`g_TOSeqSlots` `0x01865f20`, state 1..3) starts
there (`TOSeqStartScene(1, name)` `0x008af0d0`), with its slide-in. Once a
frame, at the start of `TkMenuDrawAll`, `Ps2Menu.scenes_frame` stops
(`TOSeqStopScene` `0x008af4f0`) the scenes the mod started and no draw
asked for since: leaving a screen stops them as the PS2's close does.
Scenes the PC started itself are left alone. The calllist replays start
them with `@scenes` (and `@stopscenes`) after the injection and wait 40
frames for the slide.

## The menu layout data (`ui.menus`: data)

The layout words of `TOSeqInitParam` / `TkMn2SeqGet*` and the scenes come
from `menu_script.bin`. The PC reads `ffx_ps2/ffx/master/jppc/menu/menu_script.bin`
for every language: the Japanese International's file. The PAL game reads
its own disc's `jppc/menu/menu_script.bin` (18,768 bytes; the PC's is
18,736), whose words differ in places. Config's rows were the visible case:
`r_cnameplt_nonhdd` y 65 and `num_cnameplt_nonhdd` 47 (rows 42 apart) on
the PS2 (read in PCSX2 at `0x00693f78` / `0x00693fbc`), 55 and 45 (40
apart) on the PC; both read the `_nonhdd` names (PS2 `0x002d92d0` = u32
`0x005661ec` and the PC's `GetU32_023cbeb0` are 0). The Overdrive mode
name's place on Status and Equip (`pos_ltstr`) moved to the PS2's with it.
The `ui.menus` job writes the disc's file over the PC's path (when they
differ). It holds no text.

## The PS2 cursor (`ui.menus`: code)

The PS2 cursor (`TOMakePktShapeCursor`, SLPM_675.13 `0x001fe800`, kind 0)
is meswin shape 45: part 0, the white ball (80, 144) 40 x 40 at (-42, -11),
drawn in `dum75` (`0x025d16ac`, `73 55 0b 80`: the orange glow); part 1, the
silver hand (0, 168) 40 x 24 at (-34, -3), grey. Sprites from (u, v) to
(u + w - 1, v + h - 1). FFX.exe's `TOMakePktShapeCursor` (`0x008feb00`) draws
kind 0 as the HD bobbing hand from the battle atlas; kinds 4 and 5 are PS2
layout cells. The mod hooks it: kind 0 draws shape 45's two parts with
`TOMkpShapePartIndexXYWHC2` and the PS2 atlas, the sizes given so the UVs
stay one texel inside the cells (with 0 the hand picks up a dark line from
the cells below and right). Callers: `TODrawBtlWindow` (the battle cursors
from `TOBtlSetCurPos`), the save choice, PS2-era menu code. The ported main
menu draws the same cursor. HD screens that use their own cursor
(`TkMn2DrawCrossCursor`, item lists, status) keep the HD one.

Checked with `tools/replays/ui-battle-main.txt` and `ui-battle-item.txt`
(`@ps2btl ... ps2cursor N`: the PS2 cursor in place of the `TOBtlSetCurPos`
call, after the full-screen scissor `TODrawBtlWindow` sets before its
cursors): `~/ffx-work/demaster/research/ui/battle/btl-2-ps2.png`,
`item-2-ps2.png` (and `-3-ps2-43`).

## Item icons, battle words, labels (`ui.menus`: texture and code)

**Item icons** (`icon`, ids 0..44). The PS2's `TOAddPktMenuIconRGBA`
(SLPM_675.13 `0x001f9798`) draws a GIF sprite with two `TEX0`s: the first
(PSMT8, `CLD` 2) loads the 256-colour CLUT at block `0x2ec0 + 4 (id >> 4)`,
the second draws the PSMT4 `icon` (`0x3dc0`) with `CSA = id & 15`, TCC 1,
TFX modulate, CPSM PSMCT32. `menu.clp` sits at block `0x2ea0`, so the icon
CLUTs are `menu.clp` CLUTs 8, 9, 12, 13, 10, 11, 14, 15 for id >> 4 = 0..7, each
icon its own 16-colour row. The rest of the GS state comes from the menu
init packet `0x001efb10` (ALPHA `(Cs - Cd) As + Cd`, TEST alpha != 0, TEX1
bilinear, CLAMP); no menu code writes `TEXA`, and with a PSMCT32 CLUT
alpha comes from the CLUT entry anyway. In every icon row entries 2..15 are
opaque (0x80) and each icon's background index is one of them; index 0
(alpha 0) appears only in two 3-texel corner triangles, which cut the
background into a slanted tile. So **the PS2 icons sat on opaque, slanted,
coloured tiles**: the box found earlier is the PS2 look, not a missing
detail. The converter (`uijob::convert_icon`) writes each 15 x 18 cell in its
row's colours, scaled 4x bilinear within the cell, into every folder's HD
`icon` (no text). Ids 45..50 ("NEW", the pink marker, the "B" box) and the
button art below are shapes 600+ drawn with CLUT 13 row 2 and HD-pixel UVs;
they stay HD (the HD "NEW" is per language).

**Battle words** (`TOMakePktBtlIconA`, kinds 0..3). The PS2
(`TOBtlDrawDamageValue2` `0x0021f4b0` -> `0x001f30f0` -> `0x001f9b50`) draws
them from `icon` 1:1 with CLUT 13 row `13 - kind`, RGB 0x80 and the fade
alpha; rectangles from its table `0x00570000` (PS2 texels): OVERKILL (0,
110)-(164, 127), IMMUNE (164, 96)-(231, 118), MISS (55, 96)-(152, 110), MP
(0, 96)-(45, 106) (German: VERNICHTEND, IMMUN, DANEBEN). The PC's
`TOMakePktBtlIconA` (`0x008fbcc0`) draws its HD rectangles at about half the
PS2 width (OVERKILL 310 x 38 in 1920 units against the PS2's 164 x 17 PS2
pixels). The converter puts the words at the PS2 places (4x) of every
folder's `icon`: the disc's in its languages (`jppc` -> `menu_de`, `uspc`
-> `menu_us` and `menu`), the HD words resampled there in the others; the mod
(`BattleSeqPatches.btl_icon`) draws them from there at the PS2 size,
`TOMkpShapeXYWHUVC2(600, x, y, w, h, 4 u0, 4 v0, 4 u1, 4 v1, c, c)`. The
English game reads `icon` from `menu_us` (`TOGetShapTextureName(0x3dc0)` in
the harness), and the `menu` folder gets the English source too.

**`strtex`** (27 labels: "Ekstase-Typ", "Erhaltene AP", ...; 3 columns x 9
rows of 170 x 14). On the PS2 it is loaded only in battle (`0x0020ca40`, at
`0x3f00`) and drawn by `0x00201768` / `0x002018e4` with CLUT 13 row 7 (grey
with an alpha ramp). The PC's `TOMkpShape2ECCCell_008f8d50` takes the same
cells over `TOGetImageWH(0x2ecc)` = 512 x 128. The converter writes the PS2
sheet (4x, bilinear per cell) into the disc's language folders; it matches
the HD layout cell for cell (`research/ui/items/strtex-preview-de.png`).
Not seen in the game (no replay shows a `strtex` label).

**Checked.** Previews `research/ui/items/icon-preview-us.png`,
`icon-preview-de.png` (HD left, ours right). In the game: the item list of
`tools/replays/ui-items.txt` (icons on their tiles) and the four words drawn
at fixed places by `ui-battle-overdrive.txt` (`research/ui/battle/od-2-ps2.png`:
OVERKILL in the orange ellipse, IMMUNE, MISS, MP, the PS2 English art). The
real damage display was not reached (the words' anchor, x and y as the PC
passes them, is assumed to be the PS2's top left).

## The Overdrive tab (`ui.menus`: texture and code)

**What draws it.** Not the window code: the battle sequence. Both
platforms' `TODrawBtlWindow` (PS2 `0x00214ae4`, PC `0x0089b360`) run
`TOSeqDraw(2, 0)` before the window callbacks and `TOSeqDraw(2, 1)` after.
The PS2's `TOOpenBtlWinMain` (`0x002108a0`) starts, by
`MsCheckLeftWindow` (`0x001dc368`: 0 nothing, 1 a trigger command, 2 the
gauge full, 3 both), `bnormal`/`bnormale`/`bnormall`/`bnormalt` and their
`...lay` scenes; the first opening goes through `TOOpenBtlLimitBreak`
(`0x00212f38`: `bevent`, `blimit`, `btwo`). `bnormalllay` (in
`battle_script.bin`, identical on PC and PS2) slides battle shape 413 from
(70, 318) to (46, 318) over 4 frames, then blinks its alpha 0x80 <-> 0x32
(8 + 8 frames); `blimitlay` flies it in first. Shape 413 is battle layout 13
(FFX.exe `0x00c5c2c8` = PS2 `0x00322190`): (0, 112) 160 x 16 at (-32, 0), the
"EKSTASE!!"/"OVERDRIVE" brush plate of the PS2 `battle` atlas (414 the
trigger-command plate, 421 both). The PC still starts `bnormalllay` /
`blimitlay` (not the `e`/`t` ones, nor any layer-0 scene) and still runs it,
but its `TOMakePktSeqPrimLayout_008fe0b0` draws 413 only above y 300 (the
Overdrive mini-games' cursor) and skips 411/415/507; it drew its own HD tab
inside `TODrawWaitBtlWinMain` instead, which the PS2 window port drops. The
HD `battle` cell of layout 13 holds only italic text, no plate.

**The restoration.** The converter writes the whole PS2 `battle` (CLUT 1,
PS2 size) as `menu*/d3d11/ps2btl` (as `ps2win`). `mod/BattleSeqPatches.cs`
hooks `TOSeqDraw` (`0x008ae2f0`) to know when scene 2 runs and
`TOMakePktSeqPrimLayout_008fe0b0`: shapes 413, 414, 421 below y 300 are drawn
as the PS2's drawer (`0x001fd268`) does, each part of the layout at (x + dx sx
/ 100, y + dy sy / 100), w sx / 100 x h sy / 100, in the primitive's colour
(`+4`; x, y, sx, sy at `+0xc`..`+0x12`, shape `+0x14`), with
`TOMkpShapePartIndexXYWHC2` and sheet `0x3f00` answered by `ps2btl`
(`MainMenuPatches.battle_scoped`). The slide and the blink are the
sequence's own.

**Checked.** `tools/replays/ui-battle-overdrive.txt` (ffx-rust's
`battle-overdrive.txt` battle, Tidus's gauge full): `research/ui/battle/`
`od-1-hd.png` (the HD tab), `od-2-ps2.png`, `od-3-ps2-43.png` (the PS2
window, cursor and the hook's call: the PS2 "OVERDRIVE" plate at the
scene's resting place, (14, 318) 160 x 16, above the box at y 339). The
harness can't run the hook, so the shot shows the plate at rest, not the
slide and blink. Not done: the trigger-command tabs (the PC starts no
`bnormalelay`/`bnormaltlay`, so 414/421 never come), the layer-0 scenes
(the orange glow under the box, `bnormall` track 11: a 100 x 62 rect at (0,
339)), the footer pieces the PC skips (411, 415, 417).

## The battle sequence scenes (`ui.menus`: code)

The PS2 opens every main command window with two battle sequence scenes
(PS2 `TOOpenBtlWinMain` `0x002108a0`, by `MsCheckLeftWindow` 0..3): a
layer-0 one drawn before the windows (`bnormal`, `bnormale`, `bnormall`,
`bnormalt`: lines at y 329..340, a purple bar (0, 330) 137 x 9, row bands
418 at y 341, 361, 381, the orange glow (0, 339) 100 x 62 from
`ff6400` / 0x64 to transparent) and its layer-1 twin after them
(`...lay`: a black line along y 407, the ring 417 at (26, 407), the
BEFEHL brush 415 at (-2, 392), the ">>>" plate 411 at (90, 402) pulsing
blue, the Overdrive tab 413). `TOOpenBtlLimitBreak` (`0x00212f38`) starts
`bevent` / `blimit` / `btwo` and their lay scenes. Scene data:
`battle_script.bin` (the PC's copy holds the same scenes). FFX.exe starts
only `bnormallay`, `bnormalllay` and `blimitlay`; its
`TOMakePktSeqPrimLayout_008fe0b0` skips 411, 412, 415, 419 and 507, draws
417 only at y <= 330 (an HD gauge) and 413 only at y <= 300; and its line
primitive (`TOMakePktSeqPrimLine` `0x008fe070`) is a bare `ret`, so no
scene line is drawn anywhere on the PC.

**The restoration** (`mod/BattleSeqPatches.cs`): hooks on
`TOOpenBtlWinMain` and `TOOpenBtlLimitBreak` start the PS2's other scenes
after the PC's call; the primitive hook draws 411, 415, 416, 418, 419,
417 below y 330 and 413/414/421 below y 300 the PS2 way (each part at
`x + dx sx / 100`, `w sx / 100`, the primitive's colour, texel-centre UVs
with the parts' flips, PS2 battle atlas); and the scene line's call
(`0x00a7c20d`) goes to a drawer of one-pixel gradient boxes while the
battle sequence draws (the menus' scene lines stay off: not compared).
Found by decoding `battle_script.bin` (scratch `hud/agent2/scenes.py`).
The PC's track waits compare only the first track (`TrackStep` op 2), so
the tab's blink and the plate's pulse start with the slides; not changed.

Checked: `compare-battle-kami` draws the scenes' resting frame as fixed
calls (`hud::footer`; the harness can't run hooks): BEFEHL, ring, plate,
lines, glow and bands match the PS2 shot within a pixel or two
(`docs/compare.md`). The slide-ins, the pulse and the limit-break scenes
were not seen in play.

## The battle HUD (`ui.menus`: code)

`TODrawBtlWindow` (PS2 `0x00214ae4`, PC `0x0089b360`) calls the same
pieces on both: the party list (PS2 `0x0021c498` = `TOBtlDrawStatusWin`),
the CTB list (`0x002138c8(x, y)` = `TOBtlDrawCtbWindow`, at dum19), the
window callbacks, the help bar (`0x00218338` = `TOBtlDrawHelpWin`), the
target info window (`0x00218a80(chr, 0x14, 0x46)` = `TOBtlDrawInfoWin`;
the PC passes (0xb3, 0xf0)) and its text. The PC rewrote the draws with HD
sheets and HD coordinates. `mod/BattleHudPatches.cs` replaces three of
them with the PS2 draws (PS2 units, the PS2 meswin `ps2win` and battle
atlas `ps2btl`):

- **Help bar** (`0x00218338`): while the help setting is on (MsGetSaveMenu
  bit 0x400) the scene `bhline` runs (the gold stroke along the top; the
  PC never starts it); with the help window open, the menu help bar's
  pieces: plates 0x45/0x46 at `rect_helpsubplt` in `col_helpsubplt`,
  plates 6/7 at `rect_helpplt` in black at alpha 0x40 (not `col_helpplt`
  as in the menus), the HELP label (meswin 0x3e), the text at (40, 12) in
  the 16x12 font. The PC's band and its scrolling of long texts are gone
  (the PS2 doesn't scroll).
- **CTB list** (`0x002138c8`): the battle window frame (shapes 400..408,
  `0x001fa7d8`) 58 x 174 at dum19 (438, 43), the arrows 438/439 right of
  it, a scroll bar (`0x001fbfc0`: grey box, black inside, light thumb, 8
  of 16 rows), then each row 22 apart (the PC's row state, the same
  numbers): the wait bar 40 x 20 in three purples and the face at x +
  min(wait, 20) (party 440 + chr, Rikku's other face before scenario
  0x434, aeons 448, monsters by icon type), the first row's orange
  marker 472; in target selection the targeted rows' markers 429/430 in
  dum74/dum73. `TOMakePktBoxXYWHRGBA` ignores the PC's scissor, so the
  bars are cut to the window by hand.
- **Party list** (`0x0021c498`): three rows from dum45 (206, 346), dum13
  (22) apart; the acting member's highlight in the Overdrive view
  (battle shapes 409/410 dum52 wide, dum2 -> dum1, a light line on top
  and a dark one under it), the targeted members' in the other views
  (dum40 -> dum41); the name right-aligned at x 321 in the 16x12 font in
  its HP colour; HP and MP labels (meswin 0xe9/0xea) and the shown values
  in font M; the Overdrive gauge (`0x0021d1b8`: a black box, solid orange
  when full, else a light-to-orange gradient over the full width cut at
  the value). The HP / MP views draw "value/max" (the max in colour 0x80,
  which the PS2 leaves in the register: white face, black shadow); the
  status view cycles the statuses every 21 frames, as the PC's draw did.

The target info window and the damage digits: the digits are the PS2's
(`docs/font.md`, `ui.font`); the info window (`TOBtlDrawInfoWin`) stays HD
(open).

Checked: `tools/replays/compare-battle-kami.txt` (`@ps2hud`: a jump at
each PC draw's entry to a call list with the mod's calls, on the game's
own code). Help bar and party list match the PS2 frame to the pixel but
for glyph edges; the CTB frame, bars, faces and markers too, but the PC's
turn order differs (Wakka third against the PS2's seventh: the battle's
own state from the same save, not the draw). Not seen in play.

## The Overdrive mini-game windows (`ui.menus`: code)

**What the PS2 drew.** Both platforms' `TODrawBtlWindow` (PS2 `0x00214ae4`, PC `0x0089b360`) call
the same draws between the party list and the CTB list: the enemy gauges, Libra, then Auron's,
Tidus's and Lulu's windows and the limit timer (PS2 `0x0021ad88`, `0x0021b360`, `0x0021b9d0`,
`0x0021a8f8`; PC `TOBtlDrawAuronLimitWindow` `0x00892740`, `TOBtlDrawTidusLimitWindow`
`0x00897240`, `TOBtlDrawLuluLimitWindow` `0x00895660`, `BtlDrawRatio_008955e0`). The windows'
records keep the PS2 offsets (PS2 `0x00692678` / `0x00692690` / `0x006926a8` / `0x00692658` = PC
`0x0133d6f4` / `0x0133d734` / `0x0133d708` / `0x0133f77c`) and are in PS2 units on both (the PC's
inits remap their HD constants with `graphicUiRemapX2/Y2`). A window draws in its states 3 and 4
(the input phase); the prompt line is the 16x12 font centred at y `dum225` (`0x0133f778`):

| Window | PS2 draw |
|---|---|
| Tidus (0x3010) | the bar `0x00201b68(x, y, w)` (battle shapes 500 at (x - 7, y - 4), 502 at (x + w - 8, y - 4), 501 stretched over w); the hit zone as one box (`+8`, y, `+0xc` x 9) in `dum203`; the marker, battle shape 477, at (`+0xe`, y) |
| Auron (0x3011) | each button still to press (list `+0x10`, from `+8` to the count `+0xc`) by `0x001fc778` at (`+2` + 30 i, `+4`): icon shapes 600..617 (the d-pad cross from four pieces, lit or not; triangle 608, circle / cross 609 / 610 swapped for the Japanese layout, square 611, L1/R1/L2/R2 612..615, 616, 617) |
| Lulu (0x3012) | the count (`+0x17`) "%02d" in battle shapes 490.. 16 apart at `dum212`; the fill `TODrawBoxXYWHC2FW(+2, +4, +0x14, 9, dum210, dum211, +6)`; the bar `0x00201b68` |
| Timer | the seconds left (`+0x18`) "%1.2f" at `dum201`, each character a 20-wide cell: digits battle shapes 480.., the point 479; yellow (`0x80008080`) under one second |
| Wakka (`0x0021bcc8`) | battle menu texture 0x13 put at sheet 0x3f00, the scene layer 4 (`bwakk`: the reels), text 0x3013 once `bwakk` is in state 3 |

The PS2 draws no backdrop under these windows (the PC's `TODrawLimitBG` `0x008a1ce0`, three HD
plates, has no twin) and no frame round Wakka's reels (`TODrawWakkaLimitFrame_008a1e50`). The PC
draws Tidus's hit zone as two gradient halves, an HD marker, an HD bar; Auron's buttons as HD pad
glyphs 1.5 times as big, 46 further left and lower; Lulu's counter 10 right and 8 up with its own
fill; the timer 15 right and 20 up with 16-pixel digits (and moved up during the reels); the
prompts in the battle typeface at y 71.

**The restoration** (`mod/BattleLimitPatches.cs`): hooks on the four PC draws make the PS2 calls
with FFX.exe's primitives (battle shapes from the PS2 atlas `ps2btl` through `BattleHudPatches.bshape`,
icon shapes 600.. from the `icon` sheet with half-texel-inset UVs, `TODrawBoxXYWHC2`,
`TODrawBoxXYWHC2FW`, the 16x12 font at the PS2 size); `TODrawLimitBG` and Wakka's frame draw
nothing. While Auron's, Tidus's or Lulu's window runs (not during the reels, whose scene draws from
texture 0x13 at 0x3f00), the battle sequence's primitives are all drawn from the PS2 atlas
(`BattleSeqPatches.ps2_shape`), so the limit scenes (`bauro`, `btidu`, `blulu`: the cursor 413 above
y 300 ...) are PS2 art too. The bar's geometry stays the PC's: the PS2 reads Tidus's bar from
`dum202` / `dum2021` (the PC from constants: x 118, y 196, w 275), and its width sets the marker's
travel, so changing it would change the timing (logic stays the PC's). Kimahri (Ronso Rage),
Rikku (Mix), Yuna and the aeons have no window of their own: their Overdrives are command lists
(BattlePatches' sub windows: kinds 1-4, 6, 0xe, 0x11; Mix's two item picks
`TODrawWaitBtlWinPrepare`), already drawn the PS2 way. Wakka's reels (texture 0x13) stay as the PC draws them.

**Checked.** Builds only (mod, `tools/calllist`). Replays for the harness: `tools/replays/
ui-battle-limit-tidus.txt` (fx-overdrive's battle, Spiral Cut, shot in the bar's state 4) and the
untested drafts `ui-battle-limit-auron.txt`, `-lulu.txt`, `-wakka.txt` (the character alone in
front, gauge full; whether the game takes empty party slots is a guess), through `@ps2limit`
(jmps at the PC draws, `FFX_HOOK_DISABLE=replacements`). Not compared with the PS2: no save state
reaches an Overdrive (PS2 gauge writes would be needed). Uncertain: the icon sheet's pad glyph
cells are HD art (✕ and ○ were reported swapped in the HD `icon`, "Button glyphs"); whether the
PC's `icon` sheet has art at cells 600..617 at all; the timer's point and digit cells (a 20-wide
cell per character, as the PS2 code steps).

## The Items and Abilities screens (`ui.menus`: code)

The PC kept the PS2 menu logic and window records: every field the PS2
draws read has the PS2's offset in the PC's 0x98-byte window
(`+0x2c`..`+0x94`). `mod/ItemMenuPatches.cs` hooks the PC's draws and makes
the PS2's calls (SLPM_675.13 -> FFX.exe):

| PC draw | PS2 draw | What |
|---|---|---|
| `MenuItem_ListWindowDraw_008c7750` | `0x0024f0e8` (rows `0x0024ecf0`, cursor `0x0024efd0`, bar `0x00200a60`) | the item (`+0x43` 0), key-item (1) and ability (2) lists |
| `MenuItem_TargetWindowDraw_008c80b0` | `0x0024dcd8` | the party / aeon target window |
| `MenuSubCmd_DrawRow_008e3cf0`, `MenuSubCmd_DrawItems_008e4140` | `0x00236e68`, `0x00237188` | the sub-command bars (every screen's) |
| `0x008c64a0` (the ability record's draw) | `0x00250978` (plate `0x002014e8`) | the current member's party plate |

- **List**: two columns at x 30 and 279, rows 32 apart from y 98 (abilities
  119), scissor (0, 93)-(511, 415) or (0, 114)-(511, 402); rows `-1 ..
  +0x3a` at rest, two passes in `TkMn2DrawMenuWindowScissor` while scrolling.
  A row's plate by kind (PS2 `TOAdpShapeByKind` `0x001f4c60`): meswin
  shape 0xda at x, 0xdb from x + 16 to x + 209 (`0x001f4820`: 0xd1 - w(0xda) wide; drawn 209 wide from x + 16 before 2026-11-03, 16 pixels too long), `c0 -> c1`, plus 0xd7 / 0x10b
  (a line and the black count disc) / 0x10a at x + 97 in black alpha 0x80,
  the pair from `col_bsoplt`, `col_bitemplt` (items: kind 0xa, gold),
  `col_babplt` (commands: 0x12, purple), `col_lmtplt`. (The PC's copy of the
  kind-1 builder, `0x008f1a90`, lost its 0xd7 line; the port draws it.)
  Icon at (+12, +1) 15 x 18, name at (+29, +1) in the 16x12 font (grey when
  the entry's usable byte is 0), the count (items) or MP (commands) right-
  aligned ending at x + 213, y + 6. Scroll bar: `rect_iscroll` /
  `rect_abscroll` (`0x0186a410` / `0x0186a418`), a dark track `11 11 11 80`,
  a light thumb `f0 f0 f0 80`. Cursor: the PS2 cursor at (x + 12, y + 1) of
  row `+0x48 - +0x32`, the right column when the word at `+0x58 + 4 p` is
  set.
- **Sub-command bar**: the PS2 geometry recomputed from the texts (`Σw` the
  widths, `A = Σw + 32 (n - 1) + 96`, width `max(381, min(A, 0x1f0))`,
  spacing `clamp((width - Σw - 96) / (n - 1), 32, 128)`, x centred, y 37),
  the black brush plate (meswin 0xd8/0xd9 in `0x80101010`), the texts in
  the 16x12 font, the cursor while `+0x45 <= 0`, the orange brush (meswin
  0xe2, 0xe3 stretched, 0xe4) under the picked entry. Widths come from
  `TkMn2StringWidth` (`0x008c2cb0`, `TOGetMessageWH`'s width): the PS2's own
  measure, `TkMenuDraw1612Width`, adds f32 bit patterns as integers on the
  PC (`TOGetTextWidth_008b8040`) and returns garbage. The minimum width 381
  is the item and ability opener's (`0x00237600`); other screens' openers
  pass their own (not resolved), so their bars may be narrower than the PS2's.
- **Target window**: rows 31 apart from y 135 (raised past 9 members),
  shadow box `col_isnmplt`, box `col_ibnmplt0/1`, meswin 0x52, the name plate
  0xe5/0xe6 in `col_inmplt0/1`, HP (or MP) label and values, the name in the
  member's damage colour (grey when not a target), the cursor on the picked
  row (or blinking on all for a whole-party item); places from the menu
  sequence (`pos_linm`/`pos_rinm`, `pos_lihptex`, `pos_lihp1/2`), sliding in
  by `MenuMn2_Ease`.
- **Ability plate**: the main menu's party row without the slide (plate
  4/5, name, HP/MP), a black line, meswin 0x107 (the "Next ... AP" piece),
  the sphere level and `MsGetNextAP - MsGetSaveNextAP`.

Numbers are in the PS2 fonts M and S (above); icons are the PS2 cells
drawn 15 x 18.

**Checked.** `tools/calllist` with `@ps2items` (the same calls in Rust,
`tools/calllist/src/items.rs`) and the main menu's help bar:
`tools/replays/ui-items.txt` and `ui-abilities.txt`. Screenshots in
`~/ffx-work/demaster/research/ui/items/`: `items-1-hd.png`,
`items-2-ps2.png` (gold item plates with black count discs, PS2 icons, greyed
unusable items, the scroll bar between the columns, the black bar with
Use / Sort / Equipment / Key Items and the orange brush under Use, the PS2
cursor), `items-3-ps2-43.png`, `items-4-key-ps2.png` (the cursor moved to the
right column), `abil-1-hd.png`, `abil-2-ps2.png`, `abil-3-ps2-43.png`
(Tidus's plate, Cheer on a purple plate). In the Fahrenheit install
(2026-11-03, DLL swap) the hooks installed (log `ui.menus = ps2: PS2
message, battle command windows, main menu, items and abilities`); the menus
were not opened there (no input).

**Not checked / uncertain.** Not compared with PS2 screenshots. The target
window, the scrolling passes and the second sub-command bar (Sort's choices)
were not driven in the harness (the C# draws them; the Rust copy covers the
list at rest, the first bar and the ability plate). The PS2 also starts menu
sequence scenes on these screens ("scene0", "scene9", "scene31" for items,
"scene10" for abilities, in `menu_script.bin`) that the PC never starts;
the mod starts them now (below, "The menu sequence scenes": the dark blue
band under the bar and the column between the lists).

## The Status screen (`ui.menus`: code)

`mod/StatusMenuPatches.cs` hooks two PC draws and makes the PS2's calls (SLPM_675.13 -> FFX.exe):

| PC draw | PS2 draw | What |
|---|---|---|
| `MenuStatus_DrawMainWindow` `0x008d2710` (main window `+0x10`, pointer `0x0186a838`) | `0x0022ad00` (record `0x00325850`, window from `0x0022adb8`) | page 0 (`0x0022a770`), stats (`0x0022a9a8`), status background (`0x002297a8`) |
| `MenuStatus_WindowDraw_008d2790` (windows `0x0186a820[0..5]`) | `0x0022b5a0` | an ability page; the aeon status pages use the same draw (`+0x64` set) |

The PC's `TkMn2GetStatusPos` (`0x008d4920`) reads the same names as the PS2's `0x00228c48`. PS2 `0x006938xx`/`0x003258xx` map to PC `0x0186a83c..0x0186a9f8` / `0x023cc3e0..0x023cc420`. `TkMn2SeqGetPos`/`Rect` store x, y (w, h) in order, not mirrored. The common plates (`rect_lgplt` .. `rect_wpnbline` and their second rows) are 24-byte records `{x, y, w, h, c0 +8, c1 +0xc, A +0x10, B +0x14}`, 0x18 apart, the same order on both platforms.

- **Page 0**, slid by s = ease(t) 0x155 / 4096:
  - Face (`0x002299b0`): scissor (`pos_face`, `num_face1` x `num_face2`), the 91 x 98 face sprite (`0x00200048`), the next face at x + num_face1 t / 4096 under the font shade. Drawn with `MenuPlayer_DrawFaceSlide` at the PS2 place and size.
  - Gear (`0x00229fa0`): common window (shapes 0xc8..0xcb) `rect_wpnbplt`, plate L `rect_wpnlplt` (0xda/0xdb), box `rect_wpnrplt`, the same for the second row, plate R `rect_wpnbline` x2, shape 0xd7 at rplt.x + 0x58; "Weapon"/"Armor" (status text 5/6) at `pos_wpnmoji`; gear icon and name (`0x001f5298`: icon owner 2 + 1 + armour, the 0x31 mark when +4 == +6) at `pos_wpnicon`, or status text 0x17 at `pos_wpnnm` without armour.
  - Overdrive (`0x00229d60`): boxes `rect_ltbplt`, `rect_ltplt`, plate L `rect_lgplt`, shape 0x109 ("Mode") at `pos_typshape`, shape 0x108 (brush and "OVERDRIVE") at `pos_lgshape` + 16, the mode name (`TkMenuMenu1Mes(TkMn2GetOverDriveId + 0x1033)`) at `pos_ltstr`.
  - Gauge (`0x00229828`): box at `rect_lg`, width w r / 4096, `col_lmtg0` -> the blend of `col_lmtg0`/`col_lmtg1` at r, full in `col_lmtgf0`.
  - Party plate (`0x00225f38` -> `0x00200c60`) at `rect_3nameplt`: as the Abilities plate, but the line under it is shape 0xe7 across 512 (grey).
- **Stats** (`0x0022a9a8`): table `0x00325808` = PC `0x00c5a790` ((status text, `MsGetSavePlayerPtr` byte), 7/0x2f .. 0xe/0x36). Left column from `pos_lpmplt`, right from `pos_rpmplt` (the fifth on), `num_pmplt` apart, slid by `MenuHelp_Ratio(ease 0xd0 / 4096, i % 4)`. Each: plate kind 0x10 (purple), shape 0x10a (black line and disc) at `pos_pmshape` + 0x58, name at +29, +1, the value in font M (`0x00226c80`) ending at +0xd3 + 2, y + 6. Then for party members: common window `rect_stsmw`, shape 0xf3 at `pos_extex`, status text 0x1a at `pos_exmssg`.
- **Status background** (`0x002297a8(bg, 0)`): the box `col_menubg0` -> `col_menubg1` with alpha bg >> 5, from `pos_face.y` to the bottom. It covers the page while an ability page is open.
- **Ability page** (`0x0022b5a0`):
  - List: `TkMenuGetDispCommand(+0x60, +0x58, +0x5c, chr)`.
  - Slide: ease of (+0x52, +0x54); down by e 0xd0 / 4096, or apart by e / 16 row / 32.
  - Title (`0x00226298` -> `0x00201ae8`): plate L 0xda/0xdb 213 wide in black at alpha +0x52 >> 5, the strtex cell +0x68 at +21, +5 (`0x00201768` = `TOMkpShape2ECCCell_008f8d50`) at `rect_smojiplt`.
  - Rows: `num_staplt` apart from `pos_rstaplt.y`, left `pos_lstaplt.x`, right `pos_rstaplt.x`; aeons +11 and one pixel less. Each: plate kind 0x10, shape 0xd7 at `pos_pmshape` + 0x58, then icon and name (+12/+29, +1) from `MsGetRomAbility` (icon +0x68) for kind 0, `MsGetComData` (icon +0x14) otherwise, a member name for sub 1.

**The restoration.** Every call above is drawn with FFX.exe primitives inside one packet, with `ps2win` for meswin. The common plates and windows are rebuilt from shapes with the widths from the meswin layout table (`0x00c5b460`). Logic, input and the window records stay the PC's. The PC's dividers (`0x008e71d0(4)`) and its own background are dropped. Equip's top half is the same page (`MenuStatus_DrawPage`).

**Checked.** `tools/replays/ui-status-ps2.txt` through `tools/calllist` (`@ps2screen status`, `tools/calllist/src/status.rs`): Tidus's page, 4:3, the Auto-Abilities page over the status background, Rikku's page (full gauge), her Special page (Steal, Use). Layout values read in the game: face (20, 96) 91 x 98 scissor, stats from (25, 240) step 32, `rect_stsmw` (131, 379, 381, 32), ability rows from y 119 step 27, title at (150, 94).

**Uncertain.** Not compared with PS2 screenshots. The face slide, the page fades, the aeon pages and the `sub == 1` list were not driven. The equipped mark 0x31 uses the HD icon cell.

## The Equip screen (`ui.menus`: code)

**What the PS2 drew** (SLPM_675.13, Equip record `0x003258d8`: control `0x0022c438`, `+4` draw `0x0022cc28`; the PC's record is `0x00c5a744`, `+4` `MenuEquip_Draw_008cf670`):

- The status page `0x0022a770(0, 0, chr, 0, &0x003258c8.., fade)`. Status `0x0022ad00` draws the same page with the same arguments; on the PC both call `MenuStatus_DrawPage` (`0x008d4ee0`).
- Two label plates `0x00226218(x, y, k)` -> `0x00201a70`: meswin 0xda/0xdb over 213 in `0x80000000` (`0x001ff028`, the `0x001ff1e0` shape), then strtex label k at (+21, +5), 170 x 14 (`0x00201768`). k 6 ("Equipment") at `r_somojiplt`, k 7 ("Abilities") at `r_somojiplt2`. The PC draws the same cells with `TODrawSheetCell_008f8bb0`.
- The cursor (`0x0022bbd0`, once the fade reaches 0x1000): at `0x00228b98` (= `MenuStatus_GetPos` `0x008d4e30`), which is `rect_wpnlplt`'s plate record (the armour row's at +0x48) + `pos_wpnmoji`. It blinks (`timer & 2`) while the list is open.
- Abilities (`0x0022baa8`, while `g_TkMenuEquipItem` != 0xff): `MsGetSaveWeapon(item)`. For each slot (`+0xb`) from `pos_soabplt`, 32 apart: a kind-0x10 plate (`TOAdpShapeByKind`, `col_babplt`, purple) and shape 0xd7 at + `pos_pmshape` + (0x58, 0). For each of the 4 ability words at `+0xe` (not 0 or 0xff), `0x001f5388`: the `MsGetRomAbility` record's icon (`+0x68`) at (+12, +1) and name at (+29, +1).
- The weapon list window (`+0x10` = `0x0022bdc0`; the PC's `MenuWeapon_DrawList_008cf9f0`, window at `0x0186a61c`):
  - `TkMn2DrawMenuWindowScissor(pos_sobiplt.y, 160, part, +0x46)`, with parts 1/2 while `+0x32 != +0x34`.
  - Rows first - 1 .. first + 5 from `pos_sobiplt` - (0, 32) (`0x0022bd20`). The second pass is shifted by -160 * `+0x46` / 4096.
  - Each row (`0x0022bc20`; at rest y -= `+0x46` / 128): a kind-0 plate (`col_bsoplt`, grey), shape 0xd7, and for an id in `g_TkMenuItemList`, `0x001f5298`: icon `rec[4] * 2 + rec[5] + 1`, the name `MsGetSaveWeaponName`, and icon 0x31 when `rec[4] == rec[6]`.
  - The scissor reset `(0, 0, 511, 415)`.
  - The cursor at `pos_sobiplt` + (12, 1 + 32 (`+0x48` - `+0x32`)).
  - The scroll bar `0x00225e90` -> `0x00200878` (the item list's draw) over `r_soscrol`.

**The restoration** (`mod/EquipMenuPatches.cs`): hooks on `MenuEquip_Draw_008cf670` and `MenuWeapon_DrawList_008cf9f0` make the calls above with FFX.exe's primitives.
- `TODrawMenuBG` and `MenuDrawDividerShapes(4)` are dropped.
- The page goes through `MenuStatus_DrawPage`, as the PC calls it (the Status port hooks it).
- Logic and input stay the PC's.

**Checked.** `tools/replays/ui-equip.txt` (save 5, Equip, Tidus; then the weapon list) through `tools/calllist` (`@ps2screen equip`, the same calls in `tools/calllist/src/equip.rs`). Values read in the game: `r_somojiplt` (28, 212), `r_somojiplt2` (277, 212), `pos_sobiplt` (30, 237), `pos_soabplt` (279, 237), `pos_pmshape` (9, 0), `rect_wpnlplt` record (183, 131), `pos_wpnmoji` (11, 1), `r_soscrol` (256, 221, 10, 191). Shots: `equip-1-hd`, `-2-ps2`, `-3-ps2-43`, `-4-list-hd`, `-5-list-ps2`, `-6-list-ps2-43`.

**Uncertain.** Not compared with PS2 screenshots. Scrolling, the armour row and the blink were not driven. The navy band, the column between the lists and two grey lines visible over the PS2 gradient come from the sequence scenes the PC starts here (scene0, scene11, scene20), not from this draw; the PS2 runs the same three (read in PCSX2) and shows the same band and column (`docs/compare.md`).

## The Overdrive screen (`ui.menus`: code)

The main menu's Overdrive entry, then a member on the main menu's party rows (that pick is the
main menu's own draw). PS2 record `0x00325a78` (control `0x00230aa0` = `TkMenuCtrlLimit`
`0x008d0460`, init `0x0022fb38` = PC `0x008d0cd0`); PC record `0x00c5a768`.

| PC draw | PS2 draw | What |
|---|---|---|
| `0x008d09b0` (record +4) | `0x002312d0` | the member's party plate and the Overdrive plate |
| `MenuLimit_WindowDraw_008d16f0` | `0x00230708` (list `0x002305c0`, rows `0x00230488`/`0x00230328`, cursor `0x00230568`) | the View (`+0x5c` 1) and Set Mode (0) windows |

- **Party plate** (`0x00225f38`, `0x00200c60`) at `pos_4nameplt` (0, 64): the front-line band
  (`pos_noactive`, `col_noactive0/1`, 512 wide), then the ability screen's plate (4/5, name,
  HP/MP in font S, meswin 0xe7 stretched to 512 at y, 0x107, sphere level, next AP).
- **Overdrive plate** (`0x00229d60`, the Status screen's too) at `rect_2lgplt` (161, 112): every
  piece at its layout place moved by `rect_2lgplt - rect_lgplt`: box `rect_ltbplt` in
  `col_ltbplt` (black), box `rect_ltplt` in `col_ltplt0/1`, `rect_lgplt` as a common plate
  (0xd8 / 0xd9 in `col_lgplt0/1`), shape 0x109 ("Mode") at `rect_ltplt + pos_typshape`, 0x108
  ("OVERDRIVE") at `rect_lgplt + pos_lgshape + (16, 0)`, the gauge (`0x00229828`: width
  `rect_lg.w * g / max`, `col_lmtg0` left, `col_lmtg0..col_lmtg1` mixed by the fill on the
  right, all `col_lmtgf0` when full, nothing at 0; g / max = `TkMenuGetLimit` /
  `TkMenuGetLimitMax`), the mode's name `TkMenuMenu1Mes(TkMn2GetOverDriveId + 0x1033)` at
  `rect_ltbplt + pos_ltstr`. The Status screen calls `0x00229d60` with `rect_lgplt` itself
  (`0x0022a8f8`), so there the mode box `rect_ltbplt` (268, 96, 244 x 27) ends at x 512; the
  Overdrive screen passes `rect_2lgplt`, the move is (44, 16) and the box runs 312..556, cut
  by the 512-wide screen on the PS2 as by the screen edge here.
- **Windows**: View: box `rect_lwplt` (0, 112, 159 x 27, `col_lwplt0/1`) and the member's
  Overdrive command (`MsMenuStatusNum` / `TkMenuGetCommandOffset(list, 5, chr, 0x48)` /
  `TkMn2GetCommandName`) centred, y + 4. Set Mode: `0x00201a70` at `rect_lmojiplt`: meswin
  0xda / 0xdb 213 wide in `0x80000000`, strtex label 0 ("Overdrive Modes") 170 x 14 at
  (+21, +5) (`TOMkpShape2ECCCell_008f8d50`). Rows: columns `pos_2llplt` / `pos_2lrplt` (30 and
  279, y 182), 9 rows from y - 32 under `TkMn2DrawMenuWindowScissor(y, 224, ...)` (two passes
  while paging), each a kind-0x10 plate (0xda, 0xdb from x + 16 to x + 209, `col_babplt`, purple)
  and meswin 0xd7 at x + `pos_pmshape` + 0x58; usable entries by kind: 1 icon and command name
  (+12 / +29), 2 a name (+12), 3 `TkMenuMenu1Mes(id + 0x1033)` (+0x1d; the modes). The scroll
  bar (`0x00225e90` -> `0x00200878`, as the item list's) at `rect_lscroll` over
  (count + 1) / 2 rows; the PS2 cursor at 12 + column x, `pos_2lrplt.y` + 1 + 32 (cursor / 2 - top).
- No counts or progress numbers in the mode rows. The PC's divider lines
  (`MenuDrawDividerShapes(5)`) are not on the PS2.

**The restoration** (`mod/OverdriveMenuPatches.cs`): hooks on both PC draws make the calls above
with FFX.exe's primitives; layout words by name through `TkMn2SeqGetPos` / `TkMn2SeqGetRect` /
`TkMn2SeqGetColor` (scene 1), several of them being the Status init's (`0x00228c48` / PC
`0x008d4920`). The sub-command bar (View / Set Mode) is `ItemMenuPatches`', the help bar
`MainMenuPatches`'.

**Checked.** `tools/replays/ui-overdrive.txt` (`@ps2screen overdrive`: the record's `+4`, the
window's `+0x10`, the bar and the help bar through thunks; `tools/calllist/src/overdrive.rs`):
`od-1-hd.png`, `od-2-ps2.png` (Tidus, "Swordplay", the OVERDRIVE plate, Mode "Stoic", Spiral Cut
and Slice & Dice on purple plates), `od-3-ps2-43.png`, `od-4-mode-hdlist.png`, `od-5-mode-ps2.png`
(the "Overdrive Modes" label plate and the Stoic row).

**Uncertain.** Not compared with PS2 screenshots. The gauge was empty in the save (fill and full
colours not seen); paging and the scroll bar were not driven (2 entries). The 0xd7 underline shows
faint seams between its 8-texel tiles (`TOMkpShapeXY` with w = h = 0 samples the neighbouring
texels, as with the cursor). The "OVERDRIVE" label's O is cut on the left: column 0 of the cell
(0, 224) 136 x 16 in the disc's own `uspc/menu/meswin.fmt` holds only part of the O, and the
converter copies it unchanged, so this is the PS2 English art (the German "EKSTASE" in the same
cell is whole). The front-line mask is read at `0x01841c2c`, as the ability screen port does, for
the PS2's `0x00242908`. The aeon branch of `0x00200c60` (plates 0xcc / 0xcd) is not ported (no
aeon reaches this screen).

## The Customize screen (`ui.menus`: code)

The PS2 record `0x00325aa0` (control `0x00232a60`, init `0x002323a0`) opens two windows; the PC
kept both (`MenuKaizou_OpenWindow` `0x008d6d60`, `MenuKaizou_OpenListWindow` `0x008d6ce0`).
The layout comes from the menu sequence (PS2 init `0x002315f0`; PC `0x008d6be0`): `pos_kzonmplt`
`0x00693d68` (PC `0x0186aa40`), `r_kmojiplt1`/`2` `0x00693d70`/`78` (`0x0186aa44`/`4c`),
`pos_kabplt` `0x00693d80` (`0x0186aa54`), `pos_ksobiplt` `0x00693d88` (`0x0186aa58`), `r_kscrol`
`0x00693d90` (`0x0186aa5c`), `pos_ksobiplt2` `0x00693d98` (`0x0186aa64`), `r_kbackplt`
`0x00325ad0` (`0x023cc3a0`), `r_kmojiplt3` `0x00325b00` (`0x023cc370`); the cost window's
`r_shohiplt` `0x00325a40` (`0x023cc440`), `pos_shiplt`/`shshape`/`lshohi`/`rshohi`
`0x00325a58`..`70` (`0x023cc430`/`42c`/`428`/`424`), `pos_pmshape` `0x00325878` (`0x023cc420`).
The mod reads them by name (`TOSeqGetPoint`, `TOSeqGetRect`, `TkMn2SeqGetColor`); in the game:
`pos_ksobiplt` (30, 119), `r_kscrol` (256, 103, 10, 309), `r_kmojiplt1` (28, 94), `r_kmojiplt2`
(277, 254), `pos_kzonmplt` (0, 49), `pos_kabplt` (279, 279), `r_kbackplt` (279, 119),
`r_shohiplt` 209 x 65, `pos_shshape` (40, 23), `pos_lshohi` (33, 19), `pos_rshohi` (125, 19),
`pos_pmshape` (9, 0).

**What the PS2 drew.**

- Equipment list (`0x00232188`, window `0x00693d58` = PC `0x0186aa30`; PC draw
  `MenuKaizou_Draw_008d6860`): rows -1..9 from `pos_ksobiplt`, 32 apart (`0x00231f90`), in the
  window scissor `0x002274a8(y, 0x120, part, +0x46)` (two passes while scrolling, the second
  `-0x120 * +0x46 / 4096` higher). A row (`0x00231e70`): plate kind 0 (`TOAdpShapeByKind`,
  `col_bsoplt`, grey), shape `0xd7` at `pos_pmshape + (0x58, 0)`; when the weapon record is in use
  (+2) and not flag 2 (+3): icon `2 * owner + kind + 1` at (+12, +1), the name (16x12 font,
  grey unless `0x002317c8` = `MenuEquip_IsSelectable(id, 0)`), icon `0x31` over it when +4 ==
  +6 (`0x001f5298`). Then the cursor at (`pos_ksobiplt` + (12, 1 + 32 row)), the scroll bar
  (`0x00225e90` -> `0x00200878`, dark track, light thumb), the labels (`0x00226218` ->
  `0x00201a70`: black plate `0xda`/`0xdb` 213 wide, strtex cell at (+21, +5)): cell 7 at
  `r_kmojiplt2`, 6 at `r_kmojiplt1`; a band `col_menubg0 -> col_menubg1` from `pos_kzonmplt.y`
  down with alpha `TkMn2GetStatusBG / 32` (`0x00231e18`); the picked weapon's slots
  (`0x00232030`: kind-0x10 plates with `0xd7` per slot (+0xb), its four abilities (+0xe)
  with icon (+0x68) and name); the owner's plate (`0x00225f38`: the front-line band, then
  `0x00200c60(chr, pos_kzonmplt)`: plate 4/5, name, HP/MP, shape `0xe7` across the screen,
  `0x107`, sphere level, next AP; for no owner the empty plate `0xcc`/`0xcd`, `0xe9`, `0xea`).
- Abilities to add (`0x00232730`, window `0x00693d5c` = PC `0x0186aa34`; PC draw
  `DrawGearCustomizationMenu` `0x008d5f60`): for the row's kaizou record (item list entry +3,
  8-byte records from `TkMn2GetKaizou`, item +4, count +6) the cost window `0x00227800` at
  `r_kbackplt` (window frame `0x001fa328` in `col_shohiplt0/1`, a kind-8 gold row with the
  item's icon and name, shape `0xff` (two discs and the arrow), the count held in font M at
  `pos_lshohi`, at `pos_rshohi` what is left or `TkMenuMainMes(0x15)` right-aligned in grey when
  too few) and label cell `0x10` at `r_kmojiplt3`; label 7 at `r_kmojiplt1`; rows as above with
  kind-0x10 plates and the ability's icon and name (grey unless the entry's kind is `0xb`);
  cursor; scroll bar; the equipment row's weapon on a kind-0 plate at `pos_ksobiplt2`
  (`0x002324d8`). This window draws over the equipment list in the same places.

**The restoration** (`mod/CustomizeMenuPatches.cs`): hooks on `MenuKaizou_Draw_008d6860` and
`DrawGearCustomizationMenu` make those calls with `Ps2Menu`'s primitives,
`TkMn2DrawMenuWindowScissor` `0x008c0f70` (PS2-unit twin of `0x002274a8`),
`TOMkpWindowFrame_008f9b60` (twin of `0x001fa328`), `TODrawSheetCell_008f8bb0` for the strtex
labels (the converter's PS2 strtex), `MsGetRomAbility`/`MsGetRomItem` for the icons and names.
The long shapes (`0xd7`, `0x107`, `0xff`) are drawn part by part from FFX.exe's copy of the PS2
layout table (`0x00c5b460`, ids 200.. at id - 200), each quad with its texels inset by half a
texel: with `TOMkpShapeXY` the bilinear filter blended in the opaque column left of the `0xd7`
cell and put a light tick on every 8-pixel piece. The PC's `TODrawMenuBG`, divider shapes and HD
plates are dropped; logic and input stay the PC's.

**Checked.** `tools/replays/ui-customize.txt` through `tools/calllist` (`@ps2screen customize`,
the same calls in `tools/calllist/src/customize.rs`), save 5 (`kami0100`, Customize is the 9th
main-menu entry): `cust-1-hd.png`, `cust-2-ps2.png`, `cust-3-ps2-43.png` (equipment list, cursor
on Yellow Shield), `cust-4-add-hd.png`, `cust-5-add-ps2.png`, `cust-6-add-ps2-43.png` (Yellow
Shield's abilities, cursor on Lightning Ward, cost Electro Marble 46 -> 42).

**Uncertain.** Not compared with PS2 screenshots. Not driven: the scrolling passes, the "too few
items" message, the empty party plate, the fade band while visible (it was 0x1000 with the
abilities list open; its colours equal the background, so it only hides the list below). The
strtex labels go through `graphicDrawUIElement`, outside the packet; their order looked right in
the harness. The help bar stays HD. The cost row plate (225 wide) runs past the 209-wide
`r_shohiplt` window, as the data says.

## The Config screen (`ui.menus`: code)

**What the PS2 drew** (SLPM_675.13, record `0x00326008`: control `0x002353f8`, draw `0x00235a68`, page select `0x00235ab0`, open `0x00235b50`, close `0x00235c10`). The draw runs `0x002492f8` twice, then the packet setup, then the page `0x00235a00`. The page calls `0x00235720(0, 0, row)` for each row, with a kick (`0x00225b28`) between rows. The rows are option records. Both games use the same layout: count `+4`, value `+8`, enabled `+0xc`, get/set `+0x10`/`+0x14`, title id u16 `+0x18`, choice ids u16 every 4 bytes from `+0x1c`. The PS2 records are 0x30 bytes apart from `0x00325d60`. The PS2 has four lists (`0x00325f70`/`f98`/`fb8`/`fe0`, with 9/8/10/9 rows), chosen by `0x0020b858` and `0x002d92d0`. A row draws:

- The plate `r_cnameplt` through `0x00226f30`: meswin 0xcc stretched `c_cnameplt0` -> `1`, then 0xcd. Rows are `num_cnameplt` - 5 apart.
- The title at `pos_cname` with `0x00226358` (the 16x12 font).
- The cursor (`0x00227100` -> `0x001fe580` kind 0) on row `0x00693f68`.
- The choice window `0x00226fe8` -> `0x001fa328`: meswin 200 at (x, y) in c0; 201 at (x + cw, y), w - cw wide, c0 -> c1; 202 at (x, y + ch), h - ch high, in c0; 203 at (x + cw, y + ch), (w - cw) x (h - ch), c0 -> c1. cw x ch is the size of shape 200 (16 x 24). The window sits at `pos_cchoplt`, has the size of `rect_cchoplt`, and runs `col_helpplt0` -> `1`.
- Each choice centred at x + (i + 1) w / (n + 1), y + `pos_cchoplt.y`, grey when the row is disabled.
- The brush under the value (`0x00225bf0`: slot = row in `0x006672d8`). It restarts when its place changes, takes w / 14 + 2 frames, and draws (p / steps)^2 of the width: 0xe2 at x - 5, 0xe3 from x + 11, 0xe4 at the end.
- The help of the cursor row's value (`0x002496b0` -> `0x002437e0`).

`rect_cbline`, `pos_cbline`, `c_cbline0/1` and `rect_cscroll` are read but never drawn. Values read in the game:

- `r_cnameplt` (0, 55) w 182, `c_cnameplt0/1` `34 31 46 80` -> `28 1e 32 80`, `num_cnameplt` 45.
- `pos_cname` (42, 7), `pos_cchoplt` (184, 5), `rect_cchoplt` 289 x 30, `col_helpplt` `09 09 09 80`.

The window's left edge is the corner cell's brushed alpha edge (the same pieces as the plain message window). Over the light menu gradient its gaps show the background.

**The restoration** (`mod/ConfigMenuPatches.cs`). The hook on `MenuConfig_Draw` (`0x008cb320`) calls `TkMenuDrawKick` twice, then draws the PS2 rows in one packet:

- Plate: `0x008f5e90` (= `TkMn2DrawCommonPlateR`'s `0x008f09f0`).
- Title and choices: `TOMkpBaseMesFont`; widths from `TkMn2StringWidth`.
- Cursor: the PS2 cursor.
- Window: `TOMkpShapeXYWHC2` 200..203, corner size from `TOGetWindowLayout(200)` (the shapes of `0x008f1e40`).
- Brush: 0xe2/0xe3/0xe4, with the PS2's sweep.

The rows are the PC's. `g_TkMenuConfigRows` is 8; the PC replaced Sound with the soundtrack row (`0x00c5a528`, `g_TkMenuConfigLocRow`), whose title, choices and help come from `SaveDataGetLoc` 0x1cd / 0x1d0 - 2k / 0x1d1 - 2k, as in the PC's draw. The help is set from the draw (`TkMenuConfigHelp` or `g_TkMenuConfigHelpText`, then `TkMenuSetHelpMessage`). The PC's layout words come from `0x008cc050` (the twin of the PS2 init `0x00234a38`, read by the same names).

**Checked.** `tools/replays/ui-config.txt` through `tools/calllist` (`tools/calllist/src/config.rs`: the same calls; it points the config menu record's `+4`, `0x00c5a5e8`, at the thunk). Screenshots in `/mnt/c/shots/dm-pm5/ui-config/`:

- `config-1-hd.png`: the HD screen.
- `config-2-ps2.png`: the PS2 version: plates, titles, the cursor on Music, black choice windows, brushes under the values.
- `config-3-ps2-43.png`: the same after the 4:3 write.
- `config-4-moved-ps2.png`: cursor on Names and its value moved to OFF; the brush follows and the help text changes.

**Uncertain.** Not compared with PS2 screenshots. The harness draws the brushes at rest. Thin bilinear edge lines show under the brushes and at the window's left edge (the same as in Items). The help bar stays HD in the test. The PS2's open starts the menu scenes "scene4" and "scene17" and its close stops them (`0x00227488`/`0x00227498`); the PC never starts them. Started on the PC, they draw a dark blue vertical band that slides up at the right edge (PS2 x about 476..492) and the band under the help bar; the mod starts them since 2026-11-03 ("The menu sequence scenes"). The rows' places were the PC's `menu_script.bin` (55 + 40 r); with the disc's (`ui.menus` writes it, "The menu layout data") they are the PS2's 65 + 42 r.

The screenshots of these five screens (2026-11-03) are kept in
`~/ffx-work/demaster/research/ui/submenus/<replay>/`.

## Shared pieces of the ported menus (`mod/Ps2Menu.cs`)

- `shape` / `shape_c2`: a meswin shape at its own size, every layout part as a
  `TOMkpShapeXYWHUVC2` quad with the UVs half a texel inside the part's cell.
  The PS2 drew these 1:1 and sampled texel centres; the PC's
  `TOAdpEasyShapeXYWHC2` (`0x008ed380`) samples to the cell edges, so at the
  PC's scale the filter pulls in the neighbouring texels, and shapes tiled
  from one 8-texel cell (0xd7's line, 0x10a, 0x107, 0x3e's strip) showed a
  light tick at every tile. It also skips every part of 0x3e but the last.
  calllist turns `TOMkpShapeXY(C2)` calls into the same quads
  (`menu::expand_shapes`).
- `kind_plate` (`TOAdpShapeByKind`), `scroll_bar` (`0x00200a60`),
  `label_plate` (`0x00226218`: black brush and a strtex label), `party_plate`
  (`0x00225f38` -> `0x00200c60`: the party plate of Status, Equip, Overdrive
  and Customize), the PS2 fonts, `packet`, `seq_point`.
- On every party plate the MP string's last cell ends at x 296 (`pos_mpstr`
  (206, 18), 9 cells of 10), where the S.LV label of shape 0x3d / 0x107
  starts (`pos_slvshape` (296, 33), read in the game); the label is drawn
  after the string, as on the PS2. The values come from `menu_script.bin`: the PC reads the Japanese International's `jppc` copy, the `ui.menus` job writes the PAL disc's over it ("The menu layout data"); the party plates sit at the same places with either (compare runs before and after).
- The calllist replays draw the main menu's help bar the PS2 way too
  (`@ps2help`), as the mod does on every screen.

## Not done

- The sphere grid entry and the aeon status pages (same draw as Status, not
  driven). The sphere grid's and the aeon pages' sequence scenes are not
  asked for (the other screens' are, "The menu sequence scenes").
- The HD cursor of the HD screens (`TkMn2DrawCrossCursor` `0x008c0670`,
  kinds 0/4/5): left as it is; its anchor differs from the PS2 cursor's, so
  replacing it needs each caller's PS2 position.
- The `?`/`!` icons and the other `meswin` cells outside the ported draws.
- The battle target info window (`TOBtlDrawInfoWin`, PS2 at (0x14, 0x46)),
  the enemy gauges and Libra: HD. Wakka's reels (battle menu texture
  0x13). The menus' sequence scene lines (the PC draws none) stay off.
- The battle words' place and the `strtex` labels in play (not reached).
- `worldmap` (airship map): 512x416 PS2 against a 1920x1080 HD layout.
- Sphere grid (`menu/abmap/dat00/tex`): named after VRAM pages
  (`14080_19` ...), PS2 source `jppc/menu/abmap/dat00.dat` (700 KB); not
  looked at.
- Help screens (`help*/`), `dvdcopy`, `test_proj`: not looked at.
- Language-neutral elements of text-bearing atlases are only written to
  the disc's languages.
