// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// The render.* code patches: hooks on FFX.exe's PhyreEngine side. Addresses are ffx-rust's
/// symbols/ffx.sym virtual addresses (image base 0x00400000); notes in docs/render.md.
/// </summary>
internal static unsafe class RenderPatches {
    private const uint IMAGE_BASE = 0x00400000;

    private static FhLogger? _log;
    private static int _logged;

    /// <summary>Logs a hook's first call (bit = which hook), to show in the log that it runs.</summary>
    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static FhCallHandle<T> at<T>(uint va) where T : Delegate =>
        new(new FhCallLocation("FFX.exe", (nint)(va - IMAGE_BASE)));

    // ---- render.glow ---------------------------------------------------------------------------

    /// <summary>
    /// Phyre_PPostProcessing_PLegacyGlowGPUBase_render: thiscall(glow; renderer, target, source,
    /// 0, 0) ret 0x14 -> 0 or an error. The HD glow: three levels of Gaussian blur of the glow map,
    /// the last one added (ONE + ONE) into the scene colour. Its only caller is
    /// PostProcessManager_render (and the jump 0x00694f00).
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int GlowRender(nint glow, nint renderer, nint target, nint source, int a, int b);

    private const uint GLOW_RENDER = 0x00a1de20;
    private static GlowRender? _glow;

    /// <summary>
    /// Skips the glow's blur-and-add, so nothing is added to the scene. The glow map pass before it
    /// (PostProcessManager_prepareGlowMap) only writes post-process target +0xa4, which the
    /// brightness pass overwrites in the same frame. The glow switch byte (+0x123b3) stays as the
    /// game sets it: it also gates the resizing of the post-process targets.
    /// </summary>
    public static bool glow(DemasterModule m, string value) {
        _log = m.log;
        _glow = (glow, renderer, target, source, a, b) => {
            first(1, "render.glow: the glow pass is skipped");
            return 0;
        };
        return at<GlowRender>(GLOW_RENDER).hook(m, _glow);
    }

    // ---- render.shadows ------------------------------------------------------------------------

    /// <summary>
    /// ShadowManager_selectShadowSolution: thiscall(sm; six words: ffx.sym reads them as scene
    /// +0x93c, battle flag, map id, level hash, scenario, scene +0xd0, the logged values suggest
    /// another order) ret 0x18 -> 0-4, stored at scene +0xf9cc by
    /// PhyreScene_fieldMapInitialize. 0: no shadow maps, the ground-shadow cluster (one soft shadow
    /// under each character) in the characters' world; 2 (every battle, 147 field maps) and 4 (27
    /// maps, no ground shadow at all) add cascaded shadow maps.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int SelectShadowSolution(nint sm, int a1, int a2, int a3, int a4, int a5, int a6);

    /// <summary>PhyreScene_ForceSetShadowSolution: thiscall(scene; v) ret 4, scene +0xf9cc = v (ShadowManager_ResetSolution: 0 or 2).</summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate void ForceSetShadowSolution(nint scene, int v);

    private const uint SELECT_SHADOW_SOLUTION = 0x006ccd10;
    private const uint FORCE_SET_SHADOW_SOLUTION = 0x006513f0;
    private static SelectShadowSolution? _select;
    private static ForceSetShadowSolution? _force;

    /// <summary>Every map and battle gets shadow solution 0: ground shadows, no shadow maps.</summary>
    public static bool shadows(DemasterModule m, string value) {
        _log = m.log;
        _select = (sm, a1, a2, a3, a4, a5, a6) => {
            // The original still runs: it records the level (+0x4050) and its cascade constants,
            // which nothing reads under solution 0.
            int game = at<SelectShadowSolution>(SELECT_SHADOW_SOLUTION).chain_from(_select!).fnptr!(sm, a1, a2, a3, a4, a5, a6);
            _log?.Info($"render.shadows: shadow solution {game} -> 0 (arguments {a1:x} {a2:x} {a3:x} {a4:x} {a5:x} {a6:x})");
            return 0;
        };
        _force = (scene, v) => at<ForceSetShadowSolution>(FORCE_SET_SHADOW_SOLUTION).chain_from(_force!).fnptr!(scene, 0);
        return at<SelectShadowSolution>(SELECT_SHADOW_SOLUTION).hook(m, _select)
            && at<ForceSetShadowSolution>(FORCE_SET_SHADOW_SOLUTION).hook(m, _force);
    }

