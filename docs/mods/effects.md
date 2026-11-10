# Effect textures (spells, summons, overdrives, particles)

Row 9 of [`differences.md`](../differences.md). Converter: `demaster fx`
(`demaster/src/formats/ps2fx.rs` reads the PS2 data, `demaster/src/jobs/effects.rs`
matches it to the HD textures).

```bash
./target/release/demaster fx <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out> all
./target/release/demaster fx <iso> <vbf> <out> 76 150 map/bika/bika00 btlmap/bika/bika00_a chr/mon/m028
```

It writes `<out>/efl/x/<HD path>`, a side-by-side preview per texture
(`<out>/preview/<source>/<folder>_<name>.png`, HD left, PS2 right, both over
black) and `<out>/fx-report.tsv` (sizes, correlation with the HD texture,
alpha difference, orientation). `DM_DEBUG=1` also writes the raw PS2 images
(`*.ps2.png`) and lists the PS2 images no HD texture is named after.

## Sources

The PC runs the effects from the PS2 effect code (compiled into
`magicFiles/FFX/magic_NNNN.dll` and FFX.exe), with the same data layout,
but loads each texture as a named Phyre file instead of uploading pixels
to GS memory. There are no HD effect meshes, so a texture swap restores
the PS2 look.

| PS2 file | HD folder | What |
|---|---|---|
| `_host0/magic/mag_NNNN.bin` (584) | `magic/magic_NNNN/tex*/d3d11/` (578) | spells, summons, overdrives, enemy skills |
| `jppc/{map,btlmap}/<area>/<map>/bin/mapout.txa` | `{map,btlmap}/<area>/<map>/fp/tex/d3d11/` | particle effects of field and battle maps (rain, sparks, fog) |
| `jppc/chr/mon/<m>/mdl/<m>.chr` | `chr/mon/<m>/fp/tex/d3d11/` (37) | monster particle effects |

On the disc the effect files have no `host0:/ffx/master/` path:
`cdrom.fnd` names them `host0:magic/mag_NNNN.bin`. `disc.rs` names them
`_host0/magic/mag_NNNN.bin`, as the extractor (`tools/ps2x`) does.
PS2-only: `mag_0000`, `0044`, `0052`, `0064`, `0309`, `0323`, `0507`.
HD-only folders: `magic_0687`, `map/dream`, `map/hiku/hiku28..30`,
`map/stbv/stbv04`.

A `mag_NNNN.bin` is a MIPS overlay linked at `0x01b00000` (its first words
are code pointers, `0x01b03150`, ...) followed by its data. The PC has
these files nowhere.

## The oef2 block

The effect code's texture functions all take an "oef2": FFX.exe
`op_oef2_a_get` (`0x007e3810`), `op_oef2_set` (`0x007e3b70`),
`op_oef2_address_change` (`0x007e3910`), `op_oef2_texture_load`
(`0x007e5030`), `op_oef2_tex_clut_cue_multi` (`0x007e4830`). An oef2 has two
parts with the same header layout: the **table** (what to upload where)
and the **data** block (where the bytes are). `op_oef2_set(table, data)`
writes the data block's pointers into the table.

### Header (both parts, 16-byte aligned)

| Off | Type | Meaning |
|---|---|---|
| `+0x04` .. `+0x24` | `u32` | offsets (from the header) of the sub-tables, one per kind; `op_oef2_a_get(oef, kind, i)` |
| `+0x08` | `u32` | kind 2: textures. Table: 32-byte entries. Data: one `u32` offset per texture |
| `+0x0c` | `u32` | kind 3: CLUTs. Table: 16-byte entries. Data: one `u32` offset per CLUT |
| `+0x18`, `+0x20` | `u32` | kind 6 (16-byte entries) and kind 8 (particle groups) |
| `+0x3c` | `u32` | table only: offset of its own data block |
| `+0x40` | `u16` | 0 in the file; `op_oef2_set` sets it to 1 when it has relocated the data block |
| `+0x42` .. `+0x52` | `u16` | entry counts per kind; `+0x44` textures, `+0x46` CLUTs |

