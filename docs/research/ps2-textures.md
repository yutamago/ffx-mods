# FFX PS2 textures: `.chr` (characters) and `MAP1` `.vpa`/`.txa` (field and battle maps)

Status: decoded and checked by eye. Tools: `~/ffx-work/demaster/tools/ps2tex.py` (decoder),
`gsmem.py` (GS swizzle emulation), `verify_pc_zeroed.py` (PS2 vs PC diff check).
Sample output: `~/ffx-work/demaster/research/tex/{c001,m001,n002,bsil00,btl_bsil03_a}/`.

```
uv run --with pillow python -I tools/ps2tex.py chr  ps2/jppc/chr/pc/c001/mdl/c001.chr research/tex/c001
uv run --with pillow python -I tools/ps2tex.py map  ps2/jppc/map/bsil/bsil00/bin        research/tex/bsil00
uv run --with pillow python -I tools/ps2tex.py info <file>
```

All offsets are little-endian. "FFX.exe" addresses refer to the Ghidra dump in `~/ffx-work/decomp`;
"PS2" addresses refer to `ps2/jppc/prog/SLPM_675.13` (MIPS, disassembled with capstone).

## Common GS facts

| Item | Value | Evidence |
|---|---|---|
| Alpha | GS scale, `0x80` = opaque. Decoder maps `a>=0x80` to 255, else `a*255/128` | CLUT bytes are `..80` / `..7f`; `rcdtTm2ClutAlphaToPs2` scales 255 to 128 |
| Colour order | R, G, B, A bytes (PSMCT32) | the c001 face decodes with correct skin tones |
| 256-colour CLUT | uploaded as a 16x16 PSMCT32 rectangle; TEX0 CPSM=CT32, CSM=CSM1, so texel index `i` reads stored entry `(i & 0xE7) \| (i&8)<<1 \| (i&0x10)>>1` (bits 3 and 4 swapped) | chr: `Ch_TextureSetRegs` (FFX.exe `0x0083d250`) clears CPSM/CSM/CSA (`& 0x801fff9f`) and sets TCC. Maps: `yiClutAlloc_makepacket` TRXREG = 16x16 (`0x00917050`; PS2 `0x0018aba8`, `t2=t3=0x10`). Without the swap the c001 face is scrambled (tested) |
| 16-colour use of a 256 CLUT | a PSMT4 TEX0 with `CSA=n` reads buffer entries `n*16 .. n*16+15`, i.e. stored entries `csm1(n*16+k)` | map TEX0s use CSA 0..15 on the same CBP; colours come out right |
| Swizzle | page/block/column tables as in GSdx/PCSX2 (`gsmem.py`). `columnTable8` checked against the known closed-form T8<->CT32 unswizzle (exact match on random data) | `/tmp/gstest.py` run, plus the images |

## 1. `.chr` (character archive)

The archive header is self-pointer `+0`, entry count `+4`, then (offset, size) pairs from `+0x10`
(see ffx-decomp `docs/formats/chara.md`). **Entry 1 (`u32 @ 0x18`) is the texture block** `T`.
All pointers in it are relative to `T` (they are relocated by `Ch_TexDataMapping`, FFX.exe `0x0083cba0`).

### Texture block header (`T`)

| Off | Type | Meaning | Evidence |
|---|---|---|---|
| `+0x00` | u32 | self pointer (0 in file) | `Ch_TexDataMapping` |
| `+0x04` | u16 | texture-list count | `Ch_TextureReMapping` (`0x0083cdd0`) `*(u16*)(T+4)*0x20` |
| `+0x08` | u16 | CLUT count | `Ch_TexGetNbClut` (`0x0083cd90`) |
| `+0x0c` | u16 | image count | `Ch_TexGetNbImage` (`0x0083cdb0`) |
| `+0x10` | u16 | version (`0x1126` everywhere; textures need `> 0x1028`, `+0x20` needs `> 0x1125`) | `Ch_TexDataMapping` |
| `+0x14` | u32 | texture list offset, 16 B per entry | `Ch_TextureSetRegs` |
| `+0x18` | u32 | CLUT table offset, 8 B per entry | `Ch_TexGetClut` (`0x0083cd20`) |
| `+0x1c` | u32 | image table offset, 16 B per entry | `Ch_TexGetImage` (`0x0083cd50`) |
| `+0x20` | u32 | texture-animation block offset (0 = none) | `Ch_TextureReMapping` -> `+0x6fc`, `chr_texanim_set` (`0x0077c950`) |

