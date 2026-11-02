# The font (row 13): research and plan

Status: research only, nothing converted yet. Short version: **for the
European languages the PS2 font is a texture-and-table swap** (the PC
kept the PS2 glyph grid in PS2 units); for Japanese, Chinese and Korean
the PC's per-event font pages are its own and have no PS2 counterpart
except Japanese.

## How the PC draws text

`graphicFontDraw(glyph, texture, ..., page)` (`0x00640660`, ffx-rust
`docs/formats/ui-draw.md`) draws one quad per glyph from a glyph record
(UV rectangle `+8`/`+0xc`, `+0x28`/`+0x2c`) with `OptimizedFontShader`,
shadow quads from `shadow_0_n` and face quads from `font_0_n` (two pages
each). The texture name comes from per-language tables (`0x00c59568..`):
`menu{,_us,_de,_fr,_it,_es,_kr,_ch}/base_ftc/d3d11/{font,shadow}_0_{0,1}`.
Other fonts: `xfont1208`, `subfont` (small digits and labels, `0x3d80`),
`newkit_ftc`, `help/help_ftc`.

The glyph metrics come from FTC files (`TKMallocAndReadEventFTC`
`0x0088c860` reads `ffx_ps2/ffx/master/new_<lang>pc/{menu/base,menu/newkit,
help/help,battle/btl/<b>/<b>,event/obj_ps3/<e>}.ftc`; `TOFTCTransfer`
`0x008ac130` installs one):

| Off | Type | Meaning |
|---|---|---|
| `+0` | `FTCX` | magic |
| `+4` | `u16` | glyph count (must be > 199) |
| `+8` | `u16` | font type (0 Japanese base, 4 European base, 1-3, 5 others); `TOGetImageWH` `0x3c00`/`0x3d00..0x3d60` return the page size by type |
| `+0x10` | `u32` | width-table size |
| `+0x14`, `+0x16` | `u16` | cell width, height (14 x 18 for the base fonts) |
| `+0x20`, `+0x24` | `u32` | offset and size of the glyph bitmap (PS2: 4 bits per texel; PC European: size 0) |
| `+0x28`, `+0x2a` | `u16` | page width, height in PS2 texels |
| `+0x30` | `u32` | offset of the width table (one byte per glyph) |

The 962 per-event and per-battle font pages (`battle/btl_ch`, `btl_kr`,
`new_btl`, `event/new_obj`, `obj_ch`, `obj_kr`) belong to the Japanese,
Chinese and Korean text (the glyphs a scene needs from thousands of
kanji/hanzi/hangul); the European languages do not use them.

## The PS2 font

`jppc/menu/base.ftc` on the German disc (14,096 bytes; `uspc/menu/base.ftc`
for English) is a type-4 FTC with the bitmap inside: 200 glyphs, cells
14x18, a 128x216 page at 4 bits per texel (`+0x20` = `0x40`, size 13,824),
widths at `0x3640`. The 4-bit page holds **two 2-bit pages**: bits 0-1 are
the PC's `font_0_0`, bits 2-3 `font_0_1`. The same packing is in
`xfont1208.fmt` and `subfont.fmt` (128x256), whose HD versions in
`menu_de/` etc. are the PS2 pixels at 1x already.

The PC's `uspc/menu/base.ftc` (304 bytes) is the same header in PS2 units:
230 glyphs (the PS2's 200 and 30 more: `★`, `≈`, `SM`, ...), cells 14x18, page
128x234, no bitmap, widths for 234 glyphs. The HD page
`menu_us/base_ftc/font_0_0` (512x1024) is that 128x234 grid scaled 4x
across and 4.38x down: the glyph columns match the PS2's to a texel, the
rows match at a pitch of 1024 / 234 (measured: PS2 row tops 2, 21, 39, 57
against HD 3, 23, 43, 62 in PS2 units). Glyph order is the PS2's
(`~/ffx-work/demaster/research/ui/font_us_cmp.png`: PS2 plane, HD page, per page).

`menu_de/base_ftc` (and `_fr`, `_it`, `_es`) are 128x256 pages that look
like the PS2 font already; whether the PC uses them for German text was
not checked (the harness runs in English).

## Plan (European languages)

1. Read `<lang>pc/menu/base.ftc` from the disc; split its page into the
   two 2-bit planes; check which 2-bit values are face and which are edge
   or shadow (the PC has separate `shadow_0_n` pages; the PS2 drew the
   shadow from the same glyph in another colour, `TOMakePktFont1612Shadow`).
2. Write `font_0_0`/`font_0_1` (and `shadow_0_n`, the face mask, or the
   PS2's shadow levels) at 128x234 into the language's `base_ftc` folder,
   glyphs 200-229 taken from the HD page scaled down (no PS2 glyph).
3. Write the language's `ffx_ps2/.../<lang>pc/menu/base.ftc` with the PS2
   width table for the first 200 glyphs (the PC table is wider: the HD
   typeface is not the PS2's), keeping the PC's 30 extra entries.
4. The same for `xfont1208`/`subfont` (`menu_us` has HD versions; the other
   European folders already hold the PS2 pixels) and `help_ftc`.
5. Check in the game: `draw-ui-dialogue.txt` (event message window),
   `draw-ui-menu.txt`, `battle-attack.txt`; compare spacing with PS2
   screenshots.

Effort: **S-M**, about 2 days. Risks: the PC's glyph UVs might use the
cell size from its own header and not the page size (then the 4.38x
vertical scale is a header change too); text widths that the PC laid out
for the HD typeface (menus built for the wider HD glyphs).

Japanese: `jppc/menu/base.ftc` has the bitmap for type 0 too (1008-glyph
page on PC), plus the per-event FTCs on the disc (`jppc/event/...`,
`battle/btl/<b>/<b>.ftc`); a PS2 Japanese disc would be needed to test.
Chinese and Korean have no PS2 font (the PS2 International had none).
