// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, the Config screen drawn by the PS2's draw code (SLPM_675.13 0x00235a68: the page
/// 0x00235a00, a row 0x00235720) on FFX.exe's primitives, as ItemMenuPatches does for the item
/// lists. The PC kept the PS2's option records (count +4, value +8, enabled +0xc, title u16 +0x18,
/// choice texts u16 every 4 bytes from +0x1c; get/set callbacks +0x10/+0x14) and the layout words
/// of the PS2 init 0x00234a38 (read by TkMn2GetCommonPlate / TkMn2SeqGet* at 0x008cc050), but drew
/// HD rows 740 / n PS2 pixels apart. Hooked: MenuConfig_Draw 0x008cb320 (= 0x00235a68). Each row:
/// the brush plate r_cnameplt (meswin 0xcc stretched, 0xcd) num_cnameplt - 5 pixels below the
/// last, the title at pos_cname, the cursor on the current row, the choice window (meswin
/// 200..203 at pos_cchoplt, rect_cchoplt's size, col_helpplt0 -> 1), the choices centred at
/// (i + 1) w / (n + 1), grey when the row is disabled, the orange brush under the value. The
/// rows are the PC's (its soundtrack row, g_TkMenuConfigLocRow, takes its texts from
/// SaveDataGetLoc 0x1cd / 0x1d0 - 2k / 0x1d1 - 2k as the PC's draw does); the help line of the
/// cursor row's value is set from the draw, as on both. Evidence: docs/ui.md ("The Config screen").
/// </summary>
internal static unsafe class ConfigMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int GetLoc(int id, nint out_);

    // hooked
    private const uint CONFIG_DRAW = 0x008cb320; // MenuConfig_Draw (PS2 0x00235a68)

    // called
    private const uint DRAW_KICK   = 0x008dce50; // TkMenuDrawKick (PS2 0x002492f8; the PC's draw calls it twice first, as the PS2's)
    private const uint CONFIG_MES  = 0x008dc9b0; // TkMenuConfigMes(id) (PS2 0x00249448)
    private const uint CONFIG_HELP = 0x008dc970; // TkMenuConfigHelp(id) (PS2 0x002496b0)
    private const uint SET_HELP    = 0x008aaeb0; // TkMenuSetHelpMessage (PS2 0x002437e0)
    private const uint GET_LOC     = 0x006412d0; // SaveDataGetLoc(id, out): the PC row's texts
    private const uint TEXT_WIDTH  = 0x008c2cb0; // TkMn2StringWidth: low half (the PS2's TkMenuDraw1612Width 0x00226ad8 is broken on the PC)
    private const uint WIN_LAYOUT  = 0x008fa4f0; // TOGetWindowLayout(id): u16 w +8, h +10

    // read
    private const uint LOC_ROW   = 0x0186a438; // g_TkMenuConfigLocRow
    private const uint CURSOR    = 0x0186a444; // g_TkMenuConfigCursor (i8; PS2 0x00693f68)
    private const uint ROWS      = 0x0186a448; // g_TkMenuConfigRows (PS2 0x00693f6c)
    private const uint ROW_LIST  = 0x0186a44c; // g_TkMenuConfigRowList (PS2 0x00693f70)
    private const uint HELP_TEXT = 0x0186a4a0; // g_TkMenuConfigHelpText
    // the layout (TkMn2SeqGet*: rect x, y, w, h and point x, y in order; PS2 0x00693f78..)
    private const uint PLATE        = 0x0186a450; // r_cnameplt: rect, c_cnameplt0 +8, c_cnameplt1 +0xc, shapes 0xcc +0x10, 0xcd +0x14
    private const uint RECT_CCHOPLT = 0x0186a468;
    private const uint POS_CCHOPLT  = 0x0186a470;
    private const uint POS_CNAME    = 0x0186a480;
    private const uint NUM_CNAMEPLT = 0x0186a48c;
    private const uint COL_HELPPLT0 = 0x0186a498, COL_HELPPLT1 = 0x0186a49c;

    private const uint GREY = 0x80808080;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];
    private static nint _loc;

    private static short i16(uint va) => Native.i16(va);
    private static uint u32(uint va) => Native.u32(va);
    private static int ri32(nint p, int o) => *(int*)(p + o);

    private static nint mes(int id) => Ps2Menu.fn<Ps2Menu.PtrOfInt>(CONFIG_MES)(id);
    private static nint loc(int id, nint buf) {
        Ps2Menu.fn<GetLoc>(GET_LOC)(id, buf);
        return buf;
    }
    private static int width(nint text) => (short)(Ps2Menu.fn<Ps2Menu.IntOfPtr>(TEXT_WIDTH)(text) & 0xffff);

    // ---- the draw: SLPM_675.13 0x00235a68 -----------------------------------------------------

    private static int config_draw(nint w) {
        var kick = Ps2Menu.fn<Ps2Menu.IntFn>(DRAW_KICK);
        kick();
        kick();
        return Ps2Menu.packet(() => {
            int rows = (int)u32(ROWS);
            for (int r = 0; r < rows; r++) row(r);
            if ((Interlocked.Or(ref _logged, 1) & 1) == 0) _log?.Info("ui.menus: config screen drawn the PS2 way");
        });
    }

    /// <summary>One row (PS2 0x00235720(0, 0, row)).</summary>
    private static void row(int r) {
        nint rec = *(nint*)(*(nint*)Native.ptr(ROW_LIST) + 4 * r);
        bool pc = r == (int)u32(LOC_ROW);
        int cur = *(sbyte*)Native.ptr(CURSOR);
        if (_loc == 0) _loc = Marshal.AllocHGlobal(256);
        // the plate (PS2 0x00226f30 = TkMn2DrawCommonPlateR: A stretched c0 -> c1, B in c1)
        int px = i16(PLATE), py = i16(PLATE + 2) + r * ((int)u32(NUM_CNAMEPLT) - 5);
        Ps2Menu.plate_lr((int)u32(PLATE + 0x10), (int)u32(PLATE + 0x14), px, py, i16(PLATE + 4), u32(PLATE + 8), u32(PLATE + 0xc));
        // the title (PS2 0x00226358 = TkMn2DrawMesL: the 16x12 font)
        int nx = px + i16(POS_CNAME), ny = py + i16(POS_CNAME + 2);
        Ps2Menu.font(pc ? loc(0x1cd, _loc) : mes(*(ushort*)(rec + 0x18)), nx, ny, 0);
        if (r == cur) Ps2Menu.cursor(nx, ny);
        // the choice window (PS2 0x00226fe8 = TkMn2DrawCommonWindow)
        int wx = px + i16(POS_CCHOPLT), wy = py + i16(POS_CCHOPLT + 2);
        int ww = i16(RECT_CCHOPLT + 4), wh = i16(RECT_CCHOPLT + 6);
        window(wx, wy, ww, wh, u32(COL_HELPPLT0), u32(COL_HELPPLT1));
        int n = ri32(rec, 4), value = ri32(rec, 8);
        bool on = ri32(rec, 0xc) != 0;
        int ty = wy + i16(POS_CCHOPLT + 2);
        for (int i = 0; i < n; i++) {
            int id = *(ushort*)(rec + 0x1c + 4 * i);
            nint text = pc ? loc(0x1d0 - 2 * i, _loc) : mes(id);
            int tw = width(text);
            int tx = wx + (i + 1) * ww / (n + 1) - tw / 2;
            Ps2Menu.font(text, tx, ty, on ? 0 : 1);
            if (i == value && on) brush(r, tx, ty, tw);
            if (r == cur && i == value) {
                var set = Ps2Menu.fn<Ps2Menu.IntOfPtr>(SET_HELP);
                if (pc) set(loc(0x1d1 - 2 * i, (nint)Native.ptr(HELP_TEXT)));
                else set(Ps2Menu.fn<Ps2Menu.PtrOfInt>(CONFIG_HELP)(id));
            }
        }
    }

    /// <summary>
    /// PS2 0x001fa328 (TkMn2DrawCommonWindow -> 0x008f1e40): meswin 200..203 with the corner size
    /// of 200: the corner in c0, the top stretched c0 -> c1, the left side, the body.
    /// </summary>
    private static void window(int x, int y, int w, int h, uint c0, uint c1) {
        nint l = Ps2Menu.fn<Ps2Menu.PtrOfInt>(WIN_LAYOUT)(200);
        int cw = *(ushort*)(l + 8), ch = *(ushort*)(l + 10);
        Ps2Menu.shape_wh(200, x, y, 0, 0, c0, c0);
        Ps2Menu.shape_wh(201, x + cw, y, w - cw, 0, c0, c1);
        Ps2Menu.shape_wh(202, x, y + ch, 0, h - ch, c0, c0);
        Ps2Menu.shape_wh(203, x + cw, y + ch, w - cw, h - ch, c0, c1);
    }

    private struct Brush { public bool on; public int x, y, w, p, steps; }
    private static readonly Brush[] _brush = new Brush[16];

    /// <summary>
    /// The value's brush (PS2 0x00225bf0: 0x00201d50 then 0x00201f58, slot = row): a new place
    /// starts it over, w / 14 + 2 frames; drawn (p / steps)^2 of the width: meswin 0xe2 at x - 5,
    /// 0xe3 from x + 11 stretched, 0xe4 at its end.
    /// </summary>
    private static void brush(int slot, int x, int y, int w) {
        ref Brush b = ref _brush[slot & 15];
        if (!b.on || b.x != x || b.y != y) b = new Brush { on = true, x = x, y = y, w = w, p = 0, steps = w / 14 + 2 };
        if (b.p < b.steps) b.p++;
        float f = (float)b.p / b.steps;
        int dw = (int)(b.w * (f * f));
        Ps2Menu.shape(0xe2, x - 5, y);
        if (dw > 11) Ps2Menu.shape_wh(0xe3, x + 11, y, dw - 11, 0, GREY, GREY);
        Ps2Menu.shape(0xe4, x + dw, y);
    }

    public static bool install(DemasterModule m) {
        _log = m.log;
        WinDraw d = config_draw;
        _keep.Add(d);
        return Native.fn<WinDraw>(CONFIG_DRAW).hook(m, d);
    }
}
