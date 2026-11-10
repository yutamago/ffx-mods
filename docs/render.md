# Render patches (`render.*`)

The code patches behind `render.glow`, `render.lighting`, `render.shadows`
and `render.texture_filter`: hooks in `mods/ps2-lighting/LightingPatches.cs` and `mods/ps2-characters/TexturePatches.cs`, and for
`render.lighting` also patched character shaders (converter job
`render.lighting`, `converter/src/jobs/lighting_shaders.rs`) and the PS2's light
colours in the event scripts (job `render.lighting.events`,
`converter/src/jobs/event_lights.rs`).
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
- **Converter job `render.lighting`** (`converter/src/jobs/lighting_shaders.rs`,
  `formats/dxbc.rs`): in each lit character pixel shader, every light's
  `mad rA.c, rA.c, l(0.5), l(0.5)` (fed by a `dp3` with `Light_i.m_direction`)
  becomes `mad_sat rA.c, rA.c, l(1.0), l(0.0)` = max(0, N.L) and the
  following square `mul rA.c, rA.c, rA.c` becomes `max rA.c, rA.c, rA.c` (a
  copy; 2-light variants square two components in one `mul`, two shaders use
  the half-Lambert unsquared and get the `mad` alone). Instruction lengths
  stay, so only the DXBC checksum is recomputed (MD5 with Microsoft's
  padding; the test checks it against all 12,600 containers of the 793 shader
  files and that every lit shader takes one patch per light: 1854 shaders,
  3708 lights). 199 files, written to `efl/x/ffx_data/gamedata/ps3data/shaders/d3d11/`.
- **Gouraud** (job version 2): the vertex shaders output a unit normal
  (`TEXCOORD2`), and each lit pixel shader renormalised the interpolated one
  (`dp3 rK.c, vN.xyz, vN.xyz; rsq rK.c, rK.c; mul rM.xyz, rK.cccc, vN.xyz`)
  before the lights' `dp3`. That `rsq` now reads `l(1.0)` (same length:
  a temp source and a scalar immediate are both 2 tokens), so N.L is linear
  across the triangle: the same value as interpolating each vertex's N.L
  like the PS2 does, except on triangles that cross a terminator (the PS2
  clamps each vertex at 0 before interpolating) and past the PS2's 255
  vertex clamp. The chain is followed back from each light's `dp3`
  (`lighting_shaders::normalisation`); the test checks that all 1854 lit shaders
  have it once.

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

- The sign of the light directions is the HD's (the patch keeps the HD's
  lit side). Checked in PCSX2 (below, "The HD's event lights"): the PS2's
  normal light matrix holds the `Light_Set` vectors negated, which the
  PC's `(-x, -y, z)` matches.
- Per pixel instead of per vertex: since job version 2 the interpolation
  is Gouraud-like (above); terminator triangles still clamp per pixel.
- The PS2 clamps the vertex colour at 255 (x 1.99) before the texture
  multiply; ours does not, which only shows where texture < 0.5 and lit >
  1.99 (very bright lights).
- Still on: `GlobalAmbientColor` (only magic sets it), the PC fog mix, the
  HD's per-level shade tweaks in `Ch_DrawMain` (levels `0x1ca`, `0x1c0e`,
  `0x2fa`, map `0xc1`), HDAO (a graphics option), and the extra
  `LgtMatVec` light (`setAddLight`, effects; half-Lambert, applied as
  `max(light, 1)`).

### Compared with the PS2 (the opening, 2026-11-07)

The user saw the opening's characters flat next to the PS2: the PS2's
Tidus close-up has strong facial shading, ours (shots of 18:39) an evenly
lit face. Compared now against PCSX2 (software renderer, boot to the
opening, shot every 2 s and a GS dump every 4 s) and the Fahrenheit
install (`C:\_workspaces\fh-demaster-test`, 3440 x 1440, all mod files):

