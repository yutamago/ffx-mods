# PS2 meshes in the remodelled characters (milestone 2, rows 1-4)

Build jobs `models.party`, `models.npc`, `models.monster`, `models.aeon`,
`models.weapon`, `models.object` (settings of the same names). The `chr.*`
jobs keep doing the texture swap of the models whose mesh was kept. Per
remodelled character the jobs write, under
`<mod>/efl/x/ffx_data/gamedata/ps3data/chr/<kind>/<name>/`:

- `mdl/d3d11/<name>.dae.phyre`: the HD model with the PS2 mesh in it;
- `tex/d3d11/p<name[1..]>.dds.phyre` (`c001` → `p001`): the PS2 textures
  as one ARGB8 atlas at PS2 size, one mip level, a new texture (see
  "Conversion" 4);
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
   HD texture (`phyretex::argb8_one_level`), in the model's own folder
   (c906 uses
   c106's texture), so that HD segments still drawn (cloth) keep theirs.
   The texture's asset id (`m_id`, `PS3Data/chr/pc/c001/tex/c001.dds`)
   sits in the cluster's array data, so the new id has the same length and
   is patched in place: `p001.dds`, or `p238_ps2x.dds` for `n238_hair.dds`.
   The materials of the written segments point to it
   (`ClusterEdit::set_texture`), and it is added to the `.ahwin32`
   (`ffx_phyre::assets::add_files`). The atlas and the face animation
   frames have **one mip level**: the game's texture quality drops the
   top levels of a texture with a chain (two at `TQ_LOW`, which the
   `VQ_LOW` preset sets; ffx-rust `docs/formats/phyre.md`, "How the game
   loads a texture") and never touches a one-level texture. With the mip
   chain (job versions 2-3) the user's game (`VQ_LOW`) drew every PS2
   character at a quarter of the PS2's texels: Tidus's 516x2340 atlas at
   129x585, the face smeared, the hair strands and their alpha averaged
   into flat chunky cards, the chain and earring faded out
   (`/mnt/c/shots/fix-alpha/models/cmp/tidus.png`, "before"; reproduced
   offline by drawing the glb with the atlas shrunk 4x). The PS2 drew
   characters without mip maps too.
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

The PS2 cloth entry (`0x24` bytes) says which of a piece's vertices are
simulated: `u16` free count `+0`, `u16` fixed count `+2`, total `+0x1c`
(= free + fixed in all 355 cloth entries of the disc's `.chr` files); the
vertex list `+8` holds the free vertices, then the fixed ones, each run
sorted. The fixed ones are where the piece hangs from the body: the base
of Wakka's crest (piece (0, 3): 255 free, 50 fixed), the ankle ends of his
shins ((0, 0): 28 free, 4 fixed).

So every PS2 cloth piece (of a skinned model) goes into the segment tagged
with it: the HD's segment of that piece if there is one, else a copy of
the instance's largest HD cloth segment, tagged after the import. The PS2
triangles lie exactly on the simulated ones. A fixed vertex gets the skin
colour; a free one the colour of the nearest free vertex of the HD
segment of its piece (the HD's binding mode), or the HD cloth segments'
most common free colour. Characters without HD cloth segments (no cloth
shader to copy) keep their cloth skinned. 55 characters draw their PS2
cloth this way.

**Pieces the HD has another shape for stay skinned.** When no vertex of
the PS2 piece lies within 0.1 units of the HD's segment of that piece
(`models::SHARED`), every vertex gets the skin colour: the PC's
simulation of the piece is set up for the HD's shape. Wakka's PS2 piece
(0, 0) is both shins (the bare lower legs between trousers and sandals),
the HD's is two small flaps at the ankles (0.24 apart at the nearest);
simulated, his left shin folded into a thin sliver in the intro (a gap
between knee and foot), with either colour rule; skinned it is whole,
as on the PS2 (`/mnt/c/shots/fix-alpha/models/cmp/party-wakka.png`).
The pieces the HD kept share vertices with it (0.03 apart at most among
the party). Kept skinned (build of 2026-11-07): c004 (0, 4), c005 (0, 0),
c006 (3, 4), c103 (1, 3), c104 (2, 9) and (2, 13), c105 (2, 0), m112
(0, 4), m248 (0, 3), n217 (2, 0), n222 (0, 3).

Before the PS2's fixed list was read, the colours came from the nearest HD
vertex alone, and pieces the HD never drew kept the vertices shared with
non-cloth triangles skinned. The first try kept the old colour (26, 26, 26) on every cloth vertex:
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

## Alpha (see-through texels)

Code: `converter/src/chralpha.rs`; `demaster materials-debug <vbf>
<kind/name>|<.dae.phyre>` prints each segment's texture, material, shader
variant switches and `AlphaThreshold`.

**How the PS2 drew it.** Every character polygon is drawn with alpha
blending on: the GIF tag's `PRIM` has ABE (and context 2) set for every
group (`FUN_008423c0`, the PS2 packet builder kept in FFX.exe: `PRIM` bits
`0x240`, plus TME|FST `0x110` when the group's flag bit 1 says textured).
The blend is `ALPHA_2` = `0x44`, (Cs − Cd) × As + Cd, written as register
`0x43` from the texture-list entry's bytes `+8`, `+9` (`0x44 00` in every
`.chr`; `FUN_008438b0`), with the texture's alpha (TEX0 TCC set by
`Ch_TextureSetRegs`). So a texel with GS alpha under `0x80` is
see-through: hair strands and fur and feather fringes fade out. The alpha
test is `TEST_2` ATE on, ATST `NOTEQUAL`, AREF 0 (a PCSX2 GS register
dump of the boot intro: context 2, ALPHA `0x44`, MODULATE with vertex
alpha 0x80): only texels of alpha 0 are dropped.

Texture alpha is per CLUT: a texture-list entry is an image and a CLUT,
and a model reads one 8-bit image through several CLUTs, each right for
its own part of the image. Tidus's close-up c101 has 9 entries over 2
images and 6 CLUTs: CLUTs 0, 2, 4 and 5 have alpha 0 or 0x80 only, CLUTs 1
and 3 (93 and 54 alpha values) are those of the hair (entry 1, 3,283
triangles) and of the chain and earring (entry 4); outside their parts
their alpha is noise, which no triangle samples.

**How the PC draws it.** A `PMaterial` names an effect variant
(`m_effectVariant`, a user fixup indexing the `PAssetReferenceImport`s:
`PS3Data/Shaders/PhyreChrLitShader.fx#<md5>`); the variant's switches decide
the draw. Checked on the variants' pixel shaders and pass states:

| Switches | Pass | Texture alpha |
|---|---|---|
| neither | `Opaque` | ignored (`AlphaThreshold` unused in 41 of 43 pixel shaders): solid quads |
| `UATFOPQ` | `Opaque` | alpha to coverage (blend state `AlphaToCoverageEnable`) and the alpha test at `AlphaThreshold` (0.5 on most) |
| `ALPHA_ENABLED` | `Transparent` only | blend src alpha / inverse src alpha, depth write on, texels under `AlphaThreshold` dropped (used by all 43 pixel shaders) |

Before this fix, the converted PS2 mesh went into one template material:
an `UATFOPQ` one at 0.5 for most of the party (Tidus's and Kimahri's hair
cut hard), a blended one at threshold 0.8 for Wakka; PS2 cloth now goes
into the HD cloth segments with their HD materials (Kimahri's fur and mane:
`UATFOPQ` and blended at 0.5, and opaque ones that ignore alpha); and on
kept-mesh models the PS2 texels went into HD pages whose material may not
use alpha at all.

**What the converter does.** Two ways, by job:

- **Remodelled models (`models.*`): the blended twin** (`chralpha::Draw::Blend`,
  job version 4). Segments with see-through texels get the
  `ALPHA_ENABLED` twin of their material's variant (same switches,
  `UATFOPQ` off; the lookup below) and `AlphaThreshold` 1/128, the PS2's
  `NOTEQUAL 0` (`chralpha::PS2_ALPHA_TEST`); HD materials that blend
  already (the cloth ones) keep their variant with that threshold. Drawn in
  the `Transparent` pass, after the opaque body, depth tested and written,
  in the PS2 strip order within the segment: what the PS2 did.
  **Why not the `UATFOPQ` twin any more:** alpha to coverage needs MSAA,
  and the `VQ_LOW` preset (the user's own `GameSetting.ini`, and the
  Fahrenheit test install, which reads it) has `MSAA=0`; then `UATFOPQ` is
  a plain alpha test at its threshold. The copied template materials had
  threshold 0 (`current.min(0.1)` of an opaque material), so every texel
  passed and Tidus's and Rikku's hair were solid cards with hard
  straight edges (`/mnt/c/shots/fix-alpha/models/cmp/tidus.png`). With the
  blended twin and one-level textures the close-up has the PS2's fine
  strands, the chain and the earring (same image, "after"). The battle
  problem that kept the blended twin out before (parts over the body not
  drawn) was not reproduced with the converter of 2026-11-04 (below);
  battles were not run again with this build.
- **Kept-mesh models (`chr.*`): the `UATFOPQ` twin** (`Draw::Coverage`),
  as described next; unchanged, so at `VQ_LOW` they show alpha-tested
  (hard) fringes at threshold 0.1.

- **The variant: the `UATFOPQ` twin.** The variant of the same effect with
  the same switches and `UATFOPQ` on (`ALPHA_ENABLED` off), whose own
  material has the same parameter layout (the location tables address the
  buffer by offset); if there is none, without the z pre-pass switches
  (`ADDITIONAL_ZPREPASS`, `BUILT_IN_ZPREPASS`, `ZPP_IGNORE_Z`). With MSAA
  (8x at max settings) alpha to coverage turns texture alpha into partial
  coverage, so fringes fade out in the opaque pass, depth-sorted like the
  rest of the body; without MSAA it is an alpha test at the threshold.
  `AlphaThreshold` becomes at most 0.1. A variant the model did not use
  before is added to the character's `.ahwin32` (the list loads the shader
  variants; `chralpha::list_entries`).
- **Not the blended twin** (kept meshes; the history of the remodelled
  ones too, until job version 4). `ALPHA_ENABLED` twins were tried first. In cut
  scenes they look right (Tidus's hair, Wakka's crest), but in battle the
  blended parts that lie over the character's own body were not drawn:
  Lulu's necklace and the lower half of her collar, Kimahri's leg wraps
  and belt trim, Tidus's wristband (`alpha-blend-battle-lulu.png`:
  unchanged, blended twin, `UATFOPQ` twin). Not caused by the appended
  materials, the shader list, the face animation or the PS2 cloth (each
  ruled out by a build without it); the cause is open
  (`docs/ffx-rust-findings.md`). The `UATFOPQ` twins draw everything.
  **Not reproduced with the current converter** (2026-11-04): the same
  battle and frame (devsave 51, `kimahri.txt` shot `k-09`, the framing of
  `alpha-blend-battle-lulu.png`) with c006 built three ways, the blended
  twin (`DM_BLEND_TWIN=1`), the `UATFOPQ` twin (the default) and
  unchanged (`DM_NO_BLEND=1`), gives three bit-identical frames, necklace
  and collar drawn in all. So the parts that vanished came from what the
  older build put where (it predates the PS2 cloth in the HD cloth
  segments), not from the blended twin as such; that build is gone, so
  the cause stays open. What the variants' pass states say
  (`materials-debug --states`): every pass of the character variants
  (`Opaque`, `Transparent`, `ZPrePass`, `PS2Lighting`, `FakeTransparent`,
  `HSVPass`, ...) tests depth with `LESS_EQUAL` and writes depth;
  `Transparent`, `PS2Lighting`, `FakeTransparent` and `HSVPass` blend
  src alpha / 1 - src alpha; no depth bias, no stencil. The z pre-pass
  (`PhyreScene_DrawZPrePass`) draws only materials with
  `ADDITIONAL_ZPREPASS` (no party material has it). The battle fade
  (`graphicCharacterInstanceSetMeshAlpha`) remaps a material's
  `Transparent` to `FakeTransparent` while its alpha is under 1. None of
  it lets the body behind cover a nearer blended segment; that would take
  a draw against another depth buffer, which was not found. (The
  `DM_BLEND_TWIN` switch of that test is gone: the remodelled models
  blend always, `Draw::Blend`.)
