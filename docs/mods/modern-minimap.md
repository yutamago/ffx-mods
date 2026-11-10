# Modern Minimap

`yuta.modern-minimap` replaces the field minimap with a round one: always
centred on the player, turned so that up is where the camera looks, the
character's facing as an arrow, the walkable area over a grid fixed to the
world, the game's markers in their shapes and colours, markers beyond the
edge on the rim. It is not a PS2 part; it started as an experiment of
ffx-rust's external renderer (2026-10-15, its `ffx-render` minimap) and
is that minimap moved into a Fahrenheit mod, drawn with ImGui. Settings:
[`config.md`](../config.md#minimap).

Addresses are FFX.exe VAs from ffx-rust's `symbols/ffx.sym`; the game's
minimap is described in ffx-rust's `docs/formats/field.md` ("Guide map")
and ported in its `crates/ffx-field/src/guide_update.rs`.

## The hook

`rcbgGuideUpdateEx` (`0x0091da80`, cdecl `(guide, show)`), called by
`Yn_GuideMapProcEx` (`0x00919d00`) every field frame, fades the game's
minimap, reveals its fog of war and draws it. `MinimapPatches.cs` hooks it
(Fahrenheit chains it with other hooks; the next one runs whenever the
game's own minimap is wanted). With the Modern Minimap on it does what the
original does besides drawing, in the original's order:

1. `g_GuideEnabled` (`0x00c5e478`) 0, no guide, or no mesh
   (`Guide_MeshObject`, `0x0091d460`: the guide's `+0`, else `+8`): the
   original runs (it draws nothing then).
2. `g_GuideFlags` (`0x00c5e350`) bit 0 clear: `Guide_FadeOff`
   (`0x0091d3a0`), hidden.
3. `Guide_FadeStep(show)` (`0x0091d280`); the game's drawing stays off:
   `graphicMinimapSetEnable(0)` (`0x006425f0`) and
   `graphicMinimapFogSetEnable(0)` (`0x006425d0`). `g_GuideFade`
   (`0x0193bf78`, 0..128) is ours too: 0 hides it, the rest fades it.
4. The markers: `AtelGetGideMapActorNumber` (`0x0086b710`) and
   `AtelGetGideMapActorInfo(i, out)` (`0x0086b700`; `+0` actor id, `+4`
   flags, `+8` position x y z, `+0x14` facing). Flags: low nibble the
   colour in `g_GuideMarkerPalette` (`0x00c5e370`, RGBA floats, 128 = 1),
   the next nibble the ring's colour, `0x100` square, `0x200` triangle,
   `0x400` ring, `0x800` the player (colour `0x00c5e3a0`), `0x1000`
   blinks (faded in and out as the game's minimap does: guide `+0x74 % 30` as a triangle wave, opacity phase / 15), `0x2000` destination, `0x10000` exit line (from
   `AtelGetActorStartPos`/`EndPos`, `0x0086a720`/`0x0086a390`, for actor
   types 2 and 5, `AtelGetActorType` `0x0086a7b0`), `0x20000` diamond.
   The original's exception is kept: on map `0x99` marker 6 is `0x107`
   at scenario `0x618`.
5. The fog of war (on four maps, guide `+0x70`) is revealed around the
   player: `Guide_RevealFog(guide, x, z)` (`0x0091f420`). The round
   minimap does not hide the unrevealed parts.
6. The blink counter, guide `+0x74`, goes up by one (after its phase is
   read for the blink).

An exception inside the hook turns the Modern Minimap off and the game's
back on for the rest of the run (logged).

## The camera

Up on the map is the camera's forward on the ground: the third column of
`g_CameraMatrix` (`0x02311440`, the view the game renders with, row
vectors). `g_CameraMatrixEye`/`LookAt` (`0x01139e34`/`0x01139e24`,
`MsSetCameraMatrix`) are tried first but are zero in the field. On the
ground (x, z) the world's right of a forward (fx, fz) is (fz, -fx); the
facing at marker `+0x14` is the direction (cos, sin). Seen in the game at
Thunder Plains (developer save 51): the save sphere on the character's
left shows on the left, and walking up turns the arrow up.

## The mesh

The minimap mesh (`rcbgModel`, read again when the guide's mesh object
changes): the vertex-to-world matrix at `+0xc0`, the blob at `+0x14`
(`+4` the packet list, `+8` the vertices, 3 `i16` each, `+0x12` their
count). Packets have a 16-byte header (`u16` `0xffff` ends the list, byte
`+1` the type, `u16` `+2` the count) and records of
`g_GuidePacketStrides` (`0x00c5e790`) bytes; type 0 records hold triangles,
three `u16` vertex indices at `+0xc` (as `updateGuildMapRenderData`,
`0x00717620`, reads them). Of the matrix's two orders the one whose mesh
holds the player wins (the renderer's test, kept). The outline is the
edges that belong to one triangle only. Thunder Plains' first mesh: 192
triangles, 198 outline edges.

## Drawing

`render_imgui` draws on ImGui's background list, under Fahrenheit's
windows, on the game's 16:9 picture (bars on wider screens are left
alone): 14, 18 or 24 % of its height across, 4 % from the corner. 120
world units from the centre to the edge, a grid cell every 30. The ground
triangles are cut to the disc (a 64-sided polygon) and filled without
anti-aliasing so they meet without seams. Markers within the disc sit at
their place; beyond it they sit on the rim and fade out over 1.5 radii
(smoothstep); destinations stay on the rim as an arrow pointing toward
them. The `crt` style adds scanlines every 3 pixels, a refresh band every
4 seconds, a darker rim and a sheen. Colours are the game's minimap's (the
dark green panel, the light green grid, the steel blue walkable area, the
player's yellow in a magenta ring).

The frame from the hook is used for 150 ms; with no newer one (a battle,
a menu, a cutscene without the minimap) nothing is drawn.

Not checked yet: exit lines and destination arrows on a map that has them
close by, the four fog-of-war maps, other window sizes.