1. **The PS2's lit colours.** `~/ffx-work/demaster/scratch/runtime/vcol.py`
   walks a GS dump and rasterises the Gouraud vertex colours of the
   character draws (context 2, PSMT8, with the depth test): the PS2's lit
   value per pixel, 128 = 1.0. Tidus's close-up (`d143`): the face lies at
   190-240 (lit 1.5-1.9: forehead 239/215/163, cheeks 190-197, chin 230,
   neck 210), the darkest vertex (19, 21, 48). The light colours the mod
   reads at the same shot (light 0 (1, 0.88, 0.58), light 1
   (0.52, 0.41, 0.16), light 2 (0.07, 0.03, 0.03), ambient
   (0.063, 0.071, 0.153) x 254/128 = (16, 18, 39) of 128) give the same
   range by the formula above. So the PS2 lights the face almost evenly;
   its shading (eye sockets, cheeks, neck under the chin) is painted into
   the c101 face texture (texture = pixel x 128 / lit: forehead 136,
   neck 90).
2. **The model, not the light.** The flat face of the 18:39 shots
   (`fix-alpha/fh/fh-25.png`) is a different mesh: a smooth head, no
   necklace, card hair. The opening loads `c101` (Tidus) and `c102` (Yuna)
   for the close-ups (cluster loads logged); with the install's current
   `models.party` output (the PS2 mesh and the PS2 atlas, after the alpha
   fix of 9917b6c) the close-up is the PS2 face with its painted shading.
3. **Measured** (skin pixels, orange hue, of the same credit shot, both
   scaled to 512 x 416; `scratch/runtime/skin.py`):

   | Shot | PS2 L mean / sd / p10 / p50 / p90 | Ours |
   |---|---|---|
   | Tidus close-up ("Chief Storyboard Designer") | 130.3 / 38.8 / 77 / 131 / 181 | 130.9 / 45.9 / 71 / 129 / 191 |
   | Yuna close-up ("Real-time Graphics Director") | 134.3 / 38.2 / 84 / 134 / 187 | 134.3 / 44.0 / 79 / 130 / 197 |
   | the party at the fire ("Field Programmers", mask catches ground too) | 97.8 / 37.9 / 54 / 92 / 151 | 90.4 / 31.6 / 52 / 88 / 130 |

   Mean skin brightness matches; ours spreads a little more (sd +6 on the
   close-ups). The lighting (level 254/128, no fill light, max(0, N.L))
   stayed as it was.

### Brighter highlights in the opening's close-ups (2026-11-07)

The user saw brighter highlights on Tidus's face than on the PS2. The
colour mask above is biased: skin and blond hair overlap in hue, and the
brightest skin clips red (R 254-255 on both), turns yellow and drops out of
the mask. Measured again on pose-matched frames (`scratch/light/match.py`:
the frame of our run most like the PS2 frame, 4:3 crop at 640 x 480) inside
a hand-placed face ellipse (`scratch/light/face.py`, `regions.sh`), main of
2026-11-07 (with `render.texture_quality`) before and after the Gouraud
change:

| Region | L mean / sd / p5 / p50 / p95 | G p5/50/95 | B p5/50/95 |
|---|---|---|---|
| Yuna face, PS2 `s094` | 97.3 / 43.2 / 26 / 94 / 179 | 20/80/162 | 20/57/82 |
| Yuna, before | 98.8 / 50.2 / 23 / 89 / 191 | 17/72/179 | 19/54/92 |
| Yuna, after | 97.7 / 49.9 / 22 / 88 / 191 | 17/71/178 | 19/54/92 |
| Tidus face, PS2 `s153` | 147.5 / 40.6 / 54 / 163 / 186 | 42/138/173 | 24/62/78 |
| Tidus, before | 158.2 / 41.5 / 62 / 171 / 200 | 49/149/194 | 26/67/89 |
| Tidus, after | 156.0 / 44.9 / 51 / 173 / 200 | 42/151/193 | 23/68/90 |
| Tidus's jacket (yellow), PS2 | 197.7 / 11.1 / 179 / 198 / 213 | 176/204/228 | 20/24/36 |
| jacket, before / after | 215.9 / 211.6 (L mean) | 207/231/255, 186/223/255 | 23/27/42, 24/30/42 |

