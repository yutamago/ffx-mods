// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.Demaster;

/// <summary>
/// ui.menus, battle HUD: the PS2's battle help bar, turn order (CTB) list and party list, ported
/// from SLPM_675.13 onto FFX.exe's primitives. TODrawBtlWindow (PS2 0x00214ae4, PC 0x0089b360)
/// calls the same pieces on both; the PC rewrote their draws with HD sheets and HD coordinates.
/// Here the PC's draws are replaced by the PS2 ones (PS2 units, 512 x 416), drawn from the PS2
/// meswin (ps2win) and battle atlas (ps2btl). Evidence: docs/ui.md ("The battle HUD").
/// </summary>
internal static unsafe class BattleHudPatches {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Void();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Ctb(int x, int y);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntFn();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int IntOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int SeqScene(int seq, nint name);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Scissor(int x0, int y0, int x1, int y1);

    // hooked
    private const uint DRAW_HELP = 0x008936d0;   // TOBtlDrawHelpWin() (PS2 0x00218338)
    private const uint DRAW_CTB  = 0x008a0590;   // TOBtlDrawCtbWindow(x, y) (PS2 0x002138c8)

    // called
    private const uint SAVE_MENU   = 0x00785290; // MsGetSaveMenu -> the config word (bit 0x400: help on)
    private const uint SEQ_CONTINUE = 0x008af1b0; // TOSeqStartSceneDrawContinue(seq, name) (PS2 0x0020d930)
    private const uint SEQ_STOP    = 0x008af4f0; // TOSeqStopScene(seq, name) (PS2 0x0020db20)
    private const uint SCISSOR_XYWH = 0x008f9230; // TODrawScissorXYWH (PS2 0x001f1dc0)
    private const uint SCENARIO    = 0x0086c470; // AtelGetScenarioFlag (PS2 0x0014ac00)
    private const uint ICON_TYPE   = 0x0079ae30; // MsGetRamChrIconType (PS2 0x001bc190)
    private const uint ICON_NUMBER = 0x0079ae10; // MsGetRamChrIconNumber (PS2 0x001bc1b0)
    private const uint CTB_SLIDE   = 0x008a2b10; // TOBtlCtbSlideStep_008a2b10: a row leaving (PS2 0x00213f00)

    // read
    private const uint HIDDEN      = 0x0133d6e0; // 1: the PC's own battle HUD draws return at once
    private const uint HELP_OPEN   = 0x0133c93c; // g_BtlHelpWinOpen (PS2 0x003249ab)
    private const uint HELP_TEXT   = 0x0133f538; // g_BtlHelpText (PS2 0x00692480)
    private const uint CTB_ROW     = 0x01340446; // g_CtbScrollRow, i8 (PS2 0x00324990)
    private const uint CTB_SCROLL  = 0x01340448; // g_CtbScroll, i16 (PS2 0x0032498a)
    private const uint CTB_MASK    = 0x01340470; // g_CtbAnimMask: the targeted actors (PS2 0x006902bc)
    private const uint CTB_ROWS    = 0x01340478; // g_CtbRowSlots: 32 row pointers (PS2 0x006902c0)
    private const uint WINDOWS     = 0x0133c950; // g_BtlWindowRecords, 0xf0 each (PS2 0x003249c0, 0xcc)
    private const uint DEPTH       = 0x023cc092; // g_BtlActionDepth, i8 (PS2 0x00325020)
    private const uint DUM73       = 0x025d16a4; // a targeted monster's CTB marker colour (PS2 0x003241d4)
    private const uint DUM74       = 0x025d16a8; // a targeted party member's (PS2 0x003241d0)
    // the help bar: the menu sequence's rect_helpsubplt, col_helpsubplt0/1, rect_helpplt (mirrored)
    private const uint HELPSUB_X = 0x025d1630, HELPSUB_Y = 0x025d162e, HELPSUB_W = 0x025d162c;
    private const uint COL_HELPSUB0 = 0x025d1628, COL_HELPSUB1 = 0x025d1624;
    private const uint HELP_X = 0x025d161c, HELP_Y = 0x025d161a, HELP_W = 0x025d1618;

