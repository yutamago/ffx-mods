# Field and battle map textures

How `demaster maps` turns the PS2 map textures into the HD map textures
(differences rows 5 and 8). Code: `converter/src/{gsmem,ps2map,mapjob}.rs`.
Background: [`research/ps2-textures.md`](research/ps2-textures.md) (the
`MAP1` files), [`research/hd-vs-ps2-assets.md`](research/hd-vs-ps2-assets.md)
(the HD maps keep most PS2 geometry and UVs).

```bash
cargo build --release -p demaster
./target/release/demaster maps <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out> all
./target/release/demaster maps <iso> <vbf> <out> bsil00 kami genk16_a   # maps or areas
```

In `demaster build` these are the jobs `maps.field` (`map/`) and
`maps.battle` (`btlmap/`), switched by the settings of the same names. They
write only `<map>/tex/d3d11/` and `<map>/tex/texswap/d3d11/`; the particle
textures in `<map>/fp/tex/` belong to the effects job (`docs/effects.md`).

Output of `demaster maps`: `<out>/efl/x/<HD path>` (ARGB8 at the PS2 size),
`<out>/preview/<map>/[texswap/]<name>.png` (HD scaled down | PS2), and
`<out>/maps.tsv` (one row per HD texture: result, colour distance to the
HD texture, the runner-up's, where the pixels and CLUT came from).

## HD texture names

`ffx_data/gamedata/ps3data/{map,btlmap}/<area>/<map>/tex/d3d11/` holds the
map textures, `.../tex/texswap/d3d11/` the texture-swap frames. Names are
decimal fields:

```
TBP_PSM_X_Y_BPP_W_H[_FRAME].dds.phyre       13184_20_000_000_4_256_256
```

| Field | Meaning |
|---|---|
| `TBP` | GS block address of the page (`13184` = `0x3380`) |
| `PSM` | 19 PSMT8, 20 PSMT4, 27 PSMT8H, 36 PSMT4HH, 44 PSMT4HL |
| `X`, `Y`, `W`, `H` | the region in texels of that page; `W` and `H` are not powers of two (`48x48`, `104x32`) |
| `BPP` | 8 or 4, follows from the PSM |
| `FRAME` | texswap only: the frame number |

The HD art is the region upscaled, mostly 4x (some 2x, 8x or uneven). Other
names (`bsil01_fabric_02`, `reflection3`, `*_colorcorrection`, `*_s`
specular maps, `znkd*` remodelled Zanarkand) are HD-only and stay. Of 16,781
HD map textures, 15,950 carry a region name.

## Replaying the PS2 GS memory

Per map, the PS2 files are `jppc/{map,btlmap}/<area>/<map>/bin/mapout.{txa,vpa}`
(battle maps keep the `_a`/`_b` suffix of the HD folder).

1. **Uploads** (`.txa`, `yiMapFileRead` case 2, in file order): each texture
   record is written into an emulated 4 MiB GS memory at its DBP/DBW.
   PSMT4 records are pre-swizzled and sent as PSMCT32 at half width, quarter
   height and half DBW; PSMT8, PSMT8H, PSMT4HL and PSMT4HH are linear host
   transfers in their own format (the `H` formats write the top byte or
   nibble of the 32-bit word). The five CT16 records are not emulated (no
   HD name refers to them).
2. **CLUTs** (`.txa` kind 3): the first 72 palettes go to
   `yiClutAlloc_makepacket`'s addresses as 16x16 PSMCT32 rectangles
   (`ps2map::clut_cbp`; mode 4 everywhere but five maps with mode 0, whose
   bank is not uploaded and is matched as is).
3. **TEX0** (`.vpa`): every A+D write of `TEX0_1`/`TEX0_2`, **at any 4-byte
   alignment**. VIF codes shift the GIF data by a word, and the prototype's
   8-byte scan missed about one in ten writes; those textures looked
   "unbound" (bsil00 `0x2f80`, `0x3200`, `0x3e00`). With the 4-byte scan
   every uploaded texture an HD name refers to has its TEX0.
4. **Read-back**: the HD region is read texel by texel with the name's PSM at
   TBP and the TEX0's TBW, and coloured through the CLUT the TEX0 names:
   CSM1, so index `i` reads stored entry `i` with bits 3 and 4 swapped; a
   4-bit TEX0 with `CSA n` reads entries `16n..16n+15` of the 256 loaded by
   the 8-bit TEX0 before it. GS alpha `0x80` is 255.

## Which CLUT

A page is usually drawn with several CLUTs (an atlas whose parts use
different CSA slices, or palette variants). The HD names do not say which
one; Virtuos cut the regions per draw, so each region has one. The
converter colours the region with **every CLUT a TEX0 binds to that page**
and keeps the one whose colours are closest to the HD texture (both reduced
to at most 32 texels a side, so the HD repaint's extra detail does not
count; `mapjob::distance`). Only CLUTs the map's own TEX0 writes name are
candidates. Results: 12,740 regions.

Things that did not work: switching CLUTs per 16 or 32 texel tile inside one
region made visible blocks in 30-40% of the mixed textures; dropped.

Only TEX0s of the same PSM as the texture uploaded at that address count: an
8-bit TEX0 on a 4-bit page is the CLUT load before the real 4-bit TEX0.

## Textures FFX moves at run time

Textures at `0x3c00` and above are kept in the scene's VRAM copy and
uploaded at run time (`yiLoadVramImage` on PC), and some HD names point at
the address they ended up at, not the one in the `.txa`. bsil00: the sky is
uploaded at `0x3d00` but named `16128_19` (`0x3f00`, T8, where the `.txa`
has a T4 texture). When nothing is uploaded at a name's address in its
format, or the bound result is far from the HD colours (distance > 30), the
converter tries the same region of every texture of that format, chooses by
the **luminance correlation** with the HD texture (immune to the HD colour
grading; at least 0.6, and 0.15 better than the bound result), then by
colour among its CLUTs, then, if still far off, among all uploaded CLUTs.
Flat textures (sand) have no pattern; for those a colour match under 20
replaces a bound result over 60. These are marked `[unbound]` in
`maps.tsv`: 204 regions.

## Texture-swap animation (`tex/texswap`)

`.vpa` kind-6 records (`yiMapFileRead` case 6) are the frames of a texture
animation: `data +0` the animation slot, header `+0x58` the packet's
quadwords, the packet at `+0x80` (A+D `BITBLTBUF`/`TRXPOS`/`TRXREG`, then a
GIF IMAGE tag and the pixels), header `+0x64` the quadwords of the frame's
own CLUT right after the packet (16 colours for 4-bit frames, in index
order). Frame *n* is the *n*-th record of its slot. The HD frames
`<region>_NNN` are at the PS2 size; each is replaced by PS2 frame `NNN`
with its own CLUT (bsil00: 32 grey water-caustic frames at `0x2f00`,
distance about 2.5). The main texture at a swap target takes frame 0
when that matches the HD texture better than the `.txa` image. 2,822 frames.

## Alpha

When the HD texture is opaque (every alpha >= 250), the PS2 texels are
written opaque. Many PS2 textures carry alpha `0x40`/`0x7f` that the PS2
blend mode ignores, and in the HD material it would make the surface
see-through. Otherwise the PS2 alpha is kept (cut-outs: plants, fences).

## Counts (German PAL disc, all 296 HD maps)

| Result | Textures |
|---|---|
| Converted | 15,830 (13,008 map textures, 2,822 swap frames) |
| of these, bound by the map's TEX0 / swap data | 15,626 |
| of these, unbound (relocated or guessed CLUT) | 204 |
| HD-only (no region name, or no PS2 map: hiku28-30, stbv04) | 871 |
| Not converted, left HD | 80 |

Colour distance of the converted map textures to the HD art: 83% under 20,
96% under 30.

The 80 left HD: 51 regions where no PS2 texture of the format matches the
HD texture (nagi00_a 21, mcyt00_a 12, bsil00's `0x3d80` T8 page, names with
TBP above `0x3fff`), and 29 where the bound PS2 region is far off in colour
and pattern (correlation under 0.5 and distance over 60), so the VRAM there
held something else when the HD names were made (mihn00_b, mihn04_a/b,
lchb13, ...).

## Known gaps

- Regions that span PS2 sub-images drawn with different CSAs get one CLUT
  for the whole region.
- The 80 textures above stay HD.
- The CLUT choice relies on the HD colours being close to the PS2 ones.
  Where the HD repaint changed a colour a lot, a close-but-wrong CSA can
  win (bsil00 `13568_20_000_128`: grey stone vs HD tan; the runner-up is
  within 2). `maps.tsv` lists the runner-up's distance for review.
- CT16 uploads and the 2D background layer (`.vpa` section 0, row 7) are
  not handled.
