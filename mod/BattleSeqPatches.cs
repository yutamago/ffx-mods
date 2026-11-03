// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

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

    /// <summary>The battle words at their PS2 places (PS2 texels x0, y0, x1, y1; converter uijob::BATTLE_WORDS).</summary>
    private static readonly int[][] WORDS = [[0, 110, 164, 127], [164, 96, 231, 118], [55, 96, 152, 110], [0, 96, 45, 106]];

    private static FhLogger? _log;
    private static int _logged;
    private static int _in_battle_seq;
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
        if (scene != BATTLE_SCENE) return _seq_next(scene, a, b);
        Interlocked.Increment(ref _in_battle_seq);
        try { return _seq_next(scene, a, b); } finally { Interlocked.Decrement(ref _in_battle_seq); }
    }

    private static int prim_layout(nint prim) {
        _prim_next ??= Native.fn<PrimLayout>(PRIM_LAYOUT).chain_from(_prim_hook!).fnptr!;
        int shape = *(ushort*)(prim + 0x14);
        int y = *(short*)(prim + 0xe);
        if (Volatile.Read(ref _in_battle_seq) == 0 || shape is not (413 or 414 or 421) || y <= 300) return _prim_next(prim);
        int x = *(short*)(prim + 0xc);
        int sx = *(short*)(prim + 0x10), sy = *(short*)(prim + 0x12);
        uint c = *(uint*)(prim + 4);
        _part ??= Native.fn<ShapePart>(SHAPE_PART).fnptr!;
        first(1, $"ui.menus: battle sequence shape {shape} (the Overdrive tab) drawn from the PS2 battle atlas");
        MainMenuPatches.battle_scoped(() => {
            byte* e = layout_entry(shape - 400);
            int n = *(int*)e;
            for (int k = 0; k < n; k++) {
                byte* p = e + 4 + 12 * k;
                // a part: u8 u, v; i8 dx, dy; u16 w, h; 4 more bytes. TOMkpShapePartIndexXYWHC2
                // does not add the offsets (the cursor port passes them too)
                int dx = *(sbyte*)(p + 2), dy = *(sbyte*)(p + 3), w = *(ushort*)(p + 4), h = *(ushort*)(p + 6);
                _part(shape, k, x + dx * sx / 100f, y + dy * sy / 100f, w * sx / 100f, h * sy / 100f, c, c);
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

    public static bool install(DemasterModule m) {
        _log = m.log;
        _seq_hook = seq_draw;
        _prim_hook = prim_layout;
        _icon_hook = btl_icon;
        _keep.AddRange([_seq_hook, _prim_hook, _icon_hook]);
        return Native.fn<SeqDraw>(SEQ_DRAW).hook(m, _seq_hook)
            & Native.fn<PrimLayout>(PRIM_LAYOUT).hook(m, _prim_hook)
            & Native.fn<BtlIcon>(BTL_ICON).hook(m, _icon_hook);
    }
}
