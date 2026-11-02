// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus: the PS2 window look drawn by the PS2 code FFX.exe still holds. Addresses are
/// ffx-rust's symbols/ffx.sym VAs; evidence in docs/ui.md ("The PS2 windows").
/// </summary>
internal static class MenuPatches {
    /// <summary>A window kind's draw handler (g_Windows[i] +0x78): cdecl(window) -> 0.</summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int WindowDraw(nint window);

    /// <summary>
    /// yiWindowDrawMenu: the draw handler of window kind 0 (g_YiWindowHandlers 0x00c5d788), the
    /// event message windows (dialogue, choices, the field's text boxes). The remaster draws the
    /// HD stone plate here (TODrawMenuPlateXYWHType type 10: shadow, stonetexture body, a
    /// gradient, the HD meswin frame).
    /// </summary>
    private const uint WINDOW_DRAW_MENU = 0x00907060;

    /// <summary>
    /// yiWindowDrawPlate: the draw handler of kinds 3-9, untouched PS2 code: scissor 512 x 416,
    /// TOMkpWindowPlain (0x008f9a40) over the drawn rectangle (+0x48..+0x54): meswin shapes
    /// 200-203 tinted col_mssgwind0 (0x025d16b4) to col_mssgwind1 (0x025d16b0), the colours
    /// TOSeqInitParam reads from the PS2 menu sequence data; then the text scissor (x0 + 12,
    /// y0 + 4, x1 - 12, y1 - 4). The PS2's own kind-0 handler (SLPM_675.13 0x00208758, from its
    /// handler table 0x00323710) is exactly this.
    /// </summary>
    private const uint WINDOW_DRAW_PLATE = 0x00907390;

    private static WindowDraw? _draw_menu;
    private static WindowDraw? _draw_plate;
    private static int _logged;

    /// <summary>
    /// The event message windows drawn as on the PS2: kind 0's draw runs kind 3's (the PS2
    /// plain window). The pieces it draws from meswin are put in by the converter (ui.menus job);
    /// without them the window is invisible, so the converter output must be current.
    /// </summary>
    public static bool menus(DemasterModule m, string value) {
        FhLogger log = m.log;
        _draw_menu = window => {
            // Fahrenheit hands out call pointers only after every module's init.
            _draw_plate ??= Native.fn<WindowDraw>(WINDOW_DRAW_PLATE).fnptr!;
            if (Interlocked.Exchange(ref _logged, 1) == 0)
                log.Info("ui.menus: message window drawn with the PS2 plate (yiWindowDrawPlate)");
            return _draw_plate(window);
        };
        return Native.fn<WindowDraw>(WINDOW_DRAW_MENU).hook(m, _draw_menu);
    }
}
