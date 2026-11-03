// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, main menu: the PS2 main menu screen drawn by the PS2's draw code ported onto FFX.exe's
/// own primitives. The PC kept the PS2 menu logic and layout data (TOSeqInitParam's col_*/rect_*/pos_*
/// words, TkMn2GetMainPos) but rewrote the draws with HD sheets. Here:
/// <list type="bullet">
/// <item>the background (TODrawMenuBGC2 in TkMenuDrawAll) is the PS2's full-screen gradient from
/// col_menubg0 to col_menubg1, and the HD backdrop (TODrawMenuBG) is not drawn under it;</item>
/// <item>TkMenuDrawMain is SLPM_675.13 0x0024a4a8 (party rows, gil, time, location);</item>
/// <item>MenuMain_DrawCmdWindow is 0x0024b148 / 0x0024af70 (the option plates);</item>
/// <item>the help bar (0x008a9350, a main-menu window's draw) is 0x00243888.</item>
/// </list>
/// The meswin shapes these draw come from the PS2 meswin, which the converter writes as a texture of
/// its own (menu*/d3d11/ps2win): while a ported draw runs, TOGetShapTextureName(0x3e00) answers with
/// that texture's name. FFX.exe's meswin layout table is the PS2's (checked entry by entry), so the
/// shapes' UVs fit. Evidence and addresses: docs/ui.md ("The main menu").
/// </summary>
internal static unsafe class MainMenuPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint ShapeName(int id);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Draw();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int WinDraw(nint w);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BG();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BGC2(uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxC2(int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxRGBA(int x, int y, int w, int h, int r, int g, int b, int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Plate2(int a, int b, int x, int y, int w, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXY(int id, int x, int y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void ShapeXYWHC2(int id, int x, int y, int w, int h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int ShapePart(int id, int part, float x, float y, float w, float h, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Font(nint text, int x, int y, int colour);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void FontA(nint text, int x, int y, int colour, int alpha);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void StrS(nint text, float x, float y, int colour, float sx, float sy);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void DigitRight(int value, float x, float y, int grey, float sx, float sy);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Timer(uint t, float x, float y, int colour, int colon);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt2(int a, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqPoint(int scene, nint name, int* x, int* y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int LoadTexture(nint path);

    // hooked
    private const uint SHAPE_NAME   = 0x008ac8c0; // TOGetShapTextureName(id) -> name
    private const uint MENU_BG      = 0x008f5c10; // TODrawMenuBG: the HD backdrop (ffx_bg, shape -2)
    private const uint MENU_BG_C2   = 0x008f5c60; // TODrawMenuBGC2(c0, c1): TkMenuDrawAll's background
    private const uint DRAW_ALL     = 0x008aa290; // TkMenuDrawAll
    private const uint DRAW_MAIN    = 0x008e0ba0; // TkMenuDrawMain
    private const uint DRAW_OPTIONS = 0x008e20f0; // MenuMain_DrawCmdWindow
    private const uint DRAW_HELP    = 0x008a9350; // the main menu's help bar (window +0x10, installed at 0x008a8e8e)

    // drawn with
    private const uint BOX_C2      = 0x008f3db0; // TODrawBoxXYWHC2: untextured, c0 left -> c1 right
    private const uint BOX_RGBA    = 0x008fb150; // TOMakePktBoxXYWHRGBA
    private const uint PLATE_LR    = 0x008f5e90; // (A, B, x, y, w, c0, c1): A stretched c0..c1, B in c1 (PS2 0x001ff2b8)
    private const uint PLATE_RL    = 0x008f5e20; // (A, B, x, y, w, c0, c1): A in c0, B stretched c0..c1 (PS2 0x001ff1e0)
    private const uint SHAPE_XY    = 0x00903a10; // TOMkpShapeXY
    private const uint SHAPE_XYWH  = 0x00903b70; // TOMkpShapeXYWHC2
    private const uint SHAPE_PART  = 0x009034b0; // TOMkpShapePartIndexXYWHC2
    private const uint FONT_1612   = 0x00901cb0; // TOMkpBaseMesFont(text, x, y, colour): the 16x12 font (PS2 0x001f3810)
    private const uint FONT_1612A  = 0x00901ce0; // (text, x, y, colour, alpha) (PS2 0x001f3858)
    private const uint STR_S       = 0x00901660; // TOMkpCrossEasyStrFontSClut (PS2 0x001f6500)
    private const uint DIGIT_RIGHT = 0x009055c0; // ToMakeBtlEasyDigitRight: right edge at x
    private const uint TIMER       = 0x008e19f0; // TOMenuDrawTimer(t, x, y, colour, colon colour)
    private const uint OPEN_PKT    = 0x008bef30; // TOMenuOpenPktBuffTmp
    private const uint KICK        = 0x008bea20; // TOMenuDrawKickTmp
    private const uint VU1_SYNC    = 0x0088ec60; // TkVU1SyncPath
    private const uint LOAD_TEX    = 0x006651d0; // PhyreScene_loadMenuTexture(path)

    // read
    private const uint LIST_MAX     = 0x008a9b50; // TkMenuGetPlayerListMax2
    private const uint PARTY        = 0x008a9b70; // TkMenuGetPlayerPartyMask (the front-line count)
    private const uint PLAYER       = 0x008a9b00; // TkMenuGetPlayerFromIndex2_008a9b00
    private const uint COS          = 0x0088e750; // Math_CosLookup
    private const uint CHR_NAME     = 0x008ac850; // TOGetSaveChrName
    private const uint DAMAGE_CLASS = 0x008a99f0; // MenuPlayer_DamageClass (the name colour)
    private const uint HP = 0x008a98c0, MAX_HP = 0x008a9990, MP = 0x008a9970, MAX_MP = 0x008a99b0;
    private const uint SPHERE_LV    = 0x008a9b80; // TkMenuGetSP
    private const uint MENU_TIMER   = 0x008a9c60; // TkMenuGetTimer
    private const uint GIL          = 0x00784e80; // MsGetGIL
    private const uint PLAY_TIME    = 0x008a9c50; // TkMenuGetTime
    private const uint SEQ_POINT    = 0x008ae450; // TOSeqGetPoint(scene, name, &x, &y)
    private const uint SAVE_DIC     = 0x0086c410; // AtelGetSaveDic
    private const uint DIC_NAME     = 0x0086c430; // AtelGetSaveDicName
    private const uint ENGLISH      = 0x007851d0; // MsGetSaveConfigEnglish
    private const uint ACTIVE_MASK  = 0x008aa460; // TkMenuMainGetMenuActiveMask
    private const uint OPTIONS      = 0x008e2280; // TkMenuMainGetMaxOptions
    private const uint OPTION_MENU  = 0x008e2270; // TkMenuMainGetOptionMenu
    private const uint OPTION_ON    = 0x008e2290; // TkMenuMainGetOptionEnabled
    private const uint MAIN_MES     = 0x008ddf80; // TkMenuMainMes

    // PS2 layout words (TOSeqInitParam, PC addresses; PS2 0x003241c4.. mirrored, docs/ui.md)
    private const uint DUM75 = 0x025d16ac;                   // the cursor ball's colour
    private const uint NAMEPLT_X = 0x025d1676, NAMEPLT_Y = 0x025d1674, NAMEPLT_W = 0x025d1672, NAMEPLT_SPC = 0x025d1670;
    private const uint COL_NAMEPLT0 = 0x025d166c, COL_NAMEPLT1 = 0x025d1668;
    private const uint POS_NAMESTR = 0x025d1662, POS_HPTEX = 0x025d165e, POS_HPSTR = 0x025d165a;
    private const uint POS_MPTEX = 0x025d1656, POS_MPSTR = 0x025d1652, POS_SLVSHAPE = 0x025d164e, POS_SPSTR = 0x025d164a;
    private const uint COMPLT_X = 0x025d1648, COMPLT_Y = 0x025d1646, COMPLT_W = 0x025d1644, COMPLT_H = 0x025d1642, COMPLT_SPC = 0x025d1640;
    private const uint COL_COMPLT0 = 0x025d163c, COL_COMPLT1 = 0x025d1638;
    private const uint HELPSUB_X = 0x025d1630, HELPSUB_Y = 0x025d162e, HELPSUB_W = 0x025d162c;
    private const uint COL_HELPSUB0 = 0x025d1628, COL_HELPSUB1 = 0x025d1624;
    private const uint HELP_X = 0x025d161c, HELP_Y = 0x025d161a, HELP_W = 0x025d1618;
    private const uint COL_HELP0 = 0x025d1614, COL_HELP1 = 0x025d1610, POS_HELPSTR = 0x025d160a;
    private const uint PLACESUB_X = 0x025d1608, PLACESUB_Y = 0x025d1606, PLACESUB_W = 0x025d1604;
    private const uint COL_PLACESUB0 = 0x025d1600, COL_PLACESUB1 = 0x025d15fc;
    private const uint PLACE_X = 0x025d15f8, PLACE_Y = 0x025d15f6, PLACE_W = 0x025d15f4;
    private const uint COL_PLACE0 = 0x025d15f0, COL_PLACE1 = 0x025d15ec, POS_PLACESHAPE = 0x025d15e8, POS_PLACESTR = 0x025d15e4;
    // TkMn2GetMainPos (PS2 0x00249a20)
    private const uint NOACTIVE_W = 0x023cc13c, NOACTIVE_H = 0x023cc13e, POS_NOACTIVE = 0x023cc144;
    private const uint COL_NOACTIVE0 = 0x023cc148, COL_NOACTIVE1 = 0x023cc140;
    private const uint NMBLKLINE_X = 0x0187168c, NMBLKLINE_Y = 0x0187168e, NMBLKLINE_W = 0x01871694;
    // the main menu's state (PS2 0x0069bce8..)
    private const uint ROW_SLIDE = 0x01871678;   // i16 per row
    private const uint CHOICE    = 0x0187154c;   // g_TkMenuMainChoice
    private const uint SWAP_FROM = 0x0187155c, SWAP_TO = 0x01871560;
    private const uint OPT_CURSOR = 0x01871548, OPT_SLIDE = 0x01871558;
    private const uint HELP_MES  = 0x01841c00;   // g_TkMenuHelpMessage

    private const int MESWIN = 0x3e00;
    private const uint GREY = 0x80808080;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];
    private static int _scope;        // > 0 while a ported draw runs: meswin is the PS2 atlas
    private static bool _in_draw_all; // TkMenuDrawAll is running (its background is the PS2 one)
    private static readonly Dictionary<nint, nint> _ps2_names = [];

    private static ShapeName? _shape_name_next;
    private static BG? _bg_next;
    private static BoxC2? _box;
    private static BoxRGBA? _box_rgba;
    private static Plate2? _plate_lr, _plate_rl;
    private static ShapeXY? _shape;
    private static ShapeXYWHC2? _shape_wh;
    private static ShapePart? _part;
    private static Font? _font;
    private static FontA? _font_a;
    private static StrS? _str;
    private static DigitRight? _digits;
    private static Timer? _timer;
    private static IntFn? _open, _kick, _sync, _list_max, _party, _menu_timer, _gil, _play_time, _save_dic, _english, _active, _options;
    private static IntOfInt? _player, _cos, _damage, _hp, _max_hp, _mp, _max_mp, _sphere, _option_menu, _option_on;
    private static PtrOfInt? _chr_name, _main_mes;
    private static PtrOfInt2? _dic_name;
    private static SeqPoint? _seq_point;
    private static LoadTexture? _load;

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    /// <summary>Fahrenheit hands out call pointers only after every module's init: fetched at the first draw.</summary>
    private static void bind() {
        if (_box is not null) return;
        _box_rgba = Native.fn<BoxRGBA>(BOX_RGBA).fnptr!;
        _plate_lr = Native.fn<Plate2>(PLATE_LR).fnptr!;
        _plate_rl = Native.fn<Plate2>(PLATE_RL).fnptr!;
        _shape = Native.fn<ShapeXY>(SHAPE_XY).fnptr!;
        _shape_wh = Native.fn<ShapeXYWHC2>(SHAPE_XYWH).fnptr!;
        _part = Native.fn<ShapePart>(SHAPE_PART).fnptr!;
        _font = Native.fn<Font>(FONT_1612).fnptr!;
        _font_a = Native.fn<FontA>(FONT_1612A).fnptr!;
        _str = Native.fn<StrS>(STR_S).fnptr!;
        _digits = Native.fn<DigitRight>(DIGIT_RIGHT).fnptr!;
        _timer = Native.fn<Timer>(TIMER).fnptr!;
        _open = Native.fn<IntFn>(OPEN_PKT).fnptr!;
        _kick = Native.fn<IntFn>(KICK).fnptr!;
        _sync = Native.fn<IntFn>(VU1_SYNC).fnptr!;
        _list_max = Native.fn<IntFn>(LIST_MAX).fnptr!;
        _party = Native.fn<IntFn>(PARTY).fnptr!;
        _menu_timer = Native.fn<IntFn>(MENU_TIMER).fnptr!;
        _gil = Native.fn<IntFn>(GIL).fnptr!;
        _play_time = Native.fn<IntFn>(PLAY_TIME).fnptr!;
        _save_dic = Native.fn<IntFn>(SAVE_DIC).fnptr!;
        _english = Native.fn<IntFn>(ENGLISH).fnptr!;
        _active = Native.fn<IntFn>(ACTIVE_MASK).fnptr!;
        _options = Native.fn<IntFn>(OPTIONS).fnptr!;
        _player = Native.fn<IntOfInt>(PLAYER).fnptr!;
        _cos = Native.fn<IntOfInt>(COS).fnptr!;
        _damage = Native.fn<IntOfInt>(DAMAGE_CLASS).fnptr!;
        _hp = Native.fn<IntOfInt>(HP).fnptr!;
        _max_hp = Native.fn<IntOfInt>(MAX_HP).fnptr!;
        _mp = Native.fn<IntOfInt>(MP).fnptr!;
        _max_mp = Native.fn<IntOfInt>(MAX_MP).fnptr!;
        _sphere = Native.fn<IntOfInt>(SPHERE_LV).fnptr!;
        _option_menu = Native.fn<IntOfInt>(OPTION_MENU).fnptr!;
        _option_on = Native.fn<IntOfInt>(OPTION_ON).fnptr!;
        _chr_name = Native.fn<PtrOfInt>(CHR_NAME).fnptr!;
        _main_mes = Native.fn<PtrOfInt>(MAIN_MES).fnptr!;
        _dic_name = Native.fn<PtrOfInt2>(DIC_NAME).fnptr!;
        _seq_point = Native.fn<SeqPoint>(SEQ_POINT).fnptr!;
        _load = Native.fn<LoadTexture>(LOAD_TEX).fnptr!;
        _box = Native.fn<BoxC2>(BOX_C2).fnptr!;
    }

    private static short  i16(uint va) => Native.i16(va);
    private static uint   u32(uint va) => Native.u32(va);
    private static (int x, int y) point(uint va) => (i16(va + 2), i16(va)); // stored y, x

    // ---- the PS2 atlas ------------------------------------------------------------------------

    /// <summary>
    /// The PS2 atlas's name for a meswin name: ".../menu_us/D3D11/meswin.dds.phyre" ->
    /// ".../menu_us/D3D11/ps2win.dds.phyre" (converter uijob::PS2_MESWIN). Made, and its texture
    /// loaded (PhyreScene_loadMenuTexture), the first time it is asked for; until the asynchronous
    /// load is done the menu manager skips the draws ("Need update TextureName").
    /// </summary>
    private static nint ps2_name(nint meswin) {
        lock (_ps2_names) {
            if (_ps2_names.TryGetValue(meswin, out nint known)) return known;
            string? name = Marshal.PtrToStringAnsi(meswin);
            int at = name?.LastIndexOf("meswin.dds", StringComparison.OrdinalIgnoreCase) ?? -1;
            nint ours = meswin;
            if (name is not null && at >= 0) {
                ours = Marshal.StringToHGlobalAnsi(name[..at] + "ps2win.dds" + name[(at + "meswin.dds".Length)..]);
                _load!(ours);
                _log?.Info($"ui.menus: PS2 atlas {Marshal.PtrToStringAnsi(ours)} loading");
            }
            _ps2_names[meswin] = ours;
            return ours;
        }
    }

    private static nint shape_name(int id) {
        nint name = _shape_name_next!(id);
        return id == MESWIN && Volatile.Read(ref _scope) > 0 ? ps2_name(name) : name;
    }

    private static int scoped(Func<int> draw) {
        bind();
        Interlocked.Increment(ref _scope);
        try { return draw(); } finally { Interlocked.Decrement(ref _scope); }
    }

    // ---- shared pieces --------------------------------------------------------------------------

    /// <summary>
    /// The PS2 cursor (SLPM_675.13 0x001fe800, kind 0): meswin shape 45's parts at (x, y): the white
    /// ball (part 0, (80, 144) at -42, -11) tinted with dum75, the silver hand (part 1, (0, 168) at
    /// -34, -3) in grey.
    /// </summary>
    public static void cursor(int x, int y) {
        uint ball = u32(DUM75);
        _part!(45, 0, x, y, 0, 0, ball, ball);
        _part!(45, 1, x, y, 0, 0, GREY, GREY);
    }

    // ---- the background ---------------------------------------------------------------------------

    /// <summary>TkMenuDrawAll's background: PS2 0x00243f98 draws (0, 0, 512, 416) from col_menubg0 to col_menubg1.</summary>
    private static void bg_c2(uint c0, uint c1) {
        bind();
        first(1, "ui.menus: menu background drawn as the PS2 gradient");
        _box!(0, 0, 512, 416, c0, c1);
    }

    /// <summary>The HD backdrop: not drawn over the PS2 gradient (inside TkMenuDrawAll).</summary>
    private static void bg() {
        if (_in_draw_all) return;
        _bg_next ??= Native.fn<BG>(MENU_BG).chain_from(_bg_hook!).fnptr!;
        _bg_next();
    }

    private static BG? _bg_hook;
    private static Draw? _all_hook, _all_next;

    private static int draw_all() {
        _all_next ??= Native.fn<Draw>(DRAW_ALL).chain_from(_all_hook!).fnptr!;
        _in_draw_all = true;
        try { return _all_next(); } finally { _in_draw_all = false; }
    }

    // ---- TkMenuDrawMain: SLPM_675.13 0x0024a4a8 ----------------------------------------------------

    private static int draw_main() => scoped(() => {
        int n = _list_max!();
        _sync!();
        _open!();
        *(short*)Native.ptr(NAMEPLT_SPC) = 44;  // num_namepltspc, num_compltspc: the PS2 draw sets them
        *(short*)Native.ptr(COMPLT_SPC) = 32;
        int nx = i16(NAMEPLT_X), ny = i16(NAMEPLT_Y), nw = i16(NAMEPLT_W);
        uint c0 = u32(COL_NAMEPLT0), c1 = u32(COL_NAMEPLT1);
        var namestr = point(POS_NAMESTR);
        var hptex = point(POS_HPTEX);
        var hpstr = point(POS_HPSTR);
        var mptex = point(POS_MPTEX);
        var mpstr = point(POS_MPSTR);
        var slv = point(POS_SLVSHAPE);
        var spstr = point(POS_SPSTR);
        int party = _party!();
        int choice = (int)u32(CHOICE), from = (int)u32(SWAP_FROM), to = (int)u32(SWAP_TO);
        for (int i = 0; i < n; i++) {
            int slide = i16(ROW_SLIDE + 2 * (uint)i);
            if (slide == 0) continue;
            int dx = -(((ushort)_cos!(slide)) << 10) / 0x3000;
            int chr = _player!(i);
            int x = dx + nx, y = ny + i * 44;
            if (i < party) {
                _box!(x + i16(POS_NOACTIVE), y + i16(POS_NOACTIVE + 2), i16(NOACTIVE_W), i16(NOACTIVE_H),
                      u32(COL_NOACTIVE0), u32(COL_NOACTIVE1));
            }
            _plate_lr!(4, 5, x, y, nw, c0, c1);
            _shape_wh!(0x16, x + i16(NMBLKLINE_X), y + i16(NMBLKLINE_Y), i16(NMBLKLINE_W), 0, GREY, GREY);
            _font!(_chr_name!(chr), x + namestr.x, y + namestr.y, _damage!(chr));
            _shape!(0x21, x + hptex.x, y + hptex.y);
            text($"{_hp!(chr),5}/{_max_hp!(chr),5}", t => _str!(t, x + hpstr.x, y + hpstr.y, 0x25, 1f, 0f));
            _shape!(0x22, x + mptex.x, y + mptex.y);
            text($"{_mp!(chr),4}/{_max_mp!(chr),4}", t => _str!(t, x + mpstr.x, y + mpstr.y, 0x25, 1f, 0f));
            _shape!(0x3d, x + slv.x, y + slv.y);
            // PS2 0x001f5e88: "%d" in 12-pixel cells ending at x + 2 (0x001f5f58)
            _digits!(_sphere!(chr), x + spstr.x + slv.x + 2, y + slv.y + spstr.y, 0, BattlePatches.SX, BattlePatches.SY);
            int cx = x + namestr.x, cy = y + namestr.y;
            if (choice == chr) cursor(cx, cy);
            if (from >= 0 && _player!(from) == chr) {
                if (to < 0) cursor(cx, cy);
                else if (((_menu_timer!() / 2) & 1) != 0) cursor(cx - 10, cy - 2);
            }
            if (to >= 0 && _player!(to) == chr) cursor(cx, cy);
        }
        // the footer
        var gilstr = seq_point("pos_gilstr");
        _digits!(_gil!(), gilstr.x + 2, gilstr.y, 0, BattlePatches.SX, BattlePatches.SY);
        uint t = (uint)_play_time!();
        var timestr = seq_point("pos_timestr");
        _timer!(t, timestr.x, timestr.y, 0x25, (t & 1) != 0 ? 0x25 : 1);
        var gilshape = seq_point("pos_gilshape");
        var gilshape2 = seq_point("pos_gilshape2");
        _shape!(0xd5, gilshape.x, gilshape.y);
        _shape!(0xd6, gilshape.x + gilshape2.x, gilshape.y + gilshape2.y);
        _plate_rl!(0xd2, 0xd3, i16(PLACESUB_X), i16(PLACESUB_Y), i16(PLACESUB_W), u32(COL_PLACESUB0), u32(COL_PLACESUB1));
        int px = i16(PLACE_X), py = i16(PLACE_Y);
        _plate_rl!(0xd8, 0xd9, px, py, i16(PLACE_W), u32(COL_PLACE0), u32(COL_PLACE1));
        var placeshape = point(POS_PLACESHAPE);
        var placestr = point(POS_PLACESTR);
        _shape!(0x27, px + placeshape.x, py + placeshape.y);
        _font!(_dic_name!(_save_dic!(), _english!()), px + placestr.x, py + placestr.y, 0);
        _kick!();
        first(2, "ui.menus: main menu drawn the PS2 way");
        return _sync!();
    });

    private static readonly nint[] _names = new nint[4];

    /// <summary>TOSeqGetPoint(1, name, &amp;x, &amp;y) of the menu sequence, as the PS2 draw reads its footer points.</summary>
    private static (int x, int y) seq_point(string name) {
        int k = name switch { "pos_gilstr" => 0, "pos_timestr" => 1, "pos_gilshape" => 2, _ => 3 };
        if (_names[k] == 0) _names[k] = Marshal.StringToHGlobalAnsi(name);
        int x = 0, y = 0;
        _seq_point!(1, _names[k], &x, &y);
        return ((short)x, (short)y);
    }

    /// <summary>A string for one call (the PS2's sprintf into a stack buffer).</summary>
    private static void text(string s, Action<nint> use) {
        nint p = Marshal.StringToHGlobalAnsi(s);
        try { use(p); } finally { Marshal.FreeHGlobal(p); }
    }

    // ---- MenuMain_DrawCmdWindow: SLPM_675.13 0x0024b148 -------------------------------------------

    private static int draw_options(nint w) => scoped(() => {
        int mask = _active!();
        if ((mask & 0x80800) != 0) return mask;
        _sync!();
        _open!();
        int slide = (int)u32(OPT_SLIDE), cur = (int)u32(OPT_CURSOR);
        if (slide != 0) {
            for (int i = 0; i < _options!(); i++) if (i != cur) option(i, 0, slide, cur);
        }
        option(cur, 0, slide, cur);
        _kick!();
        first(4, "ui.menus: main menu options drawn the PS2 way");
        return _sync!();
    });

    /// <summary>One option (PS2 0x0024af70): a shadow, the gradient plate, the name; plates fade and slide in by slide (0..0x1000).</summary>
    private static void option(int i, int x, int slide, int cur) {
        int rx = i16(COMPLT_X), ry = i16(COMPLT_Y), rw = i16(COMPLT_W), rh = i16(COMPLT_H), spc = i16(COMPLT_SPC);
        int y = ((ry + i * spc) * slide + (0x1000 - slide) * 3) / 4096;
        int alpha = slide / 32;
        uint a = (uint)alpha << 24;
        _box_rgba!(x + rx + 4, y + 4, rw, rh, 0, 0, 0, alpha);
        _box!(x + rx, y, rw, rh, (u32(COL_COMPLT0) & 0xffffff) | a, (u32(COL_COMPLT1) & 0xffffff) | a);
        int grey = (short)_option_on!(i) == 0 ? 1 : 0;
        _font_a!(_main_mes!(_option_menu!(i)), x + rx + 15, y + 2, grey, alpha);
        if (i == cur && slide == 0x1000) cursor(x + rx + 15, y + 2);
    }

    // ---- the help bar: SLPM_675.13 0x00243888 ------------------------------------------------------

    private static int draw_help(nint w) => scoped(() => {
        int mask = _active!();
        if ((mask & 0x80800) != 0) return mask;
        _sync!();
        _open!();
        int sx = i16(HELPSUB_X), sy = i16(HELPSUB_Y);
        _plate_lr!(0x45, 0x46, sx, sy, i16(HELPSUB_W), u32(COL_HELPSUB0), u32(COL_HELPSUB1));
        int hx = i16(HELP_X), hy = i16(HELP_Y);
        _plate_lr!(6, 7, hx, hy, i16(HELP_W), u32(COL_HELP0), u32(COL_HELP1));
        _shape!(0x3e, sx + 0x50, sy);
        nint mes = (nint)u32(HELP_MES);
        if (mes != 0) {
            var at = point(POS_HELPSTR);
            _font!(mes, hx + at.x, hy + at.y, 0);
        }
        _kick!();
        first(8, "ui.menus: menu help bar drawn the PS2 way");
        return _sync!();
    });

    // ---- install ------------------------------------------------------------------------------------

    private static bool hook<T>(DemasterModule m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(DemasterModule m) {
        _log = m.log;
        ShapeName name_hook = id => shape_name(id);
        _keep.Add(name_hook);
        bool ok = Native.fn<ShapeName>(SHAPE_NAME).hook(m, name_hook);
        // the original, for the hook (fetched lazily: call pointers exist only after init)
        _shape_name_next = id => {
            var next = Native.fn<ShapeName>(SHAPE_NAME).chain_from(name_hook).fnptr!;
            _shape_name_next = next;
            return next(id);
        };
        _bg_hook = bg;
        _all_hook = draw_all;
        return ok
            & hook(m, MENU_BG, _bg_hook)
            & hook<BGC2>(m, MENU_BG_C2, bg_c2)
            & hook(m, DRAW_ALL, _all_hook)
            & hook<Draw>(m, DRAW_MAIN, draw_main)
            & hook<WinDraw>(m, DRAW_OPTIONS, draw_options)
            & hook<WinDraw>(m, DRAW_HELP, draw_help);
    }
}
