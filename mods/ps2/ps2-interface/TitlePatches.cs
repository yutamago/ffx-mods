// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.title: the title menu's choices where the PS2 had them. Evidence: docs/mods/interface.md ("Title
/// screen and logos").
/// <para>
/// The converter (ui.title job, uijob::title_placed) lays the PS2 title picture out so the PC's
/// quad shows it at the PS2's place: its "Neues Spiel" / "Laden" (part of the picture) move from
/// the PC's lines 228 / 255 to the PS2's 280 / 302 (PS2 units). The cursor follows the title menu's
/// message (0x1c7, 0xc7), which the PC lays out with HD numbers: its rectangle from the message's
/// f32 +0x34 (y 211; TkMenuMessageLayout_008de370 also pins x to 0x160 / 0x154 on map 0x17) and
/// its second choice 16 + 21.5 lines below the first (TOLayoutMessageChoices_008b9a60, the
/// constant 0x00b655e8, read nowhere else). Here the layout runs with +0x34 62 lines lower and
/// the constant is 6.0, so the choices are 22 lines apart, as the PS2 picture's lines.
/// </para>
/// <para>
/// The water band: the PC draws an animated band over the top of the title
/// (graphicDrawMainMenuWaterEffect 0x0063e920 -> PhyreScene_drawMainMenuWaveEffect 0x0065b5b0: one of
/// the 70 menu/titl Water_NN textures a frame, 512 x 175 at (0, 0), alpha 100), called from the
/// main loop while g_YnTest20Seen (0x025d5ebc) is set. The PS2 title is a still picture: its water
/// strip and the PAL web-site banner above it are part of the title layer (the ui.title job writes
/// them). The hook skips the band.
/// </para>
/// </summary>
internal static unsafe class TitlePatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Layout(nint message);

    private const uint LAYOUT = 0x008de370;          // TkMenuMessageLayout_008de370(m) -> 0
    private const uint CURRENT_MAP = 0x012fbc40;     // g_CurrentMapNo
    private const uint CHOICE_STEP = 0x00b655e8;     // f64 21.5: the title's extra space per choice
    private const int TITLE_MAP = 0x17;
    private const float DOWN = 62f;                   // PS2 units: the PS2 picture's first menu line - the PC's
    private const double STEP = 6.0;                  // 22 (the PS2 picture's line pitch) - 16 (the PC's line feed)

    private static FhLogger? _log;
    private static int _logged;
    private static Layout? _hook, _next;
    private static Fn.Void? _water;
    private static int _water_logged;

    private const uint WATER = 0x0063e920;           // graphicDrawMainMenuWaterEffect()

    private static void water() {
        if (Interlocked.Exchange(ref _water_logged, 1) == 0) _log?.Info("ui.title: title water band skipped");
    }

    private static int layout(nint m) {
        _next ??= Native.fn<Layout>(LAYOUT).chain_from(_hook!).fnptr!;
        if (Native.u32(CURRENT_MAP) != TITLE_MAP || *(short*)(m + 0x14) != 0x1c7 || *(short*)(m + 0x16) != 0xc7)
            return _next(m);
        if (Interlocked.Exchange(ref _logged, 1) == 0) _log?.Info("ui.title: title menu laid out at the PS2 place");
        float* y = (float*)(m + 0x34);
        float old = *y;
        *y = old + DOWN;
        try { return _next(m); } finally { *y = old; }
    }

    public static bool title(ModHost m, string value) {
        _log = m.log;
        _hook = layout;
        bool ok = Native.patch_code(CHOICE_STEP, BitConverter.GetBytes(21.5), BitConverter.GetBytes(STEP), m.log);
        _water = water;
        ok &= Native.fn<Fn.Void>(WATER).hook(m, _water);
        return ok & Native.fn<Layout>(LAYOUT).hook(m, _hook);
    }
}