R clips at 255 on the lit skin of both. The shadows match (p5), the
highlights do not: lit skin and the lit jacket are 10-13% brighter in G and
B than the PS2's, the darkest parts the same. The Gouraud change is right
for the PS2 but moves these numbers by 1-2 (renormalising changes little on
the PS2 meshes' fairly dense faces), so it is not the cause. Ruled out too:
the PS2's 255 vertex clamp (the face's vertex colours peak at 244 in
`d143`), sRGB (the PC has no sRGB texture formats; the shader multiplies
`texture x COLOR0 x MaterialColour x lit` in the stored values like the
GS), the vertex shader's height gradient (`VertexColorOrigin/Modify`, 1)
and the extra light (`max(TEXCOORD4, 1)`, 1 without `setAddLight`). The
excess grows with the light (none at the ambient, ~12% where light 0
hits), which points at the directional lights' level or colour, not at
the falloff: the next check is the lit vertex colour of a PS2 vertex
(`vcol.py`, `d143`) against `m_colorIntensity` x N.L read from our game at
the same shot, light by light (the PC's glow balance, `+0x1e0`, and the
`Ch_DrawMain` light value are the suspects). The sky behind is darker in
ours (L 60 against 96), a separate (map/sky) difference.

Pictures: `C:\shots\fix-alpha\light\cmp-final.png` (PS2, before, after;
Yuna then Tidus, with the measured ellipses), the runs in `final-before\`,
`final-after\`.

### The lit areas checked light by light (2026-11-08)

Every input of the lit colour was compared between the PS2 and ours in the
intro; all match, and the cause of the brighter lit areas is still open.

- **The PS2's matrices, read in PCSX2** (debug build, intro "Field
  Programmers" shot). The character draw uploads its VU1 constants at
  `0x0010fa94` (`UNPACK 0x6c23...`): qw 16-19 from `0x00303650` (light
  colour matrix: (1, 0.816, 0.490), (0.302, 0.247, 0.169), (0.506, 0.412,
  0.114), ambient (0.078, 0.055, 0.071), the script bytes / 255 that our
  log shows for the same shot), qw 20-22 from `0x003036b0` (normal light
  matrix x bone: every light's column has length 1.0000). Program 1 has no
  `ITOF`: the normals arrive as floats, unit length (the `.chr` stores
  them at 32767). `LOI 255.0` feeds the `MINIi` clamp. So the PS2 computes
  exactly `vcol = 254 x (sum c_i max(0, L_i.N) + ambient)`, and its own
  vertex colours in `d143` fit that with N.L up to 1.0.
- **Ours, read in the game** (diagnostic hooks under Fahrenheit, not
  kept): every light's PLight `+0x24` stays 254/128 after
  `applyIntensityMultiplier`; the colours are the script's; the record
  `+0xac` is the ambient x 254/128; glow factors 0.01, unused. Tidus's and
  Yuna's shade `+0x360`, light value `+0x36c`, shade colour `+0x330..` and
  shade colour effect `+0xbc..` are all 1, so `MaterialColour` is 1, as on
  the PS2 (`Ch_DrawCalcShade`). The Zanarkand saturation only touches the
  luminances `+0x1d0..` and the ambient on znkd08/09/10/14, not the opening's
  map `0x15d`.
- **Lighting alone, same moment** ("CG Supervisor" credit; PS2 frame
  `s143` = dump `d143`): the c101 atlas replaced by flat grey 0x80 (so the
  pixel is 128 x lit, the PS2's vertex colour on the same scale), face
  G p5/50/90/99: ours 104/197/218/235, the PS2's rasterised vertex
  colours 110/187/213/218. The difference (+2 to +5% in the middle, more
  at the top, where ours is per pixel) is within the head's pose
  difference between the runs. Pictures:
  `C:\shots\fix-alpha\light2\vcol-vs-grey.png`.
- **Light directions.** `setLightDir` stores the script vector as
  (-x, -y, z). Passing the vector as the PS2 has it (stored (x, y, z))
  lights the face from the wrong side, lights 0 and 1 on the same cheek
  (face G p50 229 against the PS2's 187;
  `vcol-grey-greyflip.png`): the port's convention is right.
- **The PS2's texturing** (`d143`): character draws are `MODULATE` (TFX
  0, TCC 1, TEXA 0x80/0x80), no overlay after them (only the two
  clears are full-screen), fog on but F 249-255 at the characters (fog
  colour `0x49689e`), and the c101 CLUTs are uploaded byte for byte as in
  `c101.chr` (no runtime recolour).
- **Point lights.** See "Point lights in the opening" below.

Measured again with main of 2026-11-08 (pose-matched frames, the same
ellipses as above):

| Region | L mean / p50 / p95 | G p5/50/95 | B p5/50/95 |
|---|---|---|---|
| Yuna face, PS2 `s094` | 97.3 / 94 / 179 | 20/80/162 | 20/57/82 |
| Yuna, ours (`light2\before\t118`) | 106.6 / 97 / 194 | 25/79/183 | 24/58/93 |
| Tidus face, PS2 `s153` | 147.5 / 163 / 186 | 42/138/173 | 24/62/78 |
| Tidus, ours (`t168`) | 157.2 / 173 / 200 | 43/152/194 | 24/69/90 |

The "lighting alone within a few per cent" above compared percentiles of
frames in different poses; measured per position (next section) the
lighting alone carries the whole surplus.

Pictures (never in git): `C:\shots\fix-alpha\runtime\` `ps2\s*.png`
(PS2 boot to title, every 2 s; `d143.gs.zst`, the GS dump of the Tidus close-up), `diag3\t*.png` (ours,
every 10 s), `cmp-tidus.png`, `cmp-yuna.png`, `cmp-party.png` (PS2 left,
ours right), `vcol-tidus.png` (the PS2's lit vertex colours, left, and
the PS2 frame).

### Per position against the PS2 (2026-11-08)

The Tidus close-up ("CG Supervisor", PS2 GS dump `d143`) compared pixel by
pixel. Scripts in `~/ffx-work/demaster/scratch/texel/` (game data, not in
git): `texel.py` replays the dump (VRAM from the frozen GS state at byte
380 of the state block, every host-to-local transfer of the dump) and
rasterises the context-2 draws with their vertex colour, texture
coordinate, TEX0 and the texel the GS samples there; `match.py` finds each
decoded texture in our atlas; `register.py` fits our frame onto the PS2's
(scale 0.995 x 0.943, shift 11, 22.5 px on the 960 x 720 4:3 picture;
residual 0.025-0.034) and compares on the PS2's face page, eroded by 3 px.
Our frames come from a capture loop (`C:\shots\fix-alpha\texel\caploop.ps1`,
about 4 frames/s), so a frame within about 0.1 s of the dump's pose exists.

1. **Texels: identical.** All eight c101 pages the PS2 draws (TBP
   `0x3c00`/`0x3e00`, PSMT8, CLUTs `0x2ee0`..`0x2ef4` as uploaded that
   frame) are bit-exact in our `p101` atlas (`formats/ps2chr.rs` colour
   conversion, CSM1, CLUT choice), and our face triangles use page 9
   (CLUT `0x2ee0`) like the PS2's. The dump's texel x vertex colour / 128
   reproduces the PS2 frame.
2. **The PC's path from texel to screen: linear.** An atlas of 3-texel
   stripes of grey `0x40` and `0x80` gives a dark / bright ratio of
   0.475-0.48 on the face at every brightness from 100 to 250, in R, G
   and B (the lighting gradient across the 25 px window makes it a little
   under 0.5): no gamma, tone curve or offset after the shader. (Map
   colour correction: `znkd06` has no `_colorcorrection` texture;
   `Brightness` 50 gives `g_brightPara` 0.)
3. **Same pose, textured:** ours / PS2 on the face, median R 1.059, G
   1.088, B 1.090; by the PS2's lit level (vertex colour G): 1.016 below
   150, 1.081 at 150-190, 1.103 at 190-215, 1.109 above.
4. **Same pose, lighting alone** (atlas grey `0x80`, ours = 128 x lit
   against the PS2's vertex colours): G 1.085 (1.008 / 1.076 / 1.101 /
   1.105 by the same bins); the Gouraud shaders of job version 2 give
   1.081. Fitted per pixel: ours = 1.20 x PS2 - 22 (G; R 1.21 x - 27,
   B 1.33 x - 36): our lighting has about 1.2 times the PS2's contrast.
   The ratio maps of 3 and 4 are the same picture (brighter on the lit
   cheek and forehead, darker under the chin): the surplus is all in the
   lit value, none in the texels or after the shader.
5. **Per light.** Light 0 (1, 0.88, 0.58) and light 1 (0.52, 0.41, 0.16)
   differ in colour, so each pixel's RGB gives both N.L terms (ambient
   and light 2 known; residual rms under 0.009 on both sides). Light 0:
   PS2 p5/50/95 0.12/0.55/0.75, ours 0.00/0.61/0.88 (ours = 1.37 x PS2 -
   0.15, correlation 0.92); light 1 alike (1.31 x - 0.07, 0.76). The
   colours and levels are right (3 and 4 agree in the shadow); the
   angles between our normals and the lights are not the PS2's.
6. **Ruled out for the N.L:** the light directions' signs (z flipped as
   well, `(-x, -y, -z)`, lights the back of the head: the faces go dark,
   ratio 0.4); the per-pixel renormalisation (job version 2's Gouraud
   shaders: 1.085 -> 1.081); unskinned normals on the PS2 (the PS2 skins
   c101's normals every frame: its envelope `0x00107ec8` does when chr
   `+0x1a0` bit 0 is set, which Tidus's is, and renormalises them; a
   write watchpoint on his normal buffer breaks in `0x00124260`).
   Still open: how the PS2 weights the normals (the normal skin lists of
   `.chr` model `+0x24`, used by `0x00124058`/`0x00124260`, are separate
   from the vertex lists; our conversion gives each normal its vertex's
   four bones), and how its normal-light matrix (VU1 qw 20-22, light
   matrix x base bone) orients the lights against them. The next step
   is to read the PS2's skinned normals of the face (`0x013e5c80` in
   save state 7) and its qw 20-22 at the dump's frame, and to compute
   N.L per vertex for both. Done: both are the same as ours; the PS2's
   light colours are not (next sections, "The HD's event lights").

Not a cause, found on the way: the test install's character shaders
(2026-11-07 22:23) are still `render.lighting` job version 1 (the `rsq`
renormalisation in place; 131 bytes differ in `#c0ebfbfe…`, all 199
files differ): its stamp says the job was "not run". The measurements
above with job version 2 swapped in change by under 1%.

