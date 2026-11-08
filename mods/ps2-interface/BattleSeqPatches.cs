// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.menus, battle: the PS2 pieces the battle draws through its sequence data and its damage
/// display. Evidence: docs/ui.md ("The Overdrive tab", "Battle words").
/// <list type="bullet">
/// <item>The Overdrive tab: on the PS2 and the PC, TOOpenBtlWinMain starts the battle sequence
/// scene "bnormalllay" (or "blimitlay" the first time) when the gauge is full, and TODrawBtlWindow
/// runs it (TOSeqDraw(2, ...)). Its tracks draw battle shape 413 (battle layout 13: the "EKSTASE!!"
/// brush plate, (0, 112) 160 x 16 at -32, 0) sliding in to (46, 318) and blinking. The PC's
/// primitive drawer (TOMakePktSeqPrimLayout_008fe0b0) draws 413 only above y 300 (the overdrive
/// mini-games' cursor) and drew its own HD tab inside the main window draw instead, which the PS2
/// window port drops. Here 413/414/421 below y 300 are drawn as the PS2's drawer (0x001fd268)
/// does: each part of the battle layout at x + dx * sx / 100, y + dy * sy / 100, in the
/// primitive's colour, from the PS2 battle atlas (ps2btl).</item>
/// <item>The rest of the battle sequence's look (docs/ui.md, "The battle sequence scenes"): the PS2
/// starts a layer-0 scene under the command box with every command window (bnormal, bnormale,
/// bnormall, bnormalt by MsCheckLeftWindow: an orange glow, a purple bar and lines under the
/// Overdrive tab, row bands 418) and its layer-1 twin over it (the BEFEHL brush 415, the ring 417,
/// the "&gt;&gt;&gt;" plate 411, a line along the bottom); the PC starts only bnormallay / bnormalllay
/// and blimitlay, skips 411, 415, 417 (below y 330) and 418 in its primitive drawer, and draws no
/// scene line at all (TOMakePktSeqPrimLine 0x008fe070 is a bare ret). Here the PS2's scenes are
/// started, those shapes drawn as the PS2's drawer does, and the battle scenes' lines drawn.</item>
/// <item>The battle words (TOMakePktBtlIconA, kinds 0..3: OVERKILL, IMMUNE, MISS, MP): the PS2
/// (0x001f9b50) draws its icon-sheet rectangles 1:1; the PC draws HD rectangles at about half the
/// PS2 width. The converter puts the words at the PS2 places of every icon sheet
/// (uijob::BATTLE_WORDS), and this draws them from there at the PS2 size.</item>
/// </list>
/// </summary>
internal static unsafe class BattleSeqPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqDraw(int scene, int a, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int PrimLayout(nint prim);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int BtlIcon(int kind, int x, int y, int alpha);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int ShapePart(int id, int part, float x, float y, float w, float h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int ShapeUV(int id, float x, float y, float w, float h, float u0, float v0, float u1, float v1, uint c0, uint c1);

    private const uint SEQ_DRAW    = 0x008ae2f0; // TOSeqDraw(scene, a, b)
    private const uint PRIM_LAYOUT = 0x008fe0b0; // TOMakePktSeqPrimLayout: a sequence layout primitive
    private const uint BTL_ICON    = 0x008fbcc0; // TOMakePktBtlIconA(kind, x, y, alpha)
    private const uint SHAPE_PART  = 0x009034b0; // TOMkpShapePartIndexXYWHC2
    private const uint SHAPE_UV    = 0x00903ee0; // TOMkpShapeXYWHUVC2 (shapes 600+: sheet 0x3dc0, HD-pixel UVs)
    private const uint BATTLE_LAYOUT = 0x00c5c2c8; // the battle atlas's layout table (= SLPM_675.13 0x00322190)

    private const int BATTLE_SCENE = 2;
    private const int MENU_SCENE = 1;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int OpenWin(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int CheckLeft(int chr);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqScene(int seq, nint name);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Line(nint prim);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxC2(int x, int y, int w, int h, uint c0, uint c1);

    private const uint OPEN_MAIN   = 0x008a8640; // TOOpenBtlWinMain(w) (PS2 0x002108a0)
    private const uint OPEN_LIMIT  = 0x008a85a0; // TOOpenBtlLimitBreak(w) (PS2 0x00212f38)
    private const uint CHECK_LEFT  = 0x0078f700; // MsCheckLeftWindow(chr) -> 0..3 (PS2 0x001dc368)
    private const uint SEQ_START   = 0x008af0d0; // TOSeqStartScene(seq, name)
    private const uint LINE_CALL   = 0x00a7c20d; // the scene line's call to TOMakePktSeqPrimLine (0x008fe070, a bare ret)
    private const uint BOX_C2      = 0x008f3db0; // TODrawBoxXYWHC2(x, y, w, h, c0, c1)

    /// <summary>Battle shapes the PC's drawer skips that the PS2 scenes put below the windows' top (y &gt; 300 / 330 / any); every shape while an Overdrive mini-game window runs (BattleLimitPatches.active).</summary>
    private static bool ps2_shape(int shape, int y) => BattleLimitPatches.active() || shape switch {
        413 or 414 or 421 => y > 300,
        417 => y > 330,
        411 or 415 or 416 or 418 or 419 => true,
        _ => false,
    };

    /// <summary>The battle words at their PS2 places (PS2 texels x0, y0, x1, y1; converter uijob::BATTLE_WORDS).</summary>
    private static readonly int[][] WORDS = [[0, 110, 164, 127], [164, 96, 231, 118], [55, 96, 152, 110], [0, 96, 45, 106]];

    private static FhLogger? _log;
    private static int _logged;
    private static int _in_battle_seq, _in_menu_seq;
    private static readonly List<Delegate> _keep = [];
    private static SeqDraw? _seq_hook, _seq_next;
    private static PrimLayout? _prim_hook, _prim_next;
    private static BtlIcon? _icon_hook;
    private static ShapePart? _part;
    private static ShapeUV? _uv;

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static int seq_draw(int scene, int a, int b) {
        _seq_next ??= Native.fn<SeqDraw>(SEQ_DRAW).chain_from(_seq_hook!).fnptr!;
        if (scene == MENU_SCENE) {
            Interlocked.Increment(ref _in_menu_seq);
            try { return _seq_next(scene, a, b); } finally { Interlocked.Decrement(ref _in_menu_seq); }
        }
        if (scene != BATTLE_SCENE) return _seq_next(scene, a, b);
        Interlocked.Increment(ref _in_battle_seq);
        try { return _seq_next(scene, a, b); } finally { Interlocked.Decrement(ref _in_battle_seq); }
    }

    private static int prim_layout(nint prim) {
        _prim_next ??= Native.fn<PrimLayout>(PRIM_LAYOUT).chain_from(_prim_hook!).fnptr!;
        int shape = *(ushort*)(prim + 0x14);
        int y = *(short*)(prim + 0xe);
        if (Volatile.Read(ref _in_battle_seq) == 0 || !ps2_shape(shape, y)) return _prim_next(prim);
        int x = *(short*)(prim + 0xc);
        int sx = *(short*)(prim + 0x10), sy = *(short*)(prim + 0x12);
        uint c = *(uint*)(prim + 4);
        _part ??= Native.fn<ShapePart>(SHAPE_PART).fnptr!;
        first(1, $"ui.menus: battle sequence shapes (the Overdrive tab, the BEFEHL footer) drawn from the PS2 battle atlas");
        MainMenuPatches.battle_scoped(() => {
            byte* e = layout_entry(shape - 400);
            int n = *(int*)e;
            _uv ??= Native.fn<ShapeUV>(SHAPE_UV).fnptr!;
            for (int k = 0; k < n; k++) {
                byte* p = e + 4 + 12 * k;
                // a part: u8 u, v; i8 dx, dy; u16 w, h, flip x, flip y; sampled at texel centres
                int u = p[0], v = p[1], dx = *(sbyte*)(p + 2), dy = *(sbyte*)(p + 3), w = *(ushort*)(p + 4), h = *(ushort*)(p + 6);
                float u0 = (u + 0.5f) / 256f, u1 = (u + w - 0.5f) / 256f, v0 = (v + 0.5f) / 256f, v1 = (v + h - 0.5f) / 256f;
                if (*(ushort*)(p + 8) == 1) (u0, u1) = (u1, u0);
                if (*(ushort*)(p + 10) == 1) (v0, v1) = (v1, v0);
                _uv(shape, x + dx * sx / 100f, y + dy * sy / 100f, w * sx / 100f, h * sy / 100f, u0, v0, u1, v1, c, c);
            }
        });
        return 0;
    }

    private static byte* layout_entry(int i) {
        byte* table = (byte*)Native.ptr(BATTLE_LAYOUT);
        return table + *(uint*)(table + 4 + 4 * i);
    }

    /// <summary>TOMakePktBtlIconA: kinds 0..3 from the PS2 rectangles at the PS2 size, colour 0x808080 with the given alpha.</summary>
    private static int btl_icon(int kind, int x, int y, int alpha) {
        uint c = ((uint)alpha << 24) | 0x808080;
        if (kind is < 0 or > 3) return (int)c;
        _uv ??= Native.fn<ShapeUV>(SHAPE_UV).fnptr!;
        first(2, "ui.menus: battle words drawn from the PS2 places at the PS2 size");
        int[] r = WORDS[kind];
        return _uv(600, x, y, r[2] - r[0], r[3] - r[1], 4 * r[0], 4 * r[1], 4 * r[2], 4 * r[3], c, c);
    }

    // ---- the scenes the PS2 starts with a command window -------------------------------------

    private static OpenWin? _main_hook, _main_next, _limit_hook, _limit_next;
    private static readonly Dictionary<string, nint> _names = [];

    private static void start(string name) {
        if (!_names.TryGetValue(name, out nint p)) _names[name] = p = Marshal.StringToHGlobalAnsi(name);
        Ps2Menu.fn<SeqScene>(SEQ_START)(BATTLE_SCENE, p);
    }

    /// <summary>
    /// TOOpenBtlWinMain: the PS2 (0x002108a0) starts by MsCheckLeftWindow 0..3 the layer-0 scene
    /// bnormal / bnormale / bnormall / bnormalt and its "lay" twin; the PC starts only bnormallay
    /// and bnormalllay. The PC's call (TOSeqInitScene, its lay scene) runs first.
    /// </summary>
    private static int open_main(nint w) {
        _main_next ??= Native.fn<OpenWin>(OPEN_MAIN).chain_from(_main_hook!).fnptr!;
        int r = _main_next(w);
        int v = Ps2Menu.fn<CheckLeft>(CHECK_LEFT)(*(short*)(w + 8));
        string suffix = v switch { 1 => "e", 2 => "l", 3 => "t", _ => "" };
        start("bnormal" + suffix);
        if (v is 1 or 3) start("bnormal" + suffix + "lay");
        first(4, "ui.menus: the PS2 battle sequence scenes started with the command window");
        return r;
    }

    /// <summary>
    /// TOOpenBtlLimitBreak: the PS2 (0x00212f38) starts bevent / blimit / btwo and their "lay"
    /// scenes by MsCheckLeftWindow 1 / 2 / 3; the PC starts only blimitlay (for 2).
    /// </summary>
    private static int open_limit(nint w) {
        _limit_next ??= Native.fn<OpenWin>(OPEN_LIMIT).chain_from(_limit_hook!).fnptr!;
        int r = _limit_next(w);
        int v = Ps2Menu.fn<CheckLeft>(CHECK_LEFT)(*(short*)(w + 8));
        string? name = v switch { 1 => "bevent", 2 => "blimit", 3 => "btwo", _ => null };
        if (name is not null) {
            start(name);
            if (v != 2) start(name + "lay");
        }
        return r;
    }

    // ---- the scene lines -------------------------------------------------------------------------

    private static Line? _line;

    /// <summary>
    /// A scene line (record +4 c0, +8 c1, +0xc x, +0xe y, +0x10 x2, +0x12 y2): the PS2 draws a
    /// gouraud, alpha-blended one-pixel line (0x001fcf90); here a one-pixel gradient box, c0 at
    /// (x, y). While the battle or the menu sequence draws: the PS2's line drawer serves every
    /// sequence (the menus' lines since 2026-11-04).
    /// </summary>
    private static void line(nint p) {
        if (Volatile.Read(ref _in_battle_seq) == 0 && Volatile.Read(ref _in_menu_seq) == 0) return;
        uint c0 = *(uint*)(p + 4), c1 = *(uint*)(p + 8);
        int x = *(short*)(p + 0xc), y = *(short*)(p + 0xe), x2 = *(short*)(p + 0x10), y2 = *(short*)(p + 0x12);
        var box = Ps2Menu.fn<BoxC2>(BOX_C2);
        if (y == y2) {
            if (x2 == x) return;
            if (x2 < x) { (x, x2) = (x2, x); (c0, c1) = (c1, c0); }
            box(x, y, x2 - x, 1, c0, c1);
        } else {
            box(x, Math.Min(y, y2), 1, Math.Abs(y2 - y), c0, c0);
        }
    }

    /// <summary>Points the scene line's call (0x00a7c20d) at <see cref="line"/>.</summary>
    private static bool lines(ModHost m) {
        _line = line;
        _keep.Add(_line);
        nint fp = Marshal.GetFunctionPointerForDelegate(_line);
        uint next = Native.reloc(LINE_CALL + 5);
        byte[] original = BitConverter.GetBytes(Native.reloc(0x008fe070) - next);
        byte[] patched = BitConverter.GetBytes((uint)fp - next);
        return Native.patch_code(LINE_CALL + 1, original, patched, m.log);
    }

    public static bool install(ModHost m) {
        _log = m.log;
        _seq_hook = seq_draw;
        _prim_hook = prim_layout;
        _icon_hook = btl_icon;
        _main_hook = open_main;
        _limit_hook = open_limit;
        _keep.AddRange([_seq_hook, _prim_hook, _icon_hook, _main_hook, _limit_hook]);
        return Native.fn<SeqDraw>(SEQ_DRAW).hook(m, _seq_hook)
            & Native.fn<PrimLayout>(PRIM_LAYOUT).hook(m, _prim_hook)
            & Native.fn<BtlIcon>(BTL_ICON).hook(m, _icon_hook)
            & Native.fn<OpenWin>(OPEN_MAIN).hook(m, _main_hook)
            & Native.fn<OpenWin>(OPEN_LIMIT).hook(m, _limit_hook)
            & lines(m);
    }
}