    // ---- render.lighting -----------------------------------------------------------------------

    /// <summary>
    /// Phyre_CharacterLighting_applyIntensityMultiplier: thiscall(cl; i, flag) ret 8. Sets light i's
    /// level (PLight +0x24; the shader's Light_i.m_colorIntensity is colour x level) to
    /// (1 - glow factor) x 1.2 x map intensity x 1.16 x 0.95, and for i = 3 the ambient level
    /// (+0x1a4, x 1.05 instead) and the ambient colour x level into the lighting record +0xac
    /// (CharacterAmbientLight). On znkd08/09/10/14 it also saturates the colours (x 2.6).
    /// Every light colour change (the PS2 code's Sg_LightSetColor) goes through it.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate nint ApplyIntensityMultiplier(nint cl, int i, int flag);

    /// <summary>CharacterLighting setRotation: thiscall(cl; i, pitch, yaw) ret 0xc, the HD maps' per-map turn of light i (degrees).</summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate void SetLightRotation(nint cl, int i, float pitch, float yaw);

    private const uint APPLY_INTENSITY_MULTIPLIER = 0x006a6e90;
    private const uint SET_LIGHT_ROTATION = 0x006aaea0;
    private static ApplyIntensityMultiplier? _intensity;
    private static SetLightRotation? _rotation;

    /// <summary>
    /// The level of every character light and of the ambient light. The PS2's VU1 program turns
    /// the lit colour (colour matrix x max(0, light matrix x N) + ambient, the script colours as
    /// byte / 255) into the vertex colour by vf8 = material colour 0x7f x shade x 2
    /// (FUN_008437f0 in FFX.exe, the leftover packet code), and the GS modulates the texture by
    /// vertex colour / 128: the texture x 254/128 x lit. docs/render.md.
    /// </summary>
    private const float PS2_LEVEL = 254f / 128f;

    private static float luminance(float* rgb) => (rgb[0] * 0.3f + rgb[1] * 0.5f + rgb[2] * 0.2f) * PS2_LEVEL;

    /// <summary>
    /// The f64 0.208125 that Phyre_CharacterLighting_setShadeCoeff (0x006aaef0, its only reader)
    /// multiplies the ambient intensity by into CharacterShadeCoeff: the shaders add
    /// CharacterShadeColor x CharacterShadeCoeff to every lit pixel, an HD fill light.
    /// </summary>
    private const uint SHADE_COEFF_SCALE = 0x00b48300;

    /// <summary>
    /// Ch_DrawMain's light-value term (0x0082f64c, with g_ChrShadeByLight, on in the shipped exe):
    /// fld [chr+0x36c]; fmul qword [0.85]; fadd qword [0.15]; the HD keeps 15% of the shade when a
    /// script dims a character's light value. The PS2 (Ch_DrawCalcShade) multiplies by it as it is.
    /// </summary>
    private const uint LIGHT_VALUE_SCALE = 0x0082f652;
    private const uint K_085 = 0x00b501a8;
    private const uint K_015 = 0x00b59870;