Pictures: `C:\shots\fix-alpha\texel\tidus-per-pixel.png` (PS2, ours
registered, ratio map; textured on top, lighting alone below),
`pose-d143-vs-ours.png`, the runs in `texel\after1`, `grey2`,
`stripes`.

### Point lights in the opening (2026-11-08)

The PC's opening has one point light, the campfire (point light 0:
position (-17, -1.5, 30), grey 0.60-0.66 flickering, range 30, exponent
2, slot 0; read in the game). `Ch_PLightCalc` (`0x008305a0`, from
`Ch_DrawMain`) sums the lights in range into the character's slots
(colour x 2, `0x00c496f0`; largest component at most 2, `0x00c49700`)
and hands the last one's slot to the shader's extra light (`LgtMatVec` /
`LgtMatClr`, applied as `max(colour x half-Lambert^2, 1)`).

**The PS2:** its `Sg_LightSetChr` (`0x00106810` in SCES_504.92) puts slot
i in place of global light i only where bit i of the character's
point-light byte (chr `+0x194`; PC `+0x188`) is set. A conditional
breakpoint there (`s1 != 0`) never fired from boot through the opening to
the title, while the same site is reached for every character: the PS2
lights no character in the opening with a point light. Elsewhere it does
(effects, `pppFpPointLightProc`, and two script natives set the byte), so
the change follows the PS2's rule instead of removing point lights.