    public const uint BATTLE_LAYOUT = 0x00c5c2c8; // the battle atlas's layout table (= SLPM_675.13 0x00322190)
    private const uint GREY = 0x80808080;
    private const uint HELP_PLATE = 0x40000000;   // the PS2's help plate colour in battle (black, alpha 0x40)

    private static FhLogger? _log;
    private static int _logged;
    private static readonly List<Delegate> _keep = [];
    private static readonly nint BHLINE = Marshal.StringToHGlobalAnsi("bhline");

    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static T fn<T>(uint va) where T : Delegate => Ps2Menu.fn<T>(va);
    private static short i16(uint va) => Native.i16(va);
    private static uint u32(uint va) => Native.u32(va);

    // ---- battle atlas shapes -----------------------------------------------------------------

    /// <summary>
    /// A battle atlas shape (ids 400..598, layout table 0x00c5c2c8) as the PS2's shape draws
    /// (0x001fe1f0 / 0x001fe248 / 0x001fe2a0) put it: each part at (x + dx, y + dy), its own size
    /// unless w / h are given, sampled at texel centres (TOMkpShapeXYWHUVC2, sheet 0x3f00, UVs half
    /// a texel inside the cell, as Ps2Menu.shape_c2 does for meswin). Run inside
    /// MainMenuPatches.battle_scoped, so the sheet is the PS2 atlas.
    /// </summary>
    public static void bshape(int id, int x, int y, int w, int h, uint c0, uint c1) {
        byte* table = Native.ptr(BATTLE_LAYOUT);
        byte* e = table + *(uint*)(table + 4 + 4 * (id - 400));
        int n = *(int*)e;
        var uv = fn<Ps2Menu.ShapeUV>(Ps2Menu.SHAPE_UV);
        for (int k = 0; k < n; k++) {
            byte* p = e + 4 + 12 * k;
            int u = p[0], v = p[1], dx = *(sbyte*)(p + 2), dy = *(sbyte*)(p + 3);
            int pw = *(ushort*)(p + 4), ph = *(ushort*)(p + 6);
            float u0 = (u + 0.5f) / 256f, u1 = (u + pw - 0.5f) / 256f, v0 = (v + 0.5f) / 256f, v1 = (v + ph - 0.5f) / 256f;
            if (*(ushort*)(p + 8) == 1) (u0, u1) = (u1, u0);
            if (*(ushort*)(p + 10) == 1) (v0, v1) = (v1, v0);
            uv(id, x + dx, y + dy, w != 0 ? w : pw, h != 0 ? h : ph, u0, v0, u1, v1, c0, c1);
        }
    }

    private static void bshape(int id, int x, int y) => bshape(id, x, y, 0, 0, GREY, GREY);

    /// <summary>
    /// PS2 0x001fa7d8(x, y, w, h): the battle window frame, battle shapes 400..408 as nine
    /// pieces (body 404 stretched over w x h, corners at their own size, edges stretched).
    /// </summary>
    public static void frame(int x, int y, int w, int h) {
        bshape(0x194, x, y, w, h, GREY, GREY);
        bshape(0x190, x, y, 0, 0, GREY, GREY);
        bshape(0x191, x, y, 0, h, GREY, GREY);
        bshape(0x192, x, y + h, 0, 0, GREY, GREY);
        bshape(0x193, x, y, w, 0, GREY, GREY);
        bshape(0x195, x, y + h, w, 0, GREY, GREY);
        bshape(0x196, x + w, y, 0, 0, GREY, GREY);
        bshape(0x197, x + w, y, 0, h, GREY, GREY);
        bshape(0x198, x + w, y + h, 0, 0, GREY, GREY);
    }