### Texture entry (32 bytes, table)

| Off | Type | Meaning | Evidence |
|---|---|---|---|
| `+0x00` | `u64` | `TEX0` the effect draws with (TBP, TBW, PSM, TW, TH, CBP, ...). The low 14 bits are also the upload's DBP (`*p & 0x3fff`) | `op_oef2_texture_load`; `op_oef2_tex_pos_set` (`0x007e4ff0`) rewrites them |
| `+0x08`, `+0x09` | `u8` | upload x / 16, y / 16 | `sceGsSetDefLoadImage` arguments |
| `+0x0a` | `u8` | buffer width: DBW = `byte * 16 >> 6` | same |
| `+0x0c`, `+0x0e` | `u16` | upload width, height (may be smaller than `TEX0`'s `1 << TW`, `1 << TH`) | same |
| `+0x10` | `u32` | byte size (`w * h * bpp / 8`); 0 = unused entry | `op_oef2_tex_clut_cue_loop` sums it |
| `+0x14` | `u32` | 0 in the file; the pixel pointer after `op_oef2_set` | `op_oef2_set` writes table `+8` entries `+0x14` |
| `+0x18` | `u8` | upload PSM (always equal to `TEX0`'s) | `sceGsSetDefLoadImage` |
| `+0x1f` | `u8` | set by `op_oef2_tex_pos_set` | |

All 32 bytes are zero for an unused entry. PSMs seen: PSMT8 (`0x13`) in
nearly all, PSMCT32 (`0x00`) in 112, PSMT4 (`0x14`) in 21.

### CLUT entry (16 bytes, table)

`u16` CBP, `u16` byte size (`0x400` = 256 colours, `0x40` = 16 colours), `u32`
pointer (0 in the file), `u32 0x100`, `u32 0`. Every texture's `TEX0` CBP has
its CLUT in the same table (7,574 checked), CSA is always 0.

### Pixels and palettes

Linear, as uploaded (`sceGsSetDefLoadImage(dbp, dbw, psm, x, y, w, h)`;
`op_oef2_texture_load` splits uploads over 256 KB into 256-row strips with
the same layout). PSMT4 is 2 texels a byte, the low nibble first. A
256-colour CLUT goes up as a 16x16 PSMCT32 rectangle, so entry *i* of the
palette is stored at `(i & 0xe7) | (i & 8) << 1 | (i & 0x10) >> 1` (CSM1, as
in the characters, `research/ps2-textures.md`); 16 colours go up as 8x2,
which is linear. Alpha is the GS's: `0x80` = opaque.

To read a texture the converter writes the uploads into an emulated GS
memory (the swizzle tables of `tools/py/gsmem.py`, ported) and reads the
`TEX0` rectangle back. This matters where upload and `TEX0` differ: an
upload of 64x64 with DBW 1 read by a 128x64 `TEX0` with TBW 2
(`mag_0007`, `13984`), or two 64-wide halves at x 0 and 64, each with its
own CLUT (`mag_0154`, `13696`). The memory remembers which upload wrote each
byte, and each texel goes through that upload's CLUT.

### Finding the oef2s

The files have no index of their oef2s (the overlay code holds their
addresses: `lui`/`addiu` pairs, or a pointer such as `mag_0007 +0x4d8` =
`0x01b0fcc0`). The converter finds them by structure:

- a **table** is a 16-byte aligned header whose texture count is 1..512,
  whose texture offset is 16-aligned and in the file, and whose entries
  are each either all zero or a consistent upload (PSMs equal and known,
  DBP = TBP, 1 <= w, h <= 1024, TW, TH <= 10, size = `w * h * bpp / 8`);
- a **data** block has `+0x40` = 0, a texture count of 1..512, and
  ascending in-file offsets;
- a table takes its own data block (`+0x3c`) if that one holds its
  textures, otherwise every data block with the same texture count whose
  gaps fit the sizes. (`mag_0154`'s table points at a data block with no
  textures; the pixels are in another one. In `mag_0168` the table lists 11
  CLUTs and the data block 12.)

On the German disc this finds tables in 579 of the 584 `mag` files, in the
map `txa` files with particles and in the monster archives.

`mag_0007` worked example: table at `0xfcc0` (`+8` = `0x60` -> entries at
`0xfd20`, `+0xc` = `0x1a0` -> CLUTs at `0xfe60`, `+0x3c` = `0x260`, 10 textures,
10 CLUTs). Data block at `0xff20` (`+8` = `0x20ad0` -> offsets `0x20b00`,
`0x30b00`, ...: the first texture's pixels at `0xff20 + 0x20b00 = 0x30a20`,
the last ends at `0x85a20`, the end of the file). Texture 0: `TEX0`
`0x4007700621313400` = TBP `0x3400` (13312), TBW 4, PSMT8, 256x256, CBP
`0x3b80` -> HD `magic_0007/tex/d3d11/13312_19_0_0_256_256.dds.phyre`.

## HD names

| Where | Name | Example |
|---|---|---|
| `magic/*/tex/` | `TEX0`: `TBP_PSM_0_0_(1<<TW)_(1<<TH)` (`"%d_%d_0_0_%d_%d.dds.phyre"`, ffx-rust `vfx.md`; parsed back by `FUN_00656ae0`) | `13312_19_0_0_256_256` |
| `{map,btlmap}/*/fp/tex/` | the upload rectangle: `TBP_PSM_X_Y_W_H` (and `TEX0` names) | `13888_19_64_0_64_64` |
| `chr/mon/*/fp/tex/` | the upload rectangle with bits per texel: `TBP_X_Y_BPP_W_H`; the file holds the whole `TEX0` area | `16128_64_0_8_64_128` |
| a later upload to the same place | `_2`, `_3`, ... (or `_0`, `_1`, ...) after the name | `13440_19_0_0_256_256_2` |
| a second oef2 table in the file | `tex1/` with the plain name, and in `tex/` with the table index in the fourth field | `tex1/13632_19_0_0_256_256`, `tex/13632_19_0_1_256_256` |
| text (overdrive names, "Magic") | `tex_de/`, `tex_fr/`, `tex_it/`, `tex_es/`, `tex_us/` next to `tex/`; `tex_cn/` copies every texture | `magic_0007/tex_de/14528_19_0_0_256_128` |
| copies of the frame | `trxtex_N_W_H` (the PC skips them: `FUN_00656ae0` maps names with `trxtex` to -1) | |

Virtuos also added names with no PS2 upload (`_n`, `99999_...`, `30000_...`,
`13601_...`, 1x1 `00001_19_0_0_1_1`, `logo_hdremaster`, `_jp`): those stay HD.

### Matching

Because the suffixes do not say which PS2 upload they are, the converter
groups the HD textures of a folder by their base name (the first six
fields) and offers every PS2 image whose names share it: by `TEX0` (each
later upload that lands on an earlier one is a new variant), by upload
rectangle (in both forms), and by table. Each candidate is scored by the
correlation of its colour x alpha with the HD texture box-filtered down to
the PS2 size (needs a whole-number, equal scale on both axes), and the
group is assigned greedily, one image per HD texture; the image with the
exact name gets +0.15. Leftover HD names reuse the best image if it
correlates above 0.5, else stay HD.

**Orientation.** Many map-particle textures were stored upside down in the
HD version (755 of 1,598 map and 119 of 189 battle-map textures), a few
magic ones upside down (7) or mirrored (3). The PC draws the HD textures
as they are, so the converter turns the PS2 image the same way when the
turned image correlates above 0.5 and 0.2 better than the upright one.
Without this, the particles would draw upside down in the game.

### Alpha and brightness

The colours of the PS2 CLUTs and the HD textures agree (the correlation is
above 0.95 for 90% of the textures, above 0.8 for 97%), and so does colour
x alpha: a least-squares fit of the HD's colour x alpha against the PS2's
gives a factor of 0.7 to 1.2 for almost every texture. What differs is how
the product is split. Virtuos stored most glows, trails and rings (all
greyscale on the PS2) as **white with the brightness in alpha** (alpha =
0.7 to 1.0 x the PS2 grey level, `mag_0154` `13984`); the PS2 has the grey
level in the colour and `0x80` (opaque) in alpha.

The split matters because of how each machine clamps. The PC's effect
shaders (`vfxadditiveshader.fx`, translated from its DXBC with ffx-rust's
`ffx-dxbc`):

```
vertex: colour = vertex colour * 2            (PS2 0x80 = 1.0)
pixel:  out.rgb = tex.rgb * colour.rgb        (clamped to 1: 8-bit target)
        out.a   = tex.a * colour.a * fades    (clamped to 1)
blend:  src alpha, one (or one, one; modulate src alpha, inv src alpha)
```

The GS computes `Cf = Ct x Cv` clamped to 1.0 (`0xff`), but
`Af = At x Av` clamped to `0xff` = **2.0**, and the blend `Cs x As` can
double a colour and burn it out to white. On the PC the alpha side stops at
1.0. With the PS2's split (grey colour, alpha 1.0) a draw whose vertex
alpha is above `0x80` loses that headroom and keeps the vertex colour's hue:
Valefor's Energy Blast trails came out purple-blue, where the PS2 (and the
HD) show them white. With the HD's split (colour at full brightness,
brightness in alpha) the alpha side has room for the vertex alpha and the
colour side saturates instead, as on the PS2: the PC result equals the
PS2's whenever the vertex colour is at most `0x80` and colour x alpha stays
below 1.

