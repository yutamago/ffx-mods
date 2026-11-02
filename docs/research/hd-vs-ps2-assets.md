# HD Remaster (PC) vs PS2: which assets kept the PS2 mesh/UVs

Question: to restore the PS2 look, for which assets is swapping in the PS2 textures
enough (the HD asset still has the PS2 geometry and UV layout, with upscaled textures),
and for which do we have to convert the PS2 mesh into a new PhyreEngine model (the HD
asset was remodelled)?

Short answer:

| Category | HD assets | Verdict | Evidence |
|---|---|---|---|
| Main cast (`chr/pc`, c0xx field / c1xx battle / c9xx) | 48 models (33 with a PS2 `.chr`) | **Mesh conversion needed** | 26 remodelled outright, 6 heavily edited, 1 (c902) same mesh with a new UV layout. Field models went from about 3,000 to 6,400–10,500 triangles, battle models from about 9,500 to 10,000–19,000. HD atlases are 4–8x the PS2 page, with a new layout. 15 HD-only models (c307, c801–c817) have no PS2 counterpart |
| NPCs (`chr/npc`) | 224 | **Split**: 129 texture swap, 95 mesh conversion | 128 have exactly the PS2 vertices and UVs, with the texture at 2x. 95 (n008, n009, n026, n080–n082, n099/n100, most of n143–n356) are remodelled, at about 2.3x the triangles |
| Monsters (`chr/mon`) | 340 | **Mostly texture swap** (304), 27 mesh conversion | 144 have identical mesh and UVs (texture 2x); 123 have the same mesh with UVs moved by a per-page scale/offset (two PS2 pages packed into one HD atlas, or the unused part of the page cropped); 37 have the same mesh with partly re-laid UVs; 19 edited, 8 remodelled (m112, m124, m141, m213, m222, m248–m250) |
| Summons (`chr/sum`) | 29 | **Mostly texture swap** (25) | 12 identical, 11 per-page scale/offset, 2 partly re-laid; 3 edited (s013, s015, s016), s031 remodelled |
| Weapons (`chr/wep`) | 79 | **Texture swap** (74) | 74 keep the PS2 mesh, UVs equal up to the page normalisation (256x64 → 512x128); w031 edited; w041, w080, w082, w083 remodelled |
| Objects (`chr/obj`) | 110 | **Mostly texture swap** (89) | 89 keep the mesh; 8 edited, 13 remodelled (f006, f093–f095, f110–f118, f168) |
| Skeleton/dummy (`chr/skl`) | 27 | swap (most have no texture) | same mesh |
| Field maps (`map/`) | 240 models | **Texture swap, with crop**; some HD-only geometry | In 40 sampled maps the median share of HD vertices sitting on a PS2 vertex (world space) is 0.92 (range 0.26–1.0); about 90 % of the matched vertices carry the PS2 UV under one scale/offset per HD material. HD texture names are the PS2 GS regions (`TBP_PSM_x_y_?_w_h`), 82 % exactly 4x |
| Battle maps (`btlmap/`) | 56 | **Texture swap, with crop** | 11 sampled: 8 have ≥0.9 of their vertices on PS2 vertices; textures are named like the field maps', mostly 4x |
| Magic/effects (`magic/`, `map/*/fp/`) | 5,792 + 2,879 textures, no models | **Texture swap** | No HD meshes at all: the PS2 effect code draws them. Textures are named by PS2 VRAM address and size; 95 % are exactly 2x |
| Texture-swap animation frames (`tex/texswap`) | 490 | **Swap 1:1** | Kept at the PS2 size (x1) |
| Menus/UI (`menu/`, Tk 2D) | 193 + help pages | Swap probably works for PS2-drawn UI (not verified) | Textures are named `TBP_PSM`, i.e. PS2 VRAM pages; the quads and UVs come from the PS2 code. The Iggy/Flash screens (Esc menu, save/load, settings: `flash/*.swf`) are PC-only and have no PS2 asset |
| Fonts | 962 per-event/battle font pages + `fonts/d3d11/tuffy.fgen.phyre` | **Not a texture swap** | HD text uses new font atlases (`font_0_n`/`shadow_0_n` per event and battle) with their own glyph tables; going back to the PS2 font means rebuilding the atlas and its glyph tables |
| Minimaps (`map/*/2d/`) | 142 models | PC-only | New quads and textures; nothing to swap from the PS2 |