    /// <summary>
    /// PS2 0x001fbfc0(x, y, w, h, total, shown, first): a grey (0x70) box, a black inside one
    /// pixel in, and a light (0xc0) thumb over first .. first + shown of total; nothing when all
    /// rows show. Boxes from (x0, y0) to (x1, y1) as GS sprites.
    /// </summary>
    public static void scroll(int x, int y, int w, int h, int total, int shown, int top) {
        if (shown >= total) return;
        int ix0 = x + 1, iy0 = y + 1, ix1 = x + w - 1, iy1 = y + h - 1;
        float step = (float)(iy1 - iy0) / total;
        float t = iy0 + step * top, b = t + step * shown;
        if (top == 0) t = iy0;
        if (top + shown >= total) b = iy1;
        Ps2Menu.box_rgba(x, y, w, h, 0x70, 0x70, 0x70, 0x80);
        Ps2Menu.box_rgba(ix0, iy0, ix1 - ix0, iy1 - iy0, 0, 0, 0, 0x80);
        Ps2Menu.box_rgba(ix0, (int)t, ix1 - ix0, (int)b - (int)t, 0xc0, 0xc0, 0xc0, 0x80);
    }

    // ---- the help bar: SLPM_675.13 0x00218338 ------------------------------------------------

    /// <summary>
    /// The PS2 battle help bar: while the help setting is on, the battle sequence scene "bhline"
    /// runs (the gold stroke along the top; the PC never starts it); when the help window is open,
    /// the menu help bar's plates (0x45/0x46 at rect_helpsubplt in col_helpsubplt, 6/7 at
    /// rect_helpplt in black with alpha 0x40, not col_helpplt as in the menus), the HELP label
    /// (meswin 0x3e at helpsub + (0x50, 0)) and the text at (40, 12) in the 16x12 font. The PC
    /// draws its own HD band and scrolls long texts; the PS2 does not.
    /// </summary>
    private static void help() {
        if (u32(HIDDEN) == 1) return;
        bool on = (*(uint*)fn<PtrFn>(SAVE_MENU)() & 0x400) != 0;
        // the PS2 calls these every frame (TOSeqInitScene(2) at each window's opening ends it)
        if (on != Ps2Menu.scene_on(2, "bhline")) fn<SeqScene>(on ? SEQ_CONTINUE : SEQ_STOP)(2, BHLINE);
        if (!on || Native.u8(HELP_OPEN) != 1) return;
        MainMenuPatches.ps2_scoped(() => {
            fn<Scissor>(SCISSOR_XYWH)(0, 0, 0x200, 0x1a0);
            int sx = i16(HELPSUB_X), sy = i16(HELPSUB_Y);
            Ps2Menu.plate_lr(0x45, 0x46, sx, sy, i16(HELPSUB_W), u32(COL_HELPSUB0), u32(COL_HELPSUB1));
            Ps2Menu.plate_lr(6, 7, i16(HELP_X), i16(HELP_Y), i16(HELP_W), HELP_PLATE, HELP_PLATE);
            MainMenuPatches.help_label(sx + 0x50, sy);
            Ps2Menu.font((nint)Native.ptr(HELP_TEXT), 40, 12, 0);
            return 0;
        });
        first(1, "ui.menus: battle help bar drawn the PS2 way");
    }

    // ---- the turn order list: SLPM_675.13 0x002138c8 ----------------------------------------

