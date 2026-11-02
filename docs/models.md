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
(edited) models with the lowest share, and the class B monster m114
(0.51). m049 (0.74), m130 and m132 (0.98) keep their HD mesh and get the
texture swap. An older build converted them too: `find_frame` settled
ties between offsets by HashMap order, so the choice changed from run to
run (fixed in the config commit; the template colour's tie in `models.rs`
too). Two builds of all `chr.*` and `models.*` jobs now write the same
bytes. The other 20 class D models (60-100 % PS2 vertices) keep the HD
mesh with PS2 textures from `chr`.

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
   - skinned PS2 models (every party model, most others): into the HD
     skinned instance whose skeleton names most of the model's bones (the
     largest on a tie). m070 is three fish, each PS2 body and fin pair in
     an instance of its own; put into the largest one, two of the fish
     were drawn on top of each other. The template segment is the largest whose material
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
   PS2-mesh models have no colour stream). Not compared in detail. Cloth
   vertices: see 8.
8. Cloth: all PS2 cloth, PS2 geometry and texture, moved by the game's
   simulation (below).
9. Texture animation: the face triangles go into the HD's animated
   material (below).

## Cloth

How the PC draws cloth:

- `<name>.cdf` (`Ch_LoadClothFileTask` `0x0082a140`, read by
  `Ch_ClothMeshMapping` `0x00826340`): 6 floats (bounding box, HD
  units), then per PS2 cloth piece a header `u16 model, u16 piece, u16
  count, u16 pad`, then `count` triangles of 32 bytes (`u16 v0, v1, v2`
  PS2 vertex indices of that model, `u16` piece + 100 × k, three `f32`
  (u, v) PS2 UV pairs in 1/16 texel); `ffff ffff` ends it. It lists
  **every** PS2 cloth piece, including those the HD has no segment for
  (c001 model 1 pieces 0, 3, 4: the apron and shorts). A `.cmf` with
  baked bindings is looked for next to it; no character has one.
- The game simulates every piece on the PS2 data (`Sg_ClothStep`).
- A segment draws a piece when `PMeshSegment.m_clothmodelIndex`,
  `m_clothIndex` (`+0x20`, `+0x24`, -1 = none) name it
  (`graphicCharacterInstanceGetClothVertexBuffers` `0x0063a810`). Its
  material has `ClothWorld`; the shader takes the position from vertex
  stream 5, which `batchSetClothVertexBuffers` fills every frame.
- Each vertex of such a segment is bound to the simulated triangle it lies
  in (`Ch_PreCalcActiveClothMesh` `0x00827e90`, `InsideTriangle`
  `0x00828c10`: barycentric coordinates in the bind pose, × 1000 against
  the PS2 model vertices) unless its **vertex colour** keeps it skinned:
  all of r, g, b at most 0.3 (77) means skinned, larger values choose the
  binding mode. The HD cloth segments use (255, 255, 255) or (255, 0, 0)
  on free vertices and (26, 26, 26) on anchored ones.

So every PS2 cloth piece (of a skinned model) goes into the segment tagged
with it: the HD's segment of that piece if there is one, else a copy of
the instance's largest HD cloth segment, tagged after the import. The PS2
triangles lie exactly on the simulated ones. Each vertex gets the colour
of the nearest vertex of the HD segment of its piece; pieces the HD never
drew get the HD cloth segments' most common free colour, and their
vertices shared with non-cloth triangles (where the piece hangs from the
body) keep the skin colour. Characters without HD cloth segments (no cloth
shader to copy) keep their cloth skinned. 55 characters draw their PS2
cloth this way.

The first try kept the old colour (26, 26, 26) on every cloth vertex:
nothing was bound, and Rikku's sashes stood up in their bind pose. The
rule before that left PS2 pieces out where an HD cloth segment lay within
0.5 units of 60 % of their vertices; it matched Wakka's crest to a small
HD cloth segment (median distance 0.24) and hid the crest altogether.

## Texture animation (eyes, mouths)

Code: `converter/src/texanim.rs`; `models.rs` (`plan_face`); `chrjob.rs`
(`anim_frames`, kept meshes). `demaster texanim-debug <chr> <vbf>
<kind/name>` prints a character's PS2 entries against its HD `.ags`.