"Texture swap" here means keeping the HD `.dae.phyre` and replacing only its
`.dds.phyre` files (`ffx texture import`, which needs the same format and size). For
monsters, summons, weapons and objects with re-packed UVs, the PS2 page has to be
cropped, scaled and placed where the per-group mapping says (computed below). The
mapping is exact, so this can be scripted.

## Method

Scripts and outputs are in `~/ffx-work/demaster/scratch/` (not a git repo):

- `ps2chr.py`: PS2 `.chr` reader built from the decomp's documented layout
  (`docs/formats/chara.md`, `crates/ffx-chara/src/{mapping,chr,sys}.rs`). It reads:
  - the archive entries; entry 0 is the model header (version, model count, bones at
    `+0xa`, model table at `+0x10`, 24- or 40-byte entries);
  - per model: the vertex count `+4`, group count `+6`, vertex `i16` triples `+8` and
    groups `+0x10`;
  - strip groups as `Ch_EffectPacketCreateMain` walks them (blocks: size `[0]`, point
    offset `[1]`, strip count `[2]`, lengths from `+4`, 8-byte points
    `(vertex, normal, u, v)`) and triangle groups (24 bytes each);
  - the texture data (entry 1): the texture list `+0x14`, image table `+0x1c`, sizes and
    PSM.

  PS2 UVs are in 1/16 texel (`u / 16 / width`). Triangle counts are summed strip lengths
  minus 2, so they include degenerate triangles.
- `ffx model export --no-motions` of all 872 `chr/` models (857 export; the rest have
  no `PMeshSegment`) → `hd/`. `glbstat.py` gives per-primitive vertices, triangles,
  unique positions, UV ranges and embedded texture sizes.
- `compare.py` → `compare.tsv`/`compare.jsonl`: triangle and vertex counts, texture
  sizes, and order-free UV-set overlap.
- `compare2.py` → `compare2.jsonl`, with the classification in `chr-final.json` and
  `chr-classes.txt`:
  - **pos_hd / pos_ps2**: share of HD unique vertex positions (×1000, integer) that are
    PS2 vertices, and the reverse. For monsters, NPCs, weapons and the rest, HD
    position × 1000 equals the PS2 `i16` bit for bit, e.g. m001 −5.472..5.472 against
    −5472..5472. A search over axis swaps and translations covers models stored in
    another frame.
  - **edge_overlap**: overlap of the edge-length histograms (5 mm bins). It does not
    depend on the frame, so it still compares models whose vertices are stored in
    another frame (skinned bind pose, e.g. m046) or remodelled (c001: 0.28).
  - **uv_id**: share of position-matched HD vertices whose UV is the PS2 UV unchanged
    (normalised as `u/4096`, v flipped).
  - **uv_fit**: the same, allowing one power-of-two scale and offset per PS2 strip group.
- Classes (`chr-final.json`):
  - A: same mesh (pos ≥ 0.95 both ways, or edge ≥ 0.8 with the same triangle count)
    and `uv_id` ≥ 0.9.
  - B: same mesh and `uv_fit` ≥ 0.85.
  - C: same mesh, UVs not explained (37 of the 66 have `uv_fit` 0.6–0.85, i.e. mostly
    B with a few groups re-laid; 4 (m046, m047, m301, m330) are stored in another frame
    and their UVs were not tested).
  - D: partly PS2 (pos or edge ≥ 0.6).
  - E: remodelled.