    /// <summary>
    /// The PS2 CTB window at (x, y) (dum19): the battle frame 58 x 174, the arrows 438 (scrolled
    /// down) and 439 (more below) right of it, a scroll bar (x + 2, 4 wide, 8 of 16 rows), then
    /// each row at (x + 8 + row x, y + scroll + row y) inside the frame: the wait bar and the face
    /// (0x00213b98); the first row gets the orange marker 472. In target selection (the current
    /// command window's +2 is 1) the targeted rows get marker 429 (dum74 for the party, dum73 for
    /// monsters) and 430. The PC draws the HD list in HD coordinates (48 per row).
    /// </summary>
    private static void ctb(int x, int y) {
        if (u32(HIDDEN) == 1) return;
        MainMenuPatches.battle_scoped(() => {
            var scissor = fn<Scissor>(Ps2Menu.SCISSOR);
            scissor(0, 0, 0x200, 0x1a0);
            frame(x, y, 0x3a, 0xae);
            int row = *(sbyte*)Native.ptr(CTB_ROW);
            if (row > 0) bshape(0x1b6, x + 0x3a, y);
            if (row < 8) bshape(0x1b7, x + 0x3a, y + 0xae);
            scroll(x + 2, y, 4, 0xae, 16, 8, row);
            scissor(x, y, x + 0x3a, y + 0xae);
            _clip = (x, y, x + 0x3a, y + 0xae);
            short off = i16(CTB_SCROLL);
            nint* rows = (nint*)Native.ptr(CTB_ROWS);
            for (int i = 0; i < 32; i++) {
                nint e = rows[i];
                if (e == 0) continue;
                int ex = (short)(*(short*)(e + 0x10) + x + 8), ey = (short)(off + *(short*)(e + 0x12) + y);
                if (*(uint*)(e + 0x14) == Native.reloc(CTB_SLIDE)) continue;
                if (i == 0) bshape(0x1d8, ex, ey);
                ctb_row(*(sbyte*)(e + 1), *(int*)(e + 4), ex, ey);
            }
            int depth = *(sbyte*)Native.ptr(DEPTH);
            if (Native.u8(WINDOWS + 0xf0 * (uint)depth + 2) == 1) {
                fn<Scissor>(SCISSOR_XYWH)(x - 0x1e, y, 0x58, 0xae);
                uint mask = u32(CTB_MASK);
                for (int i = 0; i < 16; i++) {
                    nint e = rows[i];
                    if (e == 0) continue;
                    int chr = *(sbyte*)(e + 1);
                    if (((mask >> (chr & 31)) & 1) == 0) continue;
                    int ey = (short)(off + *(short*)(e + 0x12) + y);
                    // (the PS2 also draws 470 at (2 x + 8, y) here, outside this scissor)
                    uint c = u32(chr < 0x14 ? DUM74 : DUM73);
                    bshape(0x1ad, x, ey, 0, 0, c, c);
                    bshape(0x1ae, x, ey);
                }
            }
            fn<Scissor>(SCISSOR_XYWH)(0, 0, 0x200, 0x1a0);
        });
        first(2, "ui.menus: battle CTB list drawn the PS2 way");
    }

    /// <summary>
    /// One CTB row (PS2 0x00213b98(chr, wait, x, y)): the wait bar 40 x 20 in three purples
    /// (dark 0x59 0x0f 0x7c, medium 0x95 0x39 0xc1, light 0xb5 0x75 0xd6: the dark part grows
    /// from 40, the medium from 20), the face at
    /// x + min(wait, 20): party 440 + chr (Rikku's 468 before scenario 0x434), aeons 448,
    /// monsters by icon type (0x14: 459 + number, 0x15: 450, 0x16: 451 + number, 0x17: 467,
    /// else 449).
    /// </summary>
    private static void ctb_row(int chr, int wait, int x, int y) {
        const int DARK = 0, MID = 1, LIGHT = 2;
        void bar(int x0, int x1, int k) {
            (int r, int g, int b) = k switch { DARK => (0x59, 0x0f, 0x7c), MID => (0x95, 0x39, 0xc1), _ => (0xb5, 0x75, 0xd6) };
            clipped_box(x0, y, x1 - x0, 20, r, g, b);
        }
        if (wait >= 60) bar(x, x + 40, DARK);
        if (wait >= 40 && wait < 60) { bar(x, x - 40 + wait, DARK); bar(x - 40 + wait, x + 40, MID); }
        if (wait >= 20 && wait < 40) { bar(x, x - 20 + wait, MID); bar(x - 20 + wait, x + 40, LIGHT); }
        if (wait < 20) bar(x, x + wait, LIGHT);
        int fx = x + (wait < 20 ? wait : 20);
        if (chr < 8) {
            bool young = chr == 6 && fn<IntFn>(SCENARIO)() < 0x434;
            bshape(young ? 0x1d4 : 0x1b8 + chr, fx, y);
        }
        if ((uint)(chr - 8) < 10) bshape(0x1c0, fx, y);
        if (chr >= 0x14) {
            int type = fn<IntOfInt>(ICON_TYPE)(chr);
            int id = type switch {
                0x14 => 0x1cb + fn<IntOfInt>(ICON_NUMBER)(chr),
                0x15 => 0x1c2,
                0x16 => 0x1c3 + fn<IntOfInt>(ICON_NUMBER)(chr),
                0x17 => 0x1d3,
                _ => 0x1c1,
            };
            bshape(id, fx, y);
        }
    }

