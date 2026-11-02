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

    /// <summary>The level of every character light and of the ambient light: the PS2 colours as they are.</summary>
    private const float PS2_LEVEL = 1.0f;

    private static float luminance(float* rgb) => (rgb[0] * 0.3f + rgb[1] * 0.5f + rgb[2] * 0.2f) * PS2_LEVEL;

    /// <summary>
    /// Characters lit by the PS2's light colours and directions: the three directional lights and
    /// the ambient light at level 1 (no HD intensity, glow balance or Zanarkand saturation), and no
    /// HD per-map rotation of the light directions. The PC shader's falloff stays (docs/render.md).
    /// </summary>
    public static bool lighting(DemasterModule m, string value) {
        _log = m.log;
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
}