- Maps: `ps2map.py`, `ps2mapgeom.py`, `mapcmp.py`, `mapuv.py`, `mapsurvey.py` →
  `mapsurvey.jsonl`.
  - The PS2 `mapout.vpa` (`MAP1`, section 1 = render file, magic `0x214365`, version
    `0x110`; docs/formats/field.md) has kind-1 records holding VU1 VIF streams. The
    `UNPACK V3-32` elements are positions (float), `V2-16` the UVs (×4096), `V4-8` the
    colours. They pair up per batch (MSCAL to MSCAL); about 20–60 % of the elements
    fall in batches whose counts do not agree and were left out of the UV test.
  - The record's word `+0xc` looks like a vertex count (not used).
  - HD positions are taken in world space (glTF node transforms applied) and
    compared with x and y negated: PS2 = (−x, −y, z) of the HD world, on a 5 cm grid.
- Textures: `ffx texture export` of `chr/`, `menu/`, `battle/`, `event/obj/`,
  `magic/`, `map/{bsil,cdsp,kino}`, `btlmap/{bsil,kino}` → `dds/` (2.4 GB), and
  `ddsnames.py` → `ddsnames.tsv`: HD size vs the PS2 size in the file name.

`~/ffx-work/demaster/research/tex/` did not exist yet, so no output of the texture agent
was used.

## Characters (`chr/`)

### Sample

| Model | PS2 tris / verts | HD tris / unique pos | PS2 pages | HD texture | pos_hd | edge | uv_id / uv_fit | Class |
|---|---|---|---|---|---|---|---|---|
| pc/c001 Tidus (field) | 3012 / 1776 | 9862 / 4585 | 256² | 1024² | 0 | 0.28 | – | E |
| pc/c005 Wakka | 2948 / 1556 | 7875 / 3704 | 256² | 1024² | 0 | 0.30 | – | E |
| pc/c101 Tidus (battle) | 9749 / 4797 | 18924 / 7715 | 512×256 + 256² | 2048² + 1024×2048 | 0 | 0.46 | – | E |
| pc/c105 | 9177 / 4515 | 12356 / 5516 | 512×256 + 256² | 2048² | 0 | 0.65 | – | D |
| npc/n002 | 1528 / 865 | 1662 / 770 | 256² | 512² | 1.0 | 0.91 | 0.99 / 0.99 | A |
| npc/n008 | 1688 / 823 | 2694 / 1260 | 256² | 512² | 0.37 | 0.52 | 0.98 (matched part) | E |
| npc/n201 | 1616 / 824 | 2960 / 1482 | 256² | 512² | 0.28 | 0.41 | 0.68 | E |
| mon/m001 | 1516 / 825 | 1516 / 823 | 256² | 512² | 1.0 | 0.91 | 0.97 / 0.97 | A |
| mon/m009 | 1404 / 849 | 1404 / 730 | 256² | 512² | 1.0 | 0.85 | 0.97 | A |
| mon/m045 | 6816 / 3292 | 6818 / 2908 | 512×256 + 256² | 1024² | 1.0 | 0.86 | 0 / 0.90 (scale ½: two pages packed) | B |
| mon/m046 | 9756 / 4898 | 9756 / 4898 | 512×256 + 256² | 1024² | 0 (other frame) | 0.91 | untested | C |
| mon/m112 | 3030 / 1647 | 7670 / 3940 | 256² | 1024² | 0 | 0.28 | – | E |
| sum/s010 | 2216 / 1176 | 2216 / 1134 | 256² | 512² | 1.0 | 0.95 | 0.99 | A |
| sum/s001 | 3208 / 1626 | 3228 / 1530 | 512×256 | 1024×512 | 1.0 | 0.83 | 0 / 0.98 | B |
| wep/w001 | 208 / 114 | 208 / 112 | 256×64 | 512×128 | 1.0 | 0.62 | 0 / 0.80 (v ×4 = page normalisation) | C/B |
| wep/w041 | 290 / 147 | 2760 / 1382 | 128² | 512² | 0 | 0 | – | E |
| obj/f001 | 240 / 146 | 312 / 206 | 128² | 512² | 0.77 | 0.49 | 0.98 | D |
| obj/f008 | 54 / 63 | 54 / 42 | 64² | 128² | 1.0 | 0.60 | 0.93 | B |

