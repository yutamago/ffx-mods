// SPDX-License-Identifier: MIT

using System.Globalization;
using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.menus, battle: the Overdrive mini-game windows (Tidus's timing bar, Auron's button input,
/// Lulu's Fury, the limit timer) drawn as SLPM_675.13 drew them, on FFX.exe's primitives, in PS2
/// units (512 x 416) from the PS2 battle atlas (ps2btl). Both platforms' TODrawBtlWindow (PS2
/// 0x00214ae4, PC 0x0089b360) call the same draws in the same place (between the party list and
/// the CTB list); the window states (PC 0x0133d6f4 Auron, 0x0133d734 Tidus, 0x0133d708 Lulu,
/// 0x0133f77c the timer) are the PS2's records with the PS2's offsets, already in PS2 units
/// (the PC's init remaps its HD constants with graphicUiRemapX2/Y2). The PC rewrote the draws with
/// HD sheets and HD sizes, and added a backdrop (TODrawLimitBG) and a frame round Wakka's reels
/// that the PS2 does not draw. Logic, input, timing and the bar's geometry stay the PC's.
/// Evidence: docs/mods/interface.md ("The Overdrive mini-game windows").
/// </summary>
internal static unsafe class BattleLimitPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Void();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint Text(int id, int second);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void LocText(int id, nint buf, int len);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxFW(int x, int y, int w, int h, uint c0, uint c1, int full);

    // hooked
    private const uint DRAW_AURON = 0x00892740; // TOBtlDrawAuronLimitWindow (PS2 0x0021ad88)
    private const uint DRAW_TIDUS = 0x00897240; // TOBtlDrawTidusLimitWindow (PS2 0x0021b360)
    private const uint DRAW_LULU  = 0x00895660; // TOBtlDrawLuluLimitWindow (PS2 0x0021b9d0)
    private const uint DRAW_TIMER = 0x008955e0; // BtlDrawRatio_008955e0: the limit timer (PS2 0x0021a8f8)
    private const uint LIMIT_BG   = 0x008a1ce0; // TODrawLimitBG: HD backdrop plates, no PS2 twin
    private const uint WAKKA_FRAME = 0x008a1e50; // TODrawWakkaLimitFrame_008a1e50: HD frame, no PS2 twin

    // called
    private const uint ROM_BTL_TEXT = 0x0078f8f0; // MsGetRomBtlText(id, 0) (PS2 0x001dc090)
    private const uint LOC_TEXT    = 0x006412f0; // graphicGetLocTextWithLength(id, buf, len)
    private const uint BOX_C2      = 0x008f3db0; // TODrawBoxXYWHC2 (PS2 0x001f0330)
    private const uint BOX_FW      = 0x008f3eb0; // TODrawBoxXYWHC2FW(x, y, w, h, c0, c1, full) (PS2 0x001f0588)
    private const uint SHAPE_UV    = 0x00903ee0; // TOMkpShapeXYWHUVC2: ids 600.. sheet 0x3dc0 with HD-pixel UVs
    private const uint ICON_LAYOUT = 0x00c5d578; // g_TOLayoutIcon (= SLPM_675.13 0x00323440), shapes 600..
    private const uint CONFIRM     = 0x008acf30; // TOGetSystemConfirmButtonAssign: 1 circle confirms (the Japanese layout)

    // read
    private const uint WAKKA_ON   = 0x0133c93f; // u8: Wakka's reels run (PS2 0x003249ae)
    private const uint AURON      = 0x0133d6f4; // i16 state, +2/+4 the buttons' place (dum217), +8 i16 first, +0xc i32 count, +0x10 the u16 list (PS2 0x00692678)
    private const uint LULU       = 0x0133d708; // i16 state, +2/+4 (dum209), +6 width 0xc0, +8/+0xc colours (dum210/211), +0x10/+0x12 the counter (dum212), +0x14 fill, +0x17 u8 count (PS2 0x006926a8)
    private const uint TIDUS      = 0x0133d734; // i16 state, +2 x, +4 y, +6 w, +8 hit x, +0xc hit w, +0xe marker x, +0x14 hit colour (dum203) (PS2 0x00692690)
    private const uint TIMER      = 0x0133f77c; // state, +4/+6 place (dum201), +0x18 f32 shown seconds (PS2 0x00692658)
    private const uint TEXT_Y     = 0x0133f778; // i16 dum225: the prompt line's y (PS2 0x00692656)
    private const uint LULU_MODE  = 0x00c58ef0; // g_BtlLuluControlMode: 1 pad, 0 keyboard (PC only)

    private const uint GREY = 0x80808080;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static T fn<T>(uint va) where T : Delegate => Ps2Menu.fn<T>(va);
    private static short i16(uint va) => Native.i16(va);

    /// <summary>
    /// A mini-game window runs (Auron, Lulu or Tidus in any state but 0) and Wakka's reels do not:
    /// the battle sequence's limit scenes (bauro, blulu, btidu: the cursor 413 above y 300, ...)
    /// then draw every shape from the PS2 battle atlas (BattleSeqPatches). Not during the reels:
    /// their scene (bwakk, layer 4) draws from battle menu texture 0x13 put at sheet 0x3f00.
    /// </summary>
    public static bool active() => (i16(AURON) != 0 || i16(LULU) != 0 || i16(TIDUS) != 0) && Native.u8(WAKKA_ON) == 0;

    private static bool shown(uint state) => (uint)(i16(state) - 3) < 2;

    /// <summary>The prompt line (PS2: TOGetMessageWH, then 0x001f3810 at ((512 - w) / 2, dum225), colour 0): the 16x12 font at the PS2 size.</summary>
    private static void prompt(nint text) {
        if (text == 0) return;
        int w = Ps2Menu.text_width(text);
        Ps2Menu.font(text, (512 - w) / 2, i16(TEXT_Y), 0);
    }

    private static void prompt(int id) => prompt(fn<Text>(ROM_BTL_TEXT)(id, 0));

    /// <summary>
    /// PS2 0x00201b68(x, y, w) (the PC's TOMkpBtlLimitSpecialGauge): battle shape 500 at (x - 7, y - 4),
    /// 502 at (x + w - 8, y - 4), 501 stretched over w at (x, y - 4).
    /// </summary>
    private static void gauge(int x, int y, int w) {
        BattleHudPatches.bshape(0x1f4, x - 7, y - 4, 0, 0, GREY, GREY);
        BattleHudPatches.bshape(0x1f6, x + w - 8, y - 4, 0, 0, GREY, GREY);
        BattleHudPatches.bshape(0x1f5, x, y - 4, w, 0, GREY, GREY);
    }

    // ---- Tidus: SLPM_675.13 0x0021b360 -------------------------------------------------------

    /// <summary>
    /// In states 3 and 4: the bar (gauge at +2, +4, +6), the hit zone as one box (+8, y, +0xc x 9)
    /// in dum203 (+0x14), the marker battle shape 477 at (+0xe, y), the prompt 0x3010. The PC
    /// draws two gradient halves and an HD marker.
    /// </summary>
    private static void tidus() {
        if (!shown(TIDUS)) return;
        byte* s = Native.ptr(TIDUS);
        short x = *(short*)(s + 2), y = *(short*)(s + 4), w = *(short*)(s + 6);
        uint c = *(uint*)(s + 0x14);
        MainMenuPatches.battle_scoped(() => {
            gauge(x, y, w);
            fn<Ps2Menu.BoxC2>(BOX_C2)(*(short*)(s + 8), y, *(short*)(s + 0xc), 9, c, c);
            BattleHudPatches.bshape(0x1dd, *(short*)(s + 0xe), y, 0, 0, GREY, GREY);
        });
        prompt(0x3010);
        first(1, "ui.menus: Tidus's Overdrive bar drawn the PS2 way");
    }

    // ---- Auron: SLPM_675.13 0x0021ad88 -------------------------------------------------------

    /// <summary>
    /// In states 3 and 4: the buttons still to press (list +0x10 from +8 to the count +0xc), each
    /// as 0x001fc778 draws it at (+2 + 30 i, +4), then the prompt 0x3011. The PC draws HD pad
    /// glyphs 1.5 times as big, 46 further left and at another height.
    /// </summary>
    private static void auron() {
        if (!shown(AURON)) return;
        byte* s = Native.ptr(AURON);
        short x = *(short*)(s + 2), y = *(short*)(s + 4);
        int count = *(int*)(s + 0xc);
        ushort* list = *(ushort**)(s + 0x10);
        if (list != null)
            for (int i = *(short*)(s + 8); i < count; i++) pad(list[i], x + 30 * i, y);
        prompt(0x3011);
        first(2, "ui.menus: Auron's Overdrive buttons drawn the PS2 way");
    }

    /// <summary>
    /// PS2 0x001fc778(buttons, x, y): icon shapes (600..) at (x, y): with a d-pad bit the cross
    /// from four pieces, each lit (601, 603, 605, 607 for 0x1000, 0x2000, 0x4000, 0x8000) or not
    /// (600, 602, 604, 606); then 0x10 -> 608, 0x20 / 0x40 -> 609 / 610 (swapped when circle
    /// confirms: the Japanese layout, the PC's TOGetSystemConfirmButtonAssign 1), 0x80 -> 611,
    /// 4 -> 612, 8 -> 613, 1 -> 614, 2 -> 615, 0x800 -> 616, 0x100 -> 617.
    /// </summary>
    private static void pad(int b, int x, int y) {
        if ((b & 0xf000) != 0) {
            ishape((b & 0x1000) != 0 ? 0x259 : 0x258, x, y);
            ishape((b & 0x2000) != 0 ? 0x25b : 0x25a, x, y);
            ishape((b & 0x4000) != 0 ? 0x25d : 0x25c, x, y);
            ishape((b & 0x8000) != 0 ? 0x25f : 0x25e, x, y);
        }
        if ((b & 0x10) != 0) ishape(0x260, x, y);
        bool japanese = fn<IntFn>(CONFIRM)() == 1;
        if ((b & 0x20) != 0) ishape(japanese ? 0x262 : 0x261, x, y);
        if ((b & 0x40) != 0) ishape(japanese ? 0x261 : 0x262, x, y);
        if ((b & 0x80) != 0) ishape(0x263, x, y);
        if ((b & 4) != 0) ishape(0x264, x, y);
        if ((b & 8) != 0) ishape(0x265, x, y);
        if ((b & 1) != 0) ishape(0x266, x, y);
        if ((b & 2) != 0) ishape(0x267, x, y);
        if ((b & 0x800) != 0) ishape(0x268, x, y);
        if ((b & 0x100) != 0) ishape(0x269, x, y);
    }

    /// <summary>
    /// An icon shape (600.., layout table 0x00c5d578) as the PS2's 0x001fe1f0 puts it: each part at
    /// (x + dx, y + dy), its own size, UVs half a texel inside the cell. The icon sheet is the PS2
    /// layout at 4x (1024 x 512), and TOMkpShapeXYWHUVC2 takes its UVs in those pixels.
    /// </summary>
    private static void ishape(int id, int x, int y) {
        byte* table = Native.ptr(ICON_LAYOUT);
        byte* e = table + *(uint*)(table + 4 + 4 * (id - 600));
        int n = *(int*)e;
        var uv = fn<Ps2Menu.ShapeUV>(SHAPE_UV);
        for (int k = 0; k < n; k++) {
            byte* p = e + 4 + 12 * k;
            int u = p[0], v = p[1], dx = *(sbyte*)(p + 2), dy = *(sbyte*)(p + 3);
            int pw = *(ushort*)(p + 4), ph = *(ushort*)(p + 6);
            float u0 = 4 * (u + 0.5f), u1 = 4 * (u + pw - 0.5f), v0 = 4 * (v + 0.5f), v1 = 4 * (v + ph - 0.5f);
            if (*(ushort*)(p + 8) == 1) (u0, u1) = (u1, u0);
            if (*(ushort*)(p + 10) == 1) (v0, v1) = (v1, v0);
            uv(id, x + dx, y + dy, pw, ph, u0, v0, u1, v1, GREY, GREY);
        }
    }

    // ---- Lulu: SLPM_675.13 0x0021b9d0 --------------------------------------------------------

    private static readonly nint _loc = Marshal.AllocHGlobal(0x100);

    /// <summary>
    /// In states 3 and 4: the count (u8 +0x17) as "%02d" in battle shapes 490.. ('0' + 0x1ba) 16
    /// apart at (+0x10, +0x12), grey; the fill TODrawBoxXYWHC2FW(+2, +4, +0x14, 9, +8, +0xc, +6);
    /// the bar (gauge at +2, +4, +6); the prompt 0x3012 (the PC's keyboard text in keyboard mode).
    /// The PC draws an HD counter 10 right and 8 up, its own fill from the count and a backing box.
    /// </summary>
    private static void lulu() {
        if (!shown(LULU)) return;
        byte* s = Native.ptr(LULU);
        short x = *(short*)(s + 2), y = *(short*)(s + 4), w = *(short*)(s + 6);
        MainMenuPatches.battle_scoped(() => {
            digits(s[0x17].ToString("00", CultureInfo.InvariantCulture), *(short*)(s + 0x10), *(short*)(s + 0x12), 0x1ba, 16, 16, GREY);
            fn<BoxFW>(BOX_FW)(x, y, *(short*)(s + 0x14), 9, *(uint*)(s + 8), *(uint*)(s + 0xc), w);
            gauge(x, y, w);
        });
        uint mode = Native.u32(LULU_MODE);
        if (mode == 0) {
            fn<LocText>(LOC_TEXT)(0x2e8, _loc, 0x100);
            prompt(_loc);
        } else {
            prompt(0x3012);
        }
        first(4, "ui.menus: Lulu's Fury window drawn the PS2 way");
    }

    /// <summary>Digits as battle shapes (c + offset) from x, advancing `digit` per digit and `other` per other character ('.' as shape 479).</summary>
    private static void digits(string s, int x, int y, int offset, int digit, int other, uint c) {
        foreach (char ch in s) {
            if (ch is >= '0' and <= '9') { BattleHudPatches.bshape(ch + offset, x, y, 0, 0, c, c); x += digit; continue; }
            if (ch == '.') BattleHudPatches.bshape(0x1df, x, y, 0, 0, c, c);
            x += other;
        }
    }

    // ---- the limit timer: SLPM_675.13 0x0021a8f8 -> 0x001fcb90 -------------------------------

    /// <summary>
    /// While the timer runs: the seconds left (+0x18) as "%1.2f" at its place (+4, +6: dum201),
    /// each character a 20-wide cell: digits battle shapes 480.. ('0' + 0x1b0), the point 479;
    /// grey, or yellow (0x80008080) under one second. The PC draws them 15 right and 20 up, 16
    /// apart with a narrow point, and moves the timer up during Wakka's reels; the PS2 does neither.
    /// </summary>
    private static void timer() {
        if (Native.u32(TIMER) == 0) return;
        byte* s = Native.ptr(TIMER);
        float t = *(float*)(s + 0x18);
        uint c = t < 1f ? 0x80008080 : GREY;
        string text = t.ToString("0.00", CultureInfo.InvariantCulture);
        MainMenuPatches.battle_scoped(() => digits(text, *(short*)(s + 4), *(short*)(s + 6), 0x1b0, 20, 20, c));
        first(8, "ui.menus: the Overdrive timer drawn the PS2 way");
    }

    // ---- install -----------------------------------------------------------------------------

    private static bool hook<T>(ModHost m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(ModHost m) {
        _log = m.log;
        return hook<Void>(m, DRAW_TIDUS, tidus)
            & hook<Void>(m, DRAW_AURON, auron)
            & hook<Void>(m, DRAW_LULU, lulu)
            & hook<Void>(m, DRAW_TIMER, timer)
            & hook<Void>(m, LIMIT_BG, () => first(16, "ui.menus: the PC's Overdrive backdrop skipped (the PS2 has none)"))
            & hook<IntFn>(m, WAKKA_FRAME, () => { first(32, "ui.menus: the PC's frame round Wakka's reels skipped (the PS2 has none)"); return 0; });
    }
}
