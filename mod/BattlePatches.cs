// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, battle: the battle command windows drawn the PS2 way. The PC keeps the PS2's battle
/// window logic (TOBtlSetCallback, the TOWaitBtlWin* input steps, the command lists) but its
/// draws (TODrawWaitBtlWin*, TOBtlDrawCommandWindow) are HD rewrites: stone plates per row from
/// battle_kuang, ripples, rings. The PS2 draw (SLPM_675.13 0x00210a40 for the main window,
/// 0x00211e88 for the others) is one translucent box with a horizontal gradient from dum91 to
/// dum92 (the battle sequence colours TOSeqInitParam still reads: 0x025d0af4, 0x025d0af0), a
/// scroll bar, the scissor and the rows, 20 PS2 pixels apart. This file is that draw, made of
/// FFX.exe's own primitives; the window record, the command list and the input stay the PC's,
/// so PC-only commands are drawn like the others. Evidence: docs/ui.md ("The battle command
/// windows").
/// </summary>
internal static unsafe class BattlePatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxXYWHC2(int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ScrollParts(float x, float y, float w, float h, int first, int rows, int lines);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Scissor(int x0, int y0, int x1, int y1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint ComData(int cmd, int* strings);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int CheckUse(int chr, int cmd);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void SetCurPos(int i, int x, int y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate float GetTime();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int CommandMP(int chr, nint rec);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint SaveWeapon(int id, nint name);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void IconXYWH(float x, float y, float w, float h, int icon, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void WeaponAbility(int weapon, float x, float y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void SummonPrim(int chr, int x, int y);

    // The PC's window draws (TOBtlSetCallback 0x008a0de0 stores them at window +0x90).
    private const uint DRAW_MAIN    = 0x008a49b0; // kind 0
    private const uint DRAW_SUB     = 0x008a7520; // kinds 1-4, 6, 0xe, 0x11
    private const uint DRAW_LEFT    = 0x008a3680; // kind 0xc
    private const uint DRAW_RIGHT   = 0x008a6a60; // kind 0xd
    private const uint DRAW_PREPARE = 0x008a6870; // kind 0x14
    private const uint DRAW_SUMMON  = 0x008a75f0; // kind 5
    private const uint DRAW_WEAPON  = 0x008a7dc0; // kinds 7, 0xf

    // FFX.exe primitives (PS2 screen units, 512 x 416)
    private const uint BOX_XYWHC2      = 0x008f3db0; // TODrawBoxXYWHC2: untextured, c0 left -> c1 right
    private const uint SCROLL_PARTS    = 0x008e6cc0; // DrawCrossMenuScrollParts
    private const uint SCISSOR         = 0x008fdee0; // TOMakePktScissor
    private const uint COM_DATA        = 0x0079a4b0; // MsGetComData
    private const uint CHECK_USE       = 0x0089aca0; // TOCheckBtlCommandUse
    private const uint SET_CUR_POS     = 0x008999f0; // TOBtlSetCurPos
    private const uint GET_TIME        = 0x00641410; // graphicGetTime
    private const uint ITEM_NUM        = 0x007904b0; // MsGetSaveItemNum
    private const uint COMMAND_MP      = 0x0078cf70; // MsGetCommandMP
    private const uint RAM_HP          = 0x0079add0; // MsGetRamChrHP
    private const uint RAM_MP          = 0x0079ae50; // MsGetRamChrMP
    private const uint SAVE_WEAPON     = 0x007abbd0; // MsGetSaveWeapon
    private const uint ICON            = 0x008e6af0; // DrawCrossMenuIconXYWHRGBA
    private const uint WEAPON_ABILITY  = 0x008f34c0; // TOBtlDrawWeaponAbilityWindow
    private const uint SUMMON_PRIM     = 0x008a0ba0; // TOBtlDrawSummonPrim

    // TOSeqInitParam's battle layout (scene 2; SLPM_675.13 0x00324300..0x00324308, 0x00325022)
    private const uint DUM92   = 0x025d0af0; // gradient right colour (R G B A bytes)
    private const uint DUM91   = 0x025d0af4; // gradient left colour
    private const uint DUM79_Y = 0x025d0af8; // window top
    private const uint DUM79_X = 0x025d0afa; // text column start - 16
    private const uint DUMM79  = 0x023cc090; // the box's least right edge
    private const uint DUM21_X = 0x025d0aa0; // aeon panels' x
    private const uint DUM22   = 0x025d0a98; // aeon panels' pitch
    private const uint HIDDEN  = 0x0133d6e0; // 1: the PC's own draws return at once (main, sub)
    private const uint SG_FRAME = 0x023cbbf0;

    internal const float SX = 0.78f, SY = 1.0f; // the battle font size every PC battle window uses
    private const int ROW = 20;                // PS2 row pitch

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static BoxXYWHC2? _box;
    private static ScrollParts? _scroll;
    private static Scissor? _scissor;
    private static ComData? _com;
    private static CheckUse? _use;
    private static SetCurPos? _cur;
    private static GetTime? _time;
    private static IntOfInt? _item_num, _hp, _mp;
    private static CommandMP? _cmd_mp;
    private static SaveWeapon? _weapon;
    private static IconXYWH? _icon;
    private static WeaponAbility? _weapon_ability;
    private static SummonPrim? _summon;

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    /// <summary>Fahrenheit hands out call pointers only after every module's init: fetched at the first draw.</summary>
    private static void bind() {
        if (_box is not null) return;
        _scroll = Native.fn<ScrollParts>(SCROLL_PARTS).fnptr!;
        _scissor = Native.fn<Scissor>(SCISSOR).fnptr!;
        _com = Native.fn<ComData>(COM_DATA).fnptr!;
        _use = Native.fn<CheckUse>(CHECK_USE).fnptr!;
        _cur = Native.fn<SetCurPos>(SET_CUR_POS).fnptr!;
        _time = Native.fn<GetTime>(GET_TIME).fnptr!;
        _item_num = Native.fn<IntOfInt>(ITEM_NUM).fnptr!;
        _hp = Native.fn<IntOfInt>(RAM_HP).fnptr!;
        _mp = Native.fn<IntOfInt>(RAM_MP).fnptr!;
        _cmd_mp = Native.fn<CommandMP>(COMMAND_MP).fnptr!;
        _weapon = Native.fn<SaveWeapon>(SAVE_WEAPON).fnptr!;
        _icon = Native.fn<IconXYWH>(ICON).fnptr!;
        _weapon_ability = Native.fn<WeaponAbility>(WEAPON_ABILITY).fnptr!;
        _summon = Native.fn<SummonPrim>(SUMMON_PRIM).fnptr!;
        _box = Native.fn<BoxXYWHC2>(BOX_XYWHC2).fnptr!;
    }

    // ---- the window record (TOBtlSetCallback, TOBtlDrawCommandWindow) ------------------------

    private static short  i16(nint w, int o) => *(short*)(w + o);
    private static ushort u16(nint w, int o) => *(ushort*)(w + o);
    private static int    i32(nint w, int o) => *(int*)(w + o);

    /// <summary>The PS2 layout of one window, from the PC's record and the PS2 layout data.</summary>
    private struct Layout {
        public int kind, chr, count, cols, rows, first, cursor;
        public int colw;      // a column (PS2 0x00215bc0: the widest name plus the parts)
        public float namew;   // the widest name
        public int x1, y0, y1;            // the box: x from 0 (PS2 0x002105d8)
        public int tx0, ty0, tx1, ty1;    // the text rectangle (the scissor)
    }

    /// <summary>The window's open time (+0xe0), which the PC's input steps read: kept as the PC's draws keep it.</summary>
    private static void open_time(nint w) {
        float since = *(float*)(w + 0xdc);
        *(float*)(w + 0xe0) = since > 0 ? _time!() - since : 0f;
    }

    private static nint name_of(int cmd, out nint rec) {
        int strings;
        rec = _com!(cmd, &strings);
        return strings + *(ushort*)rec;
    }

    /// <summary>A name's width: the 16x12 font at the PS2's size (the PS2's 1612 draw 0x001f3810, Ps2Menu.text_width).</summary>
    private static float width_of(nint text) => Ps2Menu.text_width(text);

    /// <summary>
    /// Window kinds whose PS2 parts had no command icon (SLPM_675.13 0x002105d8: kinds 1-4 and 0xe
    /// are name and MP only); the PC added kind-1 icons to them.
    /// </summary>
    private static bool ps2_no_icon(int kind) => kind is 1 or 2 or 3 or 4 or 0xe;

    private static Layout layout(nint w) {
        Layout l = new() {
            kind = u16(w, 6), chr = i16(w, 8), count = i32(w, 0x24),
            cols = Math.Max((int)i16(w, 0x38), 1), rows = i16(w, 0x3a),
            first = i16(w, 0x40), cursor = i16(w, 0x42),
        };
        nint list = *(nint*)(w + 0x20);
        float namew = 0;
        for (int e = 0; e < l.count; e++) {
            int cmd = *(short*)(list + 2 * e);
            if (cmd == 0xff || _use!(l.chr, cmd) == -4) continue;
            namew = Math.Max(namew, width_of(name_of(cmd, out _)));
        }
        float colw = namew;
        int parts = Math.Clamp((int)i16(w, 0x28), 0, 8);
        for (int k = 0; k < parts; k++) {
            ushort p = u16(w, 0x2c + 2 * k);
            int kind = p >> 8, digits = p & 0xff;
            colw += kind switch {
                1 => ps2_no_icon(l.kind) ? 0 : 20,
                7 => 20,
                2 or 4 => 12 * digits + 5,
                3 => 12,
                5 or 6 or 0x12 => 12 * digits,
                _ => 0,
            };
        }
        l.namew = namew;
        l.colw = (int)MathF.Ceiling(colw);
        int px = *(short*)Native.ptr(DUM79_X), py = *(short*)Native.ptr(DUM79_Y);
        int total = l.colw * l.cols + (l.cols - 1) * 16;
        l.x1 = Math.Max(px + total + 32, (int)*(short*)Native.ptr(DUMM79));
        l.y0 = py;
        l.y1 = py + l.rows * ROW + 2;
        l.tx0 = px + 16;
        l.ty0 = py + 1;
        l.tx1 = l.tx0 + total;
        l.ty1 = l.ty0 + l.rows * ROW;
        return l;
    }

    /// <summary>The box, the scroll bar and the scissor (both PS2 draws begin so).</summary>
    private static void frame(in Layout l) {
        uint c0 = *(uint*)Native.ptr(DUM91), c1 = *(uint*)Native.ptr(DUM92);
        _box!(0, l.y0, l.x1, l.y1 - l.y0, c0, c1);
        int lines = (l.count - 1 + l.cols) / l.cols;
        _scroll!(l.x1 - 10, l.ty0, 4, l.ty1 - l.ty0, l.first / l.cols, l.rows, lines);
        _scissor!(l.tx0, l.ty0, l.tx1, l.ty1);
    }

    private static int cell_x(in Layout l, int e) => l.tx0 + (e % l.cols) * (l.colw + 16);
    private static int cell_y(in Layout l, int e) => l.ty0 + (e - l.first) / l.cols * ROW;
    private static bool shown(in Layout l, int e) => e >= l.first && e < l.first + l.rows * l.cols;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntToMes(int value, byte* out_, int digits, int mode);
    private const uint INT_TO_MES = 0x008b8290; // TOIntToMes(value, out, digits, mode) (PS2 0x00207650, no mode)
    private static readonly byte* _num = (byte*)Marshal.AllocHGlobal(32);

    /// <summary>
    /// A number as the PS2 draws item counts, MP costs, HP and MP in the command windows
    /// (0x001f3588 -> 0x001f3f30): TOIntToMes's text of `digits` characters, left-padded with the
    /// font's digit-wide blank, in the 16x12 font at x (the PS2's size, Ps2Menu.font), grey when
    /// unusable. The PC drew its battle digits (ToMakeBtlEasyDigitRight) at 0.78.
    /// </summary>
    private static void number(int value, int digits, float x, float y, int grey, bool show_zero) {
        if (value <= 0 && !show_zero) return;
        new Span<byte>(_num, 32).Clear();
        Ps2Menu.fn<IntToMes>(INT_TO_MES)(value, _num, Math.Clamp(digits, 1, 16), 0);
        Ps2Menu.font((nint)_num, (int)x, (int)y, grey);
    }

    /// <summary>The rows (SLPM_675.13 0x00211e88: the parts left to right; 0x00210a40: the name centred).</summary>
    private static void rows(nint w, in Layout l, bool centred) {
        nint list = *(nint*)(w + 0x20);
        int last = Math.Min(l.count, l.first + l.rows * l.cols);
        int parts = Math.Clamp((int)i16(w, 0x28), 0, 8);
        bool show_zero = i16(w, 0x2a) == 1;
        for (int e = Math.Max(l.first, 0); e < last; e++) {
            int cmd = *(short*)(list + 2 * e);
            if (cmd == 0xff) continue;
            int use = _use!(l.chr, cmd);
            if (use == -4) continue;
            int grey = use < 0 ? 1 : 0;
            nint name = name_of(cmd, out nint rec);
            float x = cell_x(l, e), y = cell_y(l, e);
            if (centred) {
                Ps2Menu.font(name, (int)(x + MathF.Floor((l.colw - width_of(name)) / 2)), (int)y, grey);
                continue;
            }
            for (int k = 0; k < parts; k++) {
                ushort p = u16(w, 0x2c + 2 * k);
                int kind = p >> 8, digits = p & 0xff;
                switch (kind) {
                    case 1:
                        if (ps2_no_icon(l.kind)) break;
                        _icon!(x, y + 2, 12, 12, *(sbyte*)(rec + 0x14), 0x80, 0x80, 0x80, 0x80);
                        x += 20;
                        break;
                    case 2: x += 5; number(_item_num!(cmd), digits, x, y, grey, show_zero); x += 12 * digits; break;
                    case 4: x += 5; number(_cmd_mp!(l.chr, rec), digits, x, y, grey, show_zero); x += 12 * digits; break;
                    case 5: number(_hp!(cmd), digits, x, y, grey, show_zero); x += 12 * digits; break;
                    case 6: number(_mp!(cmd), digits, x, y, grey, show_zero); x += 12 * digits; break;
                    case 7: {
                        byte* wp = (byte*)_weapon!(cmd, 0);
                        _icon!(x, y + 2, 12, 12, wp[4] * 2 + 1 + wp[5], 0x80, 0x80, 0x80, 0x80);
                        if (wp[4] == wp[6]) _icon!(x, y + 2, 12, 12, 0x31, 0x80, 0x80, 0x80, 0x80);
                        x += 20;
                        break;
                    }
                    case 0x10:
                        Ps2Menu.font(name, (int)x, (int)y, grey);
                        x += l.namew;
                        break;
                }
            }
        }
    }

    /// <summary>The hand at the cursor's cell (PS2 0x00211cc0 / 0x00210a40: TOBtlSetCurPos).</summary>
    private static void cursor(nint w, in Layout l, bool centred) {
        if (l.cursor < l.first || !shown(l, l.cursor)) return;
        int x = cell_x(l, l.cursor);
        if (centred) {
            nint list = *(nint*)(w + 0x20);
            x += (int)MathF.Floor((l.colw - width_of(name_of(*(short*)(list + 2 * l.cursor), out _))) / 2);
        }
        _cur!(0, x, cell_y(l, l.cursor));
    }

    private static Layout draw(nint w, bool centred, int bit, string what) {
        bind();
        open_time(w);
        Layout l = layout(w);
        frame(l);
        rows(w, l, centred);
        first(bit, $"ui.menus: battle {what} window drawn the PS2 way (kind {l.kind}, box 0,{l.y0}..{l.x1},{l.y1})");
        return l;
    }

    private static int draw_main(nint w) {
        if (*(int*)Native.ptr(HIDDEN) == 1) return 0;
        cursor(w, draw(w, true, 1, "main"), true);
        return 0;
    }

    private static int draw_sub(nint w, int bit, string what) {
        cursor(w, draw(w, false, bit, what), false);
        return 0;
    }

    /// <summary>Prepare (kind 0x14): the picked cells blink every other frame, as on the PC.</summary>
    private static int draw_prepare(nint w) {
        Layout l = draw(w, false, 0x10, "prepare");
        if ((*(uint*)Native.ptr(SG_FRAME) & 1) != 0) {
            int n = *(sbyte*)(w + 0xb);
            for (int k = 0; k < n; k++) {
                int e = u16(w, 0xe + 2 * k);
                if (shown(l, e)) _cur!(0, cell_x(l, e) - 5, cell_y(l, e) - 2);
            }
        }
        cursor(w, l, false);
        return 0;
    }

    /// <summary>Summon (kind 5): the aeon panels where the PS2 put them (0x002123c0: x dum21, y 180 + n dum22).</summary>
    private static int draw_summon(nint w) {
        Layout l = draw(w, false, 0x20, "summon");
        _scissor!(0, 0, 0x200, 0x1a0);
        int x = *(short*)Native.ptr(DUM21_X), dy = *(short*)Native.ptr(DUM22);
        nint list = *(nint*)(w + 0x20);
        int sel = l.cursor >= 0 && l.cursor < l.count ? *(short*)(list + 2 * l.cursor) : 0xff;
        if (sel == 0x30ff) {
            for (int k = 0; k < 3; k++) _summon!(0xf + k, x, 0xb4 + k * dy);
        } else if (sel != 0xff) {
            _summon!(sel, x, 0xb4 + 2 * dy);
        }
        cursor(w, l, false);
        return 0;
    }

    /// <summary>Weapon change (kinds 7, 0xf): the weapon's abilities beside the box (PS2 0x00212580: x1 + 20, y 365).</summary>
    private static int draw_weapon(nint w) {
        Layout l = draw(w, false, 0x40, "weapon");
        cursor(w, l, false);
        nint list = *(nint*)(w + 0x20);
        if (l.cursor >= 0 && l.cursor < l.count) {
            int weapon = *(short*)(list + 2 * l.cursor);
            if (weapon != 0xff) _weapon_ability!(weapon, l.x1 + 20, 365);
        }
        return 0;
    }

    private static bool hook(DemasterModule m, uint va, WinDraw d) {
        _keep.Add(d);
        return Native.fn<WinDraw>(va).hook(m, d);
    }

    /// <summary>
    /// Hooks the seven command-window draws. The change window (kind 10: TODrawWaitBtlWinChange)
    /// and the gil window (0x15, 0x16: TODrawWaitBtlWinInputGil) keep the HD draw.
    /// </summary>
    public static bool windows(DemasterModule m) {
        _log = m.log;
        return hook(m, DRAW_MAIN, draw_main)
            & hook(m, DRAW_SUB, w => *(int*)Native.ptr(HIDDEN) == 1 ? 0 : draw_sub(w, 2, "sub"))
            & hook(m, DRAW_LEFT, w => draw_sub(w, 4, "left"))
            & hook(m, DRAW_RIGHT, w => draw_sub(w, 8, "right"))
            & hook(m, DRAW_PREPARE, draw_prepare)
            & hook(m, DRAW_SUMMON, draw_summon)
            & hook(m, DRAW_WEAPON, draw_weapon);
    }
}
