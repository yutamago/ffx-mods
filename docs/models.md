# PS2 meshes in the remodelled characters (milestone 2, rows 1-4)

`demaster models <iso> <FFX_Data.vbf> <out> all|<kind/name>...` writes, per
character, `<out>/efl/x/ffx_data/gamedata/ps3data/chr/<kind>/<name>/mdl/d3d11/<name>.dae.phyre`
(the HD model with the PS2 mesh in it) and the HD base texture(s) it uses,
replaced by an ARGB8 atlas of the PS2 textures. `<out>/preview/` gets the
atlas as PNG and the generated glb. Run it after `demaster chr` into the
same folder: for the few characters both touch, the model's texture must
be the atlas.

Code: `converter/src/ps2mesh.rs` (PS2 mesh reader), `models.rs`
(conversion), `glb.rs` (glb for `ffx_phyre::import_into_cluster`).

## What `all` converts

A character is converted when the texture bake (`chr`) skips at least one
of its HD pages *and* under 60 % of its HD vertices sit on a PS2 vertex
(`bake::find_frame`). Against the survey's classes
(`scratch/chr-classes.txt`): all 147 class E (remodelled), the 18 class D
models with the lowest share, and one class B monster (m114, 0.51). The
other 20 class D models (0.6-1.0 of the vertices are PS2) keep the HD mesh
with PS2 textures from `chr`.

Of the 166 selected, 166 convert (list and per-character report:
`demaster models ... all` output). See "Checked in the game" for what was
looked at.

## The PS2 mesh (`.chr` entry 0)

Layout as in ffx-rust `docs/formats/chara.md`, plus what this needed:

- Model entry (40 bytes for version `0x1521`): `u16` base bone `+0`,
  vertex count `+4`, group count `+6`, vertices `+8` (`i16` x, y, z in the
  base bone's frame, PS2 units = HD units × 1000), normals `+0xc` (count
  `+0x16`, `i16` triples, normalise), groups `+0x10`, cloth count `+0x18`,
  cloth table `+0x1c`.
- Group (12 bytes): flags byte `+0` (bit 0 strips, bit 2 `PRG`), MAT byte
  `+1`, texture-list index `+2`, count `+4`, data `+8`.
- Points are 4 halfwords: vertex, normal, u, v (1/16 texel). A vertex or
  normal with bit 15 set repeats an earlier halfword: vertex
  `p[-(k*4/3)]`, normal `p[-1-(k*4/3)]` with `k = value & 0x7fff`, `p` the
  point's first halfword (`Ch_EffectPacketCreateMain` `0x00842c80`,
  `FUN_00844370`). The old reader masked bit 15 off, which gives wrong
  vertices.
- Strip winding (`FUN_00844370`): point k ≥ 2 makes (k-2, k-1, k) for odd
  k and (k-1, k-2, k) for even k. Triangle groups: 24 bytes, three points.
- Skin entries (header `+8` count, `+0x14` table, 12 bytes): bone, model,
  base bone, kind, list offset. The lists:
  - kind 0 (`sg_calcenvelope_asm` `0x006ee570`): `u16` n, pad, n × (x, y,
    z, vertex) full weight; then two runs of blocks `count, vertex >> 8`,
    count × (x, y, z, `u8` weight/255, `u8` vertex low), each run ended by
    `0xffff`; the first run sets, the second adds.
  - kind ≠ 0 (`sg_calcenvelopeW_asm` `0x006ed9e0`): the same first part,
    then two runs of `u16` n, n × (x, y, z, vertex, weight × 10000).
  - x, y, z are the vertex in the bone's frame. With the bind pose, Σ w ·
    bone · p equals base · stored vertex to within 0.01-0.04 units on every
    model checked, so the stored vertices are the bind pose and the lists
    give only the bones and weights.
- Bind pose: the bone table (`motion.md`), local `T·Rz·Ry·Rx·S`. It equals
  the HD nodes `SkeletonNode_i` (translation / 1000) for every bone that
  skins. Other HD nodes can differ: c001 bones 103-135 are identity in the
  HD, c101's base bone 150 has a zero matrix, c041 lacks 30 nodes.

All 857 `.chr` parse; the party has up to 8 bones per vertex, m112 up to 14.

## Conversion