(`HD tris` count the triangle lists; HD "verts" are larger because the HD vertices are
split at UV and normal seams, so compare unique positions with the PS2 vertex count.)

### Counts (857 HD models exported, 848 with a PS2 `.chr`)

| Kind | A identical | B scale/offset | C UVs partly re-laid | D edited | E remodelled | other |
|---|---|---|---|---|---|---|
| pc | 0 | 0 | 1 | 6 | 26 | 15 HD-only |
| npc | 128 | 0 | 0 | 1 | 95 | |
| mon | 144 | 123 | 37 | 19 | 8 | 9 stubs (1 triangle) |
| sum | 12 | 11 | 2 | 3 | 1 | |
| wep | 0 | 62 | 12 | 1 | 4 | |
| obj | 15 | 61 | 13 | 8 | 13 | |
| skl | 26 | 0 | 1 | 0 | 0 | |
| **all** | **325** | **257** | **66** | **38** | **147** | 24 |

A+B+C = 648 models keep the PS2 mesh, so they need textures only. D+E = 185 need mesh
conversion. 32 of them are party models: every c0xx and c1xx and all c9xx except
c902, which keeps its mesh but has a new UV layout.
Full name lists per class: `scratch/chr-classes.txt`.

Texture scale, HD page vs the PS2 page (single-page models): A 285× x2, 9× x1, 3× x4;
B 188× x2; E 82× x2, 41× x4. The keepers are almost all plain 2x upscales, while the
remodels moved to 4x or 8x atlases.

### Notes

- PS2 bone *i* = `SkeletonNode_i` (motion.md), and for the kept models the HD vertices
  are the PS2 vertices / 1000 exactly, in the same frame, so there is nothing to
  convert.
- The PS2 texture list often has two entries on one image with two CLUTs (palette
  variants); the HD side has one texture plus `*_anim_*` frames
  (`textureanimationchar.ags.phyre`). A swap has to pick the CLUT 0 variant and
  rebuild the animation frames from the PS2 texture-animation data.
- Monsters whose PS2 pages are 512×256 + 256×256 were packed into one 1024² HD atlas
  (`uv_fit` scale ½ with offsets). The fitted map per strip group (`uv_maps` in
  `compare2.jsonl`) says where each PS2 page sits in the HD atlas.
- HD `.dds.phyre` formats are DXT1/DXT5 (a few ARGB8). `ffx texture import` keeps format
  and size, so PS2 pixels would be upscaled (nearest neighbour keeps the PS2 look) and
  DXT-compressed. A size or format change needs a small extension of
  `crates/ffx-phyre/src/texture.rs` (width/height/mip fields at end − 0x10/0x0c/0x20,
  header word 20).

## Field and battle maps

The HD map folder per map (`map/<area>/<map>/`) holds:

- `mdl/d3d11/<map>.dae.phyre` (the 3D map, e.g. bsil00 8 MB: 73 meshes, 138
  primitives, 34k triangles) plus `textureanimation.ags.phyre`;
- `tex/d3d11/*.dds.phyre` (bsil00: 56), named `TBP_PSM_x_y_?_w_h`: the PS2 GS address,
  PSM (19 = PSMT8, 20 = PSMT4, 36/44 = PSMT4HL/HH), the region's position and size;
