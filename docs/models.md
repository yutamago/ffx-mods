# PS2 meshes in the remodelled characters (milestone 2, rows 1-4)

Build jobs `models.party`, `models.npc`, `models.monster`, `models.aeon`,
`models.weapon`, `models.object` (settings of the same names). The `chr.*`
jobs keep doing the texture swap of the models whose mesh was kept. Per
remodelled character the jobs write, under
`<mod>/efl/x/ffx_data/gamedata/ps3data/chr/<kind>/<name>/`:

- `mdl/d3d11/<name>.dae.phyre`: the HD model with the PS2 mesh in it;
- `tex/d3d11/p<name[1..]>.dds.phyre` (`c001` → `p001`): the PS2 textures
  as one ARGB8 atlas at PS2 size, a new texture (see "Textures");
- `<name>.ahwin32`: the asset list with that texture added.

For tests, `demaster models <iso> <FFX_Data.vbf> <out> all|<kind/name>...`
writes the same files plus `<out>/preview/<name>-atlas.png` and the
generated `<name>.glb`. `demaster models-debug <chr file> <vbf>
<kind/name>` prints a PS2 mesh against its HD model.

Code: `converter/src/ps2mesh.rs` (PS2 mesh reader), `models.rs`
(conversion and jobs), `glb.rs` (glb for `ffx_phyre::import_into_cluster`),
`phyretex.rs` (`argb8_as`).

## What gets converted

A character is converted when the texture bake skips at least one of its
HD pages *and* under 60 % of its HD vertices sit on a PS2 vertex
(`bake::find_frame`; `models::remodelled`). Against the survey's classes
(`scratch/chr-classes.txt`): all 147 class E (remodelled), the 18 class D
(edited) models with the lowest share, and one to three class B monsters
near the line (m114 at 0.51; `find_frame` samples, so the B ones vary
between runs). The other 20 class D models (60-100 % PS2 vertices) keep
the HD mesh with PS2 textures from `chr`.

Build of 2026-11-02 (German PAL disc): 165 converted, 0 failed.

| Kind | Count | Models |
|---|---|---|
| party | 32 | every c0xx, c1xx and c9xx except c902 (same mesh, new UVs) |
| NPCs | 95 | class E list |
| monsters | 16 | m069, m070, m072, m073, m075, m076, m112, m114, m124, m141, m190, m213, m222, m248-m250 |
| aeons | 2 | s015, s031 |
| weapons | 5 | w031, w041, w080, w082, w083 |
| objects | 15 | f006, f007, f093-f096, f110-f114, f116-f118, f168 |

## The PS2 mesh (`.chr` entry 0)

Layout as in ffx-rust `docs/formats/chara.md`, plus what this needed:

- Model entry (40 bytes for version `0x1521`): `u16` base bone `+0`,
  vertex count `+4`, group count `+6`, vertices `+8` (`i16` x, y, z in the
  base bone's frame; PS2 units = HD units × 1000), normals `+0xc` (count
  `+0x16`, `i16` triples, normalise), groups `+0x10`, cloth count `+0x18`,
  cloth table `+0x1c` (`0x24` bytes each: vertex list `+8`, count `+0x1c`).
- Group (12 bytes): flags byte `+0` (bit 0 strips, bit 2 `PRG`), MAT byte
  `+1`, texture-list index `+2`, count `+4`, data `+8`.
- Points are 4 halfwords: vertex, normal, u, v (1/16 texel). A vertex or
  normal with bit 15 set repeats an earlier halfword: vertex
  `p[-(k*4/3)]`, normal `p[-1-(k*4/3)]` with `k = value & 0x7fff` and `p`
  the point's first halfword (`Ch_EffectPacketCreateMain` `0x00842c80`,
  `FUN_00844370`). `ps2chr.rs` (texture bake) masks bit 15 off instead,
  which gives a wrong vertex for those points.
- Strip winding (`FUN_00844370`): point k ≥ 2 makes (k-2, k-1, k) for odd
  k and (k-1, k-2, k) for even k. Triangle groups: 24 bytes, three points.
