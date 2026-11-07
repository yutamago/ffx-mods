# Render patches (`render.*`)

The code patches behind `render.glow`, `render.lighting`, `render.shadows`
and `render.texture_filter`: hooks in `mod/RenderPatches.cs`, and for
`render.lighting` also patched character shaders (converter job
`render.lighting`, `converter/src/shaderjob.rs`).
Addresses are ffx-rust's `symbols/ffx.sym` (FFX.exe image base
`0x00400000`); the post-process chain is ffx-rust's
`docs/formats/postfx.md`, the shadow solutions `docs/formats/scene-maps.md`
(Q7), the character draws `docs/formats/scene-chars.md`.

Each patch was first tested in the ffx-rust harness with memory writes that
have the same effect (the harness cannot write code pages or run C# hooks).
The mod's hooks were then loaded in the Fahrenheit install: each one logs its
first call (`render.glow: the glow pass is skipped`,
`render.lighting: light levels set to 1,984375`, `render.shadows: shadow solution N
-> 0` for each map), and the title's demo scenes reach all three.

Comparison PNGs (HD left, patched right; never in git) are in
`~/ffx-work/demaster/research/render/`. The replays are in
`~/ffx-work/demaster/scratch/render/` and, for the lighting and the
filtering, `~/ffx-work/demaster/scratch/lighting/` (`gen.py` writes the HD
and PS2-emulation replays, `run.sh` plays them, `vudis.py` disassembles VU
code).

| File | What |
|---|---|
| `lighting/besaid-1-tidus.png` | Tidus on Besaid beach with the PS2 formula (3x zoom) |
| `lighting/djose-1-party.png` | Rikku, Tidus and the Ochu at Djose, the same (2x zoom) |
| `lighting/face-arrival-05.png` | Tidus's close-up model at the Besaid arrival, the same |
| `lighting/spheres-besaid.png` | offline reference: a grey sphere under Besaid's lights, HD / PS2 VU1 / ours |
| `lighting/intro-tidus-nearest-vs-default.png` | intro under Fahrenheit: `texture_filter = "nearest"` left, `"game"` right |
| `lighting/intro-landscape-old-vs-mips.png` | intro: PS2 textures with one level (left) and with mip chains (right) |
| `lighting/besaid-1-all-ps2.png` | Besaid beach: HD, and the PS2 textures with mip chains + the PS2 lighting |
| `glow-besaid.png` | Besaid beach, glow off |
| `glow-fx-fire.png` | Fire in the salvage-ship battle, glow off: the effect is unchanged |
| `lighting-besaid-tidus.png` | Tidus, light levels 1 (3x zoom; the earlier level, half the PS2's) |
| `shadows-besaid-tidus.png` | Tidus, shadow solution 0: the ground shadow the HD map (solution 4) leaves out |
| `shadows-agency-solution0.png` | the agency (a solution-0 map in HD too): the ground shadow as it looks |
| `all-besaid.png`, `all-besaid-tidus.png` | all three together |

## `render.glow = "ps2"`: no glow

The HD glow is `PostProcessManager_render`'s glow
(`Phyre_PPostProcessing_PLegacyGlowGPUBase_render`, `0x00a1de20`). It blurs
the glow map (the scene's colours above `NoGlowThreshold`, from the map's
`GlobalCamera`) at three levels and adds level 0 (`ONE + ONE`, x
`GlowGain`) into the scene colour. That added light is the white sand at
Besaid and the blow-out at Mi'ihen and Bikanel. The PS2 has no such pass.

**Patch:** the hook returns 0 without drawing (thiscall, `ret 0x14`).
Nothing reaches the scene. `prepareGlowMap` still writes target `+0xa4`,
which the brightness pass overwrites in the same frame.

**Not used:** clearing the glow switch byte, post-process `+0x123b3`. In the
harness that gives the same picture (`writep 0x00ccc8a0 0x123b3 00`). But
the game sets it again at every scene start, and
`0x00697f20`/`0x006980a0` only resize the post-process targets while one of
`+0x123b0..+0x123b3` is set.

**What stays:** everything the PS2 effect code draws. Effects are drawn
after the glow (`PhyreScene_DrawMostVFX`), with their own additive blending
and textures: Fire, the save sphere and the other effects are unchanged.
The distortion, motion blur, radial blur (`BlurEffect`), screen fades and
the brightness pass are separate passes. `PhyreScene_isGlowAfterVFX` moves
the glow after the effects for magic `0x2b9` and when scene
`+0xfb60`/`+0xfb74` are above 0. With the patch those effects get no HD
glow on top either. That should be right for the PS2, but it was not checked
on that magic.

UnX / Special K block vertex shader `7e29e6ac` (their CRC32) for the same
purpose. This patch does the same one pass higher up, without touching a
shader.

## `render.shadows = "ps2"`: ground shadows only

`ShadowManager_selectShadowSolution` (`0x006ccd10`) chooses per map:

| Solution | Where | Ground shadow (cluster `+0x28c4`) | Shadow maps |
|---|---|---|---|
| 0 | 248 field maps | under every character | none |
| 1, 3 | 9 maps | yes, drawn after the map | 3: cascades |
| 2 | every battle, 147 maps | yes, except for characters inside a cascade | one cascade |
| 4 | 27 maps (Besaid beach, Mi'ihen, Bikanel, ...) | removed | two cascades |

The ground shadow is a quad under each character (`Ch_DrawShadowQuad`, the
PS2 code's shadow). Its pixel shader (`chry0shadow.fx`) draws black with the
alpha of the character's tile in the 1024 x 1024 shadow atlas
(`ShadowManager_renderCharacterTiles`, `0x006caee0`). The result is a soft
dark shadow at the feet. The PS2 drew a shadow under the characters with
its own code; this is the PC's version of it, and the closest the PC has.

**Patch:** `selectShadowSolution` runs as normal, but the hook returns 0. It
runs so that the level word (`+0x4050`) and the cascade constants are set
as before. `PhyreScene_ForceSetShadowSolution` (`0x006513f0`, called by
`ShadowManager_ResetSolution` for the deck: 0 or 2) always stores 0. Every
map and battle then has the ground shadow and no shadow maps.

**Test:** the harness can't hook. On Besaid beach (solution 4) it spoofs
the level word to the deck's hash (`writep 0x00cdec28 0x4050 3b200000`) and
calls `graphicShadowReset(1)` (`call 0x00643e80 1`). That runs
`ResetSolution(keep)`: cluster into the characters' world, solution 0. Tidus
gets the ground shadow the HD map never draws (`shadows-besaid-tidus.png`).

**Uncertain:**
- In maps that were solution 2-4, `PhyreScene_fieldMapInitialize` does not
  give the map materials their cascade matrices and samplers
  (`LightViewProjection`, `MySMSampler`, `MySMScaleBias`). The map shaders
  that sample them then read whatever is bound (the material's default
  texture, or the shadow manager's 8 x 8 dummy). The runtime test above set
  the solution after the map had loaded, so it doesn't show this.
  Fahrenheit ran with the patch through the title's demo scenes (one of them
  a solution-2 level) without errors. A walk through Besaid and Mi'ihen with
  the mod should still be checked for dark or speckled map surfaces.
- With the game's `Shadow` option 0, `renderCharacterTiles` returns at once
  and the ground shadows show a stale (or empty) atlas. That is the game's
  own behaviour on solution-0 maps.

## `render.lighting = "ps2"`: the PS2 formula

### How the PC lights characters

`phyrechrlitshader.fx`, the pixel shaders that read `Light0`..`Light2`
(1854 containers in 199 of the 220 files, 216 unique per light count):

```
lit = GlobalAmbientColor + CharacterAmbientLight + CharacterShadeColor x CharacterShadeCoeff
    + sum over i of Light_i.m_colorIntensity x (dot(m_direction_i, N) x 0.5 + 0.5)^2
colour = min(texture x COLOR0 x MaterialColour x lit (+ fog), 1), then grey scale
```

There is no normal map, no rim light and no general specular term; the only
specular is the PS2's own `SgSpec_*` spot. The lights are the PS2 code's:
event scripts set them with `Light_Set` (`0x00836520`: colour bytes / 255,
two angles) through `Sg_LightSetDir`/`Sg_LightSetColor` (`0x00836430`,
`0x00836310`), which fill the PS2 VU0 matrices and pass the same floats to
`Phyre_CharacterLighting` (`g_CharacterLighting`, `0x00ccca84`). On the way
the PC adds:

1. **Levels.** `applyIntensityMultiplier` (`0x006a6e90`, thiscall(cl; i,
   flag), `ret 8`) sets light i's level (PLight `+0x24`; `m_colorIntensity`
   is colour x level) to (1 - glow factor) x 1.2 x intensity x 1.16 x 0.95;
   for the ambient x 1.05 instead of x 0.95, level at `+0x1a4`, colour x
   level into the lighting record `+0xac` (`CharacterAmbientLight`). Besaid
   beach: 2.10 for light 0 (map intensity 1.7), 1.31 for lights 1 and 2,
   1.45 for the ambient. Intensities (`+0x1a0` ambient, `+0x1a8 + 0xc i`)
   and rotations come from the HD map's lighting object
   (`PhyreScene_setupCharacterLighting`, `0x0066eb20`), the glow factors
   (`+0x1e0`) from `balanceLumWithGlow`. On znkd08/09/10/14 the colours are
   also saturated x 2.6.
2. **Rotations** of the light directions per map (`setRotation`,
   `0x006aaea0`; Besaid's are 0).
3. **A fill light:** `CharacterShadeColor x CharacterShadeCoeff`, the
   coefficient being the ambient intensity x 0.208125 (`setShadeCoeff`
   `0x006aaef0`, the f64 at `0x00b48300`, read nowhere else), the colour the
   character's PS2 shade colour (`+0x330`, normally 1).
4. **The light value softened:** `Ch_DrawMain` (`0x0082f64c`) puts shade x
   (light value `+0x36c` x 0.85 + 0.15) into `MaterialColour` when
   `g_ChrShadeByLight` (`0x00c496ea`, 1 in the shipped exe) is set. The PS2's
   `Ch_DrawCalcShade` (`0x0082eaf0`) uses the light value as it is. Scripts
   lower it to darken characters in dim places.
5. **The falloff:** half-Lambert squared, lighting sides and backs.

### The PS2 formula

The PS2 lights characters per vertex on VU1. `SLPM_675.13` has its VU code
in `.vutext` (VA `0x00578e00`): two programs loaded by DMA ref tags and VIF
`MPG`. Program 1 (412 instructions) is the character program: FFX.exe still
has the EE side (`FUN_0082e610`, the PS2 character draw, and its packet
builder `FUN_008423c0`), which unpacks the screen, clip, light colour and
normal-light matrices to VU memory qw 0..22 and the material colour to qw
35. The lighting (VU addresses `0xcd`..`0xf4`, disassembled with
`scratch/lighting/vudis.py`):

```
d     = max(0, Lmat x N)                    light matrix x normal, MAXx with vf0.x (= 0)
lit   = Cmat x d + ambient                  colour matrix: the three colours, ambient in the 4th column
vcol  = clamp(lit x vf8, 0, 255)            MAX vf0, MINIi I = 255.0, FTOI0
vf8   = material colour byte x shade x 2    FUN_008437f0: byte +4..6 x g_ChrEffectColor x 2.0
pixel = texture x vcol / 128                GS modulate (0x80 = 1.0), clamped to 255
```

The PS2 player models have one material, colour `7f 7f 7f 80`, so with
shade 1 a lit value of 1 (colours as script bytes / 255) is a vertex colour
of 254 and draws the texture x 254/128. So the PS2's level is **254/128 =
1.98**, not 1: the earlier assumption (level 1) drew characters at half the
PS2's brightness. The shade (`Ch_DrawCalcShade`: shade x light value x
shade colour x shade colour effect) multiplies in through vf8, as the PC's
`MaterialColour` does. The leftover `PS2Lighting` path of the PC (magic
models, `ffxLvMatrix`/`ffxcharlight`) is the same formula with its colour
at x 1/128.

### Patch

- **Mod** (`RenderPatches.lighting`): after `applyIntensityMultiplier`, every
  light and the ambient get level 254/128 (light `+0x24`, record `+0xac` =
  ambient colour x 254/128, luminances `+0x1d0..` to match); `setRotation`
  gets 0, 0; the f64 0.208125 becomes 0 (no fill light); the `fmul 0.85` /
  `fadd 0.15` of `Ch_DrawMain` (12 bytes at `0x0082f652`) become NOPs.
  `patch_code` checks the original bytes (with the relocated addresses) and
  logs a warning instead of patching another build.
- **Converter job `render.lighting`** (`converter/src/shaderjob.rs`,
  `dxbc.rs`): in each lit character pixel shader, every light's
  `mad rA.c, rA.c, l(0.5), l(0.5)` (fed by a `dp3` with `Light_i.m_direction`)
  becomes `mad_sat rA.c, rA.c, l(1.0), l(0.0)` = max(0, N.L) and the
  following square `mul rA.c, rA.c, rA.c` becomes `max rA.c, rA.c, rA.c` (a
  copy; 2-light variants square two components in one `mul`, two shaders use
  the half-Lambert unsquared and get the `mad` alone). Instruction lengths
  stay, so only the DXBC checksum is recomputed (MD5 with Microsoft's
  padding; the test checks it against all 12,600 containers of the 793 shader
  files and that every lit shader takes one patch per light: 1854 shaders,
  3708 lights). 199 files, written to `efl/x/ffx_data/gamedata/ps3data/shaders/d3d11/`.

### Tests

- **Harness** (ffx-rust hook; no C# hooks): the patched shaders as a mod
  folder, and the mod's level change by calls: `graphicCharacterLightingSetLightColor`
  (`call 0x0063cb00 i r g b`) with each colour x 254/128 / its current level,
  read in an HD run of the same replay 2 updates before each shot, then the
  ambient intensity `+0x1a0` = 0 (no fill light). Read back,
  `m_colorIntensity` = colour x 254/128 within 3% (the glow balance moves
  lights 1 and 2 a little). Besaid beach, Djose (battle), the Besaid arrival
  close-up: the lit side keeps about the HD brightness, sides and backs fall
  to the ambient, the face close-up (lit from behind) turns greyer and
  darker (`lighting/*.png`).
- **Offline reference** (`scratch/lighting/spheres.py`): a 0.5-grey sphere
  under Besaid's colours and directions by the HD formula, the PS2 VU1
  formula (vertex clamp, `>> 7` modulate) and ours. Ours equals the PS2's
  (facing light 0: 0.866, away: 0.537, side: 0.369 for both; HD 0.955,
  0.515, 0.61): the HD's brighter sides come from the falloff and the fill
  light.
- **Fahrenheit** (all hooks, the intro): `render.lighting: light levels set
  to 1,984375` logged, no patch warnings, the 199 shader files served
  (`replaces file`), the intro's close-ups lit from one side.

### Uncertain

- No PS2 capture was compared. The formula and the level come from the VU
  code and the EE packet code; the sign of the light directions is the HD's
  (the patch keeps the HD's lit side; the PS2's normal-light matrix builder
  was not found in the PS2 ELF, FFX.exe's port only copies the rows).
- Per pixel instead of per vertex (Gouraud on the PS2): smoother and with
  sharper terminators on coarse meshes.
- The PS2 clamps the vertex colour at 255 (x 1.99) before the texture
  multiply; ours does not, which only shows where texture < 0.5 and lit >
  1.99 (very bright lights).
- Still on: `GlobalAmbientColor` (only magic sets it), the PC fog mix, the
  HD's per-level shade tweaks in `Ch_DrawMain` (levels `0x1ca`, `0x1c0e`,
  `0x2fa`, map `0xc1`), HDAO (a graphics option), and the extra
  `LgtMatVec` light (`setAddLight`, effects; half-Lambert, applied as
  `max(light, 1)`).

## `render.texture_filter = "nearest"`

Unfiltered texels on the 3D scene's models and maps; `"game"` (the default)
installs nothing. The PS2 filtered textures bilinearly, so `"game"` is the
PS2-like choice; `"nearest"` is an option for those who want the pixels.

Where the samplers come from (logged by the hook in the Fahrenheit
install): material samplers are `PSamplerStateD3D11` objects made inside
`Phyre_PSerialization_PBinary_loadCluster` (`0x0057bce0`) of the model's own
cluster (`chr/.../mdl/D3D11/*.dae.phyre`, `map/.../mdl/...`), with min
filter 5, mag 1 and Phyre address mode 0 (clamp): addressing does not tell
them from post-process samplers. Effect shaders (`Vfx*.fx.phyre`) and the
post-process shaders bring theirs in their shader clusters; the engine makes
a few outside cluster loads. `PSamplerStateD3D11_create` (`0x00595bf0`)
fills the `D3D11_SAMPLER_DESC` through `PRendererD3D11_mapSamplerFilter`
(`0x0058aa10`), which returns `ANISOTROPIC` for every sampler when the
Anisotropic option (`0x00c25370`) is above 1.

**Patch:** three hooks. `loadCluster` keeps the path of the cluster being
loaded (per thread); `PSamplerStateD3D11_create` decides: a cluster under
`/chr/`, `/map/`, `/btlmap/`, `/battle/`, `/effect/`, `/magic/`, `/event/`
or `/obj/`, not `/2d/`, menus, fonts or `/shaders/`, and not a comparison
sampler; `mapSamplerFilter` then returns `MIN_MAG_POINT_MIP_LINEAR` (0x01)
for it. The log names the first sampler of each of the first 200 clusters
with its decision.

**Test:** Fahrenheit, the intro: the characters' and the ruins' samplers
`-> point`, the effect and post-process shaders' `-> kept`; texels show as
squares close up (`lighting/intro-tidus-nearest-vs-default.png`, left).

## Mip chains for the PS2 textures

Seen with the default filter, the PS2 textures looked pixelated and
shimmering on far surfaces: `phyretex` wrote them as one level (the HD
textures have full mip chains), so the game's trilinear/anisotropic sampler
had nothing to minify from. Magnification was always bilinear (material
samplers min 5 = linear-mip-linear, mag 1 = linear, or anisotropic). Now
`phyretex::mip_chain` writes every level down to 1 x 1 when the HD texture
has mips: box filter, colour weighted by alpha (no colour bleeding from
clear texels; colour x alpha, the effects' brightness in alpha, kept
exactly), and for cut-outs (alpha nearly only 0 or 255) the alpha of each
level scaled so as many texels pass the alpha test (0.5) as at the top, so
hair and leaves do not thin out with distance. The texture jobs are at
version 2, so builds redo them.

A chain has a cost at the game's lower texture qualities: the loader drops
the top level at `TQ_MEDIUM` and the top two at `TQ_LOW` (the `VQ_LOW`
preset, the user's setting) of every texture with that many levels, never
of a one-level texture (ffx-rust `docs/formats/phyre.md`, "How the game
loads a texture"). So at `VQ_LOW` every PS2 texture with a chain is drawn
at a quarter of its size. The remodelled characters' atlases are written
in one level again (`phyretex::argb8_one_level`, models job version 4,
`docs/models.md` "Conversion"); the other jobs still write chains.
