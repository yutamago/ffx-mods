# The font (row 13)

Status: **done for English, French, Italian, German and Spanish**
(`ui.font`). It is a swap of four textures and one width table (job
`ui.font`, `converter/src/fontjob.rs`), plus the PS2 damage digits in the
battle atlas (job `ui.menus`, `uijob.rs`). There are no code changes. Japanese,
Chinese and Korean keep the HD font.

```bash
./target/release/demaster ui <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out>
```

writes the font files with the other UI files, and previews:
`preview/menu_us_base_ftc_{font,shadow}_0_{0,1}.png` (HD page left, ours
right) and `preview/ffx_ps2_ffx_master_uspc_menu_base.ftc.png` (sample
lines with every European accent, HD font and widths above, PS2 below).

## What the European languages draw with

One font for all five languages and for every Tk text: dialogue and
event windows, menus, battle text and HP/MP counters (`ToMakeBtlEasy*`),
help lines. The damage numbers are not text (below). `TOGetFontInfo` (`0x008b7240`) gives every glyph of
languages 1-8 the texture id `0x3d40`. For that id, `TOGetTextureName`
(`0x008acf50`) returns `menu_us/base_ftc/d3d11/{font,shadow}_0_{0,1}`
(tables `0x00c595e8`, `0x00c59628`) for every European language.
`menu_de/fr/it/es/base_ftc` (128x256 pages that already look like the PS2
font) are not referenced by the exe. `TOInit` (`0x008ad4d0`) loads the
width table for these languages with `TODVDFileLoad(4, 0)`, which reads
`ffx_ps2/ffx/master/uspc/menu/base.ftc` (304 bytes, a type-4 FTC without a
bitmap). Only Japanese, Korean and Chinese read
`new_<lang>pc/menu/base.ftc`.

The other font textures are out of scope:

- `newkit_ftc` is loaded only for Japanese, Korean and Chinese.
- `help_ftc` (`0x3d01`) is only reached by the two-byte codes 0x26/0x27.
- The battle and event FTCs (`0x3d00`) are only reached by codes 0x28..0x2b.
- `subfont` (`0x3d80`) names `menu/subfont/...`, which is not in the
  archive.

Help text in the European languages uses the base font, so the swap
already covers it.

### Glyph layout (`TOGetFontInfo`)

For text byte `c` (from `0x30`), glyph `i = c - 0x30` (plus 0xd0 per
`0x2c..0x2f` prefix):

- page `i & 1`, cell top left `((i % 18) / 2 * 14, i / 18 * 18)` in PS2
  texels, so each page has 9 columns of 14 and rows of 18;
- width `w = table[i] / 4` (signed byte, quarter texels); the quad samples
  `u0 .. u0 + w`, `v0 .. v0 + 18`, and its UVs are divided by the page size
  from the FTC header (`TOGetImageWH(0x3d40)` gives dictionary 4: 128 x 234 on
  the PC);
- y offset 1.0 (the slant field `+0x18` for languages 1-8), scale 0.78
  (1.0 for the edge kinds 1 and 3);
- advance = `w` x scale + `TOGetFontSpaceInPiexl` (0.7 PS2 px rounded to
  whole screen pixels, about 0.8 at 1080p).

The face is drawn from `font_0_n`. The shadow is drawn from `shadow_0_n`,
moved by `0x018716e0`/`0x018716e4`, and the edge kinds draw `shadow_0_n`
ten times around the glyph. The HD pages are white with the coverage in
alpha (with a baked top-to-bottom grey gradient), and the shadow pages are
black with exactly the same alpha. The text colour and gradient come from
the vertex colours (top/bottom pairs per colour; the table's address is not
known yet: `0x00b5f890`, given here before, is `TOGetTextureName`'s size
table).

### Damage numbers

`TOBtlDrawDamageValue2` draws the damage digits with
`TOMakePktBtlDamageNumRGBA` (`0x008fb840`). Each digit is a shape (400) from the battle atlas
(`menu_<lang>/d3d11/battle`, `0x3f00`). Digit `n` is the cell `444 + 49 (n % 5)`, `254 + 65 (n / 5)`,
49x65 in the 1024 HD atlas, which is 111 + 12.25 col, 63.5 + 16.25 row in PS2
texels. The PS2's `battle.fmt` has its own serif digits (the
International look) at the same place. The HD redrew them in its sans
typeface, and the element matching of `ui.menus` keeps them HD because the
shapes do not overlap enough.
With `ui.font = "ps2"` the `ui.menus` job (which writes that atlas;
setting list `ui.menus`, `ui.portraits`, `ui.font`) pastes the PS2 block
`[111, 63.5, 61.25 x 32]` scaled up bilinear over the HD digits. This happens in
`menu_us` and `menu_de` (from `uspc`/`jppc`, with the other PS2 elements)
and in `menu_fr`, `menu_it`, `menu_es` (from the disc's own atlas, digits
only). The block stops half a texel short of the cell bottom, because the filter
pulled in the dark edge of the bar below.

## The PS2 font