- Remodelled models (`models.*`): each PS2 triangle of the template that
  samples a texel with alpha under 240 (PS2 `0x78`; texel centres inside
  its UV triangle, its centroid texel for tiny ones; not the corner texels,
  which sit on the island's edge) goes into segments of its own with a
  copy of the template material (`ClusterEdit::add_material`) given the
  twin (the blended one since job version 4). A PS2 cloth segment with
  any see-through texel, and the face segment with 1 % or more, get a twin
  copy of their material whole (cloth segments keep their tag; cloth
  variants have twins). A copied material that blends already (Wakka's
  crest, most HD cloth materials) keeps its variant and only gets the
  PS2 threshold; one that alpha tests (`UATFOPQ`: c101's cloth piece
  (2, 3)) gets its blended twin.
  Copies join the material set of the copied segment's mesh, so segments
  of other meshes (monsters with several skinned instances) get copies of
  their own. Rigid objects whose meshes have no material set of their own
  keep their material when other segments share it.
- Kept-mesh models (`chr.*`): an HD segment whose PS2 twins sample 1 % or
  more see-through texels gets its material changed in place: opaque ones
  to the `UATFOPQ` twin, blended HD materials (fur, feathers, flans) keep
  blending with the threshold lowered to 0.1. The job writes the model's
  `.dae.phyre` (and `.ahwin32` when a variant is new) next to the painted
  pages, not for remodelled models (the `models.*` job's).
- **The threshold** (kept meshes; the remodelled ones use the PS2's
  `NOTEQUAL 0`, 1/128, see above). For blended HD materials, 0.03 let the faint fringe texels
  write depth over feathers the same draw puts behind them (pale outlines
  on the Condor's wings); 0.1 keeps the soft fringe with outlines as faint
  as the HD's 0.3.

**Counts** (`demaster build`, 2026-11-03, German PAL disc; two builds of
`chr.*` and `models.*` byte-identical, 5,083 files). Remodelled: 149 of the
165 models have see-through texels, in 335 segments (template parts, PS2
cloth, faces); 31 materials switched to the `UATFOPQ` twin, 242 thresholds
lowered (HD blended or `UATFOPQ` materials at 0.2-0.8), 10 left (6 variants
without a twin, e.g. c002's texture-animation material; 4 rigid objects
whose material is shared). Kept meshes: 398 models get a new `.dae.phyre`,
77 materials switched, 633 thresholds lowered, 12 left without a twin. No
texture changes: the atlases and pages carried the PS2 alpha already.

**Checked in the game** (`tools/shotrun`, max settings incl. 8x MSAA;
crops in `~/ffx-work/demaster/shots/cmp/alpha-*.png`):

- Kimahri (c004), devsave 51 battle (`kimahri.txt` in the cmp folder): his
  mane and shoulder fur (PS2 cloth in the HD cloth segments) and leg wraps
  draw with soft fringes; nothing missing (`alpha-kimahri.png`: unchanged,
  fixed; `alpha-kimahri-vs-hd.png`: HD, unchanged, fixed;
  `alpha-kimahri-mane-blend.png`: unchanged, threshold only, blended twin).
  The battle camera keeps him small at 1280x720; the user's PS2 close-up
  was not reproduced shot for shot.
- Lulu (c006) in the same battle: necklace, collar, wristbands drawn
  (`alpha-blend-battle-lulu.png`: unchanged, blended twin with the parts
  missing, final).
- Tidus (c101) and Wakka (c005), ffx-rust `event-arrival.txt`: hair tips
  soft (`alpha-tidus-hair.png`), Wakka's crest drawn and soft
  (`alpha-wakka-crest.png`: HD, unchanged, fixed).
- Besaid battle (`tools/replays/battle-besaid.txt`): Condor (m028) feather
  fringes soft (`alpha-condor-wing.png`), Water Flan (m016) translucent,
  Dingo (m009) unchanged (`alpha-flan-dingo.png`).

**Boot intro against the PS2** (2026-11-07, job version 4, the campfire
scene with the staff credits that plays after the logo; the Fahrenheit
test install with the user's `VQ_LOW` settings: `TQ_LOW`, no MSAA;
PS2 frames from PCSX2, German PAL disc). PS2 / before / after, the 4:3
centre, in `/mnt/c/shots/fix-alpha/models/cmp/`:

| Image | What | Result |
|---|---|---|
| `tidus.png` | c101 close-up ("CG supervisor") | before: flat chunky hair cards with hard edges, no chain, no earring, smeared face (quarter-size texture, `UATFOPQ` at threshold 0 without MSAA); after: fine strands, the chain with its pendant, the earring, the face's texels, as on the PS2 |
| `party-wakka.png` | Wakka, Rikku, Lulu at the fire | Wakka's left shin was a sliver (simulated piece (0, 0)); whole now. Rikku's hair strands soft, no longer see-through cards |
| `kimahri.png` | Kimahri from behind | mane and fur fringes soft; before, a flat grey card |
| `yuna.png` | Yuna close-up | face, hair and necklace at the PS2's texel size (before: blurred); the blue hair ornament (cloth) hangs further out than on the PS2 |

Key frames of the runs (3440x1440) in `/mnt/c/shots/fix-alpha/models/`:
`e1/s-110.png` (one-level atlas, blended twins), `e2/s-046.png` (plus
PS2 cloth anchors: the shin still a sliver), `e3/s-046.png` (the shin
piece skinned by hand: whole), `e4/` (all of job version 4). Still
different from the PS2: see "Gaps".

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

- **Wakka's crest** (PS2 piece (0, 3), simulated) lies back as a short
  tuft in the intro where the PS2 shows it standing tall (as in the bind
  pose); the same with the colours of the nearest HD vertex (job version
  3) and with the PS2's fixed list. Skinning it would stand it up without
  its sway; not done.
- **Tidus's chain** (c101): where the two strands meet the pendant a small
  dark arrowhead shows on his chest (the strands' dark pointed ends, which
  the bind pose keeps behind the pendant); the PS2 frames show none. Maybe
  the 4-bone limit (up to 15 % of a c101 vertex's weight dropped).
- **Kept meshes** (`chr.*`) still use the `UATFOPQ` twin and HD-template
  mip chains: at `VQ_LOW` their PS2 pages load at a quarter size and
  their fringes are alpha tested. The other texture jobs (maps, effects,
  UI, backgrounds) write mip chains too and are quartered the same way.
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