1. Each PS2 vertex goes into the bind pose (PS2 bone matrices, / 1000):
   base bone's world · stored vertex. Bones: the skin lists' bones,
   summed per bone, sorted, the 4 largest kept and renormalised (the HD
   vertex format has 4). Most weight dropped on one vertex: 6-15 % on the
   party. A bone the HD mesh cannot skin to (no node, or not among the
   mesh's `m_matrixNames`) hands its weight to its nearest ancestor that
   it can (c101 bones 110, 116; c105 52, 60).
2. Normals from the PS2 normal array, rotated by the base bone.
3. Winding: the share of triangles whose face normal agrees with the
   vertex normals is compared between the HD segments and the PS2 mesh;
   the PS2 order is flipped if they disagree (so far never needed with the
   PS2 bind pose).
4. Textures: every texture-list entry used (image + CLUT; c001 has one
   image with two CLUTs, c101 nine entries) is decoded at PS2 size and
   shelf-packed into one atlas, 2 texels of edge repeated around each page.
   UVs map into the atlas (`u/16` texels; the PS2 UVs stay inside their
   page, so no wrapping is lost). The atlas is written as ARGB8
   (`phyretex.rs`) over the template segment's base texture: no new
   imports and no `.ahwin32` change.
5. Where geometry goes:
   - skinned PS2 models (every party model, most others): into the HD's
     skinned instance. The template segment is the largest whose material
     has no `ClothWorld` and no `AnimSampler`, preferring a palette of
     more than one bone (c101's palette-1 segments are hair with its own
     shader: geometry put there was invisible) and an alpha test.
     Triangles fill the template, then new segments copied from it, each
     palette kept to 64 bones (HD palettes go up to 75).
   - rigid PS2 models (no skin entry): the game draws them with their base
     bone's matrix per model number (`Ch_DrawMainForSituation`), and the HD
     has one unskinned instance per PS2 model (`MeshNode<i>Shape`). They
     go there, raw (stored vertex / 1000), no joints. 23 objects, weapons,
     monsters and s031 need this.
6. `ffx_phyre::import_into_cluster` writes the glb (one glTF mesh per
   instance) into the HD cluster; it grows palettes and adds skeleton
   matrices as needed. Then every other HD segment gets the index list
   `0 0 0` (nothing drawn) but keeps its vertices: cloth segments get
   vertex data uploaded by index every frame (`scene-chars.md`, "Cloth").
7. Vertex colour: the template segment's most common colour (the HD
   remodels use (26, 26, 26) on skin and cloth, 255 elsewhere; the kept
   PS2-mesh models have no colour stream). Not compared in detail.

## Checked in the game

Replays through `tools/shotrun` (`docs` of ffx-rust `harness.md`), mod
folder `<out>/efl/x`, `FFX_HOOK_LOG_OPENS=1`: the hook log shows `mods:`
for the `.dae.phyre` and the atlas of each converted character loaded.
Screenshots and crops in `~/ffx-work/demaster/shots/` (not in git):

| Replay | What | Result |
|---|---|---|
| `tools/replays/models-field.txt` (new) | c001 on the salvage ship, standing and running | PS2 mesh and textures, animated (idle and run), no stray vertices. `cmp/field-1.png`, `cmp/field-34.png` |
| ffx-rust `event-arrival.txt` | c101 close-ups in the Besaid arrival, c005 Wakka, c001 at the end | PS2 face and hair at full screen, animated by the event. `cmp/arrival-face.png` (HD vs ours), `cmp/arrival-sheet.png` |
| ffx-rust `battle-attack.txt` | c101 and Rikku in the underwater battle | posed and animated. `cmp/battle-chars.png` |

## Gaps

- **Texture animation (eyes, mouths)**: not done. The PS2 base image has
  the eyes open and the mouth closed, so faces are right but do not blink
  or talk. The HD path: the game still runs the PS2 texanim logic and
  passes the PS2 patch rectangle (x, y, w, h, source size) to
  `graphicTextureAnimationSetAnimatedMaterialsChar` (`0x00643eb0`, via
  `TexAnim_SetMaterialsChar_0077fbf0` from `chr_texanim_main_sub_new`);
  the shader overlays `AnimSampler` (`<name>_anim_n<k>_p0`, k = PS2 patch
  index: c001 has 23 of each) over a rectangle given by the material's
  `AnimTranslate/AnimScale_MAYA*` (HD texture space) and `_GAME*`;
  `textureanimationchar.ags.phyre` (`PSpriteAnimationInfoChar`: groups
  G0-G5, frame lists, sub-rectangles `mUStart..mVEnd`) says which part of
  each anim texture is shown. To do it: put the face triangles into the
  anim material's segment, write the PS2 patches as the `_anim_n<k>`
  textures, and set the MAYA rectangle (and the `.ags` sub-rectangles) to
  the PS2 destination in atlas space. Needs the shader's formula
  (`AnimTranslate_GAME` vs `_MAYA`) and the PS2 entry's destination
  fields (`chr_texanim_set_new` entries, `0x110` bytes) decoded first.
- **Cloth**: PS2 cloth vertices (Tidus's belt and chain, c101's four
  cloths) are skinned like the rest and stay still relative to their
  bones. The PC runs the PS2 cloth simulation and uploads the result into
  the HD cloth segments by vertex index; to use it, the PS2 cloth
  triangles would go into an HD cloth segment with vertex i = PS2 cloth
  vertex i.
- **4-bone limit**: up to 15 % of a vertex's weight is dropped
  (renormalised). Not visible in the shots so far.
- **Lighting**: the HD character shader lights the PS2 mesh; faces look
  flatter and darker than the HD (milestone 5).
- c041 (Tidus swimming, events): its HD nodes differ from the PS2 bind
  pose by up to 3.4 units for bones 87-91; converted with the PS2 bind
  pose but not seen in a replay yet.
- D-class models above 60 % keep the HD mesh (with PS2 textures where
  `chr` could paint them).

## Changes wanted in ffx-rust

None were needed. Useful later: writing new hierarchy joints
(`m_matrixNames`), so bones like c101's 110/116 could skin directly
instead of through an ancestor; a way to drop segments outright.
