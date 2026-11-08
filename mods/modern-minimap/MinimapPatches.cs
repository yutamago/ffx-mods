// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.ModernMinimap;

/// <summary>
/// The hook on the field minimap's frame, rcbgGuideUpdateEx (0x0091da80, cdecl(guide, show),
/// called by Yn_GuideMapProcEx 0x00919d00 every field frame). With the Modern Minimap on it
/// keeps what the game's minimap does besides drawing (its fade, the fog of war it reveals,
/// the blink counter), switches the game's drawing off, and hands the player, the camera,
/// the markers and the map's mesh to <see cref="MinimapView"/> (docs/minimap.md; the
/// function is ffx-rust's crates/ffx-field/src/guide_update.rs, docs/formats/field.md
/// "Guide map").
/// </summary>
internal static unsafe class MinimapPatches {
    // ---- functions (ffx-rust symbols/ffx.sym) ---------------------------------------------
    private const uint GUIDE_UPDATE   = 0x0091da80; // rcbgGuideUpdateEx(guide, show)
    private const uint MESH_OBJECT    = 0x0091d460; // Guide_MeshObject(guide): +0, else +8
    private const uint FADE_STEP      = 0x0091d280; // Guide_FadeStep(show)
    private const uint FADE_OFF       = 0x0091d3a0; // Guide_FadeOff()
    private const uint SET_ENABLE     = 0x006425f0; // graphicMinimapSetEnable(alpha)
    private const uint FOG_ENABLE     = 0x006425d0; // graphicMinimapFogSetEnable(on)
    private const uint ACTOR_NUMBER   = 0x0086b710; // AtelGetGideMapActorNumber()
    private const uint ACTOR_INFO     = 0x0086b700; // AtelGetGideMapActorInfo(i, out)
    private const uint ACTOR_TYPE     = 0x0086a7b0; // AtelGetActorType(id)
    private const uint ACTOR_START    = 0x0086a720; // AtelGetActorStartPos(id, &x, &y, &z)
    private const uint ACTOR_END      = 0x0086a390; // AtelGetActorEndPos(id, &x, &y, &z)
    private const uint SCENARIO_FLAG  = 0x00787360; // getScenerioFlag()
    private const uint REVEAL_FOG     = 0x0091f420; // Guide_RevealFog(guide, x, z)

    // ---- globals ------------------------------------------------------------------------
    private const uint ENABLED        = 0x00c5e478; // g_GuideEnabled
    private const uint FLAGS          = 0x00c5e350; // g_GuideFlags: bit 0 the minimap may show
    private const uint FADE           = 0x0193bf78; // g_GuideFade, 0..128
    private const uint PALETTE        = 0x00c5e370; // g_GuideMarkerPalette: 16 RGBA f32, 128 = 1
    private const uint PLAYER_COLOUR  = 0x00c5e3a0; // the player triangle's colour
    private const uint CURRENT_MAP    = 0x012fbc40; // g_CurrentMapNo
    private const uint CAMERA_MATRIX  = 0x02311440; // g_CameraMatrix, the game's 4x4 view
    private const uint CAMERA_EYE     = 0x01139e34; // g_CameraMatrixEye (MsSetCameraMatrix)
    private const uint CAMERA_LOOK_AT = 0x01139e24; // g_CameraMatrixLookAt

    // Marker flags (AtelGetGideMapActorInfo out +4).
    public const uint SHAPE = 0xf0300, SQUARE = 0x100, TRIANGLE = 0x200, RING = 0x400, PLAYER = 0x800,
                      BLINK = 0x1000, DESTINATION = 0x2000, EXIT_LINE = 0x10000, DIAMOND = 0x20000;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint GuideUpdate(nint guide, uint show);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate nint PtrOf(nint p);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint U32Of(uint v);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint U32();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint ActorInfo(uint i, byte* out_);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint ActorPos(uint id, float* x, float* y, float* z);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint RevealFog(nint guide, float x, float z);

    private static FhLogger? _log;
    private static GuideUpdate? _hook, _next;
    private static PtrOf? _mesh_object;
    private static U32Of? _fade_step, _set_enable, _fog_enable, _actor_type;
    private static U32? _fade_off, _actor_number, _scenario_flag;
    private static ActorInfo? _actor_info;
    private static ActorPos? _actor_start, _actor_end;
    private static RevealFog? _reveal_fog;

    /// <summary>Whether the Modern Minimap replaces the game's (minimap.style is not "game").</summary>
    public static bool active { get; private set; }
    public static bool crt { get; private set; } = true;
    /// <summary>The diameter, a share of the picture's height.</summary>
    public static float diameter { get; private set; } = 0.18f;
    public static MinimapView.Corner place { get; private set; } = MinimapView.Corner.TopRight;

    public static bool size(ModHost m, string value) {
        switch (value) {
            case "small":  diameter = 0.14f; return true;
            case "medium": diameter = 0.18f; return true;
            case "large":  diameter = 0.24f; return true;
        }
        m.log.Warning($"minimap.size: \"{value}\" is not small, medium or large");
        return false;
    }

    public static bool corner(ModHost m, string value) {
        switch (value) {
            case "top_right":    place = MinimapView.Corner.TopRight;    return true;
            case "top_left":     place = MinimapView.Corner.TopLeft;     return true;
            case "bottom_right": place = MinimapView.Corner.BottomRight; return true;
            case "bottom_left":  place = MinimapView.Corner.BottomLeft;  return true;
        }
        m.log.Warning($"minimap.corner: \"{value}\" is not top_right, top_left, bottom_right or bottom_left");
        return false;
    }

