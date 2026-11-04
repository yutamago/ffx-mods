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

    /// <summary>TODrawMenuPlateXYWHType: cdecl(x, y, w, h, type), PS2 screen units (512 x 416).</summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void MenuPlate(float x, float y, float w, float h, int type);

    /// <summary>TOMkpWindowPlain: cdecl(x, y, w, h) ints, PS2 screen units -> eax.</summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int WindowPlain(int x, int y, int w, int h);

    /// <summary>
    /// TODrawMenuPlateXYWHType: the remaster's plates. Type 10 is its message-box plate (the
    /// stone body, a gradient, the HD frame): yiWindowDrawMenu, TOMenuDrawYesNoWindow (the
    /// menus' yes/no questions), the battle-end legend notice. Where the PS2 code of these is
    /// known it drew TOMkpWindowPlain: the yes/no window (SLPM_675.13 0x002362c0, centred,
    /// +0x60 offset and +0x94 icon as on the PC) and kind 0 (above).
    /// </summary>
    private const uint MENU_PLATE = 0x008f5f70;
    private const uint WINDOW_PLAIN = 0x008f9a40;
    private const int MESSAGE_PLATE = 10;

    private static WindowDraw? _draw_menu;
    private static WindowDraw? _draw_plate;
    private static MenuPlate? _plate_hook;
    private static MenuPlate? _plate_next;
    private static WindowPlain? _plain;
    private static int _logged;

    private static void first(int bit, FhLogger log, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) log.Info(message);
    }

    /// <summary>
    /// The message windows drawn as on the PS2: kind 0's draw runs kind 3's (the PS2 plain
    /// window), and the remaster's message-box plate (type 10) becomes the same plain window.
    /// The pieces it draws from meswin are put in by the converter (ui.menus job); without them
    /// the window is invisible, so the converter output must be current.
    /// </summary>
    public static bool menus(DemasterModule m, string value) {
        FhLogger log = m.log;
        _draw_menu = window => {
            // Fahrenheit hands out call pointers only after every module's init.
            _draw_plate ??= Native.fn<WindowDraw>(WINDOW_DRAW_PLATE).fnptr!;
            first(1, log, "ui.menus: message window drawn with the PS2 plate (yiWindowDrawPlate)");
            return _draw_plate(window);
        };
        _plate_hook = (x, y, w, h, type) => {
            if (type == MESSAGE_PLATE) {
                _plain ??= Native.fn<WindowPlain>(WINDOW_PLAIN).fnptr!;
                first(2, log, "ui.menus: message-box plate drawn as the PS2 plain window");
                // the PS2 passes whole units (its windows are i16 rectangles); truncation, as ftol
                _plain((int)x, (int)y, (int)w, (int)h);
                return;
            }
            _plate_next ??= Native.fn<MenuPlate>(MENU_PLATE).chain_from(_plate_hook!).fnptr!;
            _plate_next(x, y, w, h, type);
        };
        return Native.fn<WindowDraw>(WINDOW_DRAW_MENU).hook(m, _draw_menu)
            & Native.fn<MenuPlate>(MENU_PLATE).hook(m, _plate_hook)
            & BattlePatches.windows(m)
            & MainMenuPatches.install(m)
            & BattleSeqPatches.install(m)
            & BattleHudPatches.install(m)
            & BattleLimitPatches.install(m)
            & ItemMenuPatches.install(m)
            & EquipMenuPatches.install(m)
            & StatusMenuPatches.install(m)
            & ConfigMenuPatches.install(m)
            & OverdriveMenuPatches.install(m)
            & CustomizeMenuPatches.install(m);
    }
}
