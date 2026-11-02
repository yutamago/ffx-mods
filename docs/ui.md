# The interface: menus, portraits, title, loading screen, button glyphs

Rows 12, 16 and 22 of [`differences.md`](differences.md). Converter:
`converter/src/uijob.rs`. The font (row 13) is in [`font.md`](font.md).

```bash
./target/release/demaster ui <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out>
```

writes `<out>/efl/x/<HD path>` and a preview per file
(`<out>/preview/<folder>_<name>.png`, HD left, result right). In
`demaster build` the jobs are `ui.menus` (settings `ui.menus` and
`ui.portraits`), `ui.title`, `ui.loading` and `ui.glyphs`
(`ui.button_glyphs`).

## Summary (German disc, 2026-11-02)

| Part | Files | In the game |
|---|---|---|
| Title screen (`map/titl/titl00/2d`): PS2 logo, menu, bar | 12 layers (EN x2, DE, FR, IT, ES, 6 "Install" labels) | checked (`boot.txt`, English) |
| "FINAL FANTASY X PROJECT" logo (`titl02`), "Fine" screen pieces (`titl01`) | 4 (the HD kept the PS2 pictures) | logo checked |
| "HD Remaster" under the opening's logo (`magic_0662/logo_hdremaster`) | 1, transparent | not reached by a replay |
| Loading screen (`menu/loading`: `ld00..24`, `loadingbg`) | 26, transparent | not seen: the harness loads hide it |
| Battle atlas (`menu_de`, `menu_us` `battle`): CTB faces, markers, letters | 2 (15 and 16 PS2 elements each) | checked (`battle-attack.txt`): PS2 CTB faces |
| Menu faces `face_ply`, `face_smn`, `sface_ply` | 14 (every language folder) | checked (`tools/replays/ui-status.txt`): PS2 face on the Status page |
| Button glyphs (`menu/d3d11/pad_icon`) | 1 | not checked (the harness pad shows no prompt in these replays) |
| Message window atlas `meswin` | 0 | no element matches: the HD redrew it |

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

## Not done

- `meswin` (message window frame, cursor, `?`/`!` icons): no HD element
  passes the rules; the cursor in the game is the HD silver cone.
- `icon` and `strtex` (item icons, "Miss/Immune/Overkill" graphics in
  `icon`, menu labels): CLUTs made at run time. A table of the RGBA the
  code passes per icon (`TOAddPktMenuIconRGBA` callers) would allow it.
- `worldmap` (airship map): 512x416 PS2 against a 1920x1080 HD layout.
- Sphere grid (`menu/abmap/dat00/tex`): named after VRAM pages
  (`14080_19` ...), PS2 source `jppc/menu/abmap/dat00.dat` (700 KB); not
  looked at.
- Help screens (`help*/`), `dvdcopy`, `test_proj`: not looked at.
- Language-neutral elements of text-bearing atlases are only written to
  the disc's languages.
