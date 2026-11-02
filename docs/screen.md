# Screen shape: 4:3 (row 14) and the 2D backgrounds (row 7)

Status: `render.aspect` is **not implemented** (research and plan below);
`maps.backgrounds` converts the rooms whose HD paintings keep the PS2
framing (job `maps.backgrounds`, `converter/src/bgjob.rs`).

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
framing shows the PS2 view stretched to 16:9. Which flag the 2D rooms run
with was not checked.

## Why it is not just the crop

The HD UI is laid out for the full 16:9 width: in battle the help bar
starts 97/1280 from the left, the command window at 80, the HP list ends
at 1180 (`cmp/djose-hd.png`), so bars over the side eighths cut them.
PS2-placed 2D (positions in the 512-wide PS2 screen) is spread over the
whole width by the remaster (the remaps in ffx-rust `render.md`, "UI
coordinates", go the other way: 1920/1280 to 512); the Flash screens
(Iggy) have their own 16:9 layouts.

## Plan for `render.aspect = "ps2"`

1. **Viewport.** `Phyre_PFramework_PApplication_setViewportKeepingAspect`
   (`0x0062cba0`) already fits the application's views to the aspect at
   `PApplication +0x10` (pillarbox when the window is wider) and is called
   from `PApplication_resize` (`0x0062c260`). Set `+0x10` to 4/3 and force
   a resize at start-up; check that the game's own render targets and the
   post-process copy to the back buffer use that viewport (they may use
   the full target size instead: then hook `PRendererD3D11_setViewport`
   `0x0058c790` for the final pass).
2. **Camera.** With a 4:3 viewport the 3D-scene aspect must drop from
   `a * 4/3` to `a`: hook `PhyreScene_setCameraAspectRatio` and pass
   `a * 0.75` when `g_PhyreScene3D` is set (not a patch of the double
   16/9 at `0x00b43d68`: it is shared). `PhyreScene_RenderABmap` sets 16/9
   for the area map itself (`[0x00b41cf0]`) and needs the same.
3. **UI.** Every 2D draw (sprites, the PS2-drawn menus, fonts, the Iggy
   Flash screens) goes through the same viewport and so is squeezed 3/4
   horizontally: right for PS2-placed elements (they were stretched
   4/3 by the remaster), wrong for the HD-designed Flash screens. Either
   draw the Flash layer with the full viewport (find where Iggy renders,
   `iggy_w32.dll` calls from FFX.exe) or accept squeezed Flash menus.
   Mouse input (the remaps) must use the same mapping.
4. **Movies.** The FMV quad (`PhyFMVPlayerManager_Render`, `video.fx`)
   must keep the full screen: the converted movies already carry their
   bars (`docs/movies.md`). With the viewport pillarboxed, encode the PS2
   movies at 4:3 (960 x 720) instead.
5. **Movies with a 3D scene** (mst, far, ggm, sns): with 1-2 done the
   scene's projection is the PS2's, so their PS2 versions could be
   converted too.
6. **2D rooms.** With the viewport at 4:3 the stretched rooms show at
   their PS2 shape; the re-framed rooms (below) can take their PS2
   paintings then.

Testing: the C# mod needs a real Fahrenheit run for this (the ffx-rust
harness replaces parts of the render path, `render.md` "Replaced by
ffx-render"); screenshots of the game window only.

Effort: 1 and 2 are small hooks; 3 is the bulk and needs research into
how the 2D layers and Iggy are drawn.

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
PS2 picture at its PS2 size into the HD texture (ARGB8). It keeps HD:

- rooms whose full-screen pictures correlate under 0.85 with the HD ones:
  the HD **re-framed** 37 rooms for 16:9 (bika01: the HD shows more at
  the sides, the PS2 picture is a zoomed crop of it; also hiku00/05/17,
  luca06, kami01, bltz02, ...). A PS2 painting there would not line up with
  the 3D scene; they need 4:3 (plan step 6);
- parts with several frames (233: texture animation such as the turning
  wheel in azit00; the PS2 holds one picture per part, its animation data
  was not decoded);
- parts that correlate under 0.5 with the HD (91), and 4 rooms whose part
  counts differ.

Result: 446 layers in 68 rooms: azit00, azit05, azit06, bjyt05, bsil00, bsvr04, bsvr06, bsvr09, bsyt03, bsyt04, bvyt01, bvyt02, bvyt03, bvyt04, bvyt06, djyt02, djyt04, djyt05, dome05, genk04, genk05, genk06, genk11, genk12, genk13, guad02, guad03, hiku02, ikai00, kami02, kami08, klyt02, klyt06, klyt07, lchb09, lchb10, lchb11, lchb12, luca04, luca08, maca01, mcfr05, mcfr06, mcfr07, mcfr08, mcfr09, mcfr11, mcyt03, mcyt04, mihn03, mmmc00, ptkl08, ptkl14, ptkl17, ptkl18, slik03, slik07, slik08, swin03, swin08, swin09, zkrn03, znkd00, znkd03, znkd04, znkd05, znkd07, znkd13.
In those rooms the HD painting has the PS2 framing (azit00: the same
picture, upscaled and brightened), so the swap is right in 16:9 too.

Verified: previews of every converted layer
(`demaster backgrounds <iso> <vbf> <out> all|<map>` writes HD | PS2 side by
side; azit00 and bika01 looked at). **Not verified in the game**: the
harness's saves reach no converted 2D room (`load direct 3 <event>` loads
only save 3's own map; Besaid's `bsil00` layer is a fog overlay that its
replay does not show: shots identical with and without it).