    /// <summary>The CTB window's scissor for its boxes: TOMakePktBoxXYWHRGBA ignores the PC's scissor (the PS2's GS clipped them).</summary>
    private static (int x0, int y0, int x1, int y1) _clip;

    private static void clipped_box(int x, int y, int w, int h, int r, int g, int b) {
        int x0 = Math.Max(x, _clip.x0), y0 = Math.Max(y, _clip.y0);
        int x1 = Math.Min(x + w, _clip.x1), y1 = Math.Min(y + h, _clip.y1);
        if (x1 > x0 && y1 > y0) Ps2Menu.box_rgba(x0, y0, x1 - x0, y1 - y0, r, g, b, 0x80);
    }

    // ---- the party list: SLPM_675.13 0x0021c498 ---------------------------------------------

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint PtrOfInt(int a);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate nint Text(int a, int id, int b);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void BoxFW(int x, int y, int w, int h, uint c0, uint c1, int full);

    private const uint DRAW_STATUS = 0x00896450; // TOBtlDrawStatusWin() (PS2 0x0021c498)
    private const uint CHR_NAME    = 0x0079af10; // MsGetRamChrName (PS2 0x001bb190)
    private const uint MAX_HP      = 0x0079adf0; // MsGetRamChrHPmax (PS2 0x001bb270)
    private const uint MAX_MP      = 0x0079ae70; // MsGetRamChrMPmax (PS2 0x001bb290)
    private const uint MENU_TEXT   = 0x0078fcf0; // MsMenuGetText(1, id, 1) (PS2 0x001dbd60)
    private const uint BOX_FW      = 0x008f3eb0; // TODrawBoxXYWHC2FW(x, y, w, h, c0, c1, full) (PS2 0x001f0588)
    private const uint STATUS_MODE = 0x0133d782; // i8: 0 HP, 1 MP, 2 status, 3 Overdrive (PS2 0x006928a0)
    private const uint STATUS_CHR  = 0x0133f7ac; // u16 per row, 0xff empty (PS2 0x006926e8)
    private const uint STATUS_ROWS = 0x0133f7d8; // 0x90 per row (PS2 0x006926f0)
    private const uint WINDOW_CHR  = 0x023cc088; // g_BtlWindowChr, i16 (PS2 0x003252a4)
    private const uint TARGETS     = 0x0133d178; // g_BtlTargetSelMask (PS2 0x003250d8)
    // the battle sequence's layout (TOSeqInitParam scene 2; PS2 0x0032430c..0x00324356, mirrored)
    private const uint DUM45_X = 0x025d0aee, DUM45_Y = 0x025d0aec, DUM52 = 0x025d0aea, DUM13 = 0x025d0ae8;
    private const uint DUM2 = 0x025d0ae4, DUM1 = 0x025d0ae0, DUM40 = 0x025d0adc, DUM41 = 0x025d0ad8;
    private const uint DUM10 = 0x025d0ad4, DUM3 = 0x025d0ad0, DUM4 = 0x025d0ac4, DUM5 = 0x025d0ac0;
    private const uint DUM6 = 0x025d0abc, DUM7 = 0x025d0ab8, DUM8 = 0x025d0ab4, DUM8_W = 0x025d0ab2, DUM8_H = 0x025d0ab0;
    private const uint DUM9_X = 0x025d0aa8, DUM9_Y = 0x025d0aa6, DUM9_W = 0x025d0aa4, DUM9_H = 0x025d0aa2;

