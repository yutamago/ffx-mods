// SPDX-License-Identifier: MIT

using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus: shared pieces of the ported PS2 menu draws (MainMenuPatches, ItemMenuPatches and the
/// sub-screens): FFX.exe's draw primitives the PS2 draws map to, a cache of game functions, the
/// PS2 layout words, and the PS2's small menu fonts M and S drawn from the converter's ps2fnt
/// texture. All coordinates are PS2 screen units (512 x 416). Evidence: docs/ui.md.
/// </summary>
internal static unsafe class Ps2Menu {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt2(int a, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt3(int a, int b, int c);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt2(int a, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfIntPtr(int a, int* b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfPtr(nint a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxC2(int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxRGBA(int x, int y, int w, int h, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Plate2(int a, int b, int x, int y, int w, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXY(int id, int x, int y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXYC2(int id, int x, int y, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXYWHC2(int id, int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int ShapePart(int id, int part, float x, float y, float w, float h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeUV(int id, float x, float y, float w, float h, float u0, float v0, float u1, float v1, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Font(nint text, int x, int y, int colour);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void FontA(nint text, int x, int y, int colour, int alpha);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void IconXYWH(float x, float y, float w, float h, int icon, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Scissor(int x0, int y0, int x1, int y1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqPoint(int scene, nint name, int* x, int* y);

    // FFX.exe primitives (ffx.sym VAs; the PS2 function each stands for in brackets)
    public const uint BOX_C2      = 0x008f3db0; // TODrawBoxXYWHC2: untextured, c0 left -> c1 right (PS2 0x001f0330)
    public const uint BOX_RGBA    = 0x008fb150; // TOMakePktBoxXYWHRGBA
    public const uint PLATE_LR    = 0x008f5e90; // (A, B, x, y, w, c0, c1): A stretched c0..c1, B in c1 (PS2 0x001ff2b8)
    public const uint PLATE_RL    = 0x008f5e20; // (A, B, x, y, w, c0, c1): A in c0, B stretched (PS2 0x001ff1e0)
    public const uint SHAPE_XY    = 0x00903a10; // TOMkpShapeXY (PS2 0x001fe1f0)
    public const uint SHAPE_XYC2  = 0x00903a50; // TOMkpShapeXYC2
    public const uint SHAPE_XYWH  = 0x00903b70; // TOMkpShapeXYWHC2 (w, h 0: the shape's own)
    public const uint SHAPE_PART  = 0x009034b0; // TOMkpShapePartIndexXYWHC2
    public const uint SHAPE_UV    = 0x00903ee0; // TOMkpShapeXYWHUVC2: ids below 199 sheet 0x3e00, UVs 0..1
    public const uint FONT_1612   = 0x00901cb0; // TOMkpBaseMesFont: the 16x12 font (PS2 0x001f3810)
    public const uint FONT_1612A  = 0x00901ce0; // (text, x, y, colour, alpha) (PS2 0x001f3858)
    public const uint ICON        = 0x008e6af0; // DrawCrossMenuIconXYWHRGBA (PS2 0x001f9798 draws 15 x 18 1:1)
    public const uint SCISSOR     = 0x008fdee0; // TOMakePktScissor(x0, y0, x1, y1)
    public const uint OPEN_PKT    = 0x008bef30; // TOMenuOpenPktBuffTmp
    public const uint KICK        = 0x008bea20; // TOMenuDrawKickTmp
    public const uint VU1_SYNC    = 0x0088ec60; // TkVU1SyncPath
    public const uint SEQ_POINT   = 0x008ae450; // TOSeqGetPoint(scene, name, &x, &y)
    public const uint MENU_TIMER  = 0x008a9c60; // TkMenuGetTimer

    public const uint GREY = 0x80808080;
    public const uint SHADOW = 0x80000000;

    private static readonly ConcurrentDictionary<(uint, Type), Delegate> _fns = new();

    /// <summary>A game function at an ffx.sym VA, bound on first use (Fahrenheit hands out call pointers only after every module's init).</summary>
    public static T fn<T>(uint va) where T : Delegate =>
        (T)_fns.GetOrAdd((va, typeof(T)), k => Native.fn<T>(k.Item1).fnptr!);

    public static short i16(uint va) => Native.i16(va);
    public static ushort u16(uint va) => *(ushort*)Native.ptr(va);
    public static uint u32(uint va) => Native.u32(va);
    public static int i32(uint va) => (int)Native.u32(va);
    public static byte u8(uint va) => Native.u8(va);
    /// <summary>A point of TOSeqInitParam's layout: stored mirrored (y, x) from its lower address.</summary>
    public static (int x, int y) point(uint va) => (i16(va + 2), i16(va));

    // ---- primitives ------------------------------------------------------------------------------

    public static void box(int x, int y, int w, int h, uint c0, uint c1) => fn<BoxC2>(BOX_C2)(x, y, w, h, c0, c1);
    public static void box_rgba(int x, int y, int w, int h, int r, int g, int b, int a) => fn<BoxRGBA>(BOX_RGBA)(x, y, w, h, r, g, b, a);
    public static void plate_lr(int a, int b, int x, int y, int w, uint c0, uint c1) => fn<Plate2>(PLATE_LR)(a, b, x, y, w, c0, c1);
    public static void plate_rl(int a, int b, int x, int y, int w, uint c0, uint c1) => fn<Plate2>(PLATE_RL)(a, b, x, y, w, c0, c1);
    public static void shape(int id, int x, int y) => shape_c2(id, x, y, GREY, GREY);

    public const uint MESWIN_LAYOUT = 0x00c5b460; // FFX.exe's meswin layout table (= SLPM_675.13 0x00321328)

    /// <summary>
    /// A meswin shape at its own size (PS2 0x001fe1f0 / TOMkpShapeXY): every part of the layout entry
    /// (id, or id - 200 from 200) as a sprite at (x + dx, y + dy), w x h, with the UVs half a texel
    /// inside the part's cell (TOMkpShapeXYWHUVC2, PS2 atlas 256 x 256). The PS2 drew these 1:1, so
    /// it sampled texel centres; the PC's TOAdpEasyShapeXYWHC2 samples to the cell edges, and at
    /// the PC's scale the filter pulls in the neighbouring texels: shapes tiled from 8-texel pieces
    /// (0xd7's line, 0x3e's strip) showed as dashes. It also skips all parts of 0x3e but the last.
    /// Other sheets (ids 400 and up) go to TOMkpShapeXYC2.
    /// </summary>
    public static void shape_c2(int id, int x, int y, uint c0, uint c1) {
        if (id < 0 || id >= 399 || id == 199) { fn<ShapeXYC2>(SHAPE_XYC2)(id, x, y, c0, c1); return; }
        byte* table = Native.ptr(MESWIN_LAYOUT);
        byte* e = table + *(uint*)(table + 4 + 4 * (id % 200));
        int n = *(int*)e;
        var uv = fn<ShapeUV>(SHAPE_UV);
        for (int k = 0; k < n; k++) {
            byte* p = e + 4 + 12 * k;
            int u = p[0], v = p[1], dx = *(sbyte*)(p + 2), dy = *(sbyte*)(p + 3);
            int w = *(ushort*)(p + 4), h = *(ushort*)(p + 6);
            float u0 = (u + 0.5f) / 256f, u1 = (u + w - 0.5f) / 256f, v0 = (v + 0.5f) / 256f, v1 = (v + h - 0.5f) / 256f;
            if (*(ushort*)(p + 8) == 1) (u0, u1) = (u1, u0);
            if (*(ushort*)(p + 10) == 1) (v0, v1) = (v1, v0);
            uv(id, x + dx, y + dy, w, h, u0, v0, u1, v1, c0, c1);
        }
    }
    public static void shape_wh(int id, int x, int y, int w, int h, uint c0, uint c1) => fn<ShapeXYWHC2>(SHAPE_XYWH)(id, x, y, w, h, c0, c1);
    public static void part(int id, int p, float x, float y, float w, float h, uint c0, uint c1) => fn<ShapePart>(SHAPE_PART)(id, p, x, y, w, h, c0, c1);
    public static void font(nint text, int x, int y, int colour) => fn<Font>(FONT_1612)(text, x, y, colour);
    public static void font_a(nint text, int x, int y, int colour, int alpha) => fn<FontA>(FONT_1612A)(text, x, y, colour, alpha);
    public static void icon(int x, int y, int id) => fn<IconXYWH>(ICON)(x, y, 15, 18, id, 0x80, 0x80, 0x80, 0x80);
    public static void scissor(int x0, int y0, int x1, int y1) => fn<Scissor>(SCISSOR)(x0, y0, x1, y1);
    public static void cursor(int x, int y) => MainMenuPatches.cursor(x, y);
    public static int timer() => fn<IntFn>(MENU_TIMER)();

    /// <summary>
    /// A ported PS2 draw: meswin answered by the PS2 atlas (ps2win), the draws in one packet buffer
    /// (as the PC's own menu draws: TkVU1SyncPath, TOMenuOpenPktBuffTmp, ..., TOMenuDrawKickTmp).
    /// </summary>
    public static int packet(Action draw) => MainMenuPatches.ps2_scoped(() => {
        fn<IntFn>(VU1_SYNC)();
        fn<IntFn>(OPEN_PKT)();
        draw();
        fn<IntFn>(KICK)();
        return fn<IntFn>(VU1_SYNC)();
    });

    private static readonly ConcurrentDictionary<string, nint> _names = new();

    /// <summary>A long-lived ANSI copy of a constant string (sequence names).</summary>
    public static nint cstr(string s) => _names.GetOrAdd(s, Marshal.StringToHGlobalAnsi);

    /// <summary>TOSeqGetPoint(scene, name): a point of a menu sequence (scene 1: the menu's).</summary>
    public static (int x, int y) seq_point(string name, int scene = 1) {
        int x = 0, y = 0;
        fn<SeqPoint>(SEQ_POINT)(scene, cstr(name), &x, &y);
        return ((short)x, (short)y);
    }

    /// <summary>A string for one call (the PS2's sprintf into a stack buffer).</summary>
    public static void text(string s, Action<nint> use) {
        nint p = Marshal.StringToHGlobalAnsi(s);
        try { use(p); } finally { Marshal.FreeHGlobal(p); }
    }

    /// <summary>The 16x12 font with a managed string.</summary>
    public static void font(string s, int x, int y, int colour) => text(s, p => font(p, x, y, colour));

    // ---- shared PS2 menu pieces ------------------------------------------------------------------

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void SheetCell(int cell, float x, float y, float w, float h);
    public const uint SHEET_CELL = 0x008f8bb0;  // TODrawSheetCell_008f8bb0: strtex (0x2ecc) cell (PS2 0x00201768)
    public const uint COL_BSOPLT0 = 0x025d16a0; // plate colour pairs bso, bitem, bab, lmt (c0, c1 4 bytes apart)

    /// <summary>
    /// A list row's plate by kind (PS2 0x00226d90 -> TOAdpShapeByKind 0x001f4c60; the PC's builder
    /// 0x008f1a90 lost kind 1's line): meswin 0xda at x in c0, 0xdb from x + 16, 209 wide, c0 -> c1;
    /// kind &amp; 3 adds 0xd7, 0x10b or 0x10a at x + 97 in black; kind &gt;&gt; 3 picks col_bsoplt (grey),
    /// col_bitemplt (gold), col_babplt (purple), col_lmtplt.
    /// </summary>
    public static void kind_plate(int x, int y, int kind) {
        uint at = COL_BSOPLT0 - 8 * (uint)((kind >> 3) & 3);
        uint c0 = u32(at), c1 = u32(at - 4);
        shape_wh(0xda, x, y, 0, 0, c0, c0);
        shape_wh(0xdb, x + 16, y, 209, 0, c0, c1);
        int extra = (kind & 3) switch { 1 => 0xd7, 2 => 0x10b, 3 => 0x10a, _ => 0 };
        if (extra != 0) shape_c2(extra, x + 97, y, SHADOW, SHADOW);
    }

    /// <summary>PS2 0x00200a60 / 0x00200878: a dark track, a light thumb over rows top .. top + shown of count.</summary>
    public static void scroll_bar(int x, int y, int w, int h, int top, int shown, int count) {
        box_rgba(x, y, w, h, 0x11, 0x11, 0x11, 0x80);
        float t = top == 0 ? y : y + (float)h * top / count;
        float b = top + shown >= count ? y + h : y + (float)h * (top + shown) / count;
        box_rgba(x + 1, (int)t, w - 2, (int)b - (int)t, 0xf0, 0xf0, 0xf0, 0x80);
    }

    /// <summary>A label (PS2 0x00226218 -> 0x00201a70): a black plate 0xda/0xdb 213 wide, strtex cell k at (+21, +5), 170 x 14.</summary>
    public static void label_plate(int x, int y, int k) {
        plate_rl(0xda, 0xdb, x, y, 213, SHADOW, SHADOW);
        fn<SheetCell>(SHEET_CELL)(k, x + 21, y + 5, 170, 14);
    }

    // the party plate's layout (TOSeqInitParam, mirrored points; TkMn2GetMainPos)
    private const uint NAMEPLT_W = 0x025d1672, COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144, COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;
    private const uint PARTY_MASK = 0x01841c2c; // the front-line bits
    private const uint CHR_NAME = 0x008ac850, DAMAGE_CLASS = 0x008a99f0, SPHERE_LV = 0x008a9b80;
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint NEXT_AP = 0x00784e90, SAVE_NEXT_AP = 0x007852b0;

    /// <summary>
    /// PS2 0x00225f38 -> 0x00200c60 (the PC's MenuStatus_DrawLayoutRow / TOMenuDrawChrLayoutPlate):
    /// the light band for a front-line member, then the party plate: plate 4/5, the name in the
    /// damage colour, HP/MP labels and "%5d/%5d", "%4d/%4d" in font S 0x25, shape 0xe7 across the
    /// screen, shape 0x107 (S.LV badge, "Next ... AP") at pos_slvshape + (0x68, 0), the sphere level
    /// in font M 0 and the AP to the next level in font S 0x25 at pos_slvshape + (139, -13). Aeons
    /// (chr 8 and up): plate 0xcc/0xcd, shapes 0xe9/0xea, 0xe7, 0x107. The places are the menu
    /// sequence's (the main menu's party rows use the same); the MP string's last cell ends at x 296,
    /// where the S.LV label begins.
    /// </summary>
    public static void party_plate(int x, int y, int chr) {
        if (chr >= 0 && chr < 32 && (u32(PARTY_MASK) & (1u << chr)) != 0)
            box(x + i16(POS_NOACTIVE), y + i16(POS_NOACTIVE + 2), 512, i16(NOACTIVE_H), u32(COL_NOACTIVE0), u32(COL_NOACTIVE1));
        var hptex = point(POS_HPTEX);
        var mptex = point(POS_MPTEX);
        var slv = point(POS_SLVSHAPE);
        int w = i16(NAMEPLT_W);
        uint c0 = u32(COL_NAMEPLT0), c1 = u32(COL_NAMEPLT1);
        if ((uint)chr >= 8) {
            plate_lr(0xcc, 0xcd, x, y, w, c0, c1);
            shape(0xe9, x + hptex.x, y + hptex.y);
            shape(0xea, x + mptex.x, y + mptex.y);
            shape_wh(0xe7, 0, y, 512, 0, GREY, GREY);
            shape(0x107, x + slv.x + 0x68, y + slv.y);
            return;
        }
        var v = fn<IntOfInt>;
        plate_lr(4, 5, x, y, w, c0, c1);
        var namestr = point(POS_NAMESTR);
        font(fn<PtrOfInt>(CHR_NAME)(chr), x + namestr.x, y + namestr.y, v(DAMAGE_CLASS)(chr));
        var hpstr = point(POS_HPSTR);
        var mpstr = point(POS_MPSTR);
        shape(0x21, x + hptex.x, y + hptex.y);
        font_s($"{v(HP)(chr),5}/{v(MAX_HP)(chr),5}", x + hpstr.x, y + hpstr.y, 0x25);
        shape(0x22, x + mptex.x, y + mptex.y);
        font_s($"{v(MP)(chr),4}/{v(MAX_MP)(chr),4}", x + mpstr.x, y + mpstr.y, 0x25);
        shape_wh(0xe7, 0, y, 512, 0, GREY, GREY);
        var spstr = point(POS_SPSTR);
        shape(0x107, x + slv.x + 0x68, y + slv.y);
        value_m(v(SPHERE_LV)(chr), x + slv.x + spstr.x, y + slv.y + spstr.y, 0);
        value_s(v(NEXT_AP)(chr) - v(SAVE_NEXT_AP)(chr), x + slv.x + 139, y + slv.y - 13, 0x25);
    }

    // ---- the PS2 fonts M and S ------------------------------------------------------------------

    /// <summary>
    /// The PS2's small menu fonts, both monospace in xfont1208 (0x3d80, two 2-bit pages, glyph
    /// i = c - 0x20 on page i &amp; 1): font M (SLPM_675.13 0x001f6110) cells 10 x 12 from v 144,
    /// 12 a row of a page, advance 12; font S (0x001f66f0) cells 8 x 10 from v 192, 16 a row,
    /// advance 10. Each glyph is drawn twice: the shadow at (x + 1, y + 1) with menu.clp CLUT 7
    /// row 2 (colour &gt;&gt; 4) + page, then the face at (x, y) with CLUT 6 row 2 (colour &amp; 15)
    /// + page. The PC has no working copy (its 0x3d80 sheet names a file the archive lacks), so
    /// the converter renders every colour into ps2fnt (uijob::PS2_FONT: 1024 x 1024, tiles of
    /// 256 x 256, tile k (k % 4, k / 4): faces 0..7 then shadows 0..7; glyph i of font M at
    /// (1, 1) in the 16 x 16 cell (i % 16, i / 16), of font S in (i % 16, 6 + i / 16)), and the
    /// glyphs are meswin shapes with explicit UVs while TOGetShapTextureName(0x3e00) answers
    /// ps2fnt (MainMenuPatches.font_scoped).
    /// </summary>
    private static void glyphs(string s, int x, int y, int colour, bool m) {
        MainMenuPatches.font_scoped(() => {
            var uv = fn<ShapeUV>(SHAPE_UV);
            int face = colour & 7, shade = (colour >> 4) & 7;
            int w = m ? 10 : 8, h = m ? 12 : 10, adv = m ? 12 : 10;
            foreach (char ch in s) {
                int i = ch - 0x20;
                if (i > 0 && i < 0x60) {
                    float u = 16 * (i % 16) + 1, v = 16 * ((m ? 0 : 6) + i / 16) + 1;
                    for (int pass = 0; pass < 2; pass++) {
                        int k = pass == 0 ? 8 + shade : face;
                        float tu = 256 * (k % 4) + u, tv = 256 * (k / 4) + v;
                        int d = pass == 0 ? 1 : 0;
                        uv(0, x + d, y + d, w, h, tu / 1024f, tv / 1024f, (tu + w) / 1024f, (tv + h) / 1024f, GREY, GREY);
                    }
                }
                x += adv;
            }
        });
    }

    /// <summary>Font M text at (x, y) (PS2 0x001f5ff8).</summary>
    public static void font_m(string s, int x, int y, int colour) => glyphs(s, x, y, colour, true);

    /// <summary>Font S text at (x, y) (PS2 0x001f6500 / 0x001f6630).</summary>
    public static void font_s(string s, int x, int y, int colour) => glyphs(s, x, y, colour, false);

    /// <summary>"%d" in font M right-aligned: the last cell ends at x + 2 (PS2 0x001f5e88 / 0x001f5f58).</summary>
    public static void value_m(int v, int x, int y, int colour) {
        string s = v.ToString();
        font_m(s, x + 2 - 12 * s.Length, y, colour);
    }

    /// <summary>"%d" in font S right-aligned: the last cell ends at x + 2 (PS2 0x001f63c0 / 0x001f6408).</summary>
    public static void value_s(int v, int x, int y, int colour) {
        string s = v.ToString();
        font_s(s, x + 2 - 10 * s.Length, y, colour);
    }

    /// <summary>
    /// The play time (PS2 0x0024ad28, t in seconds): "%02d" minutes at x - 18, ":" at x - 28 in
    /// the colon's colour, "%3d" hours (at most 999) at x - 58, all font S.
    /// </summary>
    public static void play_time(uint t, int x, int y, int colour, int colon) {
        uint hours = Math.Min(t / 3600, 999), minutes = t / 60 % 60;
        font_s(minutes.ToString("00"), x - 18, y, colour);
        font_s(":", x - 28, y, colon);
        font_s($"{hours,3}", x - 58, y, colour);
    }
}