    public static bool style(ModHost m, string value) {
        if (value != "crt" && value != "plain") {
            m.log.Warning($"minimap.style: \"{value}\" is not crt, plain or game");
            return false;
        }
        _log = m.log;
        crt = value == "crt";
        _hook = guide_update;
        active = Native.fn<GuideUpdate>(GUIDE_UPDATE).hook(m, _hook);
        return active;
    }

    private static T fn<T>(ref T? slot, uint va) where T : Delegate => slot ??= Native.fn<T>(va).fnptr!;

    private static uint guide_update(nint guide, uint show) {
        _next ??= Native.fn<GuideUpdate>(GUIDE_UPDATE).chain_from(_hook!).fnptr!;
        if (guide == 0 || Native.u32(ENABLED) == 0) return _next(guide, show);
        if (fn(ref _mesh_object, MESH_OBJECT)(guide) == 0) {
            MinimapView.hide();
            return _next(guide, show);
        }
        if ((Native.u32(FLAGS) & 1) == 0) {
            MinimapView.hide();
            return fn(ref _fade_off, FADE_OFF)();
        }
        try {
            return replace(guide, show);
        }
        catch (Exception e) {
            // Never let an exception into the game's frame: back to the game's minimap.
            active = false;
            _log?.Error($"The Modern Minimap failed and the game's is back: {e}");
            MinimapView.hide();
            return _next(guide, show);
        }
    }

    /// <summary>rcbgGuideUpdateEx without its drawing (see the class).</summary>
    private static uint replace(nint guide, uint show) {
        fn(ref _fade_step, FADE_STEP)(show);
        // The game's minimap mesh and fog stay off; ours is drawn instead.
        uint r = fn(ref _set_enable, SET_ENABLE)(0);
        fn(ref _fog_enable, FOG_ENABLE)(0);
        uint fade = Native.u32(FADE);
        if (fade == 0) {
            MinimapView.hide();
            return r;
        }
        nint mesh = fn(ref _mesh_object, MESH_OBJECT)(guide);
        bool fogged = *(uint*)(guide + 0x70) != 0;

        List<MinimapView.Marker> markers = [];
        MinimapView.Marker? player = null;
        uint n = Math.Min(fn(ref _actor_number, ACTOR_NUMBER)(), 256u);
        byte* info = stackalloc byte[0x40];
        for (uint i = 0; i < n; i++) {
            new Span<byte>(info, 0x40).Clear();
            if (fn(ref _actor_info, ACTOR_INFO)(i, info) == 0) continue;
            uint flags = *(uint*)(info + 4);
            // As the game: on map 0x99 marker 6 is a square of colour 7 at scenario 0x618.
            if (Native.u32(CURRENT_MAP) == 0x99 && i == 6 && (fn(ref _scenario_flag, SCENARIO_FLAG)() & 0xffff) == 0x618)
                flags = 0x107;
            if (flags == 0) continue;
            Vector3 pos = new(*(float*)(info + 8), *(float*)(info + 0xc), *(float*)(info + 0x10));
            if (!float.IsFinite(pos.X) || !float.IsFinite(pos.Z)) continue;
            float heading = *(float*)(info + 0x14);
            Vector2? a = null, b = null;
            if ((flags & SHAPE) == EXIT_LINE) {
                uint id = *(uint*)info;
                uint kind = fn(ref _actor_type, ACTOR_TYPE)(id);
                if (kind == 2 || kind == 5) {
                    float x0, y0, z0, x1, y1, z1;
                    fn(ref _actor_start, ACTOR_START)(id, &x0, &y0, &z0);
                    fn(ref _actor_end, ACTOR_END)(id, &x1, &y1, &z1);
                    a = new(x0, z0);
                    b = new(x1, z1);
                }
            }
            Vector4 colour = (flags & PLAYER) != 0 ? rgba(PLAYER_COLOUR) : rgba(PALETTE + (flags & 0xf) * 16);
            Vector4 ring = rgba(PALETTE + ((flags >> 4) & 0xf) * 16);
            MinimapView.Marker k = new(flags, colour, ring, pos, heading, a, b);
            if ((flags & PLAYER) != 0) player = k;
            else markers.Add(k);
        }
        // The fog of war (four maps) is revealed around the player, as the game does.
        if (player is not null && fogged)
            fn(ref _reveal_fog, REVEAL_FOG)(guide, player.Pos.X, player.Pos.Z);
        // The blink counter.
        *(uint*)(guide + 0x74) += 1;

        MinimapView.update(new MinimapView.Frame(
            Shown:   Math.Min(fade, 128u) / 128f,
            Player:  player,
            Markers: markers,
            Camera:  camera()), mesh, _log);
        return r;
    }

    private static Vector4 rgba(uint va) {
        float* c = (float*)Native.ptr(va);
        return Vector4.Clamp(new Vector4(c[0], c[1], c[2], c[3]) / 128f, Vector4.Zero, Vector4.One);
    }

    /// <summary>
    /// Where the camera looks, on the ground (x, z): from g_CameraMatrix (the view the game
    /// renders with), else from MsSetCameraMatrix's eye and look-at.
    /// </summary>
    private static MinimapView.Camera camera() {
        float* m = (float*)Native.ptr(CAMERA_MATRIX);
        float* eye = (float*)Native.ptr(CAMERA_EYE), at = (float*)Native.ptr(CAMERA_LOOK_AT);
        return new(
            // The view's third column: the camera's z (its forward) in the world (row vectors).
            new Vector3(m[2], m[6], m[10]),
            new Vector3(m[0], m[4], m[8]),
            new Vector3(at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]));
    }
}
