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
| Title screen (`map/titl/titl00/2d`): PS2 logo, menu, bar | 12 layers (EN x2, DE, FR, IT, ES, 6 "Install" labels) | checked (`boot.txt`, English) |
| "FINAL FANTASY X PROJECT" logo (`titl02`), "Fine" screen pieces (`titl01`) | 4 (the HD kept the PS2 pictures) | logo checked |
| "HD Remaster" under the opening's logo (`magic_0662/logo_hdremaster`) | 1, transparent | not reached by a replay |
| Loading screen (`menu/loading`: `ld00..24`, `loadingbg`) | 26, transparent | not seen: the harness loads hide it |
| Battle atlas (`menu_de`, `menu_us` `battle`): CTB faces, markers, letters; the damage digits with `ui.font` (also into `menu_fr`, `menu_it`, `menu_es`, `font.md`) | 2 (15 and 16 PS2 elements each), 5 with `ui.font` | checked (`battle-attack.txt`): PS2 CTB faces |
| Menu faces `face_ply`, `face_smn`, `sface_ply` | 14 (every language folder) | checked (`tools/replays/ui-status.txt`): PS2 face on the Status page |
| Button glyphs (`menu/d3d11/pad_icon`) | 1 | not checked (the harness pad shows no prompt in these replays) |
| Message window atlas `meswin`: the PS2 window pieces (below, "The PS2 windows") | 8 (every language folder) | checked (`tools/replays/ui-dialogue.txt`, `ui-dialogue-field.txt`): PS2 message windows |
| Battle command windows (code only: `mod/BattlePatches.cs`, below, "The battle command windows") | 0 | checked through injected game calls (`tools/replays/ui-battle-main.txt`, `ui-battle-item.txt`): PS2 gradient box and layout; hooks seen installing in Fahrenheit |

60 files in all; 20 HD textures have no PS2 counterpart (listed by
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
textures**: the main menu (`ingame-mainmenu.png`) stays the HD design with
this mod. Restoring it needs the PS2 window colours and layout in the draw
code (milestone 3; ffx-rust `menu.md` has the callers). The texture swap
restores the pieces that are still PS2 art at PS2 places: faces, CTB list,
title, markers.

## PS2 menu textures

The PS2 menu images are raw indexed images, the palettes in one file:

| PS2 (`jppc/menu/`) | Format | HD (`menu*/d3d11/`) | Scale | VRAM name (`TOGetShapTextureName`) | Palette |
|---|---|---|---|---|---|
| `meswin.fmt` | 256x256, 8 bit | `meswin` 1024x1024 | 4 | `0x3e00` | 0 |
| `battle.fmt` | 256x256, 8 bit | `battle` 1024x1024 | 4 | `0x3f00` | 1 |
| `icon.fmt` | 256x128, 4 bit | `icon` 1024x512 | 4 | `0x3dc0` | run time (below) |
| `strtex.fmt` | 512x128, 4 bit | `strtex` 2048x512 | 4 | `0x2ecc` | run time |
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
`icon` and `strtex` look wrong with every CLUT of `menu.clp` and with
every 16-colour run in the ELF: their colours are made at run time
(`TOAddPktMenuIconRGBA`, `TkMenuDrawEasyMes1612Clut`, `...ClutTypeRGBA`
take an RGBA), so they are not converted.

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

The PS2 image is the whole screen (512x448): the logo, menu and bar sit
within a few pixels of the HD ones (lower-half correlation 0.53-0.55). Its
top holds the PAL web-site banner and the water strip; in the game the PC
draws its own water band (`menu/titl/water_NN`, Tk) over that part, so
the banner is covered (`ingame-title.png`). Whether the PS2 showed the
banner on the title screen is not known here; if it did not, nothing
changes, since it is covered.

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
look.

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
not seen. (The item shots were taken with the counts' right edge at the
cell's edge, where the scissor clipped the last digit's outline; the mod
and `btl.rs` now end them 2 pixels earlier; not re-shot.)

**Uncertain.** Not compared with PS2 screenshots: the look follows the PS2
code and the run-time colours. The names use the PC's battle typeface size
(0.78), not the PS2's 1612 font draw (`0x00207f68` / `0x001f3810`, not
ported), so letter sizes and the column width derived from them differ
slightly from the PS2's; with `ui.font` the glyphs and widths are the
PS2's. The scroll bar is the PC's `DrawCrossMenuScrollParts` (grey track,
pink thumb) at the PS2's place, not the PS2's own bar. The hand cursor is
the PC's (drawn at the PS2 position). Icons are the HD `icon` art at a
12 x 12 PS2 size.

