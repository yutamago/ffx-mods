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
        if (!point_lights(m))
            m.log.Warning("render.lighting: point lights stay on every character");
        _rotation = (cl, i, pitch, yaw) => at<SetLightRotation>(SET_LIGHT_ROTATION).chain_from(_rotation!).fnptr!(cl, i, 0f, 0f);
        return at<ApplyIntensityMultiplier>(APPLY_INTENSITY_MULTIPLIER).hook(m, _intensity)
            && at<SetLightRotation>(SET_LIGHT_ROTATION).hook(m, _rotation);
    }

    /// <summary>
    /// Ch_PLightCalc (0x008305a0, from Ch_DrawMain): after summing the point lights in range into
    /// the character's four light slots (colours chr +0x384 + 16 i, directions +0x3c4 + 16 i) it
    /// hands the slot of the last light in range (edi = its colour, ebx = chr) to the shader's
    /// extra light (graphicCharacterLightingSetLightMatrix; LgtMatClr, applied as
    /// max(colour x half-Lambert^2, 1)) when bVar2 ([ebp-0x9c]) is set, else white light from
    /// above (0x00830a80), which changes nothing. The PS2 draw (Sg_LightSetChr, PS2 0x00106810)
    /// uses slot i only where bit i of the character's point-light byte (PC +0x188, PS2 +0x194)
    /// is set, else the global lights; the PC never reads that byte for drawing.
    /// </summary>
    private const uint PLIGHT_TEST = 0x008309b4;   // cmp dword [ebp-0x9c], 0; je 0x00830a80
    private const uint PLIGHT_USE = 0x008309c1;    // the slot goes to the shader
    private const uint PLIGHT_NONE = 0x00830a80;   // white light from above (no point light)

    /// <summary>
    /// The PC's point light reaches a character only where the PS2 would use that slot: a stub
    /// keeps Ch_PLightCalc's own test and takes the no-light branch unless bit slot of chr +0x188
    /// is set. Native code only: Ch_DrawMain may run on the job threads, so no managed code runs
    /// in it (an earlier managed hook on Ch_PLightCalc was suspected of crashes).
    /// </summary>
    private static bool point_lights(DemasterModule m) {
        uint use = Native.reloc(PLIGHT_USE), none = Native.reloc(PLIGHT_NONE), site = Native.reloc(PLIGHT_TEST);
        uint s = Native.alloc_code(46, a => [
            0x83, 0xbd, 0x64, 0xff, 0xff, 0xff, 0x00,                  // cmp dword [ebp-0x9c], 0
            0x0f, 0x84, .. Native.rel32(a + 13, none),                 // je none
            0x8b, 0xc7,                                                // mov eax, edi
            0x2b, 0xc3,                                                // sub eax, ebx
            0x2d, 0x84, 0x03, 0x00, 0x00,                              // sub eax, 0x384
            0xc1, 0xe8, 0x04,                                          // shr eax, 4: the slot
            0x0f, 0xb6, 0x8b, 0x88, 0x01, 0x00, 0x00,                  // movzx ecx, byte [ebx+0x188]
            0x0f, 0xa3, 0xc1,                                          // bt ecx, eax
            0x0f, 0x83, .. Native.rel32(a + 41, none),                 // jnc none
            0xe9, .. Native.rel32(a + 46, use),                        // jmp use
        ]);
        if (s == 0) return false;
        byte[] original = [0x83, 0xbd, 0x64, 0xff, 0xff, 0xff, 0x00, 0x0f, 0x84, 0xbf, 0x00, 0x00, 0x00];
        byte[] patched = [0xe9, .. Native.rel32(site + 5, s), 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90];
        if (!Native.patch_code(PLIGHT_TEST, original, patched, m.log)) return false;
        m.log.Info($"render.lighting: point lights only where the character's slot bit is set (stub 0x{s:x8})");
        return true;
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
        if (!cluster_hook(m)) return false;
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
        return at<CreateSampler>(CREATE_SAMPLER).hook(m, _create_sampler)
            && at<MapSamplerFilter>(MAP_SAMPLER_FILTER).hook(m, _map_filter);
    }

    /// <summary>
    /// The loadCluster hook that keeps the path of the cluster being loaded in <see cref="_cluster"/>
    /// (per thread; loads nest), shared by render.texture_filter and render.texture_quality and
    /// installed once.
    /// </summary>
    private static bool cluster_hook(DemasterModule m) {
        if (_load_cluster is not null) return true;
        _load_cluster = (stream, cluster, path, key) => {
            string? outer = _cluster;
            bool? outer_ours = _ours;
            _cluster = path == 0 ? "" : Marshal.PtrToStringAnsi(path) ?? "";
            _ours = null;
            try {
                return at<LoadCluster>(LOAD_CLUSTER).chain_from(_load_cluster!).fnptr!(stream, cluster, path, key);
            }
            finally {
                _cluster = outer;
                _ours = outer_ours;
            }
        };
        return at<LoadCluster>(LOAD_CLUSTER).hook(m, _load_cluster);
    }

    // ---- render.texture_quality ----------------------------------------------------------------

    /// <summary>
    /// Phyre_PRenderInterface_getTextureQuality: thiscall(render interface 0x00c94f00) -> +0x1340,
    /// the TextureQuality of GameSetting.ini (0 TQ_HIGH, 1 TQ_MEDIUM, 2 TQ_LOW). Its callers,
    /// Phyre_PBinary_loadClusterTextures (inside loadCluster) and
    /// Phyre_PBinary_reloadClusterTextures, drop that many top mip levels of every texture with a
    /// chain (ffx-rust docs/formats/phyre.md, "How the game loads a texture"). 7 bytes:
    /// mov eax, [ecx+0x1340]; ret.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int GetTextureQuality(nint render);

    private const uint GET_TEXTURE_QUALITY = 0x005947d0;
    private static GetTextureQuality? _quality;
    /// <summary>The mod's files, as paths under efl/x, lower case with '/'.</summary>
    private static HashSet<string> _replaced = [];
    /// <summary>Whether the cluster being loaded on this thread is one of the mod's files (null: not looked up yet).</summary>
    [ThreadStatic] private static bool? _ours;
    private static int _kept_full;
    /// <summary>Clusters logged with their decision (the first ones of a session).</summary>
    private const int DEBUG_LINES = 40;

    private static string norm(string path) => path.Replace('\\', '/').ToLowerInvariant();

    /// <summary>
    /// Whether <paramref name="path"/> (as loadCluster gets it) is a file the mod replaces: it, or
    /// what follows one of its '/', is a path under efl/x.
    /// </summary>
    private static bool replaced(string path) {
        string p = norm(path);
        if (_replaced.Contains(p)) return true;
        for (int i = p.IndexOf('/'); i >= 0; i = p.IndexOf('/', i + 1))
            if (_replaced.Contains(p[(i + 1)..])) return true;
        return false;
    }

    /// <summary>
    /// The PS2 textures load at their full size at every texture quality: while a file of the
    /// mod's is loading, the texture quality reads as TQ_HIGH (no mip level dropped); the game's
    /// own files keep the quality of the graphics options.
    /// </summary>
    public static bool texture_quality(DemasterModule m, string value) {
        _log = m.log;
        string root = Path.Combine(m.mod_dir, "efl", "x");
        HashSet<string> files = [];
        if (Directory.Exists(root))
            foreach (string f in Directory.EnumerateFiles(root, "*.phyre", SearchOption.AllDirectories))
                files.Add(norm(Path.GetRelativePath(root, f)));
        _replaced = files;
        _log.Info($"render.texture_quality: {files.Count} phyre files of the mod load at full size");
        if (!cluster_hook(m)) return false;
        _quality = render => {
            int q = at<GetTextureQuality>(GET_TEXTURE_QUALITY).chain_from(_quality!).fnptr!(render);
            if (q == 0 || _cluster is not { } path) return q;
            if (_ours is null) {
                _ours = replaced(path);
                if (Interlocked.Increment(ref _kept_full) <= DEBUG_LINES)
                    _log?.Info($"render.texture_quality: '{path}' {(_ours == true ? "at full size (texture quality " + q + " -> 0)" : "kept at texture quality " + q)}");
            }
            return _ours == true ? 0 : q;
        };
        return at<GetTextureQuality>(GET_TEXTURE_QUALITY).hook(m, _quality);
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