    /// <summary>
    /// The PS2 party list (0x0021c498): three rows from dum45, dum13 apart. Each: in the
    /// Overdrive view (mode 3) the acting member's highlight (battle shapes 409/410 dum52 wide,
    /// dum2 -> dum1, a light line along the top and a dark one under it, both fading in over the
    /// first 50 pixels), in the other views the targeted members' (dum40 -> dum41); the name
    /// right-aligned at dum10 in the 16x12 font in its HP colour (+0x19); then by mode: HP and MP
    /// labels (meswin 0xe9, 0xea) with the shown values in font M (+0x10, +0x28) and the Overdrive
    /// gauge (0x0021d1b8, +0x38 pixels); HP or MP "value/max"; or the member's statuses, one at a
    /// time for 21 frames each. Places from dum3 + dum4..dum8. The PC draws HD plates in HD
    /// coordinates; its row state (TOBtlCtrlStatusWin) is the PS2's.
    /// </summary>
    private static void status() {
        Ps2Menu.fn<Scissor>(Ps2Menu.SCISSOR)(0, 0, 0x200, 0x1a0);
        int mode = *(sbyte*)Native.ptr(STATUS_MODE);
        MainMenuPatches.ps2_scoped(() => {
            MainMenuPatches.battle_scoped(() => {
                for (int i = 0; i < 3; i++) {
                    int chr = *(ushort*)Native.ptr(STATUS_CHR + 2 * (uint)i);
                    if (chr == 0xff) continue;
                    status_row(i, chr, (nint)Native.ptr(STATUS_ROWS + 0x90 * (uint)i), mode);
                }
            });
            return 0;
        });
        first(4, "ui.menus: battle party list drawn the PS2 way");
    }

    private static (int x, int y) pt(uint va) => Ps2Menu.point(va);

    private static void status_row(int i, int chr, nint row, int mode) {
        int x = i16(DUM45_X), y = i16(DUM45_Y) + i * i16(DUM13), w = i16(DUM52);
        var n = pt(DUM10);
        var d3 = pt(DUM3);
        int bx = x + d3.x, by = y + d3.y;
        var l = pt(DUM4); var v = pt(DUM5); var l2 = pt(DUM6); var v2 = pt(DUM7); var g = pt(DUM8);
        (l.x, l.y, v.x, v.y, l2.x, l2.y, v2.x, v2.y, g.x, g.y) = (bx + l.x, by + l.y, bx + v.x, by + v.y, bx + l2.x, by + l2.y, bx + v2.x, by + v2.y, bx + g.x, by + g.y);
        int colour = *(byte*)(row + 0x19);
        bool targeted = chr < 32 && ((u32(TARGETS) >> chr) & 1) != 0;
        if (mode == 3 ? i16(WINDOW_CHR) == chr : mode is >= 0 and <= 2 && targeted) {
            if (mode == 3) highlight(x, y, w, u32(DUM2), u32(DUM1));
            else highlight(x, y, w, u32(DUM40), u32(DUM41));
        }
        if (mode is < 0 or > 3) return;
        nint name = fn<PtrOfInt>(CHR_NAME)(chr);
        Ps2Menu.font(name, x + n.x - Ps2Menu.text_width(name), y + n.y, colour);
        switch (mode) {
            case 3:
                Ps2Menu.shape(0xe9, l.x, l.y);
                Ps2Menu.value_m(*(int*)(row + 0x10), v.x, v.y, colour);
                Ps2Menu.shape(0xea, l2.x, l2.y);
                Ps2Menu.value_m(*(int*)(row + 0x28), v2.x, v2.y, *(byte*)(row + 0x31));
                gauge(*(int*)(row + 0x38), g.x, g.y);
                break;
            case 0:
            case 1:
                // (the PS2 leaves the max value's colour argument at 0x80: white, black shadow)
                Ps2Menu.shape(mode == 0 ? 0xe9 : 0xea, l.x, l.y);
                Ps2Menu.value_m(*(int*)(row + (mode == 0 ? 0x10 : 0x28)), v.x, v.y, colour);
                Ps2Menu.font_m("/", v.x + 8, v.y, 0);
                Ps2Menu.value_m(fn<IntOfInt>(mode == 0 ? MAX_HP : MAX_MP)(chr), v2.x, v2.y, 0x80);
                break;
            case 2: {
                short count = *(short*)(row + 0x40);
                if (count == 0) break;
                short t = *(short*)(row + 0x3c);
                *(short*)(row + 0x3c) = (short)(t - 1);
                if (t <= 0) {
                    *(short*)(row + 0x3c) = 20;
                    short k = (short)(*(short*)(row + 0x3e) + 1);
                    *(short*)(row + 0x3e) = k >= count ? (short)0 : k;
                }
                int id = *(ushort*)(row + 0x42 + 2 * *(short*)(row + 0x3e));
                Ps2Menu.font(fn<Text>(MENU_TEXT)(1, id, 1), l.x, v.y, 0);
                break;
            }
        }
    }