- `tex/texswap/` (texture-swap animation frames, PS2 size);
- `fp/tex/` (field particle textures, `TBP_PSM_x_y_w_h`, 2x);
- `2d/` (the PC's minimap quad mesh and its texture);
- `<map>.ahwin32` (the asset list).

Battle maps (`btlmap/<area>/<map>_x/`) have the same layout. Texture census
(`ddsnames.py`, maps sampled):

| Set | Files | HD/PS2 size |
|---|---|---|
| map main textures | 1,600 | 1,249× x4, 113× x2, 37× x1, 18× x8, 65× non-uniform (x4/×5.33…), 68 unnamed |
| map texswap | 368 | all x1 |
| map fp | 354 | 188× x2, 157 unnamed |
| btlmap main | 235 | 176× x4, 17× x2, 7× x8 |
| btlmap texswap | 122 | all x1 |

PS2 vs HD geometry (`mapsurvey.jsonl`, 40 field maps = the first of each area, 11 battle
maps):

- Field maps: median `pos_hd` 0.92. Twelve maps at ≥ 0.95 (zkrn00, ptkl00, bsyt00,
  bsmm00, msmm00, grid00, hiku00, ikai06, azit00, …), most at 0.82–0.95. Low:
  sins02 0.26 (PS2 1,146 unique positions, HD 11,352: remodelled), cdsp00 0.60 (the
  S.S. Winno: part replaced), maca00, ssbt00, mtgz00, slik00 at 0.74–0.77.
- Battle maps: 8 of 11 at ≥ 0.9; maca00_a 0.41, sins02_a 0.53. nagi05_c 0.04 is
  probably a transform the test misses (its UVs fit at 0.96).
- UVs: of the matched vertices, median about 0.9 carry the PS2 UV under one
  power-of-two scale and offset per HD primitive. Usually scale 1, else 2/4/8: the HD
  texture is the PS2 region cropped out of its GS page, as the names say. A few maps
  fit poorly (msmm00 0.39, bvyt09_b 0.45, bsyt00 0.62); probably texture-animated or
  scrolled materials, or UVs the HD artists re-did. 10–35 % of the HD UVs lie outside
  [0, 1] (tiling).

So maps kept the PS2 geometry and UVs, with HD-only additions (larger ground and sky
planes: bsil00's HD extent is ±367 against the PS2's ±312) and a few remodelled areas.
The PS2 look mostly comes back by cutting the PS2 GS regions named by each HD texture
out of the PS2 `mapout.txa`/texture records and upscaling them to the HD size. The
HD-only geometry stays, textured with HD art, unless it is hidden. A full PS2 map
conversion would need the VIF stream decoded properly: strip restarts (ADC), GIF
TEX0, the per-object transforms of the kind-0 records. The decomp does not have that
yet.

## Effects, menus, fonts, events

- **Magic** (`magic/magic_NNNN/tex/d3d11`, 578 folders, 5,792 textures, no models): the
  effects are the PS2 overlays (`docs/formats/magic.md`, `vfx.md`), drawn from PS2 data
  through the GS emulation. Names are `TBP_PSM_x_y_w_h`: 5,482× x2, 139× x1, 60× x4.
  A texture swap restores them fully; the geometry is PS2.
- **Battle `battle/`** (728) and the event folders `event/{new_obj,obj_ch,obj_kr,…}`:
  almost all `font/d3d11/{font,shadow}_0_n` dialogue font pages, one set per battle
  and event. `event/obj` has 19 event object textures named `<addr>_<w>_<h>`.
- **Fonts**: the HD draws text with its own glyph atlases (`graphicFontDraw`,
  `base_ftc/font_0_0…`, `docs/formats/ui-draw.md`). The PS2 fonts are different
  bitmaps with different glyph metrics, so restoring them means regenerating the
  atlases and their glyph tables. `fonts/d3d11/tuffy.fgen.phyre` is the PC UI font
  (Iggy).
- **Menus** (`menu/`: abmap = sphere grid, titl, loading, base_ftc, xfont1208,
  cloudsavetex, …): Tk menu textures are named `TBP_PSM` (PS2 VRAM pages, sizes
  1024×256, 512², 2048×1024…), and their quads and UVs come from the PS2 code. A swap
  is likely enough but was not tested here. The Iggy screens (`flash/*.swf`: Esc
  menu, save/load, settings) have no PS2 counterpart.

## Writing models: what exists in the decomp

`crates/ffx-phyre` already reads and writes model clusters:

- `model.rs`: `Model::write` rebuilds the index and vertex buffers and updates the
  `PDataBlock`/`PIndexDataBlock` fields and header words 18/19, so vertex and triangle
  counts may change; tangents are recomputed.
- `edit.rs`: `ClusterEdit` takes a cluster apart (lists, arrays, decoded fixups,
  buffers) and writes it back with every count recomputed (fixups as explicit mode-5
  records). It has `add_segment` (copies a template segment, palette, data blocks and
  streams, adds `PMeshInstanceSegmentContext`s), `set_palette`,
  `add_skeleton_matrix`, `add_material` (copies `PMaterial` and `PParameterBuffer`),
  `add_import`/`set_texture`. All 1,266 models round-trip, and an added segment was
  checked in game (phyre.md, "New segments and joints").
- `import.rs` + CLI `ffx model import <model> <glb> --mod-dir`: geometry from a glb made
  by `ffx model export`. Primitives tagged `extras.ffx_segment` replace that segment;
  untagged ones become new segments; weights to any `SkeletonNode_i`.
- `assets.rs` updates `<name>.ahwin32` for new textures; `ffx texture import` replaces
  pixels (same format and size); `ffx vbf pack` builds the archive.
- Missing:
  - new hierarchy joints ("New joints in the hierarchy are not written");
  - removing segments (untested; a glb that leaves a segment out keeps it, so an HD
    segment would have to be collapsed to a degenerate triangle or hidden);
  - texture size or format changes;
  - a PS2 `.chr` → glTF converter.

Converting a remodelled character (c001…, the 95 NPCs, 27 monsters) would take:

1. **PS2 mesh → glTF** (new code, extending `scratch/ps2chr.py` or porting to Rust in
   `ffx-chara`):
   - positions (`i16`/1000), normals (`+0xc`, count `+0x16`), strips → triangle lists
     with winding parity, triangle groups with the bit-15 back-references
     (`poly_index`), and UVs;
   - texture pages decoded (PSMT8/PSMT4 + CLUT with the CSM1 entry swizzle) and placed
     into one atlas;
   - skin weights from the model header's skin entries (`Apply_Envelope`: element,
     model, base element, kind, weight lists at `+0x14`/`+0x24`). Unlike the kept
     monsters, the PS2 party vertices are not in the HD bind frame (c001 PS2 y
     −6.9..11.2, HD −18.2..0), so vertices must be posed with the PS2 bind pose (bone
     table, `T·Rz·Ry·Rx·S`) of their base element.