**PS2** (`.chr` texture block `+0x20`, kind 2, version 1; parsed like
`chr_texanim_set_new` `0x0077f9c0` and `FUN_0077f360`): patch table at
`+4` (12 bytes: `u16` destination image width and height, `u16 w, h`,
`u32` offset of a 32-byte upload header, texels after it, 8-bit through
the image's CLUT); entries at `+8`, `0x110` bytes for each of the 32
types at `+0xc` that is not `0xff`: `u16 x, y` destination `+8`, `u16`
image `+0xc`, then 32 states × (`u32` frame list offset, `i16` count)
from `+0x10`. A frame is `i16` duration, `u8` patch, `u8` random extra.
The type byte is the mode: 0 blinks (a random 60-150 update wait, then
the list), 4 plays the list, 1 a fixed frame, 3 scrolls UVs. States 1, 2
and 21 always run, plus the one the event script sets
(`tex_anim_no_set_new`).

**PC**: the same logic calls
`TextureAnimationManager_setAnimatedMaterialsChar` with group `G<entry>`,
animation `Animation<state>` and the frame index. The record of
`textureanimationchar.ags.phyre` (`PSpriteAnimationInfoChar`) with those
names gives `m_subTextureIDs[frame]`, the texture: the n-th cluster of the
character's asset list, which lists the animation textures first, before
`textureAnimationChar` (`PhyreScene_doOnCharacterAllAssetsLoaded`
`0x0065a760` passes that range). The record's floats `mUStart, mVStart,
mUEnd, mVEnd` are the PS2 `w/W, h/H, x/W, y/H` (W, H the image size). The
PS2 floats the call also gets are not used. The material that shows it is
named `Mat_CTA_G0G1…` (it takes the groups its name lists). It stacks the
groups that were set in layers 1-3 (1-6 with `CHR_TEXTURE_ANIMATION_UV`,
newest on top), and the pixel shader replaces the colour inside each
layer's rectangle: `t = (uv - T) / S`, drawn where `0.001 ≤ t ≤ 0.999`,
`colour = lerp(colour, AnimSampler<n>(t), layer<n>)`. UVs there have v
up from the image's bottom row. Two shader variants:

- `_GAME` (107 of the converted characters: NPCs, monsters): per layer
  `AnimTranslate_GAME<n> = (mUEnd, mVEnd)`, `AnimScale_GAME<n> =
  (mUStart, mVStart)`, from the record;
- `_MAYA` (8: c001-c003, c005-c008, c043): one fixed rectangle per layer in
  the material (`AnimTranslate_MAYA<n>`, `AnimScale_MAYA<n>`), the
  record's floats unused. The HD sets only layer 1 and draws
  every frame at 128x64 over one eye area.

**Converted characters**: the triangles of the animated image's page that
touch a patch rectangle go into the `Mat_CTA` segment (its base texture
becomes the atlas). The `.ags` is rebuilt from the PS2 entries (one record
per group and state). Its rectangles are the PS2 ones placed in the atlas,
in the shader's terms (`texanim::Rect::shader`). The textures are named
`<name>_anim_n<k % 10>_p<k / 10>` (as long as the HD ids, which are
patched in place) and listed before `textureAnimationChar`; the atlas
goes after it. `_GAME`: each texture is a PS2 patch at PS2 size (or the
patch over the page where a group's frames differ in size). `_MAYA`: one
rectangle around all of the character's patches goes into every layer,
and each texture is that part of the page with the frame's patch put in.
Party rectangles nest (eyes inside the face area), so the newest group
covering the whole rectangle loses only what an older group showed
outside its own patch. 110 characters animate. Left static: c107, c041,
n295, n333, n343 and m124 (no `Mat_CTA` material in the HD), m070 and
m072-m076 (per-fish materials and groups the converter does not split),
n236 and two more (the animated material samples the base texture or
nothing), four with no triangle on the rectangles. The PS2 close-up
models (c101-c106, the c1xx and c9xx faces) have no texture animation:
their eyelids and mouths are bones, which the conversion keeps.

**Kept meshes** (`chr.*`): the HD's animation textures were HD art over
the PS2 texels. Each one is now painted like its page: the PS2 image with
the frame's patch put in, through the same triangle matches, cut to the
rectangle the shader shows it in (from the HD `.ags`, or the `_MAYA`
values). The HD `.ags` and list stay. About 2,900 frames
(`m020_anim_n0_p0` … `n7_p0`, `n040_anim_*`: the PS2 eye and mouth at
the HD layout).

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

Second pass (texture animation, cloth, NPCs, monsters), comparisons in
`~/ffx-work/demaster/shots/cmp/anim/` (HD left, ours right):

| Replay | What | Result |
|---|---|---|
| `tools/replays/face-arrival.txt` (new, lockstep) | c101 close-up, shots 4 updates apart; Wakka (c005) calling, 3 apart | close-up blinks and talks with bones: shot 09 eyes shut, 08/10 open, 02 mouth open (`arrival-c101-face.png`). Wakka's PS2 crest is back and sways like the HD's; his texture mouth frames show (`wakka-16` mouth open, `arrival-wakka.png`) |
| `tools/replays/models-field.txt` | c001 running | PS2 apron and shorts in cloth segments, hanging between the legs, nothing stray (`field-tidus-run.png`). The swing itself is not isolated in two frames |
| `tools/replays/battle-djose.txt` | Rikku (c007) | PS2 sashes with PS2 texture hanging and swinging like the HD's (`djose-rikku.png`) |
| `tools/replays/npc-luca.txt` (new) | Luca cut scene NPCs, the docks battle | Kilika Beasts (converted NPCs) with PS2 meshes and faces, posed by the event (`luca-npcs.png`, `luca-npc-faces.png`); Yuna's skirt and sleeves (`luca-yuna.png`) |
| `tools/replays/battle-ship.txt` | m070 (three fish, converted) | first in the wrong places (two fish on top of each other, see Conversion 5); fixed, now where the HD has them (`ship-m070.png`) |
| `tools/replays/face-idle.txt` (new) | c001 idle, 50 shots | too far from the camera to see eyes; kept for a closer camera |

## Gaps

- **Texture animation**: no shot shows a texture blink of c001 or c002
  close enough to see (their close-ups use the c1xx models, which blink
  with bones). Wakka's mouth frames are the in-game evidence for the
  converted path. The kept-mesh frames are checked as images only.
- Characters whose animated material is per instance (m070, m072-m076)
  keep static faces; the plan takes one `Mat_CTA` segment per character.
- **Cloth**: a piece the HD had as skinned geometry (Tidus's apron) now
  swings with the PS2 simulation; its anchored vertices are guessed (the
  ones shared with non-cloth triangles), not read from PS2 data.
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
