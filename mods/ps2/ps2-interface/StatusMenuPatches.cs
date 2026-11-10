// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.menus, the Status screen drawn by the PS2's draw code ported onto FFX.exe's primitives, as
/// ItemMenuPatches does for the item lists. The PC kept the PS2 logic, the window records and the
/// layout words (TkMn2GetStatusPos reads the same names as the PS2's 0x00228c48, into its own
/// globals), but redrew the screen with HD sheets. Hooked (PC = SLPM_675.13):
/// <list type="bullet">
/// <item>MenuStatus_DrawMainWindow 0x008d2710 = 0x0022ad00: the status page (face, equipment,
/// Overdrive gauge, party plate), the eight stats, the "show abilities" bar and the status
/// background that covers them while an ability page is open;</item>
/// <item>MenuStatus_WindowDraw_008d2790 = 0x0022b5a0: an ability page (title label, two columns
/// of plates with icon and name); the aeon status pages use the same windows.</item>
/// </list>
/// Evidence: docs/mods/interface.md ("The Status screen").
/// </summary>
internal static unsafe class StatusMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfIntPtr(int a, int* b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int DispCommand(int kind, int sub, int c, int chr, int* list);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void FaceSlide(float x, float y, float w, float h, int t, int cur, int next);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Cell(int n, float x, float y, float w, float h, int alpha);

    // hooked
    private const uint MAIN_DRAW = 0x008d2710; // MenuStatus_DrawMainWindow (PS2 0x0022ad00)
    private const uint SUB_DRAW  = 0x008d2790; // MenuStatus_WindowDraw_008d2790 (PS2 0x0022b5a0)

    // called
    private const uint CURRENT      = 0x008a9860; // TkMenuGetCurrentPlayer
    private const uint CURRENT_AEON = 0x008a9880; // TkMenuGetCurrentSummon
    private const uint EASE         = 0x008d30c0; // MenuMn2_Ease(t, dir) (PS2 0x002292c0)
    private const uint STATUS_BG    = 0x008d4910; // TkMn2GetStatusBG (PS2 0x00228b88)
    private const uint TRANS_FACE   = 0x008c2ce0; // TkMn2TransCharFace(aeon) (PS2 0x00225e78)
    private const uint FACE_SLIDE   = 0x008d32b0; // MenuPlayer_DrawFaceSlide(x, y, w, h, t, cur, next) (PS2 0x002299b0's faces)
    private const uint STATUS_MES   = 0x008bee70; // TOMenuGetStatusMes (PS2 0x00220a80)
    private const uint MENU1_MES    = 0x008ddfc0; // TkMenuMenu1Mes (PS2 0x00249368)
    private const uint OD_ID        = 0x008c1bd0; // TkMn2GetOverDriveId (PS2 0x00225608)
    private const uint LIMIT        = 0x008a9930; // TkMenuGetLimit (PS2 0x00242580)
    private const uint LIMIT_MAX    = 0x008a9950; // TkMenuGetLimitMax (PS2 0x002425a0)
    private const uint WEAPON       = 0x008a9c70; // TkMenuGetWeapon (PS2 0x002423d8)
    private const uint ARMOR        = 0x008a9820; // TkMenuGetArmor, 0xff none (PS2 0x00242450)
    private const uint SAVE_WEAPON  = 0x007abbd0; // MsGetSaveWeapon(id, &name) (PS2 0x001bfed8)
    private const uint WEAPON_NAME  = 0x007abdf0; // MsGetSaveWeaponName (PS2 0x001c0040)
    private const uint PLAYER_PTR   = 0x00785330; // MsGetSavePlayerPtr (PS2 0x001be848)
    private const uint DISP_COMMAND = 0x008d2e10; // TkMenuGetDispCommand (PS2 0x0022b280)
    private const uint ROM_ABILITY  = 0x00790970; // MsGetRomAbility(id, &strings): name u16 +0, icon +0x68 (PS2 0x001bc2e8)
    private const uint COM_DATA     = 0x0079a4b0; // MsGetComData(id, &strings): name u16 +0, icon +0x14 (PS2 0x001bbdf8)
    private const uint COMMAND_NAME = 0x008c1a20; // TkMn2GetCommandName
    private const uint CHR_NAME     = 0x008ac850; // TOGetSaveChrName
    private const uint DAMAGE_CLASS = 0x008a99f0; // MenuPlayer_DamageClass
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint SPHERE_LV    = 0x008a9b80; // TkMenuGetSP
    private const uint NEXT_AP      = 0x00784e90; // MsGetNextAP
    private const uint SAVE_NEXT_AP = 0x007852b0; // MsGetSaveNextAP
    private const uint STRTEX_CELL  = 0x008f8d50; // TOMkpShape2ECCCell_008f8d50(n, x, y, w, h, alpha) (PS2 0x00201768)

    // read: the status layout (TkMn2GetStatusPos; PS2 0x00228c48 -> 0x006938xx), points i16 x, y,
    // rectangles i16 x, y, w, h, common plates { i16 x, y, w, h; c0 +8; c1 +0xc; A +0x10; B +0x14 }
    private const uint RECT_3NAMEPLT = 0x0186a83c; // PS2 0x00693880
    private const uint POS_WPNMOJI   = 0x0186a86c; // g_TkMenuStatusOrigin (PS2 0x006938b8)
    private const uint POS_WPNICON   = 0x0186a870; // PS2 0x006938c0
    private const uint POS_WPNNM     = 0x0186a874; // PS2 0x006938c8
    private const uint POS_LTSTR     = 0x0186a878; // PS2 0x006938d0
    private const uint POS_LPMPLT    = 0x0186a87c; // PS2 0x006938d8
    private const uint POS_RPMPLT    = 0x0186a880; // PS2 0x006938e0
    private const uint NUM_PMPLT     = 0x0186a884; // PS2 0x006938e4
    private const uint PL_LGPLT      = 0x0186a888; // common plates, 0x18 apart (PS2 0x006938e8 ..)
    private const uint PL_3LGPLT     = 0x0186a8a0;
    private const uint PL_LTPLT      = 0x0186a8b8;
    private const uint PL_LTBPLT     = 0x0186a8d0;
    private const uint PL_WPNBPLT    = 0x0186a8e8;
    private const uint PL_WPNLPLT    = 0x0186a900;
    private const uint PL_WPNRPLT    = 0x0186a918;
    private const uint PL_WPNBPLT2   = 0x0186a930; // the second row: y + num_wpnplt
    private const uint PL_WPNLPLT2   = 0x0186a948;
    private const uint PL_WPNRPLT2   = 0x0186a960;
    private const uint PL_WPNBLINE   = 0x0186a978;
    private const uint PL_WPNBLINE2  = 0x0186a990;
    private const uint POS_TYPSHAPE  = 0x0186a9a8; // PS2 0x00693a08
    private const uint POS_LGSHAPE   = 0x0186a9b0; // PS2 0x00693a18
    private const uint POS_3LGSHAPE  = 0x0186a9b4; // PS2 0x00693a20
    private const uint COL_LMTGF0    = 0x0186a9bc; // PS2 0x00693a2c
    private const uint RECT_LG       = 0x0186a9c0; // PS2 0x00693a30
    private const uint COL_LMTG0     = 0x0186a9cc; // PS2 0x00693a3c
    private const uint COL_LMTG1     = 0x0186a9d0; // PS2 0x00693a40
    private const uint RECT_SMOJIPLT = 0x0186a9d8; // PS2 0x00693a48
    private const uint POS_LSTAPLT   = 0x0186a9e0; // PS2 0x00693a50
    private const uint POS_RSTAPLT   = 0x0186a9e4; // PS2 0x00693a58
    private const uint NUM_STAPLT    = 0x0186a9e8; // PS2 0x00693a5c
    private const uint NUM_FACE1     = 0x0186a9ec; // PS2 0x00693a60
    private const uint NUM_FACE2     = 0x0186a9f0; // PS2 0x00693a64
    private const uint POS_FACE      = 0x0186a9f4; // PS2 0x00693a68
    private const uint PL_STSMW      = 0x023cc400; // PS2 0x00325888
    private const uint POS_EXTEX     = 0x023cc3e8; // PS2 0x003258a0
    private const uint POS_EXMSSG    = 0x023cc3e4; // PS2 0x003258a8
    private const uint COL_MENUBG1   = 0x023cc418; // PS2 0x00325880
    private const uint COL_MENUBG0   = 0x023cc41c; // PS2 0x0032587c
    private const uint POS_PMSHAPE   = 0x023cc420; // PS2 0x00325878
    private const uint FACE_T = 0x023cc3c8, FACE_CUR = 0x023cc3c4, FACE_NEXT = 0x023cc3c0; // PS2 0x003258c8/cc/d0
    private const uint STATS         = 0x00c5a790; // (mes, field) i32 pairs up to mes < 0 (PS2 0x00325808)
    private const uint MESWIN_LAYOUT = 0x00c5b460; // the PS2 meswin layout table (PS2 0x00321328)
    // the main menu's layout (MainMenuPatches / ItemMenuPatches): the party plate's places
    private const uint NAMEPLT_W = 0x025d1672;
    private const uint COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144;
    private const uint COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;
    private const uint PARTY_MASK = 0x01841c2c;
    private const uint COL_BSOPLT0 = 0x025d16a0;

    private const uint GREY = Ps2Menu.GREY;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static int call(uint va) => Ps2Menu.fn<Ps2Menu.IntFn>(va)();
    private static int call(uint va, int a) => Ps2Menu.fn<Ps2Menu.IntOfInt>(va)(a);
    private static nint ptr(uint va, int a) => Ps2Menu.fn<Ps2Menu.PtrOfInt>(va)(a);
    private static short i16(uint va) => Ps2Menu.i16(va);
    private static int i32(uint va) => Ps2Menu.i32(va);
    private static uint u32(uint va) => Ps2Menu.u32(va);
    /// <summary>A TkMn2SeqGetPos point: i16 x, y.</summary>
    private static (int x, int y) pos(uint va) => (i16(va), i16(va + 2));
    private static short wi16(nint w, int o) => *(short*)(w + o);
    private static int wi32(nint w, int o) => *(int*)(w + o);

    /// <summary>C's division (toward zero) by 4096, as the PS2's (v + 0xfff if negative) &gt;&gt; 12.</summary>
    private static int q12(int v) => v / 4096;

    /// <summary>MenuHelp_Ratio (PS2 0x00224958): (2a (b + 2)) / 4, toward zero.</summary>
    private static int ratio(int a, int b) => 2 * a * (b + 2) / 4;

    /// <summary>A meswin shape's first part size (the PS2's 0x001fd770 lookup: ids 200.. are 0..).</summary>
    private static (int w, int h) dims(int id) {
        int i = id >= 200 ? id - 200 : id;
        uint e = MESWIN_LAYOUT + u32(MESWIN_LAYOUT + 4 + 4 * (uint)i);
        return (Ps2Menu.u16(e + 8), Ps2Menu.u16(e + 10));
    }

    /// <summary>TOMkpShapeXY (the PS2's 0x001fe1f0, grey): Ps2Menu.shape, part by part inside the cells.</summary>
    private static void shape(int id, int x, int y) => Ps2Menu.shape(id, x, y);

    // ---- the PS2's common plate records (0x00226e98, 0x00226f30, 0x00226fe8, 0x00226fc8) ----------

    private readonly struct Plate(uint va, int dx = 0, int dy = 0) {
        public readonly int x = i16(va) + dx, y = i16(va + 2) + dy, w = i16(va + 4), h = i16(va + 6);
        public readonly uint c0 = u32(va + 8), c1 = u32(va + 12);
        public readonly int a = i32(va + 16), b = i32(va + 20);
    }

    /// <summary>PS2 0x001ff028 (TkMn2DrawCommonPlateL): A at x in c0, B from x + A's width, stretched, c0 -> c1.</summary>
    private static void plate_l(int a, int b, int x, int y, int w, uint c0, uint c1) {
        int wa = dims(a).w;
        Ps2Menu.shape_wh(a, x, y, 0, 0, c0, c0);
        Ps2Menu.shape_wh(b, x + wa, y, w - wa, 0, c0, c1);
    }
    private static void plate_l(in Plate p) => plate_l(p.a, p.b, p.x, p.y, p.w, p.c0, p.c1);

    /// <summary>PS2 0x001ff100 (TkMn2DrawCommonPlateR): A stretched to w - B's width, c0 -> c1, then B in c1.</summary>
    private static void plate_r(in Plate p) {
        int wb = dims(p.b).w;
        Ps2Menu.shape_wh(p.a, p.x, p.y, p.w - wb, 0, p.c0, p.c1);
        Ps2Menu.shape_wh(p.b, p.x + p.w - wb, p.y, 0, 0, p.c1, p.c1);
    }

    /// <summary>PS2 0x001fa328 (TkMn2DrawCommonWindow): shapes 0xc8..0xcb, the corner in c0, the rest stretched.</summary>
    private static void window(in Plate p) {
        var (wa, ha) = dims(0xc8);
        Ps2Menu.shape_wh(0xc8, p.x, p.y, 0, 0, p.c0, p.c0);
        Ps2Menu.shape_wh(0xc9, p.x + wa, p.y, p.w - wa, 0, p.c0, p.c1);
        Ps2Menu.shape_wh(0xca, p.x, p.y + ha, 0, p.h - ha, p.c0, p.c0);
        Ps2Menu.shape_wh(0xcb, p.x + wa, p.y + ha, p.w - wa, p.h - ha, p.c0, p.c1);
    }

    /// <summary>PS2 0x00226fc8: the record's rectangle as an untextured box, c0 left -> c1 right.</summary>
    private static void box(in Plate p) => Ps2Menu.box(p.x, p.y, p.w, p.h, p.c0, p.c1);

    private static void kind_plate(int x, int y, int kind) => Ps2Menu.kind_plate(x, y, kind);

    /// <summary>An icon at (+12, +1) and a name at (+29, +1) (PS2 0x001f5068 / 0x001f5388 / 0x001f5298).</summary>
    private static void icon_name(int icon, nint name, int x, int y, int colour) {
        Ps2Menu.icon(x + 12, y + 1, icon);
        Ps2Menu.font(name, x + 29, y + 1, colour);
    }

    // ---- the main window: SLPM_675.13 0x0022ad00 -------------------------------------------------

    private static int main_draw(nint w) => Ps2Menu.packet(() => {
        Ps2Menu.scenes("scene0", "scene11");
        int chr = call(CURRENT);
        int t = wi16(w, 0x52);
        page0(0, 0, chr, t);
        stats(0, chr, t);
        status_bg(call(STATUS_BG));
        first(1, "ui.menus: status page drawn the PS2 way");
    });

    /// <summary>PS2 0x0022a770, page 0 (the PC's MenuStatus_DrawPage): face, equipment, gauge, party plate.</summary>
    internal static void page0(int x, int y, int chr, int t) {
        int s = q12(call(EASE, t, 1) * 0x155);
        // the face (PS2 0x002299b0): scissored to num_face1 x num_face2
        call(TRANS_FACE, 0);
        var face = pos(POS_FACE);
        int fx = x + face.x, fy = y + face.y, fw = i32(NUM_FACE1), fh = i32(NUM_FACE2);
        Ps2Menu.scissor(fx, fy, fx + fw, fy + fh);
        // the PS2 face is a 91 x 98 sprite (0x00200048); the slide moves the next one in by fw t / 4096
        Ps2Menu.fn<FaceSlide>(FACE_SLIDE)(fx, fy, 91, 98, i32(FACE_T), i32(FACE_CUR), i32(FACE_NEXT));
        Ps2Menu.scissor(0, 0, 512, 416);
        equipment(chr, s);
        var lg = new Plate(PL_LGPLT);
        overdrive(s + lg.x, lg.y, chr);
        var np = new Plate(RECT_3NAMEPLT);
        party_row(np.x, np.y, chr);
    }

    private static int call(uint va, int a, int b) => Ps2Menu.fn<Ps2Menu.IntOfInt2>(va)(a, b);

    /// <summary>PS2 0x00229fa0: the weapon and armour rows.</summary>
    private static void equipment(int chr, int dx) {
        window(new Plate(PL_WPNBPLT, dx));
        plate_l(new Plate(PL_WPNLPLT, dx));
        box(new Plate(PL_WPNRPLT, dx));
        window(new Plate(PL_WPNBPLT2, dx));
        plate_l(new Plate(PL_WPNLPLT2, dx));
        box(new Plate(PL_WPNRPLT2, dx));
        plate_r(new Plate(PL_WPNBLINE, dx));
        plate_r(new Plate(PL_WPNBLINE2, dx));
        var r1 = new Plate(PL_WPNRPLT, dx);
        var r2 = new Plate(PL_WPNRPLT2, dx);
        shape(0xd7, r1.x + 0x58, r1.y);
        shape(0xd7, r2.x + 0x58, r2.y);
        var l1 = new Plate(PL_WPNLPLT, dx);
        var l2 = new Plate(PL_WPNLPLT2, dx);
        var moji = pos(POS_WPNMOJI);
        Ps2Menu.font(ptr(STATUS_MES, 5), l1.x + moji.x, l1.y + moji.y, 0);
        Ps2Menu.font(ptr(STATUS_MES, 6), l2.x + moji.x, l2.y + moji.y, 0);
        var icon = pos(POS_WPNICON);
        gear(call(WEAPON, chr), r1.x + icon.x, r1.y + icon.y, 0);
        int armor = call(ARMOR, chr);
        if (armor != 0xff) gear(armor, r2.x + icon.x, r2.y + icon.y, 0);
        else {
            var nm = pos(POS_WPNNM);
            Ps2Menu.font(ptr(STATUS_MES, 0x17), r2.x + nm.x, r2.y + nm.y, 0);
        }
    }

    /// <summary>PS2 0x001f5298: a piece of gear's icon (owner 2 + 1 + armour) and name, the "equipped" mark 0x31 when worn by its owner.</summary>
    private static void gear(int id, int x, int y, int colour) {
        int name;
        nint rec = Ps2Menu.fn<PtrOfIntPtr>(SAVE_WEAPON)(id, &name);
        byte* r = (byte*)rec;
        Ps2Menu.icon(x + 12, y + 1, (r[4] * 2 + 1 + r[5]) & 0xff);
        Ps2Menu.font(ptr(WEAPON_NAME, id), x + 29, y + 1, colour);
        if (r[4] == r[6]) Ps2Menu.icon(x + 12, y + 1, 0x31);
    }

    /// <summary>PS2 0x00229d60: the Overdrive plates, the mode's name and the gauge (0x00229828).</summary>
    private static void overdrive(int x, int y, int chr) {
        var lg = new Plate(PL_LGPLT);
        int dx = x - lg.x, dy = y - lg.y;
        int fp = 0;
        if (chr < 8) {
            box(new Plate(PL_LTBPLT, dx, dy));
            box(new Plate(PL_LTPLT, dx, dy));
            plate_l(new Plate(PL_LGPLT, dx, dy));
            var lt = new Plate(PL_LTPLT);
            var typ = pos(POS_TYPSHAPE);
            shape(0x109, lt.x + typ.x + dx, lt.y + typ.y + dy);
            var sh = pos(POS_LGSHAPE);
            shape(0x108, lg.x + sh.x + dx + 0x10, lg.y + sh.y + dy);
        } else {
            fp = 0xf;
            var big = new Plate(PL_3LGPLT);
            var p = new Plate(PL_LGPLT, dx, dy);
            plate_l(p.a, p.b, p.x, p.y, big.w, big.c1, p.c1);
            var sh = pos(POS_3LGSHAPE);
            shape(0x108, lg.x + sh.x + dx + 0x10, lg.y + sh.y + dy);
        }
        var rl = (x: i16(RECT_LG), y: i16(RECT_LG + 2));
        gauge(rl.x + dx + fp, rl.y + dy, call(LIMIT, chr), call(LIMIT_MAX, chr));
        if (chr < 8) {
            var ltb = new Plate(PL_LTBPLT);
            var str = pos(POS_LTSTR);
            Ps2Menu.font(ptr(MENU1_MES, call(OD_ID, chr) + 0x1033), ltb.x + str.x + dx, ltb.y + str.y + dy, 0);
        }
    }

    /// <summary>
    /// PS2 0x00229828: the gauge, rect_lg's size, r = v / max (4096 = full): w r, from col_lmtg0 to
    /// the blend of col_lmtg0 and col_lmtg1 at r; full in col_lmtgf0.
    /// </summary>
    private static void gauge(int x, int y, int v, int max) {
        if (max == 0 || v == 0) return;
        int r = v >= max ? 0x1000 : (v << 12) / max;
        int w = q12(i16(RECT_LG + 4) * r), h = i16(RECT_LG + 6);
        uint c0, c1;
        if (r == 0x1000) c0 = c1 = u32(COL_LMTGF0);
        else {
            c0 = u32(COL_LMTG0);
            c1 = blend(c0, u32(COL_LMTG1), r);
        }
        Ps2Menu.box(x, y, w, h, c0, c1);
    }

    private static uint blend(uint a, uint b, int r) {
        uint c = 0;
        for (int k = 0; k < 32; k += 8) {
            int ca = (int)((a >> k) & 0xff), cb = (int)((b >> k) & 0xff);
            c |= (uint)(q12(ca * (0x1000 - r) + cb * r) & 0xff) << k;
        }
        return c;
    }

    private static void party_row(int x, int y, int chr) => Ps2Menu.party_plate(x, y, chr);

    /// <summary>
    /// PS2 0x0022a9a8 (the PC's TkMn2MainStatusPlayerStatus): the stats in two columns of four from
    /// pos_lpmplt / pos_rpmplt, num_pmplt apart, sliding in by MenuHelp_Ratio; then the "show
    /// abilities" bar (common window rect_stsmw, shape 0xf3, status text 0x1a) for party members.
    /// </summary>
    private static void stats(int y, int chr, int t) {
        int slide = q12(call(EASE, t, 1) * 0xd0);
        var (sx, sy) = pos(POS_LPMPLT);
        var pm = pos(POS_PMSHAPE);
        byte* p = (byte*)ptr(PLAYER_PTR, chr);
        for (int i = 0; ; i++) {
            int mes = i32(STATS + 8 * (uint)i), field = i32(STATS + 8 * (uint)i + 4);
            if (mes < 0) break;
            if (i == 4) (sx, sy) = pos(POS_RPMPLT);
            int py = y + sy + ratio(slide, i % 4);
            // PS2 0x002271b8
            kind_plate(sx, py, 0x10);
            shape(0x10a, sx + pm.x + 0x58, py + pm.y);
            Ps2Menu.font(ptr(STATUS_MES, mes), sx + 0x1d, py + 1, 0);
            Ps2Menu.value_m(p[field], sx + 0xd3, py + 6, 0);
            sy += i32(NUM_PMPLT);
        }
        if (chr < 0xe || chr >= 0x12) {
            var mw = new Plate(PL_STSMW, 0, y);
            window(mw);
            var ex = pos(POS_EXTEX);
            shape(0xf3, mw.x + ex.x, mw.y + ex.y);
            var em = pos(POS_EXMSSG);
            Ps2Menu.font(ptr(STATUS_MES, 0x1a), mw.x + em.x, mw.y + em.y, 0);
        }
    }

    /// <summary>
    /// PS2 0x002297a8(bg, 0): the menu background gradient (col_menubg0 -> col_menubg1) at alpha bg / 32
    /// from pos_face.y down, over the page while an ability page is open.
    /// </summary>
    private static void status_bg(int bg) {
        uint a = (uint)(bg >> 5) << 24;
        if (a == 0) return;
        uint c0 = (u32(COL_MENUBG0) & 0xffffff) | a, c1 = (u32(COL_MENUBG1) & 0xffffff) | a;
        int y = i16(POS_FACE + 2);
        Ps2Menu.box(0, y, 0x200, 0x1a0 - y, c0, c1);
    }

    // ---- an ability page: SLPM_675.13 0x0022b5a0 -------------------------------------------------

    private static int sub_draw(nint w) => Ps2Menu.packet(() => {
        int sub = wi32(w, 0x58), c = wi32(w, 0x5c), kind = wi32(w, 0x60);
        bool aeon = wi32(w, 0x64) != 0;
        int chr = call(aeon ? CURRENT_AEON : CURRENT);
        int ty = i16(RECT_SMOJIPLT + 2), ry = i16(POS_RSTAPLT + 2), step = i32(NUM_STAPLT);
        if (aeon) { ty += 0xb; ry += 0xb; step -= 1; }
        int* list = stackalloc int[22];
        int n = Ps2Menu.fn<DispCommand>(DISP_COMMAND)(kind, sub, c, chr, list);
        int e = call(EASE, wi16(w, 0x52), wi16(w, 0x54));
        int dy = 0, dx = 0;
        if (wi16(w, 0x54) > 0) dy = q12(e * 0xd0);
        else dx = e / 16;
        // the title (PS2 0x00226298 -> 0x00201ae8): a black brush in the fade's alpha, the strtex label
        int alpha = wi16(w, 0x52) >> 5;
        uint ca = (uint)alpha << 24;
        int tx = i16(RECT_SMOJIPLT);
        plate_l(0xda, 0xdb, tx, ty + dy, 0xd5, ca, ca);
        Ps2Menu.fn<Cell>(STRTEX_CELL)(wi32(w, 0x68), tx + 0x15, ty + dy + 5, 170, 14, alpha);
        var pm = pos(POS_PMSHAPE);
        for (int k = 0; k < n; k++) {
            int row = k / 2;
            int y = step * row + ry + ratio(dy, row);
            int x = (k & 1) != 0 ? i16(POS_RSTAPLT) + dx * row / 32 : i16(POS_LSTAPLT) - dx * row / 32;
            if (y > 0x1a0 || x > 0x200 || x < -0x200) continue;
            kind_plate(x, y, 0x10);
            shape(0xd7, x + pm.x + 0x58, y + pm.y);
            int id = list[k];
            if (id == 0) continue;
            int strings;
            if (sub == 1) {
                // PS2 0x00226690: a party member (icon 0x13, name), 100 the "summon" command's name
                if (id == 100) Ps2Menu.font(ptr(COMMAND_NAME, 0x30ff), x + 12, y + 1, 0);
                else {
                    Ps2Menu.icon(x + 15, y + 2, 0x13);
                    Ps2Menu.font(ptr(CHR_NAME, id), x + 34, y + 1, 0);
                }
            } else if (kind == 0) {
                nint rec = Ps2Menu.fn<PtrOfIntPtr>(ROM_ABILITY)(id, &strings);
                icon_name(*(byte*)(rec + 0x68), strings + *(ushort*)rec, x, y, 0);
            } else {
                nint rec = Ps2Menu.fn<PtrOfIntPtr>(COM_DATA)(id, &strings);
                icon_name(*(byte*)(rec + 0x14), strings + *(ushort*)rec, x, y, 0);
            }
        }
        first(2, "ui.menus: status ability page drawn the PS2 way");
    });

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(ModHost m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(ModHost m) {
        _log = m.log;
        return hook<WinDraw>(m, MAIN_DRAW, main_draw)
            & hook<WinDraw>(m, SUB_DRAW, sub_draw);
    }
}
