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