    /// <summary>
    /// Characters lit the PS2's way: the three directional lights and the ambient light at the
    /// PS2's level (no HD intensity, glow balance or Zanarkand saturation), no HD per-map rotation
    /// of the light directions, no HD shade-coefficient fill light, the script's light value as a
    /// plain multiplier. The PS2's falloff (max(0, N.L)) is in the shader files of the
    /// render.lighting converter job (docs/render.md).
    /// </summary>
    public static bool lighting(DemasterModule m, string value) {
        _log = m.log;
        if (!Native.patch_code(SHADE_COEFF_SCALE, [0, 0, 0, 0x10, 0xd7, 0xa3, 0xca, 0x3f], new byte[8], m.log))
            m.log.Warning("render.lighting: the HD shade coefficient stays");
        byte[] k085 = BitConverter.GetBytes(Native.reloc(K_085));
        byte[] k015 = BitConverter.GetBytes(Native.reloc(K_015));
        byte[] original = [0xdc, 0x0d, .. k085, 0xdc, 0x05, .. k015];
        if (!Native.patch_code(LIGHT_VALUE_SCALE, original, [0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90], m.log))
            m.log.Warning("render.lighting: the HD light value scale (0.85 v + 0.15) stays");
        _intensity = (cl, i, flag) => {
            first(2, $"render.lighting: light levels set to {PS2_LEVEL} (first call: light {i})");
            nint r = at<ApplyIntensityMultiplier>(APPLY_INTENSITY_MULTIPLIER).chain_from(_intensity!).fnptr!(cl, i, flag);
            byte* c = (byte*)cl;
            if (i is >= 0 and < 3) {
                nint set = *(nint*)(c + 0xf0);
                if (set == 0) return r;
                nint lights = *(nint*)((byte*)set + 4);
                if (lights == 0) return r;
                nint light = *(nint*)((byte*)lights + i * 4);
                if (light == 0) return r;
                *(float*)((byte*)light + 0x24) = PS2_LEVEL;
                *(float*)(c + 0x1d0 + i * 4) = luminance((float*)(c + 0x124 + i * 0x10));
            }
            else if (i == 3) {
                *(float*)(c + 0x1a4) = PS2_LEVEL;
                float* ambient = (float*)(c + 0x190);
                nint record = *(nint*)(c + 0x18c);
                if (record != 0) {
                    float* dst = (float*)((byte*)record + 0xac);
                    dst[0] = ambient[0] * PS2_LEVEL;
                    dst[1] = ambient[1] * PS2_LEVEL;
                    dst[2] = ambient[2] * PS2_LEVEL;
                }
                *(float*)(c + 0x1dc) = luminance(ambient);
            }
            return r;
        };
        _rotation = (cl, i, pitch, yaw) => at<SetLightRotation>(SET_LIGHT_ROTATION).chain_from(_rotation!).fnptr!(cl, i, 0f, 0f);
        return at<ApplyIntensityMultiplier>(APPLY_INTENSITY_MULTIPLIER).hook(m, _intensity)
            && at<SetLightRotation>(SET_LIGHT_ROTATION).hook(m, _rotation);
    }

    // ---- render.texture_filter -----------------------------------------------------------------

    /// <summary>
    /// Phyre_PSerialization_PBinary_loadCluster: cdecl(stream, cluster, path, key) -> 0 or an
    /// error. Every Direct3D object of a cluster (textures, buffers, shaders, its samplers) is made
    /// inside this call, on the calling thread (ffx-rust docs/formats/phyre.md).
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int LoadCluster(nint stream, nint cluster, nint path, nint key);

    /// <summary>
    /// PSamplerStateD3D11_create: thiscall(sampler) -> 0/0xe/0x16. Fills a D3D11_SAMPLER_DESC from
    /// the PSamplerState (+0 min filter, +1 mag filter, +2..+4 address modes, +0x1c bit 6
    /// comparison) and calls ID3D11Device::CreateSamplerState.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int CreateSampler(nint sampler);

    /// <summary>
    /// PRendererD3D11_mapSamplerFilter: cdecl(min, mag, comparison) -> D3D11_FILTER. Returns
    /// ANISOTROPIC (0x55) for every sampler when the Anisotropic option (the f32 at 0x00c25370)
    /// is above 1, whatever the filters say.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int MapSamplerFilter(int min, int mag, int comparison);

