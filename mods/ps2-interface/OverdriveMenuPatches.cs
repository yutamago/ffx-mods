// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.menus: the main menu's Overdrive screen drawn by the PS2's draw code (SLPM_675.13 record
/// 0x00325a78, control 0x00230aa0 = TkMenuCtrlLimit) on FFX.exe's primitives. Hooked (PC = PS2):
/// <list type="bullet">
/// <item>the record's draw 0x008d09b0 (record 0x00c5a768 +4) = 0x002312d0: the member's party plate
/// (0x00225f38 -> 0x00200c60) at pos_4nameplt and the Overdrive plate (0x00229d60: mode box, title
/// box, gauge plate, "type" and "OVERDRIVE" labels, the gauge, the mode's name) at rect_2lgplt;</item>
/// <item>MenuLimit_WindowDraw_008d16f0 = 0x00230708: both windows of the screen (+0x5c 1: View, the
/// Overdrive list under its title box rect_lwplt; 0: Set Mode, the mode list under the black plate
/// rect_lmojiplt with strtex label 0), two columns at pos_2llplt / pos_2lrplt, rows 32 apart
/// (0x002305c0, rows 0x00230328), the scroll bar at rect_lscroll, the cursor (0x00230568).</item>
/// </list>
/// The sub-command bar (View / Set Mode) is ItemMenuPatches' (MenuSubCmd_DrawRow), the help bar
/// MainMenuPatches'. Layout words are read by name (TkMn2SeqGet*, scene 1), as both platforms' init
/// functions read them (PS2 0x0022fb38 and the Status init 0x00228c48; PC 0x008d0cd0, 0x008d4920).
/// Evidence: docs/ui.md ("The Overdrive screen").
/// </summary>
internal static unsafe class OverdriveMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Draw();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void SeqRect(nint name, short* r);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int CommandOffset(nint list, int kind, int chr, int n);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Label(int n, float x, float y, float w, float h, int alpha);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void WinScissor(int y, int h, int part, int ratio);

    // hooked
    private const uint RECORD_DRAW = 0x008d09b0; // the Overdrive record's +4 draw (PS2 0x002312d0)
    private const uint WINDOW_DRAW = 0x008d16f0; // MenuLimit_WindowDraw_008d16f0 (PS2 0x00230708)

    // read
    private const uint SEQ_RECT     = 0x008c2b90; // TkMn2SeqGetRect(name, &rect): i16 x, y, w, h
    private const uint SEQ_COLOR    = 0x008c2ad0; // TkMn2SeqGetColor(name)
    private const uint CURRENT      = 0x008a9860; // TkMenuGetCurrentPlayer
    private const uint STATUS_NUM   = 0x00790200; // MsMenuStatusNum(0, &n): the command lists
    private const uint CMD_OFFSET   = 0x008d2dc0; // TkMenuGetCommandOffset(list, 5, chr, 0x48)
    private const uint CMD_NAME     = 0x008c1a20; // TkMn2GetCommandName(id) (PS2 0x002275f8)
    private const uint MENU1_MES    = 0x008ddfc0; // TkMenuMenu1Mes(id) (PS2 0x00249368)
    private const uint NAME_KIND2   = 0x00873b50; // the PC's twin of PS2 0x0014f1c0 (row kind 2)
    private const uint COM_DATA     = 0x0079a4b0; // MsGetComData(id, &strings) -> record (name u16 +0, icon +0x14)
    private const uint OD_ID        = 0x008c1bd0; // TkMn2GetOverDriveId (PS2 0x00225608)
    private const uint LIMIT        = 0x008a9930; // TkMenuGetLimit (PS2 0x00242580)
    private const uint LIMIT_MAX    = 0x008a9950; // TkMenuGetLimitMax (PS2 0x002425a0)
    private const uint CHR_NAME     = 0x008ac850; // TOGetSaveChrName
    private const uint DAMAGE_CLASS = 0x008a99f0; // MenuPlayer_DamageClass
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint SPHERE_LV    = 0x008a9b80; // TkMenuGetSP
    private const uint NEXT_AP      = 0x00784e90; // MsGetNextAP(chr)
    private const uint SAVE_NEXT_AP = 0x007852b0; // MsGetSaveNextAP(chr)
    private const uint LABEL        = 0x008f8d50; // TOMkpShape2ECCCell_008f8d50(n, x, y, w, h, alpha): strtex cell n
    private const uint WIN_SCISSOR  = 0x008c0f70; // TkMn2DrawMenuWindowScissor(y, h, part, ratio) (PS2 0x002274a8)
    private const uint ITEM_LIST    = 0x01597770; // g_TkMenuItemList: { u16 id, u8 kind, u8 usable }
    private const uint PARTY_MASK   = 0x01841c2c; // the front-line bits (as ItemMenuPatches' ability plate)
    // the main menu's layout (TOSeqInitParam, mirrored storage; ItemMenuPatches)
    private const uint NAMEPLT_W = 0x025d1672;
    private const uint COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144;
    private const uint COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;
    private const uint COL_BABPLT0 = 0x025d1690; // col_babplt0 (c1 4 bytes below): plate kind 0x10's pair

    private const int ROW = 32;
    private const uint GREY = Ps2Menu.GREY;
    private const uint SHADOW = Ps2Menu.SHADOW;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static short wi16(nint w, int o) => *(short*)(w + o);
    private static ushort wu16(nint w, int o) => *(ushort*)(w + o);
    private static sbyte wi8(nint w, int o) => *(sbyte*)(w + o);
    private static int wi32(nint w, int o) => *(int*)(w + o);
    private static int call(uint va, int a) => Ps2Menu.fn<Ps2Menu.IntOfInt>(va)(a);

    // ---- the layout, by name (scene 1) ------------------------------------------------------------

    private static (int x, int y) pos(string name) => Ps2Menu.seq_point(name);

    private static (int x, int y, int w, int h) rect(string name) {
        short* r = stackalloc short[4];
        Ps2Menu.fn<SeqRect>(SEQ_RECT)(Ps2Menu.cstr(name), r);
        return (r[0], r[1], r[2], r[3]);
    }

    private static uint colour(string name) => (uint)Ps2Menu.fn<Ps2Menu.IntOfPtr>(SEQ_COLOR)(Ps2Menu.cstr(name));

    // ---- the record's draw: SLPM_675.13 0x002312d0 ------------------------------------------------

    private static int record_draw() => Ps2Menu.packet(() => {
        Ps2Menu.scenes("scene0", "scene14");
        int chr = Ps2Menu.fn<Ps2Menu.IntFn>(CURRENT)();
        var p = pos("pos_4nameplt");
        status_plate(p.x, p.y, chr);
        var g = rect("rect_2lgplt");
        gauge_plate(g.x, g.y, chr);
        first(1, "ui.menus: Overdrive screen's plates drawn the PS2 way");
    });

    private static void status_plate(int x, int y, int chr) => Ps2Menu.party_plate(x, y, chr);

    /// <summary>
    /// PS2 0x00229d60(x, y, chr) (the Status screen's too): the Overdrive plate's pieces at their
    /// layout places moved by (x, y) - rect_lgplt: the mode box rect_ltbplt (col_ltbplt), the title
    /// box rect_ltplt (col_ltplt0/1), the gauge plate rect_lgplt (meswin 0xd8 / 0xd9 in
    /// col_lgplt0/1), shape 0x109 at rect_ltplt + pos_typshape, shape 0x108 at rect_lgplt +
    /// pos_lgshape + (16, 0), the gauge at rect_lg, the mode's name at rect_ltbplt + pos_ltstr.
    /// </summary>
    private static void gauge_plate(int x, int y, int chr) {
        var lgplt = rect("rect_lgplt");
        int dx = x - lgplt.x, dy = y - lgplt.y;
        var ltb = rect("rect_ltbplt");
        uint cb = colour("col_ltbplt");
        Ps2Menu.box(ltb.x + dx, ltb.y + dy, ltb.w, ltb.h, cb, cb);
        var lt = rect("rect_ltplt");
        Ps2Menu.box(lt.x + dx, lt.y + dy, lt.w, lt.h, colour("col_ltplt0"), colour("col_ltplt1"));
        Ps2Menu.plate_rl(0xd8, 0xd9, x, y, lgplt.w, colour("col_lgplt0"), colour("col_lgplt1"));
        var typ = pos("pos_typshape");
        Ps2Menu.shape(0x109, lt.x + typ.x + dx, lt.y + typ.y + dy);
        var lgs = pos("pos_lgshape");
        Ps2Menu.shape(0x108, x + lgs.x + 16, y + lgs.y);
        var lg = rect("rect_lg");
        gauge(lg.x + dx, lg.y + dy, lg.w, lg.h, call(LIMIT, chr), call(LIMIT_MAX, chr));
        var str = pos("pos_ltstr");
        nint name = Ps2Menu.fn<Ps2Menu.PtrOfInt>(MENU1_MES)(call(OD_ID, chr) + 0x1033);
        Ps2Menu.font(name, ltb.x + str.x + dx, ltb.y + str.y + dy, 0);
    }

    /// <summary>
    /// PS2 0x00229828: the gauge filled to f = g / max (4096 = full) of w: col_lmtg0 on the left to
    /// col_lmtg0 .. col_lmtg1 mixed by f on the right; full, all col_lmtgf0. Nothing at 0.
    /// </summary>
    private static void gauge(int x, int y, int w, int h, int g, int max) {
        if (max == 0 || g == 0) return;
        int f = g >= max ? 0x1000 : (g << 12) / max;
        uint c0, c1;
        if (f == 0x1000) c0 = c1 = colour("col_lmtgf0");
        else {
            c0 = colour("col_lmtg0");
            c1 = mix(c0, colour("col_lmtg1"), f);
        }
        Ps2Menu.box(x, y, w * f >> 12, h, c0, c1);
    }

    private static uint mix(uint a, uint b, int f) {
        uint r = 0;
        for (int s = 0; s < 32; s += 8) {
            int v = ((int)(a >> s & 0xff) * (0x1000 - f) + (int)(b >> s & 0xff) * f) / 4096;
            r |= (uint)v << s;
        }
        return r;
    }

    // ---- the windows: SLPM_675.13 0x00230708 ------------------------------------------------------

    private static int window_draw(nint w) => Ps2Menu.packet(() => {
        int chr = Ps2Menu.fn<Ps2Menu.IntFn>(CURRENT)();
        if (wi32(w, 0x5c) != 0) {
            // the Overdrive list: rect_lwplt in col_lwplt0/1, the character's Overdrive command centred
            var r = rect("rect_lwplt");
            Ps2Menu.box(r.x, r.y, r.w, r.h, colour("col_lwplt0"), colour("col_lwplt1"));
            int n;
            nint list = Ps2Menu.fn<Ps2Menu.PtrOfIntPtr>(STATUS_NUM)(0, &n);
            int k = Ps2Menu.fn<CommandOffset>(CMD_OFFSET)(list, 5, chr, 0x48);
            nint name = Ps2Menu.fn<Ps2Menu.PtrOfInt>(CMD_NAME)(*(ushort*)(list + 2 + 4 * k));
            int tw = Ps2Menu.text_width(name);
            Ps2Menu.font(name, r.x + (r.w - tw) / 2, r.y + 4, 0);
        } else {
            // the mode list: PS2 0x00201a70 at rect_lmojiplt: a black plate 0xda / 0xdb 213 wide, strtex label 0
            var r = rect("rect_lmojiplt");
            mode_label(r.x, r.y);
        }
        list(w);
        first(2, "ui.menus: Overdrive / mode list drawn the PS2 way");
    });

    private static void mode_label(int x, int y) {
        Ps2Menu.plate_rl(0xda, 0xdb, x, y, 0xd5, SHADOW, SHADOW);
        Ps2Menu.fn<Label>(LABEL)(0, x + 0x15, y + 5, 170, 14, 0x80);
    }

    /// <summary>PS2 0x002305c0: the rows (two passes while paging), the scroll bar, the cursor.</summary>
    private static void list(nint w) {
        var l = pos("pos_2llplt");
        var rr = pos("pos_2lrplt");
        int top = wi16(w, 0x32), next = wi16(w, 0x34), f = wi16(w, 0x46);
        var win = Ps2Menu.fn<WinScissor>(WIN_SCISSOR);
        if (top == next) {
            win(rr.y, 0xe0, 0, f);
            rows(w, 0, top, l, rr, -ROW);
        } else {
            win(rr.y, 0xe0, 1, f);
            rows(w, 1, top, l, rr, -ROW);
            win(rr.y, 0xe0, 2, f);
            rows(w, 2, next, l, rr, -(f * 0xe0) / 4096 - ROW);
        }
        Ps2Menu.scissor(0, 0, 512, 416);
        var s = rect("rect_lscroll");
        int count = wu16(w, 0x30), shown = wu16(w, 0x3a), total = (count + 1) >> 1;
        if (shown < total) scroll_bar(s.x, s.y, s.w, s.h, top, shown, total);
        if (wi8(w, 0x45) >= 0) {
            int c = wi16(w, 0x48);
            int x = 12 + ((c & 1) != 0 ? rr.x : l.x);
            Ps2Menu.cursor(x, rr.y + 1 + ROW * ((c >> 1) - top));
        }
    }

    /// <summary>PS2 0x00230488: 9 rows from y - 32, left column entries 2 top - 2, ..., right + 1.</summary>
    private static void rows(nint w, int mode, int top, (int x, int y) l, (int x, int y) r, int dy) {
        int y = l.y + dy;
        for (int k = 0; k < 9; k++, y += ROW) {
            row(w, l.x, y, 2 * top - 2 + 2 * k, mode);
            row(w, r.x, y, 2 * top - 1 + 2 * k, mode);
        }
    }

    /// <summary>
    /// PS2 0x00230328: a purple plate (TOAdpShapeByKind kind 0x10: 0xda, 0xdb from x + 16, 209 wide,
    /// col_babplt), meswin 0xd7 at x + pos_pmshape + (0x58, 0), then by the entry's kind: 1 a
    /// command (icon at +12, +1, name at +29, +1), 2 a name at +12, 3 TkMenuMenu1Mes(id + 0x1033)
    /// at +0x1d (the modes). Nothing when the entry's usable byte is 0.
    /// </summary>
    private static void row(nint w, int x, int y, int idx, int mode) {
        if (mode == 0) y -= wi16(w, 0x46) / 128;
        if (idx < 0 || idx >= wu16(w, 0x30)) return;
        uint c0 = Ps2Menu.u32(COL_BABPLT0), c1 = Ps2Menu.u32(COL_BABPLT0 - 4);
        Ps2Menu.shape_wh(0xda, x, y, 0, 0, c0, c0);
        Ps2Menu.shape_wh(0xdb, x + 16, y, 209 - 16, 0, c0, c1); // PS2 0x001f4820: 0xd1 - w(0xda) from x + w(0xda): 209 in all
        var pm = pos("pos_pmshape");
        Ps2Menu.shape(0xd7, x + pm.x + 0x58, y + pm.y);
        byte* e = Native.ptr(ITEM_LIST + 4 * (uint)idx);
        if (e[3] == 0) return;
        int id = *(ushort*)e;
        switch (e[2]) {
            case 1: {
                int strings;
                nint rec = Ps2Menu.fn<Ps2Menu.PtrOfIntPtr>(COM_DATA)(id, &strings);
                Ps2Menu.icon(x + 12, y + 1, *(byte*)(rec + 0x14));
                Ps2Menu.font(strings + *(ushort*)rec, x + 29, y + 1, 0);
                break;
            }
            case 2:
                Ps2Menu.font(Ps2Menu.fn<Ps2Menu.PtrOfInt>(NAME_KIND2)(id), x + 12, y + 1, 0);
                break;
            case 3:
                Ps2Menu.font(Ps2Menu.fn<Ps2Menu.PtrOfInt>(MENU1_MES)(id + 0x1033), x + 0x1d, y + 1, 0);
                break;
        }
    }

    private static void scroll_bar(int x, int y, int w, int h, int top, int shown, int total) => Ps2Menu.scroll_bar(x, y, w, h, top, shown, total);

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(ModHost m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(ModHost m) {
        _log = m.log;
        return hook<Draw>(m, RECORD_DRAW, record_draw)
            & hook<WinDraw>(m, WINDOW_DRAW, window_draw);
    }
}