2. **Into the HD cluster**: export the HD model, replace its primitives with the PS2
   ones (tagging them onto existing segments where the material fits, adding segments
   otherwise) with joints named `SkeletonNode_i`, then `ffx model import`. Joints the HD
   mesh has no skeleton matrix for get one automatically. Since the motions are PS2
   (`motion.md`), animation needs no changes.
3. **Textures**: import the PS2 atlas into the HD texture slot (same size: upscale), or
   add a new texture and `.ahwin32` entry (supported).
4. **Problem areas**:
   - Cloth: HD cloth segments (c001 #6, 7, 10–12) get their vertex stream 5 from
     `Ch_Cloth` per vertex index of the PS2 cloth tables (`scene-chars.md`); a PS2
     mesh would have to keep the cloth vertices consistent.
   - Texture animation (eyes/mouth `*_anim_*`, `.ags`) is keyed to HD materials and UVs.
   - The HD character shader and lighting differ from the PS2 look anyway.

## Caveats

- `ps2chr.py` is a reader written for this survey, not decomp code. Triangle groups
  with back-references contribute UVs but not vertex indices. The "group" used for the
  UV fit is the PS2 strip group, not the texture page (the MAT byte was always 0).
- UV tolerances: 1/512 for the UV tests; a 5 cm grid for maps, 1/1000 unit for
  characters.
- The map survey covers 40 of 240 field maps and 11 of 56 battle maps, and only main
  textures were sampled from bsil, cdsp and kino.
- PS2 sources are the German PAL disc's files, extracted under the PC's names
  (`ps2/jppc/`, see `../disc.md`; the folder name comes from the PC's name table). The PC VBF
  ships the same `.chr`/`.vpa` with the texture regions zeroed.