**Patch** (`RenderPatches.point_lights`, under `render.lighting = "ps2"`):
the 13 bytes at `0x008309b4` (`cmp [ebp-0x9c], 0; je 0x00830a80`, the
"any light in range" test before the slot goes to the shader) become a
`jmp` to a 46-byte native stub in memory of its own (`Native.alloc_code`):
the original test, then slot = (colour pointer `edi` - chr `ebx` -
`0x384`) / 16 and `bt` of `byte [chr + 0x188]`; without the bit, the
no-light branch (`0x00830a80`, white from above: changes nothing). No
managed code runs in it, so it is safe wherever `Ch_DrawMain` runs.

**Checked in the game** (Fahrenheit install, 6 runs with the patch, none
crashed: `texel\after1`, `stripes`, `grey` twice, `grey2`, `flipz`, plus a
logging run): the log line `render.lighting: point lights only where the
character's slot bit is set`. Before / after on pose-matched frames of the
same build with only this patch off (`texel\before2` against `after1`):
the campfire shot ("Art Directors", Wakka nearest the fire) Wakka 51.6 /
51.7, Rikku 68.6 / 68.8, Kimahri 45.1 / 45.2 (L mean; per-pixel ratio
median 1.00); "Field Programmers" the same within 0.4%. So the campfire
light changes nothing visible in the opening on the PC either (the x 1.14
reported earlier does not show in the rendered frames); the patch matters
where a point light does reach a character. Pictures:
`texel\plight-fire.png`, `plight-before-after.png`.

