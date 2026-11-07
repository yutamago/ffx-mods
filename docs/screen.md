# Screen shape: 4:3 (row 14) and the 2D backgrounds (row 7)

Status: `render.aspect = "ps2"` **done** (below, "How 4:3 is made"): the
picture is the PS2's 4:3, pillarboxed, with the PS2 projection, the
PS2-placed 2D where the PS2 had it, the Flash screens squeezed into the
4:3, 4:3 movies, and the PS2 paintings in every fixed-camera room
(`maps.backgrounds`, `converter/src/bgjob.rs`). What is left: "Not done".

## The 3D projection: the PS2 view is the middle 4:3 of the HD's

The game sets the camera from the PS2 projection (`0x007b0000` region,
caller of `graphicCameraSetFOVy`/`graphicCameraSetAspectRatio`):
fovy from the PS2 screen distance, and the aspect
`a = (512 / (2 fx)) / (448 / (2 fy))` from the PS2 scales at `0x01137010`
(`fx`) and `0x01137014` (`fy`). On Besaid beach (harness, save 3) they are
0.8 and 0.9333, so **a = 320 / 240 = 4/3**: the PS2 frustum is exactly 4:3
for its 4:3 screen.

`PhyreScene_setCameraAspectRatio` (`0x0066cbe0`) stores `a` as is when
`g_PhyreScene3D` (`0x00ccb9c0`) is 0, and `a * 3 * 0.25 * 16/9` (= 16/9 for
a = 4/3) when it is 1 (Besaid: 1). The vertical FOV is unchanged. So in
3D scenes the HD shows the PS2's vertical extent and 4/3 times its
horizontal extent at the same scale: **the PS2 picture is the central
4:3 of the HD picture**. No FOV change is needed for a PS2 4:3 view; it
is a crop (pillarbox) of the 16:9 frame, 240 of 1920 pixels each side.

Illustration (HD shots with the side eighths blacked out, not a PS2
capture): `~/ffx-work/demaster/scratch/mov/cmp/aspect-besaid-43.png`,
`aspect-djose-43.png` (from `field-besaid` / `battle-djose`, max
graphics).

