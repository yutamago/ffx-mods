# Render patches (`render.*`)

The code patches behind `render.glow`, `render.lighting`, `render.shadows`
and (planned) `render.texture_filter`: hooks in `mod/RenderPatches.cs`.
Addresses are ffx-rust's `symbols/ffx.sym` (FFX.exe image base
`0x00400000`); the post-process chain is ffx-rust's
`docs/formats/postfx.md`, the shadow solutions `docs/formats/scene-maps.md`
(Q7), the character draws `docs/formats/scene-chars.md`.

Each patch was first tested in the ffx-rust harness with memory writes that
have the same effect (the harness cannot write code pages or run C# hooks).
The mod's hooks were then loaded in the Fahrenheit install: each one logs its
first call (`render.glow: the glow pass is skipped`,
`render.lighting: light levels set to 1`, `render.shadows: shadow solution N
-> 0` for each map), and the title's demo scenes reach all three.

Comparison PNGs (HD left, patched right; never in git) are in
`~/ffx-work/demaster/research/render/`. The replays are in
`~/ffx-work/demaster/scratch/render/`.

| File | What |
|---|---|
| `glow-besaid.png` | Besaid beach, glow off |
| `glow-fx-fire.png` | Fire in the salvage-ship battle, glow off: the effect is unchanged |
| `lighting-besaid-tidus.png` | Tidus, light levels 1 (3x zoom) |
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

## `render.lighting = "ps2"`: the PS2 light values

How the PC lights characters (`phyrechrlitshader.fx`, every lit
variant):

```
lit = GlobalAmbientColor + CharacterAmbientLight + CharacterShadeColor x CharacterShadeCoeff
    + sum over i = 0..2 of Light_i.m_colorIntensity x (dot(L_i, N) x 0.5 + 0.5)^2
colour = texture x vertex colour x MaterialColour x lit, then fog and grey scale
```

There is no normal map, no rim light and no general specular term. The only
specular is the PS2's own `SgSpec_*` spot, which the PS2 had too. The lights
are the PS2 code's: event scripts set them through `Sg_LightSetDir` and
`Sg_LightSetColor` (`0x00836430`, `0x00836310`). Those fill the PS2 VU0
matrices and pass the same floats to `Phyre_CharacterLighting`
(`g_CharacterLighting`, `0x00ccca84`). The PC adds three things on the way:

1. **Levels.** `applyIntensityMultiplier` (`0x006a6e90`, thiscall(cl; i,
   flag), `ret 8`) sets light i's level (PLight `+0x24`; the shader's
   `m_colorIntensity` is colour x level) to (1 - glow factor) x 1.2 x
   intensity x 1.16 x 0.95. For the ambient it uses 1.05 instead of 0.95,
   writes the level to `+0x1a4`, and puts colour x level into the lighting
   record `+0xac` (`CharacterAmbientLight`). On Besaid beach that comes to
   2.10 for light 0 (map intensity 1.7), 1.31 for lights 1 and 2, and 1.45
   for the ambient. The intensities (`+0x1a8 + 0xc i`) and rotations come
   from the HD map's lighting object (`PhyreScene_setupCharacterLighting`,
   `0x0066eb20`). The glow factors come from `balanceLumWithGlow`. On znkd08,
   09, 10 and 14 the colours are also saturated x 2.6.
2. **Rotations** of the light directions per map
   (`CharacterLighting setRotation`, `0x006aaea0`; Besaid's are 0).
3. **The falloff** in the shader: half-Lambert squared instead of the PS2's
   `max(0, N.L)`.

What the PS2 did: the PC still carries the PS2 formula, in the
`PS2Lighting` vertex path that magic uses (`LvMatrix`/`LcMatrix`/`charlight`,
`PS2LightingSetup_apply` `0x006e5a40`):

```
lit = max(0, LcMatrix x max(0, LvMatrix x N) + ambient) x charlight / 128
```

These are the same colour matrices at unit scale, with a plain Lambert
term.

**Patch:** after the game's `applyIntensityMultiplier`, the hook sets the
level of each light and of the ambient to 1. That is light `+0x24` = 1, and
the record `+0xac` = the ambient colour, with the luminances `+0x1d0..`
recomputed to match. The hook on `setRotation` passes 0, 0. Characters are
then lit by the PS2 colours and directions as the scripts set them. No
shader is changed.

**Test:** on Besaid, with the glow factors 0 and `+0x230` set (so the
multiplier is 1.2 x I x 0.95), `graphicCharacterLightingSetLightColor`
(`call 0x0063cb00 i r g b`) was called with each colour divided by that
level. The ambient record was written directly (`writep 0x00cc9cd8 0xfc
...`, `g_PhyreApplication +0x50 +0xac`). Read back: light 0's luminance is
0.608, the colour's at level 1. Tidus comes out darker and flatter
(`lighting-besaid-tidus.png`).

**Uncertain:**
- **The scale.** Level 1 assumes the PS2's `charlight` is 128, so that
  `lit` = colour, as the PS2Lighting path computes it. That is not checked
  against a PS2 frame; no PS2 capture is at hand. If PS2 screenshots show
  brighter characters, `PS2_LEVEL` in `RenderPatches.cs` is the one number
  to change.
- **The falloff** stays half-Lambert squared. Back and side faces get
  `(0.5 + 0.5 N.L)^2` (0.25 at 90 degrees) where the PS2 gives 0, so
  characters stay softer-shaded than on the PS2. There is no memory route:
  the shader has no saturate to exploit. Ways to fix it:
  - Ship patched `phyrechrlitshader.fx#*.phyre` files: about 650 lit pixel
    shader variants in 220 files, each with `mad r.w, r.w, 0.5, 0.5` /
    `mul r.w, r.w, r.w` replaced by `max r.w, r.w, 0`, and the DXBC
    checksum recomputed. This would be a converter job.
  - Swap pixel shaders by bytecode hash in an `ID3D11Device::CreatePixelShader`
    hook.
- Still on: the HD-only `CharacterShadeColor x 0.208` term (`setShadeCoeff`),
  `GlobalAmbientColor` (only magic sets it), the PC fog mix, and HDAO (a
  graphics option).

## `render.texture_filter = "nearest"` (planned)

Not implemented; low priority. The game's samplers are
`PSamplerStateD3D11` objects (defaults in `PSamplerStateBase_ctor`
`0x004be870`: min filter 5, mag 1, anisotropy 1). The material files set
their own, and the `Anisotropic` option raises the anisotropy.

The plan: hook the D3D11 device's `CreateSamplerState`. For samplers of
textured 3D materials (wrap addressing, not comparison samplers), set the
filter to `MIN_MAG_POINT_MIP_LINEAR` and `MaxAnisotropy` to 1. Leave
clamp-addressed samplers alone: those are the post-process passes, the UI
and the shadow atlas. `"game"` installs nothing.
