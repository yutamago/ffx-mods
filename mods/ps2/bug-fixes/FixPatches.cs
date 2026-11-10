// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.BugFixes;

/// <summary>
/// The bug fixes (docs/mods/patches.md). Each Apply function returns whether everything it needs is
/// in place; a byte patch only goes in when the bytes are those of the FFX.exe it was written for
/// (ffx.sym's sha256).
/// </summary>
internal static class FixPatches {
    // ---- fixes.farplane_video ----------------------------------------------------------------
    // Native 0xb00a (MovieStartPicture, handler 0x0076eb80; only ikai0800's Farplane loop, movie
    // 104 FAL) plays the movie only when graphicInitFMVPlayer returns nonzero, but
    // PhyFMVPlayerManager_initialize always returns 0. The je that skips graphicPlayVideo and the
    // picture flags goes.
    public static bool farplane_video(ModHost m, string value) =>
        Native.patch_code(0x0076ebb4, [0x74, 0x28], [0x90, 0x90], m.log);

    // ---- fixes.npc_mouths --------------------------------------------------------------------
    // Ch_Mouth (0x008344f0) sets the mouth texture animation (the type-1 slots of 208 NPC models,
    // two frames: closed, open) with TexAnim_PosSet(i, 1, v). The slot shows its list's frame 0
    // when v != 0 and frame 1 when v == 0, and starts with v = 1 (closed). The PS2
    // (SLPM_675.13 0x0010a030) passes v = 0 while the jaw angle is above g_ChrMouthTh and 1
    // otherwise; the PC (0x0083470d) passes the opposite, so after a line the NPC keeps the open
    // frame until the map reloads. The jle that picks push 0 becomes jg. docs/mods/patches.md.
    public static bool npc_mouths(ModHost m, string value) =>
        Native.patch_code(0x00834714, [0x7e, 0x04], [0x7f, 0x04], m.log);

    // ---- fixes.lulu_fury -------------------------------------------------------------------------
    // TOBtlCtrlLuluLimitWindow (0x00891b80), state 4 (i16 0x0133d708) with the stick
    // (u32 0x00c58ef0 != 0): each call reads the right stick twice (TkGetControlerAnalogHist
    // 0x00888c20, hist -1 and 0), turns each reading into a diagonal code 4..7 and appends a code
    // that differs from the last one to the buffer at 0x0133d721 (count i8 0x0133d731). With 5
    // codes it compares them with a full circle either way and empties the buffer. The PC samples
    // the stick about once per frame, so a fast spin skips a diagonal (4 -> 6) and the whole
    // circle fails. The fix fills in the skipped diagonal in the spin's direction, before the
    // original appends (first new code) or right after (second new code). Spins still have to be
    // real; nothing is counted that the stick didn't go through.
    private const uint LULU_STATE = 0x0133d708, LULU_BUF = 0x0133d721, LULU_COUNT = 0x0133d731, STICK_MODE = 0x00c58ef0;
    private static Fn.Void? _lulu_hook;
    private static Fn.Void? _lulu_next;
    private static Fn.AnalogHist? _analog;
    private static int _lulu_last = -1;   // the last code appended, kept across the buffer's clears
    private static int _lulu_dir;         // +1 / -1 once two neighbouring codes were seen, else 0
    public static int LuluFilled;         // diagonals filled in, for the log
    private static FhLogger? _log;

    private static int zone(byte x, byte y) =>
        x >= 0xa0 ? (y < 0x60 ? 4 : y >= 0xa0 ? 5 : -1)
      : x <  0x60 ? (y >= 0xa0 ? 6 : y < 0x60 ? 7 : -1)
      : -1;

    /// <summary>Steps from a to b around 4-5-6-7: 1, 2 (a skipped diagonal) or 3 (= -1).</summary>
    private static int step(int a, int b) => ((b - a) % 4 + 4) % 4;

    private static int between(int a, int dir) => (a - 4 + dir + 4) % 4 + 4;

    private static void note_step(int a, int b) {
        int s = step(a, b);
        if (s == 1) _lulu_dir = 1;
        else if (s == 3) _lulu_dir = -1;
    }

    private static void append(int code) {
        int n = Native.i8(LULU_COUNT);
        if (n < 0 || n >= 15) return;
        Native.set_u8(LULU_BUF + (uint)n, (byte)code);
        Native.set_u8(LULU_COUNT, (byte)(n + 1));
        LuluFilled++;
    }

    private static int last_code() {
        int n = Native.i8(LULU_COUNT);
        return n > 0 ? Native.u8(LULU_BUF + (uint)(n - 1)) : _lulu_last;
    }

    private static void lulu_step() {
        _lulu_next ??= Native.fn<Fn.Void>(0x00891b80).chain_from(_lulu_hook!).fnptr!;
        if (Native.i16(LULU_STATE) != 4 || Native.u32(STICK_MODE) == 0) {
            if (LuluFilled > 0) {
                _log?.Info($"fixes.lulu_fury: filled in {LuluFilled} skipped diagonal(s); level {Native.u8(0x0133d71f)}, count {Native.u8(0x0133d71e)}");
                LuluFilled = 0;
            }
            _lulu_last = -1;
            _lulu_dir = 0;
            _lulu_next();
            return;
        }
        // The same readings the original takes this call (a pure read of the pad history).
        // Fahrenheit hands out call pointers only after every module's init.
        _analog ??= Native.fn<Fn.AnalogHist>(0x00888c20).fnptr!;
        int c1 = zone(_analog!(0, 0, 0, -1), _analog!(0, 0, 1, -1));
        int c2 = zone(_analog!(0, 0, 0, 0), _analog!(0, 0, 1, 0));
        int last = last_code();
        // The codes the original will append, in order.
        int first = c1 >= 4 && c1 != last ? c1 : -1;
        int prev = first >= 4 ? first : last;
        int second = c2 >= 4 && c2 != prev ? c2 : -1;

        int a = last;
        foreach (int c in new[] { first, second }) {
            if (c < 4) continue;
            if (a >= 4 && step(a, c) == 2 && _lulu_dir != 0) {
                int mid = between(a, _lulu_dir);
                if (c == first) append(mid);   // before the original appends it
                else { _pending_mid = mid; _pending_after = c; }
            }
            else if (a >= 4) note_step(a, c);
            a = c;
        }

        _lulu_next();

        if (_pending_mid >= 4) {
            // The second code is already in; slip the skipped diagonal in front of it, unless the
            // original has just emptied the buffer (then that circle is lost, as before).
            int n = Native.i8(LULU_COUNT);
            if (n >= 1 && n < 15 && Native.u8(LULU_BUF + (uint)(n - 1)) == _pending_after) {
                Native.set_u8(LULU_BUF + (uint)(n - 1), (byte)_pending_mid);
                Native.set_u8(LULU_BUF + (uint)n, (byte)_pending_after);
                Native.set_u8(LULU_COUNT, (byte)(n + 1));
                LuluFilled++;
            }
            _pending_mid = _pending_after = -1;
        }
        if (a >= 4) _lulu_last = a;
    }

    private static int _pending_mid = -1, _pending_after = -1;

    public static bool lulu_fury(ModHost m, string value) {
        _log = m.log;
        _lulu_hook = lulu_step;
        return Native.fn<Fn.Void>(0x00891b80).hook(m, _lulu_hook);
    }
}