    /// <summary>
    /// A party row's highlight: battle shape 409 in c0 and 410 stretched to w, c0 -> c1 (PS2
    /// 0x001ff1e0), and GS lines (PS2 0x001f2d30) along its top (0x00808080 -> 0x80a0a0a0 over 50
    /// pixels, then 0x80a0a0a0) and its bottom (to 0x80000000), drawn as one-line boxes.
    /// </summary>
    private static void highlight(int x, int y, int w, uint c0, uint c1) {
        bshape(0x199, x, y, 0, 0, c0, c0);
        bshape(0x19a, x + 24, y, w - 24, 0, c0, c1);
        var box = Ps2Menu.fn<Ps2Menu.BoxC2>(Ps2Menu.BOX_C2);
        box(x, y, 50, 1, 0x00808080, 0x80a0a0a0);
        box(x + 50, y, w, 1, 0x80a0a0a0, 0x80a0a0a0);
        box(x, y + 19, 50, 1, 0x00808080, 0x80000000);
        box(x + 50, y + 19, w, 1, 0x80000000, 0x80000000);
    }

    /// <summary>
    /// The Overdrive gauge (PS2 0x0021d1b8(g, x, y), g in pixels): a black box dum8.w x dum8.h,
    /// then from (x + dum9.x - 1, y + dum9.y) solid orange when full, else a gradient
    /// (255, 248, 138) -> (227, 119, 23) running over the full width, cut at g.
    /// </summary>
    private static void gauge(int g, int x, int y) {
        Ps2Menu.box_rgba(x, y, i16(DUM8_W), i16(DUM8_H), 0, 0, 0, 0x80);
        int fx = x + i16(DUM9_X) - 1, fy = y + i16(DUM9_Y), fw = i16(DUM9_W), fh = i16(DUM9_H);
        if (fw == 0 || g == 0) return;
        if (g >= fw) Ps2Menu.box_rgba(fx, fy, fw, fh, 0xf7, 0x79, 0x2e, 0x80);
        else fn<BoxFW>(BOX_FW)(fx, fy, g, fh, 0x808af8ff, 0x801777e3, fw);
    }

    // ---- install -----------------------------------------------------------------------------

    private static bool hook<T>(DemasterModule m, uint va, T d) where T : Delegate {
        _keep.Add(d);
        return Native.fn<T>(va).hook(m, d);
    }

    public static bool install(DemasterModule m) {
        _log = m.log;
        return hook<Void>(m, DRAW_HELP, help)
            & hook<Ctb>(m, DRAW_CTB, ctb)
            & hook<Void>(m, DRAW_STATUS, status);
    }
}
