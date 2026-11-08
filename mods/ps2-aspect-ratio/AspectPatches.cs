// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.AspectRatio;

/// <summary>
/// render.aspect: the PS2's 4:3 picture with the PS2 projection. Addresses are ffx-rust's
/// symbols/ffx.sym virtual addresses (image base 0x00400000); notes in docs/screen.md.
/// </summary>
internal static unsafe class AspectPatches {
    private const uint IMAGE_BASE = 0x00400000;

    private static FhLogger? _log;
    private static int _logged;

    /// <summary>Logs a hook's first call (bit = which hook), to show in the log that it runs.</summary>
    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static FhCallHandle<T> at<T>(uint va) where T : Delegate =>
        new(new FhCallLocation("FFX.exe", (nint)(va - IMAGE_BASE)));

    // ---- render.aspect ----------------------------------------------------------------------

    /// <summary>
    /// The f32 16/9 that PostProcessManager_setNon16By9Viewport (0x00698750, its only reader)
    /// fits the last copy of the frame into the back buffer with (pillarbox or letterbox), and
    /// the Iggy menus' viewport rectangle with it. In .data.
    /// </summary>
    private const uint OUTPUT_ASPECT = 0x00c34154;

    /// <summary>PhyreScene_setCameraAspectRatio: thiscall(scene; a) ret 4 (docs/screen.md).</summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate void SetCameraAspectRatio(nint scene, float a);

    private const uint SET_CAMERA_ASPECT = 0x0066cbe0;
    /// <summary>g_PhyreScene3D: 1 in 3D maps and battles, where setCameraAspectRatio widens a by 4/3.</summary>
    private const uint SCENE_3D = 0x00ccb9c0;
    /// <summary>PhyreScene_RenderABmap's fld dword ptr [0x00b41cf0] (16/9, also the UI camera's): the sphere grid camera's aspect.</summary>
    private const uint ABMAP_ASPECT_LOAD = 0x00652a7a;
    private const uint SIXTEEN_NINE = 0x00b41cf0;

    private static SetCameraAspectRatio? _camera_aspect;
    private static nint _four_thirds;

    /// <summary>
    /// The PS2's 4:3 picture, pillarboxed. The frame is still drawn into the game's 16:9 targets,
    /// but anamorphic: the last copy squeezes it into the middle 4:3 of the window (the game's own
    /// path for windows that are not 16:9, with 4/3 as the target aspect). Everything the HD
    /// places in the 16:9 frame from PS2 coordinates (x 3.75, the menus, the message windows,
    /// the battle HUD, the minimap) lands where the PS2 had it; the 3D cameras keep the PS2's
    /// aspect instead of the HD's 4/3 wider one, so the squeeze shows the PS2 frustum; the
    /// fixed-camera rooms' full-screen paintings take their PS2 shape. The Flash (Iggy) screens
    /// get the same 4:3 rectangle from the game and are drawn squeezed into it.
    /// docs/screen.md.
    /// </summary>
    public static bool aspect(ModHost m, string value) {
        _log = m.log;
        float* output = (float*)Native.ptr(OUTPUT_ASPECT);
        if (*output != 16f / 9f && *output != 4f / 3f) {
            m.log.Warning($"render.aspect: 0x{OUTPUT_ASPECT:x8} holds {*output}, not 16/9; not changed");
            return false;
        }

        _four_thirds = Marshal.AllocHGlobal(4);
        *(float*)_four_thirds = 4f / 3f;
        byte[] original = [0xd9, 0x05, .. BitConverter.GetBytes(Native.reloc(SIXTEEN_NINE))];
        byte[] patched  = [0xd9, 0x05, .. BitConverter.GetBytes((uint)_four_thirds)];
        bool abmap = Native.patch_code(ABMAP_ASPECT_LOAD, original, patched, m.log);
        if (!abmap) m.log.Warning("render.aspect: the sphere grid keeps the HD aspect (squeezed)");

        // In 3D scenes the game stores a * 3/4 * 16/9 (the PS2 frustum widened to 16:9); a * 3/4
        // in makes it store a, which the squeeze turns back into the PS2's view. 2D scenes store
        // a as it is, already the PS2's (shown stretched in the HD).
        _camera_aspect = (scene, a) => {
            bool scene3d = Native.u8(SCENE_3D) != 0;
            first(scene3d ? 8 : 16, $"render.aspect: camera aspect {a} in a {(scene3d ? "3D" : "2D")} scene");
            at<SetCameraAspectRatio>(SET_CAMERA_ASPECT).chain_from(_camera_aspect!).fnptr!(scene, scene3d ? a * 0.75f : a);
        };
        if (!at<SetCameraAspectRatio>(SET_CAMERA_ASPECT).hook(m, _camera_aspect)) return false;
        if (!at<UpdateAtelShp>(UPDATE_ATEL_SHP).hook(m, _atel_shp = atel_shp)) return false;

        *output = 4f / 3f;
        return true;
    }