`jppc/menu/base.ftc` on the German disc is byte-identical to `uspc/menu/base.ftc`
(14,096 bytes, the International font). It is type 4, 200 glyphs, cells
14x18, with a bitmap at `0x40`: a 128x216 page at 4 bits, low nibble first.
Bits 0-1 are page 0 and bits 2-3 page 1. A 2-bit value is coverage
(3 = full, 1-2 anti-aliasing at the stroke edges). The width table at
`0x3640` has 208 bytes, in whole texels. The same layout as the PC's
(glyph `i` at the same cell), so the PC glyph order is the PS2's.

Ink and widths: every PS2 glyph starts at the cell's left edge and is
exactly `width` texels wide ('0' ink 0..7, width 7). Glyphs 0..185 have
PS2 pixels. 99 and 103 (`Œ`, `œ` on the PC) and 170, 187-229 have none:
these are the glyphs the PC added (`★ ☆ ■ □ ∞ ¾ ÷ ă SM ...`). The PS2
font is narrower than the HD typeface. Sample widths (texels, PS2 / HD):
`A` 7 / 10.5, `M` 9 / 11.25, `W` 9 / 13.25, `0`-`9` 7 / 9, space 4 / 6,
`m` 10 / 10.75. Lines and windows that the game sizes from the text get
narrower: `TOGetMessageWH` sums these widths, so the dialogue windows
shrink to the PS2 width.

## The conversion (`fontjob.rs`)

1. Read the disc's `jppc/menu/base.ftc`, falling back to `uspc/menu/base.ftc` if
   that one is not a type-4 font with a bitmap (a Japanese disc). Split the
   4-bit page into the two 2-bit pages.
2. For each glyph of the PC table (230): if the PS2 font has it (index <
   200 and ink or a non-zero width), copy its cell with alpha = coverage x
   85 and set the PC width to the PS2 width x 4. Otherwise box-filter the
   HD cell's alpha down to 14x18 and keep the PC width. The result on the
   German disc is 183 PS2 glyphs, 9 HD-only glyphs and 173 of 230 widths changed.
3. Write `font_0_n` (white) and `shadow_0_n` (black, same alpha) at 4x the
   PC page size (512x936). The game's UVs are relative, so any multiple
   works. The 4x page is filtered up bilinear *within each glyph cell*:
   at PS2 size (128x234) the game's bilinear filter blended in the last
   texel row of the cell above. In the first in-game test, a `g`
   descender showed as a dash over the `y` of "System". At 4x the filter
   reaches 1/8 of a PS2 texel past a cell, and the glyphs look as soft as
   at PS2 size.
4. Write `uspc/menu/base.ftc` with the new width table. The header and size
   stay the PC's: 128x234 page, 230 glyphs. `CdLib` reads the size from its
   own table.

Choices without PS2 evidence:

- The PS2 colour lookup table (CLUT) that turned coverage into colour and
  alpha is made at run time and is not in the ELF. Coverage maps linearly
  to alpha on a white face.
- The PC's letter space (0.7 PS2 px) stays.

## Checked in the game

`tools/shotrun`, max graphics, English (the harness language). For each
replay there is an HD run and a run with only the font files (and for the
damage numbers the battle atlas) as the mod. The screenshots are 1280x720.
The crops are in `~/ffx-work/demaster/research/ui/font-*.png`, HD above
and ours below:

| Replay | What | Result | Crop |
|---|---|---|---|
| `draw-ui-dialogue.txt` (ffx-rust; captures removed) | New Game's "Select a Sphere Grid" event window and the title window above it | PS2 glyphs and widths; the windows shrink to the PS2 text width | `font-dialogue-title.png`, `font-dialogue-choice.png` |
| `menu-browse.txt` | main menu | help line, names, HP/MP, menu items, gil, location in the PS2 font, right-aligned numbers in place | `font-menu-main.png` |
| `tools/replays/ui-status.txt` | Status page | PS2 font, nothing clipped or overlapping | `font-menu-status.png` |
| `battle-attack.txt` | battle help line, command window, party HP/MP | PS2 font | `font-battle.png` |
| `battle-attack.txt` with shots every 10 frames after the attack (scratch copy) | the damage number "57" | PS2 serif digits | `font-damage.png` |

The first in-game test wrote the pages at PS2 size (128x234). A `g` descender
showed as a dash over the `y` of "System" (the bilinear bleed above). The
4x pages fix it (`font-dialogue-title.png`).

Not checked in the game: German umlauts and the French, Spanish and
Italian accents. The harness runs in English. The five languages share the
pages and the table, and the sample preview (`font-sample-text.png`:
`Grüße`, `Ärger`, `À bientôt`, `¿Qué`, `Señor`, `Città`, HD above, ours
below) draws them with the game's layout rules. Labels that are atlas art,
not text (`HELP`, `HP`/`MP`, `S.Lv`, `OVERDRIVE`, `Commands`, `AP
Earned`), stay HD with `ui.menus` (`docs/ui.md`).

## Japanese, Chinese, Korean

They keep the HD font. `jppc/menu/base.ftc` has a type-0 bitmap too (a
1008-glyph page on the PC), plus the per-event FTCs on the disc
(`jppc/event/...`, `battle/btl/<b>/<b>.ftc`), but testing them needs a PS2
Japanese disc. The PC's 962 per-event and per-battle font pages (`btl_ch`,
`btl_kr`, `new_btl`, `new_obj`, `obj_ch`, `obj_kr`) belong to these
languages. Chinese and Korean have no PS2 font.