- Skin entries (header `+8` count, `+0x14` table, 12 bytes): bone, model,
  base bone, kind, list offset. The lists:
  - kind 0 (`sg_calcenvelope_asm` `0x006ee570`): `u16` n, pad, n × (x, y,
    z, vertex) at full weight; then two runs of blocks `count, vertex >>
    8`, count × (x, y, z, `u8` weight/255, `u8` vertex low), each run
    ended by `0xffff`; the first run sets, the second adds.
  - kind ≠ 0 (`sg_calcenvelopeW_asm` `0x006ed9e0`): the same first part,
    then two runs of `u16` n, n × (x, y, z, vertex, weight × 10000).
  - x, y, z are the vertex in that bone's frame. With the bind pose,
    Σ w · bone · p equals base · stored vertex to within 0.01-0.04 units on
    the models checked, so the stored vertices are the bind pose and the
    lists only give bones and weights.
- Bind pose: the bone table (`motion.md`), local `T·Rz·Ry·Rx·S`. It equals
  the HD nodes `SkeletonNode_i` (translation / 1000) for every bone that
  skins. Other HD nodes can differ: c001 bones 103-135 are identity in the
  HD, c101's base bone 150 has a zero matrix (all its vertices collapsed
  to one point when the HD node was used), c041 lacks 30 nodes and differs
  on bones 87-91.

The party has up to 8 bones per vertex, m112 up to 14.

## Conversion

1. Each PS2 vertex goes into the bind pose (PS2 bone matrices, / 1000):
   base bone's world · stored vertex. Bones: the skin lists' bones, summed
   per bone, sorted, the 4 largest kept and renormalised (the HD vertex
   has 4). Most weight dropped on one vertex: 6-15 % on the party. A bone
   the HD mesh cannot skin to (no node, or not among the mesh's
   `m_matrixNames`) hands its weight to its nearest ancestor that it can
   (c101 bones 110, 116; c105 52, 60; c107 158, 165, 173-175).
2. Normals from the PS2 normal array, rotated by the base bone.
3. Winding: the share of triangles whose face normal agrees with their
   vertex normals is compared between the HD segments and the PS2 mesh,
   and the PS2 order is flipped if they disagree (never needed so far).
4. Textures: every texture-list entry used (image + CLUT; c001 has one
   image with two CLUTs, c101 nine entries) is decoded at PS2 size and
   shelf-packed into one atlas, 2 texels of edge repeated around each
   page. UVs map into the atlas (the PS2 UVs stay inside their page, so no
   wrapping is lost). The atlas is a new texture made from the template's
   HD texture (`phyretex::argb8_as`), in the model's own folder (c906 uses
   c106's texture), so that HD segments still drawn (cloth) keep theirs.
   The texture's asset id (`m_id`, `PS3Data/chr/pc/c001/tex/c001.dds`)
   sits in the cluster's array data, so the new id has the same length and
   is patched in place: `p001.dds`, or `p238_ps2x.dds` for `n238_hair.dds`.
   The materials of the written segments point to it
   (`ClusterEdit::set_texture`), and it is added to the `.ahwin32`
   (`ffx_phyre::assets::add_files`).
5. Where the geometry goes:
   - skinned PS2 models (every party model, most others): into the HD's
     skinned instance. The template segment is the largest whose material
     has no `ClothWorld` and no `AnimSampler`, preferring a palette of
     more than one bone (c101's palette-1 segments are hair with its own
     shader: geometry put there was not drawn) and an alpha test.
     Triangles fill the template, then new segments copied from it, each
     palette kept to 64 bones (HD palettes go up to 75).
   - rigid PS2 models (no skin entry): the game draws them with their base
     bone's matrix per model number (`Ch_DrawMainForSituation`), and the HD
     has one unskinned instance per PS2 model (`MeshNode<i>Shape`). They
     go there raw (stored vertex / 1000), without joints (23 objects,
     weapons, monsters and s031).
6. `ffx_phyre::import_into_cluster` writes the glb (one glTF mesh per HD
   instance) into the HD cluster; it grows palettes and adds skeleton
   matrices as needed. Every other HD segment then gets the index list
   `0 0 0` (nothing drawn) but keeps its vertices, since cloth segments
   get vertex data uploaded by index every frame (`scene-chars.md`,
   "Cloth").
7. Vertex colour: the template segment's most common colour (the HD
   remodels use (26, 26, 26) on skin and cloth, 255 elsewhere; the kept
   PS2-mesh models have no colour stream). Not compared in detail.
8. Cloth. The PC runs the PS2 cloth simulation and draws its result
   through the HD cloth segments (materials with `ClothWorld`; the PC's
   `<name>.cdf` lists PS2 cloth triangles by PS2 vertex index with PS2
   UVs). Only skinned, PS2 cloth stands in its bind pose (Rikku's sashes
   pointed up in battle). So a PS2 cloth piece that HD cloth segments
   cover in the bind pose (60 % of its vertices within 0.5 units of
   theirs) is left out, and those HD cloth segments stay drawn with their
   HD texture. Other PS2 cloth pieces (Tidus's apron and shorts, plain
   skinned geometry in the HD) stay as skinned PS2 geometry.