    private const uint LOAD_CLUSTER = 0x0057bce0;
    private const uint CREATE_SAMPLER = 0x00595bf0;
    private const uint MAP_SAMPLER_FILTER = 0x0058aa10;
    /// <summary>D3D11_FILTER_MIN_MAG_POINT_MIP_LINEAR: texels unfiltered, mip levels blended.</summary>
    private const int FILTER_POINT_MIP_LINEAR = 0x01;

    private static LoadCluster? _load_cluster;
    private static CreateSampler? _create_sampler;
    private static MapSamplerFilter? _map_filter;
    [ThreadStatic] private static string? _cluster;
    [ThreadStatic] private static bool _point;
    private static readonly HashSet<string> _logged_clusters = [];

    /// <summary>
    /// The clusters whose samplers get point sampling: models, maps and effects, the 3D scene.
    /// The 2D rooms' paintings (map/.../2d/), menus, fonts and the PC's own screens keep theirs;
    /// samplers the engine makes outside cluster loads (post-process, shadows, glow, text) too.
    /// </summary>
    private static bool scene_cluster(string path) {
        string p = path.Replace('\\', '/').ToLowerInvariant();
        if (p.Contains("/2d/") || p.Contains("/menu") || p.Contains("/font") || p.Contains("/shaders/")) return false;
        return p.Contains("/chr/") || p.Contains("/map/") || p.Contains("/btlmap/") || p.Contains("/battle/")
            || p.Contains("/effect/") || p.Contains("/magic/") || p.Contains("/event/") || p.Contains("/obj/");
    }

    /// <summary>
    /// Nearest-neighbour texels on the 3D scene's materials: their samplers are made with
    /// MIN_MAG_POINT_MIP_LINEAR instead of the game's (anisotropic or trilinear) filter.
    /// </summary>
    public static bool texture_filter(DemasterModule m, string value) {
        _log = m.log;
        _load_cluster = (stream, cluster, path, key) => {
            string? outer = _cluster;
            _cluster = path == 0 ? "" : Marshal.PtrToStringAnsi(path) ?? "";
            try {
                return at<LoadCluster>(LOAD_CLUSTER).chain_from(_load_cluster!).fnptr!(stream, cluster, path, key);
            }
            finally {
                _cluster = outer;
            }
        };
        _create_sampler = sampler => {
            byte* s = (byte*)sampler;
            bool point = _cluster is { } path && (s[0x1c] & 0x40) == 0 && scene_cluster(path);
            // One line for the first sampler of each of the first 200 clusters (and of the engine's own).
            bool first_of_cluster;
            lock (_logged_clusters) first_of_cluster = _logged_clusters.Count < 200 && _logged_clusters.Add(_cluster ?? "");
            if (first_of_cluster)
                _log?.Info($"render.texture_filter: sampler min {s[0]} mag {s[1]} address {s[2]} {s[3]} {s[4]} flags {s[0x1c]:x2} cluster '{_cluster}' -> {(point ? "point" : "kept")}");
            _point = point;
            try {
                return at<CreateSampler>(CREATE_SAMPLER).chain_from(_create_sampler!).fnptr!(sampler);
            }
            finally {
                _point = false;
            }
        };
        _map_filter = (min, mag, comparison) => _point
            ? FILTER_POINT_MIP_LINEAR
            : at<MapSamplerFilter>(MAP_SAMPLER_FILTER).chain_from(_map_filter!).fnptr!(min, mag, comparison);
        return at<LoadCluster>(LOAD_CLUSTER).hook(m, _load_cluster)
            && at<CreateSampler>(CREATE_SAMPLER).hook(m, _create_sampler)
            && at<MapSamplerFilter>(MAP_SAMPLER_FILTER).hook(m, _map_filter);
    }

    // ---- render.aspect -------------------------------------------------------------------------

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
    public static bool aspect(DemasterModule m, string value) {
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

        *output = 4f / 3f;
        return true;
    }
}
