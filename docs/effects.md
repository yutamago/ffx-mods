# Effect textures (spells, summons, overdrives, particles)

Row 9 of [`differences.md`](differences.md). Converter: `demaster fx`
(`converter/src/ps2fx.rs` reads the PS2 data, `converter/src/fxjob.rs`
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
above 0.95 for 90% of the textures, above 0.8 for 97%), but many HD
textures moved brightness into alpha: white colour with alpha = grey level
times ~0.7 (`mag_0154`, `13984`). The PC blends effects as the PS2 did
(additive `one, one` or `src alpha, one`, modulate `src alpha, inv src
alpha`; ffx-rust `effects-draw.md`), so the PS2 CLUT alpha (`0x80` =
opaque) gives the PS2 result. The converter keeps the PS2 alpha.

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
| converted | 7,327 |
| HD-only (Virtuos additions, above) | 49 |
| `trxtex` frame copies | 1,098 |
| text of another language (kept HD) | 212 |
| failed | 30: all of `magic_0668`, whose HD files are big-endian PS3 clusters (`PHYR...GCM`) the writer can't use as a template |

Correlation with the HD texture: 7,112 of 7,322 measured at 0.8 or more,
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

The runs reach the spell at different updates (the battle start differs
by about 180 updates between runs), so random particles land elsewhere;
compare looks, not pixels. Energy Blast at the same point of the effect
shows the alpha difference above: the HD trails saturate to white, the PS2
ones keep the effect's purple vertex colour. Worth checking against PS2
footage.

## Open

- `magic_0668`: needs a D3D11 template (another texture's cluster with the
  path changed).
- The PS2 images that no HD texture is named after (most are partial
  uploads whose `TEX0` image went in instead).
- In-game check of field-map particles (only the Thunder Plains battle
  map was looked at); Thunder and Cure only as previews.
