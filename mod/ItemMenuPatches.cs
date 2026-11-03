// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, the Items and Abilities screens drawn by the PS2's draw code ported onto FFX.exe's
/// primitives, as MainMenuPatches does for the main menu. The PC kept the PS2 logic and the window
/// records (every field these draws read has the PS2's offset), but redrew the windows with HD
/// sheets and 1920 x 1080 numbers. Hooked (PC = SLPM_675.13):
/// <list type="bullet">
/// <item>MenuItem_ListWindowDraw_008c7750 = 0x0024f0e8: the item, key-item and ability lists
/// (two columns, 32 pixels a row, plates by kind, icons, names, counts / MP, scroll bar, cursor);</item>
/// <item>MenuItem_TargetWindowDraw_008c80b0 = 0x0024dcd8: the party / aeon target window;</item>
/// <item>MenuSubCmd_DrawRow_008e3cf0 = 0x00236e68 and MenuSubCmd_DrawItems_008e4140 = 0x00237188:
/// the sub-command bars (Use / Sort / ...; every screen's, one routine on both);</item>
/// <item>the ability screen's draw 0x008c64a0 = 0x00250978: the current member's party plate.</item>
/// </list>
/// Evidence and the open points: docs/ui.md ("The Items and Abilities screens").
/// </summary>
internal static unsafe class ItemMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Draw();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxC2(int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxRGBA(int x, int y, int w, int h, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Plate2(int a, int b, int x, int y, int w, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXY(int id, int x, int y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXYC2(int id, int x, int y, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXYWHC2(int id, int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Font(nint text, int x, int y, int colour);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void StrS(nint text, float x, float y, int colour, float sx, float sy);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void DigitRight(int value, float x, float y, int grey, float sx, float sy);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void IconXYWH(float x, float y, float w, float h, int icon, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Scissor(int x0, int y0, int x1, int y1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void WinScissor(int y, int h, int part, int ratio);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt2(int a, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint ComData(int cmd, int* strings);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int TextWidth(nint text);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Usable(nint w, int i, int z);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqPoint(int scene, nint name, int* x, int* y);

    // hooked
    private const uint LIST_DRAW    = 0x008c7750; // MenuItem_ListWindowDraw_008c7750 (PS2 0x0024f0e8)
    private const uint TARGET_DRAW  = 0x008c80b0; // MenuItem_TargetWindowDraw_008c80b0 (PS2 0x0024dcd8)
    private const uint SUBCMD_DRAW  = 0x008e3cf0; // MenuSubCmd_DrawRow_008e3cf0 (PS2 0x00236e68)
    private const uint SUBCMD2_DRAW = 0x008e4140; // MenuSubCmd_DrawItems_008e4140 (PS2 0x00237188)
    private const uint ABILITY_DRAW = 0x008c64a0; // the ability screen's record draw (PS2 0x00250978)

    // drawn with
    private const uint BOX_C2     = 0x008f3db0; // TODrawBoxXYWHC2
    private const uint BOX_RGBA   = 0x008fb150; // TOMakePktBoxXYWHRGBA
    private const uint PLATE_LR   = 0x008f5e90; // (A, B, x, y, w, c0, c1): A stretched c0..c1, B in c1 (PS2 0x001ff2b8)
    private const uint PLATE_RL   = 0x008f5e20; // (A, B, x, y, w, c0, c1): A in c0, B stretched (PS2 0x001ff1e0)
    private const uint SHAPE_XY   = 0x00903a10; // TOMkpShapeXY
    private const uint SHAPE_XYC2 = 0x00903a50; // TOMkpShapeXYC2
    private const uint SHAPE_XYWH = 0x00903b70; // TOMkpShapeXYWHC2
    private const uint FONT_1612  = 0x00901cb0; // TOMkpBaseMesFont (PS2 0x001f3810)
    private const uint STR_S      = 0x00901660; // TOMkpCrossEasyStrFontSClut (PS2 0x001f6500)
    private const uint DIGIT_RIGHT = 0x009055c0; // ToMakeBtlEasyDigitRight: the PS2's "%d" right-aligned (font M / S stand-in)
    private const uint ICON       = 0x008e6af0; // DrawCrossMenuIconXYWHRGBA (PS2 0x001f9798 draws 15 x 18 1:1)
    private const uint SCISSOR    = 0x008fdee0; // TOMakePktScissor(x0, y0, x1, y1)
    private const uint WIN_SCISSOR = 0x008c0f70; // TkMn2DrawMenuWindowScissor(y, h, part, ratio) (PS2 0x002274a8)
    private const uint OPEN_PKT   = 0x008bef30; // TOMenuOpenPktBuffTmp
    private const uint KICK       = 0x008bea20; // TOMenuDrawKickTmp
    private const uint VU1_SYNC   = 0x0088ec60; // TkVU1SyncPath

    // read
    private const uint ITEM_LIST    = 0x01597770; // g_TkMenuItemList: { u16 id, u8 kind, u8 usable }
    private const uint KEY_LIST     = 0x01596f70; // g_TkMenuImportantList: u16 ids
    private const uint ITEM_MAX     = 0x008c8c30; // TOAmapGetItemListMax
    private const uint KEY_MAX      = 0x008c8a20; // TkMenuImportantListMax
    private const uint ROWS_MAX     = 0x008c8a00; // MenuItem_GetRows (the ability list)
    private const uint KEY_GET      = 0x008ab2c0; // TkMsImportantGet(id)
    private const uint KEY_PTR      = 0x007908a0; // MsImportantPtr(id, &strings) -> record (name u16 +0, icon +0x12)
    private const uint COM_DATA     = 0x0079a4b0; // MsGetComData(id, &strings) -> record (name u16 +0, icon +0x14)
    private const uint PLY_COMMAND  = 0x00790a90; // MsGetRomPlyCommand(id) -> record (MP +0x25)
    private const uint ITEM_NUM     = 0x007904b0; // MsGetSaveItemNum
    private const uint MENU_TIMER   = 0x008a9c60; // TkMenuGetTimer
    // TkMn2StringWidth: TOGetMessageWH's width in the low half (TkMenuDraw1612Width, the PS2's
    // 0x00226ad8 name, sums f32 bit patterns as integers on the PC and returns garbage)
    private const uint TEXT_WIDTH   = 0x008c2cb0;
    private const uint PLAYERS      = 0x008a9b30; // TkMenuGetPlayerListMax
    private const uint SUMMONS      = 0x008a9bf0; // TkMenuGetSummonListMax
    private const uint PLAYER       = 0x008a9ad0; // TkMenuGetPlayerFromIndex2
    private const uint SUMMON       = 0x008a9bc0; // TkMenuGetSummonFromIndex
    private const uint CURRENT      = 0x008a9860; // TkMenuGetCurrentPlayer
    private const uint EASE         = 0x008d30c0; // MenuMn2_Ease(t, dir)
    private const uint CHR_NAME     = 0x008ac850; // TOGetSaveChrName
    private const uint DAMAGE_CLASS = 0x008a99f0; // MenuPlayer_DamageClass
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint SPHERE_LV    = 0x008a9b80; // TkMenuGetSP
    private const uint NEXT_AP      = 0x00784e90; // MsGetNextAP(chr)
    private const uint SAVE_NEXT_AP = 0x007852b0; // MsGetSaveNextAP(chr)
    private const uint SEQ_POINT    = 0x008ae450; // TOSeqGetPoint(scene, name, &x, &y)

    private const uint ABILITY_COUNTER = 0x0186a390;  // u8 the screens' draws count up
    private const uint TARGET_FROM_ITEM = 0x0186a408; // g_TkMenuTargetFromItem: 1 the item list's scroll rect
    private const uint TARGET_MODE  = 0x0186a40c;     // g_TkMenuItemTargetMode
    private const uint RECT_ISCROLL = 0x0186a410;     // rect_iscroll (i16 x, y, w, h)
    private const uint RECT_ABSCROLL = 0x0186a418;    // rect_abscroll
    private const uint LINM_LINE = 0x0186a400, RINM_LINE = 0x0186a404; // p_linmline, p_rinmline (i16 x, y)
    private const uint TARGET_MASK  = 0x0186a3a4;     // g_TkMenuTargetMask
    private const uint SINGLE       = 0x0186a391;     // g_TkMenuAbilitySingle
    private const uint PARTY_MASK   = 0x01841c2c;     // the front-line bits (TOMenuGetYesNoWindow reads it)
    private const uint SUBCMD_WINDOW = 0x023cc128;    // g_TkMenuSubCmdWindow (the first bar)
    // TOSeqInitParam's menu layout (mirrored storage: points y, x; MainMenuPatches)
    private const uint COL_BSOPLT0 = 0x025d16a0;      // plate pairs: bso, bitem, bab, lmt (c0, c1 4 bytes apart)
    private const uint COL_ISNMPLT = 0x025d15dc, COL_ISNMPLT_A = 0x025d15e0;
    private const uint COL_IBNMPLT0 = 0x025d15d8, COL_IBNMPLT1 = 0x025d15d4;
    private const uint COL_INMPLT0 = 0x025d15c8, COL_INMPLT1 = 0x025d15c4;
    private const uint NAMEPLT_W = 0x025d1672;
    private const uint COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144;
    private const uint COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;

    private const int ROW = 32;
    private const uint GREY = 0x80808080;
    private const uint SHADOW = 0x80000000;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static BoxC2? _box;
    private static BoxRGBA? _box_rgba;
    private static Plate2? _plate_lr, _plate_rl;
    private static ShapeXY? _shape;
    private static ShapeXYC2? _shape_c2;
    private static ShapeXYWHC2? _shape_wh;
    private static Font? _font;
    private static StrS? _str;
    private static DigitRight? _digits;
    private static IconXYWH? _icon;
    private static Scissor? _scissor;
    private static WinScissor? _win_scissor;
    private static IntFn? _open, _kick, _sync, _item_max, _key_max, _rows_max, _timer, _players, _summons, _current;
    private static IntOfInt? _key_get, _item_num, _player, _summon, _damage, _hp, _max_hp, _mp, _max_mp, _sphere, _next_ap, _save_next_ap;
    private static IntOfInt2? _ease;
    private static PtrOfInt? _ply_command, _chr_name;
    private static ComData? _com, _key_ptr;
    private static TextWidth? _width;
    private static SeqPoint? _seq_point;

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static void bind() {
        if (_box is not null) return;
        _box_rgba = Native.fn<BoxRGBA>(BOX_RGBA).fnptr!;
        _plate_lr = Native.fn<Plate2>(PLATE_LR).fnptr!;
        _plate_rl = Native.fn<Plate2>(PLATE_RL).fnptr!;
        _shape = Native.fn<ShapeXY>(SHAPE_XY).fnptr!;
        _shape_c2 = Native.fn<ShapeXYC2>(SHAPE_XYC2).fnptr!;
        _shape_wh = Native.fn<ShapeXYWHC2>(SHAPE_XYWH).fnptr!;
        _font = Native.fn<Font>(FONT_1612).fnptr!;
        _str = Native.fn<StrS>(STR_S).fnptr!;
        _digits = Native.fn<DigitRight>(DIGIT_RIGHT).fnptr!;
        _icon = Native.fn<IconXYWH>(ICON).fnptr!;
        _scissor = Native.fn<Scissor>(SCISSOR).fnptr!;
        _win_scissor = Native.fn<WinScissor>(WIN_SCISSOR).fnptr!;
        _open = Native.fn<IntFn>(OPEN_PKT).fnptr!;
        _kick = Native.fn<IntFn>(KICK).fnptr!;
        _sync = Native.fn<IntFn>(VU1_SYNC).fnptr!;
        _item_max = Native.fn<IntFn>(ITEM_MAX).fnptr!;
        _key_max = Native.fn<IntFn>(KEY_MAX).fnptr!;
        _rows_max = Native.fn<IntFn>(ROWS_MAX).fnptr!;
        _timer = Native.fn<IntFn>(MENU_TIMER).fnptr!;
        _players = Native.fn<IntFn>(PLAYERS).fnptr!;
        _summons = Native.fn<IntFn>(SUMMONS).fnptr!;
        _current = Native.fn<IntFn>(CURRENT).fnptr!;
        _key_get = Native.fn<IntOfInt>(KEY_GET).fnptr!;
        _item_num = Native.fn<IntOfInt>(ITEM_NUM).fnptr!;
        _player = Native.fn<IntOfInt>(PLAYER).fnptr!;
        _summon = Native.fn<IntOfInt>(SUMMON).fnptr!;
        _damage = Native.fn<IntOfInt>(DAMAGE_CLASS).fnptr!;
        _hp = Native.fn<IntOfInt>(HP).fnptr!;
        _max_hp = Native.fn<IntOfInt>(MAX_HP).fnptr!;
        _mp = Native.fn<IntOfInt>(MP).fnptr!;
        _max_mp = Native.fn<IntOfInt>(MAX_MP).fnptr!;
        _sphere = Native.fn<IntOfInt>(SPHERE_LV).fnptr!;
        _next_ap = Native.fn<IntOfInt>(NEXT_AP).fnptr!;
        _save_next_ap = Native.fn<IntOfInt>(SAVE_NEXT_AP).fnptr!;
        _ease = Native.fn<IntOfInt2>(EASE).fnptr!;
        _ply_command = Native.fn<PtrOfInt>(PLY_COMMAND).fnptr!;
        _chr_name = Native.fn<PtrOfInt>(CHR_NAME).fnptr!;
        _com = Native.fn<ComData>(COM_DATA).fnptr!;
        _key_ptr = Native.fn<ComData>(KEY_PTR).fnptr!;
        _width = Native.fn<TextWidth>(TEXT_WIDTH).fnptr!;
        _seq_point = Native.fn<SeqPoint>(SEQ_POINT).fnptr!;
        _box = Native.fn<BoxC2>(BOX_C2).fnptr!;
    }

    private static short i16(uint va) => Native.i16(va);
    private static uint u32(uint va) => Native.u32(va);
    private static (int x, int y) point(uint va) => (i16(va + 2), i16(va)); // stored y, x
    private static short wi16(nint w, int o) => *(short*)(w + o);
    private static ushort wu16(nint w, int o) => *(ushort*)(w + o);
    private static sbyte wi8(nint w, int o) => *(sbyte*)(w + o);
    private static byte wu8(nint w, int o) => *(byte*)(w + o);
    private static int wi32(nint w, int o) => *(int*)(w + o);

    /// <summary>A PS2 draw: the PS2 meswin scope (MainMenuPatches) and one packet buffer.</summary>
    private static int packet(Action draw) => MainMenuPatches.ps2_scoped(() => {
        bind();
        _sync!();
        _open!();
        draw();
        _kick!();
        return _sync!();
    });

    private static readonly Dictionary<string, nint> _names = [];

    /// <summary>TOSeqGetPoint(1, name): a point of the menu sequence (the PS2 draws read their places so).</summary>
    private static (int x, int y) seq_point(string name) {
        if (!_names.TryGetValue(name, out nint p)) _names[name] = p = Marshal.StringToHGlobalAnsi(name);
        int x = 0, y = 0;
        _seq_point!(1, p, &x, &y);
        return ((short)x, (short)y);
    }

    private static void text(string s, Action<nint> use) {
        nint p = Marshal.StringToHGlobalAnsi(s);
        try { use(p); } finally { Marshal.FreeHGlobal(p); }
    }

    // ---- shared pieces ------------------------------------------------------------------------

    /// <summary>
    /// A list row's plate by kind (PS2 TOAdpShapeByKind 0x001f4c60; the PC's 0x008f1b50 lost kind 1's
    /// line): shape 0xda at x in c0, 0xdb from x + 16, 209 wide, c0 -> c1; kind &amp; 3 adds 0xd7, 0x10b
    /// or 0x10a at x + 97 in black (alpha 0x80); kind &gt;&gt; 3 picks the colour pair col_bsoplt,
    /// col_bitemplt, col_babplt, col_lmtplt.
    /// </summary>
    private static void plate(int x, int y, int kind) {
        uint at = COL_BSOPLT0 - 8 * (uint)((kind >> 3) & 3);
        uint c0 = u32(at), c1 = u32(at - 4);
        _shape_wh!(0xda, x, y, 0, 0, c0, c0);
        _shape_wh!(0xdb, x + 16, y, 209, 0, c0, c1);
        int extra = (kind & 3) switch { 1 => 0xd7, 2 => 0x10b, 3 => 0x10a, _ => 0 };
        if (extra != 0) _shape_c2!(extra, x + 97, y, SHADOW, SHADOW);
    }

    /// <summary>An icon and a name (PS2 0x001f5100 / 0x001f5068 / 0x001f5420): icon at (+12, +1), 15 x 18; name at (+29, +1) in the 16x12 font.</summary>
    private static void icon_name(int icon, nint name, int x, int y, int colour) {
        _icon!(x + 12, y + 1, 15, 18, icon, 0x80, 0x80, 0x80, 0x80);
        _font!(name, x + 29, y + 1, colour);
    }

    /// <summary>PS2 0x001f5f58 / 0x001f6408: "%d" right-aligned ending at x + 2 (the PC's battle digits stand in).</summary>
    private static void value(int v, int x, int y, int grey) =>
        _digits!(v, x + 2, y, grey, BattlePatches.SX, BattlePatches.SY);

    // ---- the list window: SLPM_675.13 0x0024f0e8 ----------------------------------------------

    private static int list_draw(nint w) => packet(() => {
        int kind = wu8(w, 0x43);
        int count = wu16(w, 0x30), top = wi16(w, 0x32), next = wi16(w, 0x34), shown = wu16(w, 0x3a);
        // the scroll bar (PS2 0x00200a60): rect_iscroll from the item list's target, else rect_abscroll
        uint r = i32_at(TARGET_FROM_ITEM) == 1 ? RECT_ISCROLL : RECT_ABSCROLL;
        if (shown < count) scroll_bar(i16(r), i16(r + 2), i16(r + 4), i16(r + 6), top, shown, count);
        int y0 = kind == 2 ? 119 : 98;
        if (kind == 2) _scissor!(0, 114, 511, 114 + 288);
        else _scissor!(0, 93, 511, 415);
        if (top == next) {
            for (int k = -1; k <= shown; k++) {
                row(w, 30, y0 + ROW * k, 2 * k, 0);
                row(w, 279, y0 + ROW * k, 2 * k + 1, 0);
            }
        } else {
            int scroll = wi16(w, 0x46);
            _win_scissor!(y0 - 8, ROW * shown + 8, 1, scroll);
            for (int k = 0; k < shown; k++) {
                row(w, 30, y0 + ROW * k, 2 * k, 2);
                row(w, 279, y0 + ROW * k, 2 * k + 1, 2);
            }
            _win_scissor!(y0 - 8, ROW * shown + 8, 2, scroll);
            int yn = y0 - scroll * 319 / 4096;
            for (int k = 0; k < shown; k++) {
                row(w, 30, yn + ROW * k, 2 * k, 1);
                row(w, 279, yn + ROW * k, 2 * k + 1, 1);
            }
        }
        _scissor!(0, 0, 512, 416);
        if (wu8(w, 0x94) != 0) list_cursor(w, 30, y0);
        first(1, "ui.menus: item / ability list drawn the PS2 way");
    });

    private static int i32_at(uint va) => (int)u32(va);

    /// <summary>PS2 0x00200a60: a dark track, a light thumb over rows top .. top + shown of count.</summary>
    private static void scroll_bar(int x, int y, int w, int h, int top, int shown, int count) {
        _box_rgba!(x, y, w, h, 0x11, 0x11, 0x11, 0x80);
        float t = top == 0 ? y : y + (float)h * top / count;
        float b = top + shown >= count ? y + h : y + (float)h * (top + shown) / count;
        _box_rgba!(x + 1, (int)t, w - 2, (int)b - (int)t, 0xf0, 0xf0, 0xf0, 0x80);
    }

    /// <summary>One row (PS2 0x0024ecf0) by the list kind (+0x43): 0 items, 1 key items, 2 commands.</summary>
    private static void row(nint w, int x, int y, int idx, int mode) {
        if (mode == 0) { idx += 2 * wi16(w, 0x32); y -= wi16(w, 0x46) / 128; }
        else if (mode == 1) idx += 2 * wi16(w, 0x34);
        else idx += 2 * wi16(w, 0x32);
        if (idx < 0) return;
        int kind = wu8(w, 0x43);
        if (kind == 1) {
            if (idx >= _key_max!()) return;
            int id = *(ushort*)Native.ptr(KEY_LIST + 2 * (uint)idx);
            if (id == 0 || _key_get!(id) == 0) return;
            plate(x, y, 1);
            int strings;
            nint rec = _key_ptr!(id, &strings);
            icon_name(*(byte*)(rec + 0x12), strings + *(ushort*)rec, x, y, 0);
        } else {
            if (idx >= (kind == 0 ? _item_max!() : _rows_max!())) return;
            byte* e = (byte*)Native.ptr(ITEM_LIST + 4 * (uint)idx);
            int id = *(ushort*)e;
            int dim = e[3] == 0 ? 1 : 0;
            if (kind == 0) {
                plate(x, y, 0xa);
                int n = id == 0 ? 0 : _item_num!(id);
                if (n != 0) {
                    int strings;
                    nint rec = _com!(id, &strings);
                    icon_name(*(byte*)(rec + 0x14), strings + *(ushort*)rec, x, y, dim);
                    value(n, x + 211, y + 6, dim);
                }
            } else if (e[2] == 1) {
                plate(x, y, 0x12);
                int strings;
                nint rec = _com!(id, &strings);
                icon_name(*(byte*)(rec + 0x14), strings + *(ushort*)rec, x, y, dim);
                int mp = *(byte*)(_ply_command!(id) + 0x25);
                if (mp != 0) value(mp, x + 211, y + 6, dim);
            } else if (e[2] == 2) {
                plate(x, y, 2);
            }
        }
        // Sort's first pick blinks
        if (wu8(w, 0x42) == 2 && wi8(w, 0x45) > 0 && idx == wi32(w, 0x58) + 2 * wi16(w, 0x2c) && ((_timer!() / 2) & 1) != 0)
            MainMenuPatches.cursor(x + 2, y + 1);
    }

    /// <summary>The list cursor (PS2 0x0024efd0): row +0x48 - +0x32, column from the word at +0x58 + 4 p.</summary>
    private static void list_cursor(nint w, int x, int y) {
        int n = wu8(w, 0x42), v = wi8(w, 0x45);
        int p = v < n ? v : n - 1;
        bool show = (v < n || n == 2) && v >= 0;
        int mode = i32_at(TARGET_MODE);
        if (!show && mode == 0) return;
        if (wi32(w, 0x58 + 4 * p) != 0) x += 255;
        y += ROW * (wi16(w, 0x48) - wi16(w, 0x32));
        if (mode == 2 && ((_timer!() / 2) & 1) != 0 && v >= 1) return;
        MainMenuPatches.cursor(x + 12, y + 1);
    }

    // ---- the target window: SLPM_675.13 0x0024dcd8 ---------------------------------------------

    private static int target_draw(nint w) => packet(() => {
        bool aeons = wi32(w, 0x58) != 0;
        int n = aeons ? _summons!() : _players!();
        int sl = wi16(w, 0x54) >= 0 ? _ease!(wi16(w, 0x52), wi16(w, 0x54)) / 16 : 0;
        bool left = wi32(w, 0x60) == 1;
        if (left) sl = -sl;
        var nm = seq_point(left ? "pos_linm" : "pos_rinm");
        var tex = seq_point(left ? "pos_lihptex" : "pos_rihptex");
        var hp1 = seq_point(left ? "pos_lihp1" : "pos_rihp1");
        var hp2 = seq_point(left ? "pos_lihp2" : "pos_rihp2");
        // i16 x, y (not mirrored: a TkMn2SeqGetPos record)
        uint la = left ? LINM_LINE : RINM_LINE;
        var line = (x: (int)i16(la), y: (int)i16(la + 2));
        int b1 = left ? 5 : 286, b2 = left ? 0 : 281, w1 = left ? 254 : 231, w3 = left ? 183 : 145;
        int s5 = n >= 9 ? 50 - 31 * (n - 9) : 50;
        bool mp = wi32(w, 0x5c) == 1;
        uint mask = u32(TARGET_MASK);
        bool single = *(byte*)Native.ptr(SINGLE) == 1;
        int cur = wi16(w, 0x48);
        int timer = _timer!();
        for (int i = 0; i < n; i++, s5 += 31) {
            int chr = aeons ? _summon!(i) : _player!(i);
            int y = s5 + 85;
            _box!(b1 + sl, s5 + 91, w1, 29, u32(COL_ISNMPLT_A), u32(COL_ISNMPLT));
            _box!(b2 + sl, y, w1, 29, u32(COL_IBNMPLT0), u32(COL_IBNMPLT1));
            int px = b2 + sl;
            _shape!(0x52, px + line.x, line.y + y);
            _plate_lr!(0xe5, 0xe6, px, y, w3, u32(COL_INMPLT0), u32(COL_INMPLT1));
            _shape!(mp ? 0x22 : 0x21, px + tex.x, y + tex.y);
            int now = mp ? _mp!(chr) : _hp!(chr), max = mp ? _max_mp!(chr) : _max_hp!(chr);
            value(now, px + hp1.x, y + hp1.y, 0);
            value(max, px + hp2.x, y + hp2.y, 0);
            text("/", s => _str!(s, px + hp2.x - 60, y + hp2.y, 0x25, 1f, 0f));
            int colour = (mask & (1u << i)) != 0 ? _damage!(chr) : 1;
            _font!(_chr_name!(chr), px + nm.x, y + nm.y, colour);
            if (wi16(w, 0x52) == 0x1000 && (single ? i == cur : ((timer / 2 + i) & 1) == 0))
                MainMenuPatches.cursor(px + nm.x - 4, y + nm.y);
        }
        first(2, "ui.menus: item target window drawn the PS2 way");
    });

    // ---- the sub-command bars: SLPM_675.13 0x00236e68, 0x00237188 ------------------------------

    private static int text_w(nint text) => (short)(_width!(text) & 0xffff);

    /// <summary>An entry of a sub-command window: text at +0x58 + 12 i.</summary>
    private static nint entry_text(nint w, int i) => *(nint*)(w + 0x58 + 12 * i);

    /// <summary>The first bar's PS2 geometry (PS2 0x002373e0, minimum width 381 as the item and ability opener 0x00237600).</summary>
    private static (int x, int y, int w, int spacing, int sum) bar_geometry(nint w) {
        int n = Math.Max((int)wu16(w, 0x30), 1);
        int sum = 0;
        for (int i = 0; i < n; i++) sum += text_w(entry_text(w, i));
        int a = sum + 32 * (n - 1) + 96;
        int width = a <= 381 ? 381 : Math.Min(a, 0x1f0);
        int spacing = n < 2 ? 0 : Math.Clamp((width - sum - 96) / (n - 1), 32, 128);
        return ((512 - width) / 2, 37, width, spacing, sum);
    }

    private static int subcmd_draw(nint w) => packet(() => {
        var (x, y, width, spacing, _) = bar_geometry(w);
        int n = wu16(w, 0x30), f = wi16(w, 0x46), k = wi16(w, 0x48);
        int wk = n > 0 ? text_w(entry_text(w, Math.Clamp(k, 0, n - 1))) : 0;
        int cw = (width * f + (wk + 24) * (0x1000 - f)) >> 12;
        int before = 0;
        for (int i = 0; i < k && i < n; i++) before += text_w(entry_text(w, i)) + spacing;
        int tx = ((x * f + (x - before - 36) * (0x1000 - f)) >> 12) + 48;
        _scissor!(0, 0, 512, 416);
        if (wu8(w, 0x94) != 1) _plate_rl!(0xd8, 0xd9, x, y, width, 0x80101010, 0x80101010);
        entries(w, n, k, tx, y, spacing, x + 12, cw - 24);
        first(4, "ui.menus: sub-command bar drawn the PS2 way");
    });

    private static int subcmd2_draw(nint w) => packet(() => {
        nint s = *(nint*)Native.ptr(SUBCMD_WINDOW);
        if (s == 0) return;
        var (sx, _, swidth, sspacing, _) = bar_geometry(s);
        int sn = wu16(s, 0x30), sk = wi16(s, 0x48), sf = wi16(s, 0x46);
        int swk = sn > 0 ? text_w(entry_text(s, Math.Clamp(sk, 0, sn - 1))) : 0;
        int cws = (swidth * sf + (swk + 24) * (0x1000 - sf)) >> 12;
        int width = swidth - 24 - swk;
        int x = sx + swidth - width;
        int n = wu16(w, 0x30), k = wi16(w, 0x48);
        _scissor!(0, 0, 512, 416);
        entries(w, n, k, sx + cws + 48, 37, wi16(w, 0x3c), x, width);
    });

    /// <summary>The bar's texts, the cursor on entry k while +0x45 &lt;= 0, the brush under it once picked.</summary>
    private static void entries(nint w, int n, int k, int tx, int y, int spacing, int clip_x, int clip_w) {
        for (int i = 0; i < n; i++) {
            nint t = entry_text(w, i);
            int tw = text_w(t);
            _scissor!(clip_x, 0, clip_x + clip_w, 415);
            _font!(t, tx, y + 2, 0);
            _scissor!(0, 0, 512, 416);
            if (i == k) {
                if (wi8(w, 0x45) <= 0) MainMenuPatches.cursor(tx, y + 2);
                else {
                    // PS2 0x002020a8 at rest: shapes 0xe2, 0xe3 (stretched), 0xe4 under the text
                    _shape!(0xe2, tx - 5, y + 2);
                    _shape_wh!(0xe3, tx + 11, y + 2, tw - 11, 0, GREY, GREY);
                    _shape!(0xe4, tx + tw, y + 2);
                }
            }
            tx += tw + spacing;
        }
    }

    // ---- the ability screen: SLPM_675.13 0x00250978 ---------------------------------------------

    private static Draw? _ability_hook;

    private static int ability_draw() => packet(() => {
        *(byte*)Native.ptr(ABILITY_COUNTER) += 1; // as the PC's draw (its other half, TODrawMenuBG, is off in TkMenuDrawAll)
        int chr = _current!();
        if ((u32(PARTY_MASK) & (1u << chr)) != 0) {
            _box!(i16(POS_NOACTIVE), i16(POS_NOACTIVE + 2) + 49, 512, i16(NOACTIVE_H), u32(COL_NOACTIVE0), u32(COL_NOACTIVE1));
        }
        party_plate(chr, 0, 49);
        first(8, "ui.menus: ability screen's party plate drawn the PS2 way");
    });

    /// <summary>PS2 0x002014e8(chr, x, y): the main menu's party row without the slide, the next-AP count.</summary>
    private static void party_plate(int chr, int x, int y) {
        _plate_lr!(4, 5, x, y, i16(NAMEPLT_W), u32(COL_NAMEPLT0), u32(COL_NAMEPLT1));
        var namestr = point(POS_NAMESTR);
        _font!(_chr_name!(chr), x + namestr.x, y + namestr.y, _damage!(chr));
        var hptex = point(POS_HPTEX);
        var hpstr = point(POS_HPSTR);
        _shape!(0x21, x + hptex.x, y + hptex.y);
        text($"{_hp!(chr),5}/{_max_hp!(chr),5}", t => _str!(t, x + hpstr.x, y + hpstr.y, 0x25, 1f, 0f));
        var mptex = point(POS_MPTEX);
        var mpstr = point(POS_MPSTR);
        _shape!(0x22, x + mptex.x, y + mptex.y);
        text($"{_mp!(chr),4}/{_max_mp!(chr),4}", t => _str!(t, x + mpstr.x, y + mpstr.y, 0x25, 1f, 0f));
        _box_rgba!(0, y + 33, 512, 3, 0, 0, 0, 0x80);
        var slv = point(POS_SLVSHAPE);
        var spstr = point(POS_SPSTR);
        _shape!(0x107, x + slv.x + 0x68, y + slv.y);
        value(_sphere!(chr), x + slv.x + spstr.x, y + slv.y + spstr.y, 0);
        value(_next_ap!(chr) - _save_next_ap!(chr), x + slv.x + 139, y + slv.y - 13, 0);
    }

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(DemasterModule m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(DemasterModule m) {
        _log = m.log;
        _ability_hook = ability_draw;
        return hook<WinDraw>(m, LIST_DRAW, list_draw)
            & hook<WinDraw>(m, TARGET_DRAW, target_draw)
            & hook<WinDraw>(m, SUBCMD_DRAW, subcmd_draw)
            & hook<WinDraw>(m, SUBCMD2_DRAW, subcmd2_draw)
            & hook(m, ABILITY_DRAW, _ability_hook);
    }
}