    /// <summary>
    /// updateVFXRenderDataAtelShp: cdecl(id lo, id hi, shapes, record, frame) -> 1/0. Fills the
    /// six render data (id + 10000 k) of one event sprite (g_EventSprites, AtelExecDrawShape;
    /// InitVFXRenderDataAtelShp calls it too). Corners in 1/15 PS2 pixel x h/416, the record's
    /// matrix (identity, the sprite's x/y scale, its PS2 position) with the position x w/512 and
    /// y h/416 (graphicFontGetScreenWH: 1920 x 1080).
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int UpdateAtelShp(uint lo, int hi, nint shapes, nint record, ushort frame);

    /// <summary>graphicVFXGetRenderDataByInstance: cdecl(id lo, id hi) -> render data or 0.</summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate nint GetRenderDataByInstance(uint lo, int hi);

    private const uint UPDATE_ATEL_SHP = 0x007196f0;
    private const uint GET_RENDER_DATA = 0x006453d0;
    /// <summary>
    /// The PS2 x unit over the corners' y unit in the 1920 x 1080 frame: (1920 / 512) / (1080 / 416)
    /// = 13/9. The HD sizes the sprites in square pixels (h/416 both ways) but spreads their
    /// positions over the 16:9 width (w/512); the anamorphic frame needs w/512 for both.
    /// </summary>
    private const float ATEL_X_SCALE = 1920f / 512f / (1080f / 416f);
    /// <summary>
    /// The corners are in GS units, 1/16 pixel, as the PS2 draws them; the HD divides by 15, so
    /// every event sprite is 16/15 too large around its position (measured on the credits).
    /// </summary>
    private const float ATEL_UNIT = 15f / 16f;

    private static UpdateAtelShp? _atel_shp;

    /// <summary>
    /// The event sprites (the opening's staff credits, the 2D pictures of events) in PS2 shape: after
    /// the game filled each render data matrix, its x output column x 13/9 (widths in the PS2 x
    /// unit, w/512, like the positions; the squeeze makes them the PS2's) and both columns x 15/16
    /// (the PS2's 1/16 corner unit). Translation stays. The game copies the matrix afresh on every
    /// call that returns 1 (docs/screen.md, "Event sprites").
    /// </summary>
    private static int atel_shp(uint lo, int hi, nint shapes, nint record, ushort frame) {
        int r = at<UpdateAtelShp>(UPDATE_ATEL_SHP).chain_from(_atel_shp!).fnptr!(lo, hi, shapes, record, frame);
        if (r == 0 || record == 0) return r; // 0: returned before copying the matrices
        first(32, "render.aspect: event sprites in the PS2 size");
        var get = at<GetRenderDataByInstance>(GET_RENDER_DATA).fnptr!;
        ulong id = ((ulong)(uint)hi << 32) | lo;
        for (ulong k = 0; k < 6; k++) {
            ulong i = id + k * 10000;
            float* rd = (float*)get((uint)i, (int)(i >> 32));
            if (rd == null) continue;
            for (int row = 0; row < 3; row++) {
                rd[4 * row]     *= ATEL_X_SCALE * ATEL_UNIT;
                rd[4 * row + 1] *= ATEL_UNIT;
            }
        }
        return r;
    }
}