One run with the test install's DLL as found (another agent's build) crashed
50 s in, inside `Phyre_PRendering_PRendererBase_generateMaterialParameterCaptureBuffer`
(`0x005b7f30`, from `PhyreScene_render` on the main thread): not this
patch, and maybe what was blamed on the earlier `Ch_PLightCalc` hook.

### The HD's event lights (2026-11-08)

The cause of the brighter lit areas in the opening's close-ups ("Per
position against the PS2" above): **the HD Remaster retuned the light
colours in the opening's event script**, so the PC lit Tidus with other
lights than the PS2. Not the normals, not the light directions.

- **Normals: the same as the PS2's.** The PS2 skins normals with lists of
  their own: the mesh header's `+0x24` is a second skin-entry table, one
  entry per vertex skin entry (same bone, model, base bone and kind), read
  by the envelope `0x00107ec8` (SCES_504.92) with the vertex entry's
  matrix; `0x00124058` (kind 0) and `0x00124260` (weighted) rotate them,
  `0x001243e8` renormalises. Parsed now (`formats/ps2mesh.rs`,
  `Model::normal_influences`; `demaster models-debug` prints both checks):
  in the bind pose they give the normal array that the conversion uses
  (at most 0.2 degrees apart), and every corner's normal has exactly its
  vertex's bones and weights (c001, c002, c005, c101, c102). Our skinned
  normals are the PS2's.
- **Light directions: the same.** Read in PCSX2 at the close-up (chr
  `0x00bcc350` = c101, after `Sg_LightSetChr`): the raw `Light_Set`
  vectors at `0x003035c0` are the PC's (`setLightDir` input, logged under
  Fahrenheit: light 0 (0.4234, 0.2588, 0.8682) on both), and the normal
  light matrix (qw 20-22, `0x003036b0`) holds them negated (the
  `sceVu0NormalLightMatrix` convention: vectors towards the light; c101's
  base bone 150 has no rotation in that shot), which is what the PC's
  `(-x, -y, z)` and its world flip amount to.
- **Light colours: not the same.** The PS2 during the whole Tidus shot:
  light 0 (225, 199, 118) / 255, ambient (20, 22, 49); the PC (a hook on
  `Sg_LightSetColor` `0x00836310`): light 0 (255, 225, 147), ambient
  (16, 18, 39); lights 1 and 2 equal. The PC's `znkd0600.ebp` has those
  numbers in its `Light_Set` calls: 11 of its 36 calls differ from the
  PS2 disc's (the angles never do), e.g. the Tidus shot, two sets of the
  party at the fire (light 0 (255, 190, 125) for the PS2's (255, 208,
  125), lights 2 and the ambient darker), Yuna's light 1 (155 for 141).
  The previous agent's per-light split assumed the PC's colours for the
  PS2 frame: with the PS2 at 0.88x light 0 and 1.25x ambient, that split
  reads as a "1.37x spread N.L".

