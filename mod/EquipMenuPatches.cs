// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, the Equip screen drawn by the PS2's draw code ported onto FFX.exe's primitives, as
/// ItemMenuPatches does for the item lists. Hooked (PC = SLPM_675.13):
/// <list type="bullet">
/// <item>MenuEquip_Draw_008cf670 (the equip menu record's +4, 0x00c5a748) = 0x0022cc28: the status
/// page (MenuStatus_DrawPage = 0x0022a770, called as the PC calls it: whatever draws it, HD or a
/// port hooking it, draws it here), the two label plates (0x00226218: black plate, strtex labels 6
/// "Equipment" and 7 "Abilities"), the weapon / armour cursor (0x0022bbd0) and the picked gear's
/// ability slots (0x0022baa8);</item>
/// <item>MenuWeapon_DrawList_008cf9f0 (the weapon list window's +0x10) = 0x0022bdc0: 5 rows of 32
/// (0x0022bc20: plate kind 0, the dotted line 0xd7, icon and name, the equipped mark), the cursor,
/// the scroll bar.</item>
/// </list>
/// Logic and input stay the PC's. Evidence and the open points: docs/ui.md ("The Equip screen").
/// </summary>
internal static unsafe class EquipMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Draw();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Page(int x, int y, int chr, int page, nint a, nint b, nint c, nint d, int t);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void SheetCell(int cell, float x, float y, float w, float h);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void WinScissor(int y, int h, int part, int ratio);

    // hooked
    private const uint EQUIP_DRAW = 0x008cf670; // MenuEquip_Draw_008cf670 (PS2 0x0022cc28)
    private const uint LIST_DRAW  = 0x008cf9f0; // MenuWeapon_DrawList_008cf9f0 (PS2 0x0022bdc0)

    // called
    private const uint KICK_MENU   = 0x008dce50; // TkMenuDrawKick
    private const uint VU1_THUNK   = 0x008c1790; // TkVU1SyncPath_thunk_008c1790
    private const uint INIT_PKT    = 0x008c0be0; // TkMn2DrawInitPacket
    private const uint STATUS_PAGE = 0x008d4ee0; // MenuStatus_DrawPage (PS2 0x0022a770)
    private const uint SHEET_CELL  = 0x008f8bb0; // TODrawSheetCell_008f8bb0: strtex cell (PS2 0x00201768)
    private const uint WIN_SCISSOR = 0x008c0f70; // TkMn2DrawMenuWindowScissor (PS2 0x002274a8)
    private const uint CURRENT     = 0x008a9860; // TkMenuGetCurrentPlayer
    private const uint SAVE_WEAPON = 0x007abbd0; // MsGetSaveWeapon(id, &name or 0)
    private const uint WEAPON_NAME = 0x007abdf0; // MsGetSaveWeaponName(id)
    private const uint ROM_ABILITY = 0x00790970; // MsGetRomAbility(id, &strings) -> record (name u16 +0, icon +0x68)

    // read (PS2 -> PC)
    private const uint FADE       = 0x0186a614; // g_TkMenuEquipFade (PS2 0x00693aa0)
    private const uint ALT        = 0x0186a624; // g_TkMenuEquipAltMode: armour row (PS2 0x00693ab4)
    private const uint LIST_OPEN  = 0x0186a62c; // g_TkMenuEquipListOpen (PS2 0x00693abc)
    private const uint ITEM       = 0x0186a630; // g_TkMenuEquipItem (PS2 0x00693ac0)
    private const uint CUR_A = 0x023cc3c8, CUR_B = 0x023cc3c4, CUR_C = 0x023cc3c0, CUR_D = 0x023cc3bc; // the page's cursor words (PS2 0x003258c8..d4)
    // TkMn2SeqGetRect / TkMn2SeqGetPos records (i16 x, y[, w, h], not mirrored), code_008cf820's
    private const uint R_SOMOJIPLT  = 0x0186a634; // (PS2 0x00693ac8)
    private const uint POS_SOBIPLT  = 0x0186a63c; // (PS2 0x00693ad0)
    private const uint R_SOMOJIPLT2 = 0x0186a640; // (PS2 0x00693ad8)
    private const uint POS_SOABPLT  = 0x0186a648; // (PS2 0x00693ae0)
    private const uint R_SOSCROL    = 0x0186a64c; // (PS2 0x00693ae8)
    // TkMn2GetStatusPos's
    private const uint POS_PMSHAPE = 0x023cc420; // pos_pmshape (PS2 0x00325878)
    private const uint WPNLPLT     = 0x0186a900; // rect_wpnlplt's plate record; the armour row's at +0x48 (PS2 0x00693960 / 0x006939a8)
    private const uint POS_WPNMOJI = 0x0186a86c; // pos_wpnmoji (PS2 0x006938b8)
    private const uint COL_BSOPLT0 = 0x025d16a0; // plate pairs bso, bitem, bab, lmt (c0, c1 4 bytes apart)
    private const uint ITEM_LIST   = 0x01597770; // g_TkMenuItemList: { u16 id, u8, u8 }

    private const int ROW = 32;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static (int x, int y) xy(uint va) => (Ps2Menu.i16(va), Ps2Menu.i16(va + 2));
    private static short wi16(nint w, int o) => *(short*)(w + o);
    private static ushort wu16(nint w, int o) => *(ushort*)(w + o);

    private static bool blink_off() => ((Ps2Menu.timer() / 2) & 1) != 0;

    // ---- shared pieces ----------------------------------------------------------------------------

    /// <summary>A row plate by kind (PS2 TOAdpShapeByKind 0x001f4c60), as ItemMenuPatches.plate.</summary>
    private static void plate(int x, int y, int kind) {
        uint at = COL_BSOPLT0 - 8 * (uint)((kind >> 3) & 3);
        uint c0 = Ps2Menu.u32(at), c1 = Ps2Menu.u32(at - 4);
        Ps2Menu.shape_wh(0xda, x, y, 0, 0, c0, c0);
        Ps2Menu.shape_wh(0xdb, x + 16, y, 209, 0, c0, c1);
        int extra = (kind & 3) switch { 1 => 0xd7, 2 => 0x10b, 3 => 0x10a, _ => 0 };
        if (extra != 0) Ps2Menu.shape_c2(extra, x + 97, y, Ps2Menu.SHADOW, Ps2Menu.SHADOW);
    }

    /// <summary>PS2 0x001f5298(id, x, y, 0): the gear's icon (+12, +1) and name (+29, +1), icon 0x31 (the mark) when its wearer is its owner.</summary>
    private static void weapon_name(int id, int x, int y) {
        byte* r = (byte*)Ps2Menu.fn<Ps2Menu.PtrOfInt2>(SAVE_WEAPON)(id, 0);
        nint name = Ps2Menu.fn<Ps2Menu.PtrOfInt>(WEAPON_NAME)(id);
        Ps2Menu.icon(x + 12, y + 1, (r[4] * 2 + r[5] + 1) & 0xff);
        Ps2Menu.font(name, x + 29, y + 1, 0);
        if (r[4] == r[6]) Ps2Menu.icon(x + 12, y + 1, 0x31);
    }

    // ---- the screen: SLPM_675.13 0x0022cc28 ---------------------------------------------------------

    private static int equip_draw() {
        // the PC's packet steps before its page (TODrawMenuBG and MenuDrawDividerShapes(4) dropped:
        // the PS2 has the menu gradient only)
        Ps2Menu.fn<Ps2Menu.IntFn>(KICK_MENU)();
        Ps2Menu.fn<Ps2Menu.IntFn>(KICK_MENU)();
        Ps2Menu.fn<Ps2Menu.IntFn>(VU1_THUNK)();
        Ps2Menu.fn<Ps2Menu.IntFn>(INIT_PKT)();
        int chr = Ps2Menu.fn<Ps2Menu.IntFn>(CURRENT)();
        int fade = Ps2Menu.i32(FADE);
        Ps2Menu.fn<Page>(STATUS_PAGE)(0, 0, chr, 0, (nint)Native.reloc(CUR_A), (nint)Native.reloc(CUR_B),
            (nint)Native.reloc(CUR_C), (nint)Native.reloc(CUR_D), fade);
        return Ps2Menu.packet(() => {
            var r1 = xy(R_SOMOJIPLT);
            var r2 = xy(R_SOMOJIPLT2);
            label_plate(r1.x, r1.y, 6);
            label_plate(r2.x, r2.y, 7);
            // the cursor (PS2 0x0022bbd0): the weapon (armour) caption's place (PS2 0x00228b98 =
            // MenuStatus_GetPos), blinking while the list is open
            if (fade >= 0x1000 && !(Ps2Menu.i32(LIST_OPEN) != 0 && blink_off())) {
                var rec = xy(Ps2Menu.i32(ALT) != 0 ? WPNLPLT + 0x48 : WPNLPLT);
                var moji = xy(POS_WPNMOJI);
                Ps2Menu.cursor(rec.x + moji.x, rec.y + moji.y);
            }
            abilities();
            first(1, "ui.menus: equip screen drawn the PS2 way");
        });
    }

    /// <summary>PS2 0x00226218(x, y, k) -&gt; 0x00201a70: a black plate (0xda, 0xdb over 213), strtex label k at (+21, +5).</summary>
    private static void label_plate(int x, int y, int k) {
        Ps2Menu.plate_rl(0xda, 0xdb, x, y, 213, Ps2Menu.SHADOW, Ps2Menu.SHADOW);
        Ps2Menu.fn<SheetCell>(SHEET_CELL)(k, x + 21, y + 5, 170, 14);
    }

    /// <summary>PS2 0x0022baa8: per slot of the picked gear a purple plate (kind 0x10) and the dotted line, then the abilities' icons and names.</summary>
    private static void abilities() {
        int item = Ps2Menu.i32(ITEM);
        if (item == 0xff) return;
        byte* r = (byte*)Ps2Menu.fn<Ps2Menu.PtrOfInt2>(SAVE_WEAPON)(item, 0);
        var (x, y0) = xy(POS_SOABPLT);
        var pm = xy(POS_PMSHAPE);
        for (int i = 0; i < r[0xb]; i++) {
            int y = y0 + ROW * i;
            plate(x, y, 0x10);
            Ps2Menu.shape(0xd7, x + pm.x + 0x58, y + pm.y);
        }
        for (int k = 0; k < 4; k++) {
            int id = *(ushort*)(r + 0xe + 2 * k);
            if (id == 0 || id == 0xff) continue;
            int y = y0 + ROW * k;
            int strings;
            byte* a = (byte*)Ps2Menu.fn<Ps2Menu.PtrOfIntPtr>(ROM_ABILITY)(id, &strings);
            // PS2 0x001f5388: icon (+12, +1), name (+29, +1)
            Ps2Menu.icon(x + 12, y + 1, a[0x68]);
            Ps2Menu.font(strings + *(ushort*)a, x + 29, y + 1, 0);
        }
    }

    // ---- the weapon list: SLPM_675.13 0x0022bdc0 ----------------------------------------------------

    private static int list_draw(nint w) => Ps2Menu.packet(() => {
        var (px, py) = xy(POS_SOBIPLT);
        int top = wi16(w, 0x32), next = wi16(w, 0x34), scroll = wi16(w, 0x46);
        var scissor = Ps2Menu.fn<WinScissor>(WIN_SCISSOR);
        if (top == next) {
            scissor(py, 160, 0, scroll);
            rows(w, px, py - 32, top, 0);
        } else {
            scissor(py, 160, 1, scroll);
            rows(w, px, py - 32, top, 1);
            scissor(py, 160, 2, scroll);
            rows(w, px, py - 160 * scroll / 4096 - 32, next, 2);
        }
        Ps2Menu.scissor(0, 0, 511, 415);
        Ps2Menu.cursor(px + 12, py + 1 + ROW * (wi16(w, 0x48) - top));
        int count = wu16(w, 0x30), shown = wu16(w, 0x3a);
        if (shown < count) {
            var (rx, ry) = xy(R_SOSCROL);
            var (rw, rh) = xy(R_SOSCROL + 4);
            scroll_bar(rx, ry, rw, rh, top, shown, count);
        }
        first(2, "ui.menus: equip weapon list drawn the PS2 way");
    });

    /// <summary>PS2 0x0022bd20: rows first - 1 .. first + 5 from y.</summary>
    private static void rows(nint w, int x, int y, int first, int mode) {
        for (int k = -1; k < 6; k++, y += ROW) row(w, x, y, first + k, mode);
    }

    /// <summary>One row (PS2 0x0022bc20): mode 0 follows the scroll (+0x46 / 128).</summary>
    private static void row(nint w, int x, int y, int idx, int mode) {
        if (mode == 0) y -= wi16(w, 0x46) / 128;
        if (idx < 0 || idx >= wu16(w, 0x30)) return;
        plate(x, y, 0);
        var pm = xy(POS_PMSHAPE);
        Ps2Menu.shape(0xd7, x + pm.x + 0x58, y + pm.y);
        int id = *(ushort*)Native.ptr(ITEM_LIST + 4 * (uint)idx);
        if (id != 0) weapon_name(id, x, y);
    }

    /// <summary>PS2 0x00225e90 -&gt; 0x00200878: a dark track, a light thumb over rows top .. top + shown of count.</summary>
    private static void scroll_bar(int x, int y, int w, int h, int top, int shown, int count) {
        Ps2Menu.box_rgba(x, y, w, h, 0x11, 0x11, 0x11, 0x80);
        float t = top == 0 ? y : y + (float)h * top / count;
        float b = top + shown >= count ? y + h : y + (float)h * (top + shown) / count;
        Ps2Menu.box_rgba(x + 1, (int)t, w - 2, (int)b - (int)t, 0xf0, 0xf0, 0xf0, 0x80);
    }

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(DemasterModule m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(DemasterModule m) {
        _log = m.log;
        return hook<Draw>(m, EQUIP_DRAW, equip_draw)
            & hook<WinDraw>(m, LIST_DRAW, list_draw);
    }
}