**Texture list entry** (16 B; a *material* = image + CLUT pair):

| Off | Type | Meaning |
|---|---|---|
| `+0` | u8 | image index (goes to the runtime record's `+2`, used for TEX0.TBP by `Ch_TextureSetTex0` `0x008313f0`) |
| `+1` | u8 | CLUT index (runtime `+0`, used for TEX0.CBP) |
| `+2` | u16 | PSM (always `0x13` = PSMT8 in all 857 files) -> TEX0 bits 20..25 |
| `+4`,`+6` | u16 | width, height (TW/TH = ceil log2, TBW = w/64) |
| `+8`,`+9` | u8 | copied to the runtime `+0x11`/`+0x12` (`0x44 00` everywhere; meaning unknown) |

**CLUT table entry** (8 B): `u32` (`0x400` in every file; `Ch_TextureSetClut` `0x0083d1e0` reserves
1 GS block if it is `<= 0x10`, else 4), `u32` data offset. The data is preceded by a **32-byte
upload header**: 12 zero bytes, VIF `DIRECT` (`0x50000041`, PC patches it to `0x51` DIRECTHL),
then a GIFtag `NLOOP=0x40, EOP, FLG=IMAGE`. That gives 64 qwords = 256 RGBA entries in CSM1 order.

**Image table entry** (16 B):

| Off | Type | Meaning |
|---|---|---|
| `+0` | u16 | PSM (`0x13`) |
| `+2` | u16 | size in GS blocks (256x256x8 bpp -> `0x100`) -> runtime `+0x36` |
| `+4` | u32 | 0 |
| `+8`,`+0xa` | u16 | width, height |
| `+0xc` | u32 | data offset (relative to `T`); the 32-byte header before it is the same VIF DIRECT + GIF IMAGE tag (`NLOOP = w*h/16`) |

**Pixel data is linear PSMT8** (row-major, 1 byte per texel), not swizzled. The image is uploaded
with a PSMT8 BITBLTBUF (DBW = `w>>6`, `Ch_TextureSetImage` `0x0083d220`) and the GS swizzles it.
A single image is shared by several list entries with different CLUTs: c001 has one 256x256 image
and two CLUTs (face, skin, hair with CLUT 0; clothes and metal with CLUT 1). Each region only looks
right with its own CLUT.

c001 worked example: `T=0x2a180`; list `T+0x24` (2 entries), CLUT table `T+0x44` -> CLUT data
`T+0x80` and `T+0x500`; image table `T+0x900` -> 256x256 data `T+0x980 = 0x2ab00 .. 0x3ab00`;
animation block `T+0x10980 = 0x3ab00`.

### Texture-animation block (`T + [T+0x20]`)

| Off | Meaning | Evidence |
|---|---|---|
| `+0` u8 | kind (1, or 2 = patch animation) | `chr_texanim_set` |
| `+1` u8 | animation count | `chr_texanim_set_new` (`0x0077f9c0`) |
| `+2` u8 | version (0 old, 1 new) | `chr_texanim_set` |
| `+4` u32 | patch table offset (12 B: `u16` destination image width and height, `u16 w, h`, `u32` offset of a 32-byte upload header; the pixels follow the header) | `FUN_0077f360`, `chr_texanim_draw_sub_new` ("tex anim area over" check) |
| `+8` u32 | animation entries, `0x110` B each: `u16 x, y` destination `+8`, `u16` image `+0xc`, then 32 states × (`u32` frame list offset, `i16` count) from `+0x10`; a frame is `i16` duration, `u8` patch, `u8` random extra (`docs/models.md`, "Texture animation") | `chr_texanim_set_new` steps `0x110`; `FUN_0077f360` |
| `+0xc` | 32 bytes: per-animation type (`0xff` = unused) | copied to work `+0xd08` |

Patches are 8-bit sub-images (eye and mouth frames) that `setLoadImageTagsTexAnim` blits into the
main image. They use the main image's CLUT 0.

### Proof that the PC build zeroes exactly the texture pixels

`tools/verify_pc_zeroed.py`: for all **857** `.chr` files present on both sides (all the same size),
**every byte that differs** between the PS2 and the PC copy lies inside an image's pixel data or a
texanim patch's pixel data located with the tables above (857/857, 0 outside). CLUTs and upload
headers are kept on PC. For c001: diff `0x2ab00-0x3ab00` = image 0 exactly, and `0x3b6a0-0x405e0` =
patches 0..22 (patch 0 data `0x3ab00+0xb80+0x20 = 0x3b6a0`; patch 22 ends `0x3ab00+0x5700+0x20+40*24 = 0x405e0`).
Patch headers inside that range are kept. Exceptions where the PC *kept* the pixels: `k999`, `c999`
(test files) and the texanim patches of 17 monsters and NPCs (m003, m030, m105, n238, n356, ...).

Survey of all `.chr` files: PSM is always PSMT8 and CLUTs are always 256 colours. Sizes run from
64x64 to 512x512. All images are 8-bit.

## 2. Field and battle maps: `MAP1` `.vpa` + `.txa`

Both files start with `MAP1`; `u32` section offsets follow from `+0x10` (`yiObjReadCallback` /
`yiMapReadMaster` `0x009093b0`). Section 1 of each file is a **yiArc render file**: magic `0x214365`,
version `0x110`, `+8` size, `+0xc` record count, records from `+0x40`, each one a 0x40-byte header
(`+0` kind, `+4` data length) plus its data (`yiMapFileRead`, FFX.exe `0x0090f900`).

- `mapout.txa` section 1 has **only textures (kind 2) and the CLUT bank (kind 3)**. The PC build
  does not ship `.txa` at all.
- `mapout.vpa` section 1 has objects/geometry (kinds 0, 1, 4, 5, 6, 8). Kind 1 and kind 6 records
  hold the VIF/GS packets, including the **TEX0 registers that bind textures to CLUTs**. The PC
  zeroes most of them (bsil00: 2.0 MB of 2.47 MB of kind 1, 259 KB of kind 6). Section 0 is the 2D
  background (`YNDT`/`YNSC`, `rcbgLoadBin`), and its image is zeroed on PC as well
  (`0x1a1-0x38a00` in bsil00).

### Texture record (kind 2), at record offset `R`

| Off | Meaning |
|---|---|
| `R+0x00` | `2`, `R+4` length (= `0x40 + data`) |
| `R+0x40` | `u32` 1 |
| `R+0x44` | DBP (GS block address, for example `0x2f00`) |
| `R+0x48` | DBW (64-texel units, 4 = 256 wide) |
| `R+0x4c` | PSM: `0x14` T4, `0x13` T8, `0x1b` T8H, `0x2c` T4HL, `0x24` T4HH, `0x02` CT16 (5 maps) |
| `R+0x50`,`R+0x54` | width, height |
| `R+0x78` | copied to the runtime entry `+0x20` (`0x13c7c8`; meaning unknown) |
| `R+0x80` | pixel data, `len-0x40` bytes |

How each one is uploaded (`yiMapFileRead` case 2), and so how it is stored:

| PSM | Upload | File layout | Decode |
|---|---|---|---|
| `0x14` PSMT4 | **as PSMCT32**, width/2, height/4, DBW/2 (`if (psm == 0x14) { h>>=2; w>>=1; dbw>>=1; psm=0 }`) | **pre-swizzled**: linear CT32 words of a (w/2)x(h/4) rectangle | write it as CT32 into emulated GS memory at DBP/DBW/2, read it back as PSMT4 at DBP/DBW |
| `0x13` PSMT8 | as PSMT8 | linear 8 bpp | direct |
| `0x1b`/`0x2c`/`0x24` T8H/T4HL/T4HH | as is | linear 8/4 bpp (low nibble = left texel) | direct |
| `0x02` CT16 | as is | linear RGBA5551 | direct |

With DBP `>= 0x3c00` the PC copies the data into a software VRAM image (`yiLoadVramImage` `0x0090e5d0`;
T8 goes through `yiSwizzleImage` `0x0090e0b0`). The file format is the same, and the same decode
works for these too (checked by eye, for example bsil00 `tex17_3f00`).

Survey of the 295 txa: T4 256x256 1830, T8 256x256 1536, T8H 1014, T4HL 518, T4HH 420, plus a
few 128x128 T4/T8 and 5 CT16 128x64.

### CLUT bank (kind 3)

`R+0x24` = **allocation mode** (2, 3 or 4; 0 in 5 files, where the previous mode stays). Data =
`N x 0x400` bytes of bare 256-entry CT32 palettes (72 in mode 4; 96 when the mode is 0). Each one
is uploaded as a 16x16 CT32 rectangle, DBW 1. The PC build leaves BITBLTBUF/TRXPOS/TRXREG as 0, but
the PS2 `yiClutAlloc_makepacket` (`0x0018aba8`) gives the address:

```
mode 2: base 0xb4000 words, split 32, offsets {0x400,0x500,0x600,0x700}  (table 0x312ea0)
mode 3: base 0x18000 words, split 32, offsets {0x100,0x300,0x500,0x700}  (table 0x312eb0)
mode 4: base 0x82000 words, split 16, offsets {0x100,0x300,0x500,0x700}  (table 0x312eb0)
i <  split: CBP = (base + 0x800*(i/4) + off[i%4]) >> 6
i >= split: CBP = (0xb8000 + 0x100*(i-split)) >> 6
```

bsil00 (mode 4): CBP `0x2084` -> CLUT 0, `0x2094` -> 2, `0x209c` -> 3, `0x20a4` -> 4, ... `0x20cc` -> 9.
All 367 decoded maps resolved with the mode-4 table. In bsil00 only CLUTs 0..15 hold data, and 16..71 are zero.

### Texture-to-CLUT binding (TEX0 in the `.vpa`)

The geometry packets carry A+D `TEX0_2` (reg 7) writes, usually two in a row: first a **PSMT8 TEX0
with CLD set, just to load the 256-entry CLUT**, then the real one, for example `PSMT4, CBP 0x2084, CSA 6`,
followed by `CLAMP_2` (reg 9). bsil00 `0xf3ce0`:
`00 2f 31 21 86 10 04 60 | 07..` (T8, CLD=3) and then `00 2f 41 21 86 10 04 06 | 07..` (T4, CSA 6).
After that comes a GIFtag `NLOOP 2, PRIM 0x2bb` (triangle, Gouraud, **TME, FST=0** -> `ST` float
UVs, context 2), `NREG 9`, regs `ST, RGBAQ, XYZ2` x3.

The decoder scans the `.vpa` for `(value, 6|7)` qwords and keeps those whose TBP **and** PSM match a
texture record. That drops the CLUT-load-only T8 writes. Each `(CBP, CSA)` pair is written as a
separate PNG. One T4 page is an atlas whose regions use different 16-colour slices, so look at the
region that goes with the CSA.

Textures without a matching static TEX0 (about 10%: 1215 of 11881 PNGs across all 367 maps; for
example bsil00 `0x2f80`, `0x3200`, `0x3e00`) are written with a grey ramp (`*_nopal_gray.png`). They
are probably selected at runtime by texture-swap animation (vpa kind 6, "64 texture swaps of 0x720
bytes" in field.md). That is still open.

### 2D background layer (vpa section 0, `YNSC`)

bsil00: layer header `0xd0` (`0x8041`, 512x448). Then `+0x184` BITBLTBUF high word `0x13083c00`
(DBP `0x3c00`, DBW 8, PSMT8), TRXREG 512x448, GIFtag `NLOOP 0x3800` IMAGE, **linear T8** data at
`0x1b0`. Its CLUT follows at `0x381c0` (BITBLTBUF DBP `0x2e00` CT32, TRXREG 16x16, GIF IMAGE 0x40
qwords; data `0x381f0`; a second 0x400 palette follows). In bsil00 it is a grey vignette/fog layer
(alpha `0x4c`). The decoder finds these by scanning for IMAGE GIF tags (`bg_layers`).

## 3. UVs (not decoded)

- Maps: the VU1 microcode outputs perspective `ST` (PRIM `0x2bb`, FST=0). The VIF input UVs are in
  the per-node chains. `field.md` ("poly buffer's packet builders") shows post-VU blocks with three
  `UV` quadwords per triangle, and the minimap rebuild writes `u16` UVs in 1/4096 units. The raw
  unpack format was not checked.
- Characters: `Ch_EffectPacketStorePoly` (`0x00845820`) converts mesh UVs as `v*4096/(size*16)`, so
  stored UVs are probably 12.4 fixed-point texels (`v/16` = texel). The texture-list index per mesh
  block is in the 32-byte block tag (`chara.md`, "Effect packets"). Not checked.

## Output index

`research/tex/c001/c001_t00_img0_clut0.png` (Tidus head and skin), `c001_t01_img0_clut1.png`
(clothes), `c001_anim_p00..22` (eye and mouth patches); `m001/`; `n002/`; `bsil00/` (+ `_manifest.json`
with DBP/PSM/txa offset per texture); `btl_bsil03_a/` (battle map, including T8H/T4HL/T4HH).
