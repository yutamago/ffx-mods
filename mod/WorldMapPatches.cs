// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, the world map (the map picture TkMenuCtrlWorldMap shows, with the current place's
/// marker and name): SLPM_675.13's draw 0x00224700 (control 0x00224438 = TkMenuCtrlWorldMap
/// 0x008e2960) ported onto FFX.exe's primitives. Both draw the picture (texture 0x2eac) over the
/// 512 x 416 screen and the marker scene "scene30" at the map marker / 2; the PC then draws an HD
/// bar (an HD sheet quad and meswin shape 200) and the place name in the HD font at HD places,
/// where the PS2 drew the main menu's location plates (meswin 0xd2/0xd3 at rect_placesubplt,
/// 0xd8/0xd9 at rect_placeplt, the "Location" label 0xef = layout entry 39 = 0x27, the name in
/// the 16x12 font), the whole group centred on the screen. The converter writes the PS2 picture
/// (sgjob.rs). Evidence: docs/ui.md ("The world map").
/// </summary>
internal static unsafe class WorldMapPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Draw(nint window);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint DicName(int dic, int english);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Picture(int x, int y, uint c0, uint c1);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Scissor(int x, int y, int w, int h);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int ScenePos(int seq, nint name, int x, int y);

    private const uint DRAW       = 0x008e2b60; // MenuWorldMap_Draw_008e2b60 (window +0x10; PS2 0x00224700)
    private const uint PICTURE    = 0x00902810; // TOMkpFont_00902810: the picture 0x2eac over 512 x 416 (PS2 0x002002d8)
    private const uint SCISSOR    = 0x008f9230; // TODrawScissorXYWH (PS2 0x001f1dc0)
    private const uint MARKER_X   = 0x00873ae0; // AtelGetMapMarkerX (PS2 0x0014f3a0)
    private const uint MARKER_Y   = 0x00873af0; // AtelGetMapMarkerY (PS2 0x0014f3c0)
    private const uint SCENE_POS  = 0x008af060; // TOSeqSetScenePosByName (PS2 0x0020df18)
    private const uint SAVE_DIC   = 0x0086c410; // AtelGetSaveDic (PS2 0x001487d0)
    private const uint DIC_NAME   = 0x0086c430; // AtelGetSaveDicName (PS2 0x00148800)
    private const uint ENGLISH    = 0x007851d0; // MsGetSaveConfigEnglish (PS2 0x001bd360)

    // the main menu's footer words (TOSeqInitParam, mirrored; PS2 0x00324270..0x00324296, read by
    // the PS2 main menu 0x0024aa3c and this draw alike)
    private const uint PLACESUB_X = 0x025d1608, PLACESUB_Y = 0x025d1606, PLACESUB_W = 0x025d1604;
    private const uint COL_PLACESUB0 = 0x025d1600, COL_PLACESUB1 = 0x025d15fc;
    private const uint PLACE_X = 0x025d15f8, PLACE_Y = 0x025d15f6, PLACE_W = 0x025d15f4;
    private const uint COL_PLACE0 = 0x025d15f0, COL_PLACE1 = 0x025d15ec, POS_PLACESHAPE = 0x025d15e8, POS_PLACESTR = 0x025d15e4;

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];

    private static T fn<T>(uint va) where T : Delegate => Ps2Menu.fn<T>(va);
    private static short i16(uint va) => Native.i16(va);
    private static uint u32(uint va) => Native.u32(va);

    /// <summary>
    /// PS2 0x00224700(window): scissor (0, 0, 511, 415), the picture tinted alpha +0x64 (the
    /// window's fade); once it is fully in, the marker scene at (marker x / 2, marker y / 2 - 16)
    /// when the marker is set, then the location plates and the place name, shifted by
    /// (512 - (right - left)) / 2 - left so the group is centred (left = the smaller plate x,
    /// right = the larger plate end).
    /// </summary>
    private static int draw(nint window) => Ps2Menu.packet(() => {
        fn<Scissor>(SCISSOR)(0, 0, 0x200, 0x1a0);
        int a = *(int*)(window + 0x64);
        uint c = (uint)a << 24 | 0x808080;
        fn<Picture>(PICTURE)(0, 0, c, c);
        if (a != 0x80) return;
        int mx = fn<IntFn>(MARKER_X)(), my = fn<IntFn>(MARKER_Y)();
        if (mx != 0 || my != 0) {
            nint scene = Ps2Menu.cstr("scene30");
            fn<Ps2Menu.SeqScene>(Ps2Menu.SEQ_START)(1, scene);
            fn<ScenePos>(SCENE_POS)(1, scene, mx / 2, my / 2 - 16);
        }
        int sx = i16(PLACESUB_X), sw = i16(PLACESUB_W);
        int px = i16(PLACE_X), pw = i16(PLACE_W);
        int left = Math.Min(sx, px);
        int right = Math.Max(sx + sw, px + pw);
        int shift = (512 - (right - left)) / 2 - left;
        Ps2Menu.plate_rl(0xd2, 0xd3, sx + shift, i16(PLACESUB_Y), sw, u32(COL_PLACESUB0), u32(COL_PLACESUB1));
        int py = i16(PLACE_Y);
        Ps2Menu.plate_rl(0xd8, 0xd9, px + shift, py, pw, u32(COL_PLACE0), u32(COL_PLACE1));
        var shape = Ps2Menu.point(POS_PLACESHAPE);
        var str = Ps2Menu.point(POS_PLACESTR);
        Ps2Menu.shape(0xef, px + shape.x + shift, py + shape.y);
        nint name = fn<DicName>(DIC_NAME)(fn<IntFn>(SAVE_DIC)(), fn<IntFn>(ENGLISH)());
        Ps2Menu.font(name, px + str.x + shift, py + str.y, 0);
        if ((Interlocked.Or(ref _logged, 1) & 1) == 0) _log?.Info("ui.menus: world map drawn the PS2 way");
    });

    public static bool install(DemasterModule m) {
        _log = m.log;
        Draw d = draw;
        _keep.Add(d);
        return Native.fn<Draw>(DRAW).hook(m, d);
    }
}