**Fix** (converter job `render.lighting.events`, under `render.lighting =
"ps2"`): every event `.ebp` of `FFX_Data.vbf` is paired with the disc's;
the `Light_Set` calls (`func_facc 0x4013` after six `pushimm`) are taken
worker by worker in code order, and where both files have the same count
and the same light numbers the PS2's immediates are written into the PC
file in place (same instruction lengths, nothing else changes; the test
checks that only those operands move). 6 scripts change: `znkd0600`,
`loopdemo` (the title's demo, the same code), `bjyt0200` (14 calls),
`bvyt0400` (6), `maca0200` (4), `dome0400` (1); 47 of 9679 calls. Not
paired, left as they are: `genk1000` (164 calls on the PS2, 160 on the
PC), `maca0500` (4 on the PS2, none on the PC). 311 scripts have the same
lights; 77 PC events are not on the disc.

**Measured** (Fahrenheit install, the shaders of `render.lighting` job
version 2 installed for all runs; same-pose registration on the dump
`d143`, scripts in `scratch/texel/`):

| ours / PS2, G (face) | all | vcol < 150 | 150-190 | 190-215 | > 215 |
|---|---|---|---|---|---|
| before (`nl\diag1\f031308`) | 1.078 | 1.008 | 1.070 | 1.093 | 1.104 |
| after (`nl\ev1\f010216`) | 1.006 | 1.012 | 1.008 | 1.001 | 1.005 |

R 1.050 -> 1.004, B 1.081 -> 1.012. Face ellipses on pose-matched frames
(as in "Brighter highlights"; `nl\base2`, `nl\ev2`):

| Region | L mean / sd / p5 / p50 / p95 | G p5/50/95 | B p5/50/95 |
|---|---|---|---|
| Tidus, PS2 `s153` | 147.5 / 40.6 / 54 / 163 / 186 | 42/138/173 | 24/62/78 |
| Tidus, before | 160.6 / 39.9 / 69 / 175 / 199 | 54/155/193 | 28/70/88 |
| Tidus, after | 147.6 / 41.0 / 54 / 163 / 188 | 43/138/176 | 25/62/80 |
| Yuna, PS2 `s094` | 97.3 / 43.2 / 26 / 94 / 179 | 20/80/162 | 20/57/82 |
| Yuna, before | 101.9 / 48.8 / 28 / 93 / 192 | 21/75/180 | 22/56/93 |
| Yuna, after | 101.1 / 45.2 / 27 / 96 / 185 | 21/82/170 | 22/58/85 |

Yuna's shot changes only light 1's red; her remaining few per cent are
within the head's pose difference (the PS2 frame turns her head further).
The party at the fire (box over the characters, L mean ours / PS2): "Art
Directors" `s065` 0.888 -> 0.940, `s068` 0.861 -> 0.902, `s076` 0.855 ->
0.912, "Lead field designers" `s124` 0.883 -> 0.906; the rest is the map
around them (ground and sky darker than the PS2's, a map matter). Yuna's
wider shot `s106` 0.975 -> 1.001. No daylight scene with a changed script
was reached (the four other scripts are later events).

Pictures: `C:\shots\fix-alpha\nl\tidus-before-after.png` (PS2,
ours, ours / PS2 ratio map; before on top), `faces-yuna.png`,
`faces-tidus.png`, `fire-before-after.png`.

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
loads a texture"). So at `VQ_LOW` every PS2 texture with a chain was drawn
at a quarter of its size. Now: `render.texture_quality` (next section)
keeps the chains and loads them whole; the textures that never need one
have one level.

## Texture quality (`render.texture_quality`)

**What the game does.** `Settings_SetValue` (`0x00401930`) reads
`TextureQuality` of `GameSetting.ini` (the one in the user's
`Documents\Square Enix\FINAL FANTASY X&X-2 HD Remaster\`; the presets set
it: `VQ_LOW` = `TQ_LOW`) into `g_Settings +0x34` as 0 (`TQ_HIGH`), 1
(`TQ_MEDIUM`) or 2 (`TQ_LOW`); `Phyre_PFramework_PApplication_onInit`
hands it to `Phyre_PRenderInterface_setTextureQuality` (`0x0058e000`,
render interface `0x00c94f00` `+0x1340`). `Phyre_PBinary_loadClusterTextures`
(`0x00465ab0`, inside `loadCluster`, on the loading thread) asks
`Phyre_PRenderInterface_getTextureQuality` (`0x005947d0`) once per
cluster and drops that many top mip levels of every 2D texture with more
levels than that: none at `TQ_HIGH`, the top one at `TQ_MEDIUM` (half the
size), the top two at `TQ_LOW` (a quarter). The drop backs off while the
remaining top level would be under 4 texels wide or high (a 16 x 16
texture keeps 4 x 4, an 8 x 8 one loses only one level), and a texture
with only one level is never reduced (ffx-rust `docs/formats/phyre.md`,
"How the game loads a texture"; ffx.sym).

**Choices measured.**

| Way | Look at `TQ_LOW` | Look at `TQ_HIGH` | Size |
|---|---|---|---|
| chains as before | a quarter of the PS2 size: the pyreflies' sparkle a smeared band, map and effect texels 4x as big | PS2 texels | full build: 3.7 GB of chained PS2 textures (top levels 2.7 GB; `maps.field` 1.5 GB, `fx.textures` 1.35 GB, `maps.battle` 0.34 GB, `chr.*` 0.42 GB) |
| top two levels as x4 / x2 upscales of the PS2 image | the PS2 texels (level 2) | the upscale, the same picture (nearest: blockier than the PS2's bilinear) | each chain 16x: **about +54 GB** on disk, 16x the VRAM at `TQ_HIGH`; rejected |
| one level everywhere | PS2 texels | PS2 texels; far surfaces shimmer (why chains were added) | -0.9 GB |
| **hook: our files at quality 0** (chosen) | PS2 texels, chains kept | PS2 texels | nothing |

**Patch** (`RenderPatches.texture_quality`, default `"ps2"`): at start the
mod lists every `*.phyre` under its `efl/x` (32,473 in the test install);
the shared `loadCluster` hook keeps the path of the cluster being loaded
per thread (`/FFX_Data/GameData/PS3Data/...`), and a hook on
`getTextureQuality` (7 bytes, `mov eax, [ecx+0x1340]; ret`) returns 0
while that path, or what follows one of its `/`, is one of the mod's
files. The game's own textures keep the user's quality, so `VQ_LOW` still
saves its VRAM on the HD textures. `"hd"` installs nothing. Not covered: a
texture quality change while the game runs reloads every texture through
`Phyre_PBinary_reloadClusterTextures` outside `loadCluster` (from
`Phyre_PFramework_PApplication_frame` when the application asks for it;
the PC game sets the option in its launcher), which would drop our levels
until the next load of that file.

**One level where nothing is minified.** The menus, portraits, title,
loading screen, button glyphs, font pages and the world map picture
(`ui.*` job version 9) and the 2D rooms' paintings (`maps.backgrounds`
version 3) are drawn on the screen at the PS2 size or larger (the 4:3
picture is 1440 lines tall at 1440p, 2.8 times the PS2's 512 x 416 frame),
so a chain is never sampled below its top level: they have one level
again (`phyretex::argb8_one_level`), which no quality reduces even with
`render.texture_quality = "hd"`, and are a quarter smaller. The sphere
grid's pages zoom out and keep their chains. Maps, effects and the
kept-mesh characters (`chr.*`) keep their chains (far surfaces), the
remodelled characters' atlases stay one level (`docs/models.md`).

**Checked in the game** (Fahrenheit test install, the boot intro and the
title; PS2 frames from PCSX2 in `/mnt/c/shots/fix-alpha/`; game frames in
`/mnt/c/shots/fix-alpha/tex/`): `low-off-*` the user's `VQ_LOW` with the
patch off (before), `low-on-*` / `low-on3-*` `VQ_LOW` with the patch, `max-on-*`
every option at maximum. The log names each cluster with its decision: in
the intro the map's 13 `fp/tex` and 30 `texswap` textures, the character
atlases, the weapons, the fire effect (`magic_0662`), the loading and menu
textures load whole; the map's 59 HD textures, the HD menus and fonts keep
`TQ_LOW`. The pyreflies streaming over the ruins have the PS2's fine
sparkle with the patch and at max settings, a smeared band without
(`/mnt/c/shots/fix-alpha/tex/cmp/pyreflies.png`: PS2, before, after, max).
The title menu is the same in all four and as the PS2's
(`/mnt/c/shots/fix-alpha/tex/cmp/title.png`): most of its layers had one
level already. At `TQ_HIGH` the hook returns at once (quality 0).