With `g_PhyreScene3D` = 0 the camera gets `a` itself on a 16:9 screen,
i.e. the PS2 view stretched by 4/3. The 2D-background rooms set their
camera from the background (`rcbgSceneLoad` area, `0x00920000` region:
the same formula from the room's `+0xb0`/`+0xb4`), and their layers are
full-screen 512 x 448 quads, so a room whose HD painting keeps the PS2
framing shows the PS2 view stretched to 16:9. The 2D rooms run with the
flag at 0 (`PhyreScene_fieldMapLoad` sets 1 only for 3D maps, minus a
list of 30 level hashes at `0x00b421c8`, and the unload clears it;
measured in hiku00: flag 0, camera aspect 4/3, `a` = 4/3 from
`rcbgSceneSetMatrix`, whose `+0xb4 = +0xb0 x 1.1666666` always gives
512/448 x 7/6 = 4/3).

## Why it is not just the crop

The HD UI is laid out for the full 16:9 width: in battle the help bar
starts 97/1280 from the left, the command window at 80, the HP list ends
at 1180 (`cmp/djose-hd.png`), so bars over the side eighths cut them.
PS2-placed 2D (positions in the 512-wide PS2 screen) is spread over the
whole width by the remaster (the remaps in ffx-rust `render.md`, "UI
coordinates", go the other way: 1920/1280 to 512); the Flash screens
(Iggy) have their own 16:9 layouts.

## How 4:3 is made (`render.aspect = "ps2"`)

The game draws its frame into 16:9 targets: `PhyreScene_setResolutionScale`
(`0x00652910`) makes the scene's size (`+0x10dd0`/`+0x10dd4`) the largest
16:9 inside the window, and the last copy of the frame into the back
buffer (`PostProcessManager_copyBuffer` with the back buffer as target)
goes through `PostProcessManager_setNon16By9Viewport` (`0x00698750`),
which fits it to the f32 at `0x00c34154` (16/9; that function is its only
reader) with bars, and gives the Iggy menus the same rectangle
(`IggyMenuManager_setViewportRect_00a26bd0`). That is the HD's own
handling of 4:3 and 21:9 monitors.

The mod keeps the 16:9 targets and draws the frame **anamorphic**:
everything in it is 4/3 too wide, and the last copy squeezes it into the
middle 4:3 of the window. Four changes (`mod/RenderPatches.cs`,
`aspect`):

1. **Output aspect.** `0x00c34154` = 4/3. The copy pillarboxes the frame
   to 4:3 (on a 16:9 window: 1440 x 1080 in the middle of 1920 x 1080; on
   the user's 3440 x 1440: 1920 x 1440 at x 759). The copy samples
   linearly, so the squeeze is a filtered 4:3 downscale, no dropped
   columns (`zoom-text` check in the harness).
2. **3D camera.** Hook on `PhyreScene_setCameraAspectRatio`
   (`0x0066cbe0`, thiscall(scene; a)): with `g_PhyreScene3D` set it passes
   `a * 3/4`, so the main and minimap cameras store `a` (the PS2's 4/3)
   instead of `a * 4/3` (16/9). On the 16:9 target that is the PS2 frustum
   stretched by 4/3, which the squeeze undoes. 2D scenes (flag 0) already
   store `a`: they were the stretched ones in the HD and come out right
   without a change.
3. **Sphere grid.** `PhyreScene_RenderABmap` sets its camera to
   `[0x00b41cf0]` (16/9; the same constant gives the UI camera its 16/9,
   so it is not changed): the `fld` at `0x00652a7a` is pointed at a 4/3 of
   the mod's. Not looked at in the game.
4. **Event sprites** (the opening's staff credits): hook on
   `updateVFXRenderDataAtelShp`, their widths in the PS2 x unit and the
   PS2's 1/16 corner unit (below, "Event sprites").

What follows from the squeeze, with no further change:

- **PS2-placed 2D** (the Tk path: message windows, the battle HUD and
  windows, the main menu, the minimap, `graphicDrawUIElement` /
  `graphicFontDraw` in PS2 units x 3.75 / 2.596) lands exactly where the
  PS2 had it: the HD spread the 512-wide PS2 screen over 1920, the squeeze
  takes it back to 4:3. The HD typeface and the HD-made pieces of the
  battle HUD are squeezed with it (text 3/4 as wide as in the HD; the PS2
  font, `ui.font`, is planned). The minimap is the PS2's square at the
  PS2's place.
- **3D-attached 2D** (damage numbers, cursors) is placed by the PS2 code
  or through the camera, both consistent with the PS2 frustum now.
- **Fixed-camera rooms**: their layers are full-screen quads, so a
  painting fills the 4:3 at its PS2 shape and the camera (`a` = 4/3) is
  the PS2's. The re-framed rooms need their PS2 paintings, which
  `maps.backgrounds` writes when `render.aspect` is `"ps2"` (below).
- **Movies** are drawn over the whole target, so a 960 x 720 movie fills
  the 4:3: the `movies` job encodes 960 x 720 without bars when
  `render.aspect` is `"ps2"`, and cuts the HD movies that have the 3D
  scene on top (mst, far, ggm, sns) to their middle 4:3, where the 3D
  scene now is (`docs/movies.md`).
- **Flash screens (Iggy)**: the Esc menu, its settings pages, save/load
  get the game's own 4:3 rectangle and are drawn squeezed into it.
  Chosen over full width: the game then stays one 4:3 picture, the Esc
  menu's dimmed background (the frozen frame) matches it, and Iggy's
  mouse mapping uses the rectangle the game gives it. Text is 3/4 as wide,
  still clear at 1080p.

The alternative, a real 4:3 target (hooking `setResolutionScale` to make
the scene 1440 x 1080), was tried in the harness first: the minimap's
viewport (`PhyreScene_getMinimapViewportSize` etc., computed from the
target width as if it were 16:9) shrinks to 3/4 and every other
width-based placement would need its own fix. The anamorphic frame keeps
every HD placement as it is and costs a filtered horizontal downscale.

## Not done

- **HD movies with `movies = "hd"`** (or the 4 movies the job does not
  convert when it fails): a 1280 x 720 movie fills the target and is
  squeezed to 4:3. Use `movies = "ps2"` with `render.aspect = "ps2"`.
- **Mouse** on the PS2-placed menus (`GuiTouch_*` pointer zones): not
  checked; their rectangles may still assume the full window.
- **Animated parts of re-framed rooms** get the one PS2 picture in every
  frame (their animation stops); the animated parts of the other rooms
  stay HD (they line up).
- **Other cameras** the game sets to 16/9 once (`PhyreScene_initScene`:
  scene `+0x394`, `+0x238`) and anything drawn with them would be
  squeezed; none seen.
- **Framebuffer effects** that read `graphicGetScreenRatio` (16/9: the
  model distortion `KeDttMdl_MkGifPk`, `updateVFXRenderData_FakeEnvModel`)
  still work on the 16:9 target, so they should be right; not looked at.
- **Not seen in the game**: the sphere grid, the 3D-scene movies, a
  window that is not 16:9 or 21:9, a split-screen camera (`+0x10d8c`).

## Event sprites: the opening's credits (done)

Before (PS2 `fix-alpha/ps2-13.png` against ours, "SATOSHI TSUKAMOTO",
white pixels): PS2 x 11.5 %..51.4 % of the 4:3 picture (39.8 % wide),
ours 17.4 %..47.0 % (29.6 %): 0.745 as wide, the same centre, the same
letter height.

What draws them: **event sprites** (`g_EventSprites`, set by the event
script), not the Tk text and not the `pppDrawShape*` effects.
`graphicFontDraw`, `graphicDrawUIElement` and `TOMakePktFont1612Font`
count 0 calls through the opening (Fahrenheit counters); the PS2 draws
the credits as about 450 GS sprites a frame of one PSMT8 font texture
(GS dump: context 1, TBP `0x1a00`, ALPHA `0x44`). On the PC
`AtelExecDrawShape` (`0x00874420`) hands each sprite to
`updateVFXRenderDataAtelShp` (`0x007196f0`, also called from
`InitVFXRenderDataAtelShp`), with a matrix that is the identity with the
sprite's x/y scale and its PS2 position. That function (ffx-rust
`symbols/ffx.sym`; 1920 x 1080 from `graphicFontGetScreenWH`):

- corners: i16 / 15 x **h/416** for x and y (square pixels);
- position: matrix x **w/512** - w/2, y h/416 - h/2.

So the HD spreads the sprite positions over the 16:9 width but keeps the
glyphs square: in the anamorphic frame a glyph is (h/416) / (w/512) =
9/13 = 0.692 of the PS2 width, x 16/15 (below) = 0.738 (measured 0.745).
Corner units: the PS2's are
GS 1/16 pixel; the HD's / 15 makes every event sprite 16/15 too large
around its position (with only the x unit fixed the line measured 42.8 %
against 39.8 %, 1.073, and the letters 3.26 % against 3.12 % high).

The fix (`mod/RenderPatches.cs`, `atel_shp`, part of
`render.aspect = "ps2"`): a hook on `updateVFXRenderDataAtelShp` that,
after the game filled the six render data (`id + 10000 k`, through
`graphicVFXGetRenderDataByInstance`), multiplies the matrix's x output
column (`[0]`, `[4]`, `[8]`) by (w/512)/(h/416) = 13/9 and both x and y
columns by 15/16. Translation stays. The game copies the matrix afresh on
every call that returns 1 (the early return 0 copies nothing, so the hook
leaves those alone and never scales twice). The 85 % keep-on-screen move
inside (screens 720 high or more, unscaled matrix) is computed before and
does not reach x at 1920.

After (Fahrenheit, 3440 x 1440, 2026-11-07): "SATOSHI TSUKAMOTO" 11.2 %
..51.3 % (40.1 %, centre 31.3 %; PS2 39.8 %, 31.4 %), letters 3.06 % high
(PS2 3.12 %); "HIROSHI KUWABARA", "CG SUPERVISOR" and the Directors block
(names and the small "(EVENT)" words) line up with the PS2 to a pixel or
two by eye. Pictures (never in git): `C:\shots\fix-alpha\credits\`
`cmp-cg.png` (PS2 / before / after) and `cmp-directors.png` (PS2 /
after).

Unchanged and already right:

- The boot logo ("FINAL FANTASY X PROJECT") and the title's "FINAL
  FANTASY X" logo and its letters: the same with and without the hook,
  so another path; the title logo has the PS2's size and place. The boot
  logo is a `titl02` layer (a full-screen quad, not an event sprite); the
  HD's redrawn art looked squeezed, now the PS2's picture at the PS2's
  place (`ui.title`, docs/ui.md "Title screen and logos").
- Sparks and pyreflies of the opening are 3D effects (`pppDrawShape*`:
  corners through the part matrix and the camera), so the camera aspect
  hook already gives them the PS2 proportions; not measured (they are a
  few pixels).
- With `render.aspect = "hd"` the event sprites keep the HD's square
  glyphs and the 16/15 size.

## Checked

- **Fahrenheit** (`C:\_workspaces\fh-demaster-test`, 3440 x 1440
  full screen, 2026-11-02): the hooks install (`render.aspect = ps2`, no
  warning, the sphere grid patch's bytes matched), the camera hook runs in
  2D and 3D scenes (log: `camera aspect 1,3333334 in a 2D scene` / `3D
  scene`); window captures (by window handle, PrintWindow) of the boot,
  the opening's campfire scene with its credits and the title: the
  picture is exactly 1920 x 1440 (4:3) at x 759..2678, black bars, the 3D
  in PS2 proportions.
- **ffx-rust harness** (1280 x 720), which can't run the hooks:
  `tools/replays/aspect-besaid.txt` and `aspect-djose.txt` do the same
  with memory writes (output aspect, and the PS2 screen scale `fx`
  `0x01137010` x 4/3 so that `Cam_Apply` hands in `a * 3/4`: the camera
  then stores 4/3, read back). The 4:3 3D is exactly the middle 960 of the
  HD shot; the HUD and minimap are at the PS2 places. Also checked with
  writes: the title, the New Game choice window, the main menu, the Esc
  menu and its System setting page (Iggy, squeezed), the airship room
  hiku00 (devsave 68, a re-framed room, 2D scene) with the converted PS2
  paintings (characters stand on the painted floor, no gaps), and IWK
  (Wakka's movie, ffx-rust's `movie-fmv` steps) encoded 960 x 720,
  filling the 4:3.

- **Against the PS2** (`tools/compare`, 2026-11-04, `docs/compare.md`,
  "The 3D framing"): a battle (Thunder Plains, save 5) and a field
  (salvage ship, save 1) from the same saves in PCSX2: the PS2 frame is
  the middle 4:3 of the HD frame at the same scale in both. The battle's
  "wider" framing in the first comparison was the replay writing only the
  output aspect, not the mod. Cutscenes not compared.

Pictures (HD left, 4:3 right; never in git):
`~/ffx-work/demaster/research/aspect/` `besaid.png`, `djose.png`,
`dialogue.png`, `menu.png`, `escmenu.png`, `title.png`,
`room-hiku00.png`, `fmv-iwk-43.png`, `fahrenheit-opening-3440x1440.png`,
`fahrenheit-title-3440x1440.png`.

## 2D backgrounds (`maps.backgrounds`)

PC: `map/<area>/<map>/2d/tex/d3d11/<layer>_<part>_<w>_<h>_<frame>.dds.phyre`
(2048-square DXT5 for the 512 x 448 parts), drawn as full-screen quads in
PS2 units (ffx-debug-revival `docs/research/old-backgrounds.md`). PS2:
`mapout.vpa` section 0 (`YNSC`), one PSMT8 image transfer per part with
its 16 x 16 CT32 CLUT after it, zeroed on the PC
(`research/ps2-textures.md`; decoder `uijob::bg_layers`, shared with the
title screen).

The job pairs the HD parts sorted by (layer, part) with the PS2 transfers
in file order (sizes within 8 pixels: the PS2 pads them) and writes each
PS2 picture at its PS2 size into the HD texture (ARGB8), in one mip level
(job version 3): a painting fills the screen and is never minified, and the
game's texture quality reduces no one-level texture (`docs/render.md`,
"Texture quality"). It keeps HD:

- with `render.aspect = "hd"` only: rooms whose full-screen pictures
  correlate under 0.85 with the HD ones: the HD **re-framed** 37 rooms
  for 16:9 (bika01, the world map: the HD shows more at the sides, the
  PS2 picture is a zoomed crop of it; also hiku00/05/17, luca06, kami01,
  bltz02, ...). A PS2 painting there would not line up with the HD's 16:9
  scene. With `render.aspect = "ps2"` they get every part in PS2 (no
  correlation check, animated parts with the PS2 picture in every frame):
  3D and painting are then both the PS2's. Some of them (hiku00 and the
  other airship rooms, correlation 0.00) differ mostly by a black
  rectangle in a PS2 part that another layer covers: hiku00 in the
  harness looks whole;
- parts with several frames (233: texture animation such as the turning
  wheel in azit00; the PS2 holds one picture per part, its animation data
  was not decoded), except in re-framed rooms under 4:3;
- parts that correlate under 0.5 with the HD (91; not in re-framed rooms
  under 4:3), and 4 rooms whose part counts differ.

With `render.aspect = "ps2"` (the default) the job writes 4109 layer
files (animated frames counted one by one) in 105 rooms.
`demaster backgrounds` converts as for 4:3 unless `DEMASTER_ASPECT=hd`.

Result: 446 layers in 68 rooms: azit00, azit05, azit06, bjyt05, bsil00, bsvr04, bsvr06, bsvr09, bsyt03, bsyt04, bvyt01, bvyt02, bvyt03, bvyt04, bvyt06, djyt02, djyt04, djyt05, dome05, genk04, genk05, genk06, genk11, genk12, genk13, guad02, guad03, hiku02, ikai00, kami02, kami08, klyt02, klyt06, klyt07, lchb09, lchb10, lchb11, lchb12, luca04, luca08, maca01, mcfr05, mcfr06, mcfr07, mcfr08, mcfr09, mcfr11, mcyt03, mcyt04, mihn03, mmmc00, ptkl08, ptkl14, ptkl17, ptkl18, slik03, slik07, slik08, swin03, swin08, swin09, zkrn03, znkd00, znkd03, znkd04, znkd05, znkd07, znkd13.
In those rooms the HD painting has the PS2 framing (azit00: the same
picture, upscaled and brightened), so the swap is right in 16:9 too.

Verified: previews of every converted layer
(`demaster backgrounds <iso> <vbf> <out> all|<map>` writes HD | PS2 side by
side; azit00 and bika01 looked at). In the game: hiku00 (developer save
68, `load devsave 68`) with the 4:3 writes and the PS2 paintings
(`room-hiku00.png`); the 68 rooms converted under 16:9 too were not
looked at in the game.