So the converter writes the PS2 image in the HD's split wherever the HD
texture uses it (at least 90% of its alpha on pixels with every channel
at 235 or more): alpha = alpha x the brightest channel, colour = colour /
the brightest channel. Colour x alpha (what an additive draw adds) stays
the PS2's, the hue stays the PS2's, and greyscale textures become white +
alpha like the HD ones. The same goes for textures the HD keeps in colour but with the
brightness in alpha: fire and torch atlases, coloured glows and sparks
(the HD alpha correlates with the brightest channel at 0.9 or more, mean
alpha at most 160, and the PS2 image is opaque). The intro's campfire is
one: `map/znkd/znkd06` `14336_19_0_0_256_128`, an 8 x 4 flame atlas the HD
stores with the PS2's colours and alpha about 1.46 x the brightest channel.
The PS2 image has alpha `0x80` everywhere, so in the PS2 split the flame
got none of the vertex alpha's headroom on the PC and burnt as a few
embers over the logs (the HD's flame is as small); with the brightness in
alpha it burns bright, as on the PS2 (`fx.textures` version 3). Textures
the HD keeps with real colour and real alpha (smoke, rock, text) may be
drawn with normal alpha blending, where alpha is coverage; they keep the
PS2 split. On the German disc this moves 2,832 of 7,357 textures to white
+ alpha (2,291 magic, 520 field-map, 21 battle-map, no monster ones) and
367 more to colour + brightness in alpha (189 magic, 178 field-map; the
report's `convention` column says `white` or `bright`).

### Inverted textures

`magic_0662` (the opening's logo) `12544_19_0_0_512_512` is the shadow of
the forming logo, in two blends. The PS2's is a light grey logo on an
opaque white field (rows 0..374), a multiply: white leaves the scene as it
is. The HD's is an alpha blend: dark colour with the darkness in alpha
(colour about 255 - 1.2 x alpha, alpha up to 213, 0 off the logo). Written
from the PS2 (correlation -0.17), the PC drew the opaque field as a
lavender-white rectangle over three quarters of the picture while the logo
formed (it faded with the effect). `effects::inverted` recognises such a pair
(negative correlation upright, 0.8 or more with the PS2 image inverted;
0.85 here) and writes the HD's file: the same picture, already in the PC's
blend (job version 4). Of the 7357 converted textures only this one
qualifies (the other two below -0.1, `magic_0105` `13312` and `kami00`
`14080`, are no negatives). Checked in the game: the logo forms over the
scene without a rectangle (`C:\shots\fix-alpha\title\cmp-logo.png`: PS2
x2, before, after x2).

`blank` HD textures (black or transparent everywhere) are written as the
HD's file too: `magic_0662` `12544_19_0_0_256_64` holds "INTERNATIONAL" on
the PS2 and is transparent in the HD; the PAL PS2 does not show it either.

### Languages

Text textures (overdrive names, "Luck", "Magic") exist in `tex/` and in
the European language folders. The converter writes the disc's text only
into the folder of the disc's language (German disc: `tex_de/`; US/UK:
`tex_us/` and `tex/`) and leaves the others HD. (The German disc's `mag_0007`
says "Luck" in English, where the HD `tex_de` texture is empty.)

## Results (German disc, 2026-11-02)

917 sources (577 magic, 260 field maps, 43 battle maps, 37 monsters).

| | Textures |
|---|---|
| converted | 7,357 |
| HD-only (Virtuos additions, above) | 49 |
| `trxtex` frame copies | 1,098 |
| text of another language (kept HD) | 212 |
| failed | 0 |

Correlation with the HD texture: 7,142 of 7,352 measured at 0.8 or more,
57 below 0.5. The low ones are textures Virtuos repainted or redrew (the
German text in `tex_de`, `dome06`'s brick walls, `kami00`'s crystal,
`mag_0105`'s ring, `djyt00`'s caustics) or shifted.

## In the game (2026-11-02)

Replays `tools/replays/fx-fire.txt` (also `fx-thunder.txt`, `fx-cure.txt`:
the item trick of ffx-rust's battle-magic with another effect ID),
`fx-overdrive.txt` (Spiral Cut, Valefor's entry, Energy Blast) and
`fx-ifrit.txt` (Ifrit's entry on the Thunder Plains), each run once without
and once with the converter's output (`tools/shotrun`, `SHOTRUN_STARTUP=600`
on a busy machine). `FFX_HOOK_LOG_OPENS=1` shows `mods:` for `magic_0076`,
`0390`, `0595`, `0596`, `0604` and `btlmap/kami/kami00_a/fp`. Every effect
draws, in the right orientation, with the PS2 textures.

With the brightness moved into alpha (2026-11-02, max graphics settings,
`fx2` shots), Energy Blast's trails burn out white as on the PS2 and in
the HD version (they were purple-blue with the PS2's split), and Fire,
Thunder, Cure, Valefor's entry and Ifrit's entry look like the HD effects
at the same frame, at PS2 resolution. Runs can reach the spell at
different updates (the battle start varies), in which case random
particles land elsewhere.

## The intro's campfire (2026-11-07)

The boot intro (map `0x15d`, the campfire under the staff credits) draws
its flame from `map/znkd/znkd06/fp/tex/d3d11/14336_19_0_0_256_128`.
Under Fahrenheit (max settings) the PS2 split burnt only embers on the
logs, about as small as the HD version's own flame. With the brightness in
alpha (`bright` above) the flame burns bright with a glow, closer to the
PS2 (`ps2b-14..16`), whose orange glow around the flame is still
somewhat larger. Random flicker and the music-timed scene make frames
differ between runs.

## Open

- The PS2 images that no HD texture is named after (most are partial
  uploads whose `TEX0` image went in instead).
- In-game check of field-map particles (only the Thunder Plains battle
  map was looked at).
- Textures the HD keeps in colour and real alpha keep the PS2 split; a draw of one of
  them with vertex alpha above `0x80` still lacks the PS2's headroom (no
  case seen).