## Not done

- The rest of `meswin`: the cursor, the `?`/`!` icons, and the brush
  plates (entries 4-7 and 16-28) that the PS2 menus tinted with the
  `col_*plt` colours above. The HD cursor (a silver cone with a glow) is
  close to the PS2 one (shape 208: a white ball, entry 8 at (80, 144),
  under a silver triangle at (0, 168)), but the remaster draws it from its
  own cells. Not changed: the battle windows place the PC's cursor at the
  PS2 positions.
- Main menu and sub-menus (items, abilities, equip, status, config, sphere
  grid entry): not done. What a port needs, as far as found:
  - The PS2 draws are in SLPM_675.13's menu module: the party rows and the
    footer `0x0024a4a8` (uses `rect_nameplt` `0x00324204`, `col_nameplt`,
    `rect_placeplt`; overrides `num_namepltspc` to 44 and `num_compltspc`
    to 32 at its start), the option list `0x0024af70` (`rect_complt`
    `0x00324230`, `col_complt`). Their PC counterparts `TkMenuDrawMain`
    (`0x008e0ba0`) and `MenuMain_DrawCmdWindow` (`0x008e20f0`) are HD
    rewrites. The help plate (`rect_helpplt` `0x0032425c`) is read by
    `0x00209360`, `0x00216fe8`, `0x00218338`, `0x00223b08`, `0x00243888`.
  - The PC still holds a PS2-era party plate,
    `TOMenuDrawChrLayoutPlate_008f0a70` (meswin shapes 4/5 with explicit
    UVs, tinted `col_nameplt`, name at `pos_namestr`, HP/MP labels 0x21/0x22,
    `"%5d/%5d"`, sphere level, all placed by the `TOSeqInitParam` words), now
    only called from status page 2 (`MenuStatus_DrawLayoutRow`).
    `TOAdpShapeByKind` (the `col_bsoplt`.. pairs) is likewise still there.
  - Blocker: the pieces these draw are the PS2 brush plates of `meswin`
    (shapes 4/5 = the (0, 0) 48x40 region, layout entries 4-7 and 16-28,
    0xcc/0xcd, 0xda/0xdb). In the HD `meswin` those places hold other HD art
    (the feather pen sits at (0, 0)), so the converter can't paste them
    where they were: it needs free cells in an atlas plus remapped UVs in
    the drawing code, or a separate texture name. Then the draws: background
    (`col_menubg*`), party rows, option plates, help/place/gil plates, the
    sub-menus' lists, ported function by function from the MIPS (about 50
    callees in `0x0024a4a8` alone).
- `icon` and `strtex` (item icons, "Miss/Immune/Overkill" graphics in
  `icon`, menu labels): CLUTs made at run time. A table of the RGBA the
  code passes per icon (`TOAddPktMenuIconRGBA` callers) would allow it.
  Not looked for in this round (2026-11-03); the battle windows draw the HD
  icons.
- `worldmap` (airship map): 512x416 PS2 against a 1920x1080 HD layout.
- Sphere grid (`menu/abmap/dat00/tex`): named after VRAM pages
  (`14080_19` ...), PS2 source `jppc/menu/abmap/dat00.dat` (700 KB); not
  looked at.
- Help screens (`help*/`), `dvdcopy`, `test_proj`: not looked at.
- Language-neutral elements of text-bearing atlases are only written to
  the disc's languages.