## Checked in the game

Replays through `tools/shotrun` (max graphics settings) with the mod
folder of `demaster build --only chr.*,models.*`, one game at a time;
`FFX_HOOK_LOG_OPENS=1` logs `mods:` for each character's `.dae.phyre`,
`p….dds.phyre` and `.ahwin32`. HD baselines from the same replays without
the mod. Screenshots and crops are in `~/ffx-work/demaster/shots/`, the
comparisons in its `cmp/` folder (game data, not committed):

| Replay | What | Result |
|---|---|---|
| `tools/replays/models-field.txt` (new) | c001 on the salvage ship, idle and running | PS2 mesh and textures, animated, no stray vertices. `cmp/field-1.png`, `cmp/field-34.png` |
| ffx-rust `event-arrival.txt` | c101 close-ups (Besaid arrival), the Aurochs with c005 Wakka, c001 at the end | PS2 face and hair full screen, animated by the event. `cmp/arrival-face.png` (HD left, ours right), `cmp/arrival-sheet.png` |
| ffx-rust `battle-attack.txt` | c101 and Rikku underwater | posed and animated. `cmp/battle-chars.png` |
| `tools/replays/battle-djose.txt` | Tidus, Rikku (c107), Wakka in battle | PS2 models; Rikku's sashes hang (HD cloth). `cmp/djose-chars.png`, `cmp/djose-tidus.png` |
| ffx-rust `devsave-22-luca.txt` | Yuna, Tidus, Lulu in battle | PS2 party, `cmp/luca-party.png`. NPCs n177, n182-n184, n216, n217 and n234 load from the mod in its cut scene, but the replay takes no shot there |

## Gaps

- **Texture animation (eyes, mouths)**: not done. The PS2 base image has
  the eyes open and the mouth closed, so faces look right but do not
  blink or talk. How the HD does it: the game still runs the PS2 texanim
  logic and passes the PS2 patch rectangle (x, y, w, h, source size) to
  `graphicTextureAnimationSetAnimatedMaterialsChar` (`0x00643eb0`, via
  `TexAnim_SetMaterialsChar_0077fbf0` from `chr_texanim_main_sub_new`).
  The shader overlays `AnimSampler` (`<name>_anim_n<k>_p0`, k = PS2 patch
  index: c001 has 23 patches and 23 such textures) on a rectangle given by
  the material's `AnimTranslate/AnimScale_MAYA*` (HD texture space) and
  `_GAME*`. `textureanimationchar.ags.phyre` (`PSpriteAnimationInfoChar`:
  groups G0-G5, frame lists, sub-rectangles `mUStart..mVEnd`) says which
  part of each anim texture is shown. To do it: put the face triangles
  into the anim material's segment, write the PS2 patches as the
  `_anim_n<k>` textures, and set the MAYA rectangle (and the `.ags`
  sub-rectangles) to the PS2 destination in atlas space. That needs the
  shader's formula (`_GAME` against `_MAYA`) and the PS2 entries'
  destination fields (`chr_texanim_set_new`, `0x110`-byte entries)
  decoded first.
- **Cloth**: where the HD has a cloth segment, that piece is drawn with HD
  geometry and HD texture (moved by the PS2 simulation); PS2 cloth pieces
  without an HD counterpart are skinned and do not swing. All-PS2 cloth
  would need the PS2 cloth triangles in an HD cloth segment and a matching
  `.cdf` (format not decoded).
- NPCs and monsters were not looked at close up in a replay.
- **4-bone limit**: up to 15 % of a vertex's weight is dropped
  (renormalised). Not visible in the shots.
- **Lighting**: the HD character shader lights the PS2 mesh; the face
  comes out flatter and darker than on the PS2 (milestone 5).
- c041 (Tidus swimming, events) is converted with the PS2 bind pose, but
  its HD nodes differ from it on bones 87-91; not seen in a replay.
- D-class models above 60 % keep the HD mesh (with PS2 textures where
  `chr` can paint them).

## Changes wanted in ffx-rust

None were needed. Useful later: writing new hierarchy joints
(`m_matrixNames`), so bones like c101's 110/116 could skin directly
instead of through an ancestor; dropping segments outright; changing a
texture's `m_id` to any length.
