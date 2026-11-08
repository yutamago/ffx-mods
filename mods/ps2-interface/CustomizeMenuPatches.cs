// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.menus, the Customize screen drawn by the PS2's draw code ported onto FFX.exe's primitives (as
/// ItemMenuPatches does for Items). The PC kept the PS2 logic and its two windows; hooked
/// (PC = SLPM_675.13):
/// <list type="bullet">
/// <item>MenuKaizou_Draw_008d6860 = 0x00232188 (window 0x0186aa30 = PS2 0x00693d58): the equipment
/// list (rows 0x00231e70, plate kind 0 and line 0xd7, icon and name), its cursor, scroll bar,
/// the "Equipment" / "Abilities" labels (strtex cells 6, 7), the fade band (0x00231e18), the
/// picked weapon's ability slots (0x00232030) and the owner's party plate (0x00225f38 -> 0x00200c60);</item>
/// <item>DrawGearCustomizationMenu 0x008d5f60 = 0x00232730 (window 0x0186aa34 = PS2 0x00693d5c):
/// the abilities to add (rows 0x00232568, plate kind 0x10), the item cost window (0x00227800) with
/// its label (strtex 0x10), the "Abilities" label (7), cursor, scroll bar and the weapon's title
/// plate (0x002324d8).</item>
/// </list>
/// The layout comes from the menu sequence by name, as the PS2's init 0x002315f0 reads it
/// (pos_ksobiplt, r_kmojiplt1 ...). Evidence and the open points: docs/ui.md ("The Customize screen").
/// </summary>
internal static unsafe class CustomizeMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqRect(int scene, nint name, int* x, int* y, int* w, int* h);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void WinScissor(int y, int h, int part, int ratio);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Frame(int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Cell(int cell, float x, float y, float w, float h);

    // hooked
    private const uint LIST_DRAW = 0x008d6860;    // MenuKaizou_Draw_008d6860 (PS2 0x00232188)
    private const uint ADD_DRAW  = 0x008d5f60;    // DrawGearCustomizationMenu (PS2 0x00232730)

    // drawn with (besides Ps2Menu's)
    private const uint WIN_SCISSOR = 0x008c0f70;  // TkMn2DrawMenuWindowScissor(y, h, part, ratio) (PS2 0x002274a8)
    private const uint FRAME       = 0x008f9b60;  // TOMkpWindowFrame_008f9b60: shapes 0xc8..0xcb (PS2 0x001fa328)
    private const uint SHEET_CELL  = 0x008f8bb0;  // TODrawSheetCell_008f8bb0: strtex (0x2ecc) cell, 170 x 14 a cell (PS2 0x00201768)

    // read
    private const uint SEQ_RECT     = 0x008ae4b0; // TOSeqGetRect(scene, name, &x, &y, &w, &h): i16s
    private const uint SEQ_COLOR    = 0x008c2ad0; // TkMn2SeqGetColor(name)
    private const uint WEAPONS      = 0x0186aa38; // u16* the customise list (MsGetSaveWeaponList; PS2 0x00693d60)
    private const uint ITEM_LIST    = 0x01597770; // g_TkMenuItemList: { u16 id, u8 kind, u8 kaizou index } (PS2 0x01d57300)
    private const uint SAVE_WEAPON  = 0x007abbd0; // MsGetSaveWeapon(id, &name) (PS2 0x001bfed8)
    private const uint WEAPON_NAME  = 0x007abdf0; // MsGetSaveWeaponName (PS2 0x001c0040)
    private const uint SELECTABLE   = 0x008d5750; // MenuEquip_IsSelectable(id, any) (PS2 0x002317c8)
    private const uint OWNER        = 0x008d56f0; // MenuKaizou_WeaponByte4(w) (PS2 0x00231dd0)
    private const uint KAIZOU       = 0x008c1b70; // TkMn2GetKaizou: 8-byte records, item u16 +4, count u8 +6 (PS2 0x00227ed8)
    private const uint ROM_ABILITY  = 0x00790970; // MsGetRomAbility(id, &strings): name u16 +0, icon +0x68 (PS2 0x001bc2e8)
    private const uint ROM_ITEM     = 0x007909f0; // MsGetRomItem(id, &strings): name u16 +0, icon +0x14 (PS2 0x001bc2c0)
    private const uint ITEM_NUM     = 0x007904b0; // MsGetSaveItemNum (PS2 0x001bfe30)
    private const uint MAIN_MES     = 0x008ddf80; // TkMenuMainMes (PS2 0x00249330)
    private const uint STATUS_BG    = 0x008d4910; // TkMn2GetStatusBG (PS2 0x00228b88)
    private const uint CHR_NAME     = 0x008ac850; // TOGetSaveChrName
    private const uint DAMAGE_CLASS = 0x008a99f0; // MenuPlayer_DamageClass
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint SPHERE_LV    = 0x008a9b80; // TkMenuGetSP
    private const uint NEXT_AP      = 0x00784e90; // MsGetNextAP(chr)
    private const uint SAVE_NEXT_AP = 0x007852b0; // MsGetSaveNextAP(chr)
    private const uint PARTY_MASK   = 0x01841c2c; // the front-line bits (PS2 0x00242908)
    // TOSeqInitParam's main-menu layout (mirrored storage: points y, x; MainMenuPatches)
    private const uint NAMEPLT_W = 0x025d1672;
    private const uint COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144;
    private const uint COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;
    private const uint COL_BSOPLT0 = 0x025d16a0;  // plate pairs: bso, bitem, bab, lmt (c0, c1 4 bytes apart)

    private const uint MESWIN_LAYOUT = 0x00c5b460; // the PS2's meswin shape table (SLPM_675.13 0x00321328)

    private const int ROW = 32;
    private const int LIST_H = 0x120;              // nine rows
    private const uint GREY = Ps2Menu.GREY;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static T fn<T>(uint va) where T : Delegate => Ps2Menu.fn<T>(va);
    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static short wi16(nint w, int o) => *(short*)(w + o);
    private static ushort wu16(nint w, int o) => *(ushort*)(w + o);

    // ---- the menu sequence -----------------------------------------------------------------------

    private static (int x, int y) pos(string name) => Ps2Menu.seq_point(name);

    private static (int x, int y, int w, int h) rect(string name) {
        int x = 0, y = 0, w = 0, h = 0;
        fn<SeqRect>(SEQ_RECT)(1, Ps2Menu.cstr(name), &x, &y, &w, &h);
        return ((short)x, (short)y, (short)w, (short)h);
    }

    private static uint colour(string name) => (uint)fn<Ps2Menu.IntOfPtr>(SEQ_COLOR)(Ps2Menu.cstr(name));

    // ---- shared pieces (private copies: ItemMenuPatches has the same plate and party plate) -----

    private static void plate(int x, int y, int kind) => Ps2Menu.kind_plate(x, y, kind);

    /// <summary>A row: the plate (PS2 0x00226d90) and shape 0xd7 at pos_pmshape + (0x58, 0) (0x00226e18).</summary>
    private static void row_plate(int x, int y, int kind, (int x, int y) pm) {
        plate(x, y, kind);
        parts(0xd7, x + pm.x + 0x58, y + pm.y);
    }

    private static void parts(int id, int x, int y) => Ps2Menu.shape(id, x, y);

    private static void label(int x, int y, int k) => Ps2Menu.label_plate(x, y, k);

    /// <summary>A name with its record's icon (PS2 0x001f5100 / 0x001f5388): icon at (+12, +1), name at (+29, +1).</summary>
    private static void icon_name(uint data, int id, int icon_at, int x, int y, int colour) {
        int strings;
        nint rec = fn<Ps2Menu.PtrOfIntPtr>(data)(id, &strings);
        Ps2Menu.icon(x + 12, y + 1, *(byte*)(rec + icon_at));
        Ps2Menu.font(strings + *(ushort*)rec, x + 29, y + 1, colour);
    }

    /// <summary>A weapon (PS2 0x001f5298): icon 2 owner + kind + 1, the name, icon 0x31 over it when its owner wears it.</summary>
    private static void weapon_name(int id, int x, int y, int colour) {
        int unused;
        byte* rec = (byte*)fn<Ps2Menu.PtrOfIntPtr>(SAVE_WEAPON)(id, &unused);
        Ps2Menu.icon(x + 12, y + 1, (rec[5] + 2 * rec[4] + 1) & 0xff);
        Ps2Menu.font(fn<Ps2Menu.PtrOfInt>(WEAPON_NAME)(id), x + 29, y + 1, colour);
        if (rec[4] == rec[6]) Ps2Menu.icon(x + 12, y + 1, 0x31);
    }

    private static int weapon_id(int i) => *(ushort*)(*(nint*)Native.ptr(WEAPONS) + 2 * i);

    private static byte* weapon(int id) {
        int unused;
        return (byte*)fn<Ps2Menu.PtrOfIntPtr>(SAVE_WEAPON)(id, &unused);
    }

    private static void scroll_bar((int x, int y, int w, int h) r, int top, int shown, int count) {
        if (shown < count) Ps2Menu.scroll_bar(r.x, r.y, r.w, r.h, top, shown, count);
    }

    /// <summary>
    /// The list rows -1 .. 9 from pos_ksobiplt (PS2 0x00231f90 / 0x00232690) inside the window
    /// scissor (0x002274a8): at rest one pass, while scrolling the old rows (part 1) and the new
    /// ones (part 2) 0x120 * +0x46 / 4096 higher.
    /// </summary>
    private static void rows(nint w, (int x, int y) at, Action<int, int, int> row) {
        int top = wi16(w, 0x32), next = wi16(w, 0x34), scroll = wi16(w, 0x46);
        var sc = fn<WinScissor>(WIN_SCISSOR);
        if (top == next) {
            sc(at.y, LIST_H, 0, scroll);
            for (int i = -1; i < 10; i++) row(at.x, at.y + ROW * i - scroll / 128, top + i);
        } else {
            sc(at.y, LIST_H, 1, scroll);
            for (int i = -1; i < 10; i++) row(at.x, at.y + ROW * i, top + i);
            sc(at.y, LIST_H, 2, scroll);
            int dy = -(scroll * LIST_H) / 4096;
            for (int i = -1; i < 10; i++) row(at.x, at.y + dy + ROW * i, next + i);
        }
        Ps2Menu.scissor(0, 0, 512, 416);
    }

    // ---- the equipment list: SLPM_675.13 0x00232188 ----------------------------------------------

    private static int list_draw(nint w) => Ps2Menu.packet(() => {
        Ps2Menu.scenes("scene0", "scene21");
        var at = pos("pos_ksobiplt");
        var pm = pos("pos_pmshape");
        int count = wu16(w, 0x30);
        rows(w, at, (x, y, i) => {
            if (i < 0 || i >= count) return;
            row_plate(x, y, 0, pm);
            int id = weapon_id(i);
            byte* rec = weapon(id);
            if (rec[2] == 0 || (rec[3] & 2) != 0) return;
            weapon_name(id, x, y, fn<Ps2Menu.IntOfInt2>(SELECTABLE)(id, 0) == 0 ? 1 : 0);
        });
        Ps2Menu.cursor(at.x + 12, at.y + 1 + ROW * (wi16(w, 0x48) - wi16(w, 0x32)));
        scroll_bar(rect("r_kscrol"), wi16(w, 0x32), wu16(w, 0x3a), count);
        var m2 = rect("r_kmojiplt2");
        label(m2.x, m2.y, 7);
        var m1 = rect("r_kmojiplt1");
        label(m1.x, m1.y, 6);
        var zone = pos("pos_kzonmplt");
        fade_band(fn<Ps2Menu.IntFn>(STATUS_BG)(), zone.y);
        slots(wi16(w, 0x48), pm);
        member(zone.x, zone.y, fn<Ps2Menu.IntOfPtr>(OWNER)(w));
        first(1, "ui.menus: customize equipment list drawn the PS2 way");
    });

    /// <summary>PS2 0x00231e18: col_menubg0 -> col_menubg1 from pos_kzonmplt.y down, alpha TkMn2GetStatusBG / 32.</summary>
    private static void fade_band(int c, int y) {
        uint a = (uint)(c >> 5) << 24;
        if (a == 0) return;
        Ps2Menu.box(0, y, 512, 416 - y, (colour("col_menubg0") & 0xffffff) | a, (colour("col_menubg1") & 0xffffff) | a);
    }

    /// <summary>PS2 0x00232030: the picked weapon's slots (plate kind 0x10) and its abilities, from pos_kabplt.</summary>
    private static void slots(int row, (int x, int y) pm) {
        byte* rec = weapon(weapon_id(row));
        if (rec[2] == 0) return;
        var at = pos("pos_kabplt");
        for (int i = 0; i < rec[0xb]; i++) row_plate(at.x, at.y + ROW * i, 0x10, pm);
        for (int i = 0; i < 4; i++) {
            int a = *(ushort*)(rec + 0xe + 2 * i);
            if (a != 0 && a != 0xff) icon_name(ROM_ABILITY, a, 0x68, at.x, at.y + ROW * i, 0);
        }
    }

    private static void member(int x, int y, int chr) => Ps2Menu.party_plate(x, y, chr);

    // ---- the abilities to add: SLPM_675.13 0x00232730 --------------------------------------------

    private static int add_draw(nint w) => Ps2Menu.packet(() => {
        Ps2Menu.scenes("scene0", "scene21", "scene23"); // scene23: the ability list (PS2 0x00232948)
        var pm = pos("pos_pmshape");
        byte* e = Native.ptr(ITEM_LIST + 4 * (uint)wi16(w, 0x48));
        if (e[3] != 0xff) {
            byte* k = (byte*)fn<Ps2Menu.PtrFn>(KAIZOU)() + 8 * e[3];
            var back = rect("r_kbackplt");
            require(back.x, back.y, *(ushort*)(k + 4), k[6], pm);
            var m3 = rect("r_kmojiplt3");
            label(m3.x, m3.y, 0x10);
        }
        var m1 = rect("r_kmojiplt1");
        label(m1.x, m1.y, 7);
        var at = pos("pos_ksobiplt");
        int count = wu16(w, 0x30);
        rows(w, at, (x, y, i) => {
            if (i < 0 || i >= count) return;
            byte* r = Native.ptr(ITEM_LIST + 4 * (uint)i);
            row_plate(x, y, 0x10, pm);
            if (r[3] != 0xff) icon_name(ROM_ABILITY, *(ushort*)r, 0x68, x, y, r[2] == 0xb ? 0 : 1);
        });
        Ps2Menu.cursor(at.x + 12, at.y + 1 + ROW * (wi16(w, 0x48) - wi16(w, 0x32)));
        scroll_bar(rect("r_kscrol"), wi16(w, 0x32), wu16(w, 0x3a), count);
        // PS2 0x002324d8: the weapon of the equipment list's row on a kind-0 plate
        nint list = *(nint*)Native.ptr(0x0186aa30);
        if (list != 0) {
            var t = pos("pos_ksobiplt2");
            row_plate(t.x, t.y, 0, pm);
            Ps2Menu.font(fn<Ps2Menu.PtrOfInt>(WEAPON_NAME)(weapon_id(wi16(list, 0x48))), t.x + 0x1d, t.y + 1, 0);
        }
        first(2, "ui.menus: customize ability list drawn the PS2 way");
    });

    /// <summary>
    /// The item cost window (PS2 0x00227800(x, y, item, need)): the r_shohiplt window at (x, y), a
    /// gold row plate at pos_shiplt with the item, shape 0xff at pos_shshape + 16, the count held
    /// (font M) at pos_shshape + pos_lshohi and, at pos_shshape + pos_rshohi, what is left after the
    /// customisation or TkMenuMainMes(0x15) right-aligned in grey when too few.
    /// </summary>
    private static void require(int x, int y, int item, int need, (int x, int y) pm) {
        var r = rect("r_shohiplt");
        fn<Frame>(FRAME)(x, y, r.w, r.h, colour("col_shohiplt0"), colour("col_shohiplt1"));
        var sh = pos("pos_shiplt");
        var shape = pos("pos_shshape");
        int sx = x + sh.x, sy = y + sh.y;
        row_plate(sx, sy, 8, pm);
        parts(0xff, x + shape.x + 0x10, y + shape.y);
        if (item < 0) return;
        int held = fn<Ps2Menu.IntOfInt>(ITEM_NUM)(item);
        int few = held < need ? 1 : 0;
        icon_name(ROM_ITEM, item, 0x14, sx, sy, few);
        var l = pos("pos_lshohi");
        var rr = pos("pos_rshohi");
        Ps2Menu.value_m(held, x + shape.x + l.x, y + shape.y + l.y, few);
        int rx = x + shape.x + rr.x, ry = y + shape.y + rr.y;
        if (few == 1) {
            nint text = fn<Ps2Menu.PtrOfInt>(MAIN_MES)(0x15);
            Ps2Menu.font(text, rx - Ps2Menu.text_width(text), ry, 1);
        } else {
            Ps2Menu.value_m(held - need, rx, ry, 0);
        }
    }

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(ModHost m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(ModHost m) {
        _log = m.log;
        return hook<WinDraw>(m, LIST_DRAW, list_draw)
            & hook<WinDraw>(m, ADD_DRAW, add_draw);
    }
}
