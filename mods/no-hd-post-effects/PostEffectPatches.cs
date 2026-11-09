// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.PostEffects;

/// <summary>
/// render.glow and render.shadows: hooks on FFX.exe's PhyreEngine side. Addresses are ffx-rust's
/// symbols/ffx.sym virtual addresses (image base 0x00400000); notes in docs/render.md.
/// </summary>
internal static unsafe class PostEffectPatches {
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
    public static bool glow(ModHost m, string value) {
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
    public static bool shadows(ModHost m, string value) {
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

}
