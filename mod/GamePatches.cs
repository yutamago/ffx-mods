// SPDX-License-Identifier: MIT

namespace Yuta.Demaster;

/// <summary>
/// The code patches of docs/patches.md. Each Apply function returns whether everything it needs
/// is in place; a byte patch only goes in when the bytes are those of the FFX.exe it was written
/// for (ffx.sym's sha256).
/// </summary>
internal static class GamePatches {
    // ---- ui.battle_menu_delay ----------------------------------------------------------------
    // TOWaitBtlWin* (0x008a87a0..0x008a8bd0): each battle window's per-frame input step reads the
    // pad only once the window has been open (f32 w+0xe0) 0.3 s (f32 0x00b3eb44) or 0.45 s
    // (f64 0x00b43d50). The constants are shared with other code, so the jumps that skip the pad
    // read go instead.
    private static readonly (uint Va, byte[] Original, byte[] Patched, string What)[] MENU_DELAY = [
        (0x008a87b7, [0x74, 0x09], [0x90, 0x90], "TOWaitBtlWinChange 0.3 s"),
        (0x008a8a57, [0x7b, 0x09], [0x90, 0x90], "TOWaitBtlWinLeft 0.45 s"),
        (0x008a8aa1, [0x7b, 0x09], [0x90, 0x90], "TOWaitBtlWinMain 0.45 s"),
        (0x008a8ad7, [0x74, 0x09], [0x90, 0x90], "TOWaitBtlWinPrepare 0.3 s"),
        (0x008a8b07, [0x7b, 0x09], [0x90, 0x90], "TOWaitBtlWinRight 0.45 s"),
        (0x008a8b37, [0x74, 0x09], [0x90, 0x90], "TOWaitBtlWinStd 0.3 s"),
        (0x008a8b67, [0x7b, 0x1d], [0x90, 0x90], "TOWaitBtlWinStdMode2 0.45 s"),
        (0x008a8ba7, [0x7a, 0x04], [0xeb, 0x04], "TOWaitBtlWinStdMode3 0.45 s"),
    ];

    public static bool battle_menu_delay(DemasterModule m, string value) {
        // Check every site first, so the game never runs with half of them patched.
        foreach (var p in MENU_DELAY) {
            unsafe {
                ReadOnlySpan<byte> now = new(Native.ptr(p.Va), p.Original.Length);
                if (!now.SequenceEqual(p.Original) && !now.SequenceEqual(p.Patched)) {
                    m.log.Warning($"ui.battle_menu_delay: {p.What} at 0x{p.Va:x8} is {Convert.ToHexString(now)}, expected {Convert.ToHexString(p.Original)}");
                    return false;
                }
            }
        }
        return MENU_DELAY.All(p => Native.patch_code(p.Va, p.Original, p.Patched, m.log));
    }

    // ---- ui.battle_subtitles -----------------------------------------------------------------
    // Battle scripts ask native 0x70de (0x007a79b0 -> 0x00785210, MsGetSaveConfigSubTitle) before
    // each voiced line and draw the subtitle window (0x7072, TODrawAMES_EVJIMAKU) when it says 1.
    // It answers 0 while g_BtlMuteSubtitle (0x01133354, the PC's speed-up) is set, else bit 9 of
    // g_SaveMenu (0x0113079c), which comes from the save (the PC's new-game and PC saves measured
    // have it set; nothing on PC can change it). The load of g_SaveMenu becomes mov eax, 0x200, so
    // a save with the bit clear gets the subtitles too. Row 17's missing "battle quotes" are
    // something else (docs/patches.md).
    // The game is relocated (its absolute operands move with the load address), so the expected
    // operand is g_SaveMenu's address in this process.
    public static bool battle_subtitles(DemasterModule m, string value) {
        byte[] original = [0xa1, .. BitConverter.GetBytes(Native.reloc(0x0113079c))];
        return Native.patch_code(0x0078521c, original, [0xb8, 0x00, 0x02, 0x00, 0x00], m.log);
    }

    // ---- fixes.farplane_video ----------------------------------------------------------------
    // Native 0xb00a (MovieStartPicture, handler 0x0076eb80; only ikai0800's Farplane loop, movie
    // 104 FAL) plays the movie only when graphicInitFMVPlayer returns nonzero, but
    // PhyFMVPlayerManager_initialize always returns 0. The je that skips graphicPlayVideo and the
    // picture flags goes.
    public static bool farplane_video(DemasterModule m, string value) =>
        Native.patch_code(0x0076ebb4, [0x74, 0x28], [0x90, 0x90], m.log);

    // ---- timing.run_speed --------------------------------------------------------------------
    // Sg_PlayerInitDefault (0x0082d8d0, run at every event jump) sets the player's walk, run and
    // swim speeds: Sg_PlayerSetSpeed(2, f32 [0x00b59830] = 38.0). The PS2 disc's has 34.0
    // (SLPM_675.13 0x001164d0: lui 0x4208), and the PC's own scripts still restore 34 after
    // their special runs (Bsil0100, Mihn*, Nagi0000, ...). The fld at 0x0082d8e6 reads the .rdata
    // 34.0 at 0x00b656c4 instead (the only 34.0 there; .rdata is read-only, so it stays 34).
    // docs/timing.md, "Run speed".
    public static bool run_speed(DemasterModule m, string value) {
        unsafe {
            if (*(float*)Native.ptr(0x00b656c4) != 34.0f || *(float*)Native.ptr(0x00b59830) != 38.0f) {
                m.log.Warning("timing.run_speed: the speed constants are not 34.0/38.0; not patched");
                return false;
            }
        }
        byte[] original = BitConverter.GetBytes(Native.reloc(0x00b59830));
        byte[] patched  = BitConverter.GetBytes(Native.reloc(0x00b656c4));
        return Native.patch_code(0x0082d8e8, original, patched, m.log);
    }

    // ---- ui.loading (splash videos) ----------------------------------------------------------
    // graphicInitScene loops on PhyreScene_isOpeningScreenPlaying (0x00660400, its only caller)
    // while OpeningScreen.swf plays the Square Enix / Virtuos / CRI / Dolby logos, then deletes
    // the player and goes on loading. Answering "not playing" skips the loop; the rest of the
    // start-up (the loading wait, asyncFFXInitialize) runs as after the logos.
    private static Fn.RetByte? _opening_hook;

    public static bool skip_splash(DemasterModule m, string value) {
        FhLogger log = m.log;
        bool logged = false;
        _opening_hook = () => {
            if (!logged) { logged = true; log.Info("ui.loading: start-up logos skipped"); }
            return 0;
        };
        return Native.fn<Fn.RetByte>(0x00660400).hook(m, _opening_hook);
    }

    // ---- audio.soundtrack ----------------------------------------------------------------------
    // The soundtrack choice lives in the save: byte g_SaveData (0x0112ca90) +0x5ec, 0 arranged,
    // 1 original (FmodMusic set 1 = Music_PS2/ffx_music_PS2.fev). MsSetSaveStartGame (0x00786b00)
    // builds the new-game save (from MsBattleInit at boot); the mod sets the byte to 1 there and
    // queues iMusicRefreshBgmType (0x008cc170). Loaded saves keep the choice they were saved with,
    // and the in-game menu (MenuConfig_SetBgmType) still changes it: apply once, don't force.
    private const uint SAVE_DATA = 0x0112ca90;
    private const uint FMOD_MANAGER = 0x00ce9010;
    private static Fn.Void? _start_game_hook;
    private static Fn.Void? _start_game_next;

    public static bool soundtrack(DemasterModule m, string value) {
        _start_game_hook = () => {
            _start_game_next ??= Native.fn<Fn.Void>(0x00786b00).chain_from(_start_game_hook!).fnptr!;
            _start_game_next();
            Native.set_u8(SAVE_DATA + 0x5ec, 1);
            // FMOD's queue needs its manager; at boot iSoundInit has made it before MsBattleInit.
            bool fmod = Native.u32(FMOD_MANAGER) != 0;
            if (fmod) Native.fn<Fn.Void>(0x008cc170).fnptr!();
            m.log.Info($"audio.soundtrack: new-game save set to the original soundtrack{(fmod ? "" : " (FMOD not up yet: not switched now)")}"
                     + $"; g_SaveMenu 0x{Native.u32(0x0113079c):x8} (bit 9 = the PS2 subtitle option)");
        };
        return Native.fn<Fn.Void>(0x00786b00).hook(m, _start_game_hook);
    }

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

    public static bool lulu_fury(DemasterModule m, string value) {
        _log = m.log;
        _lulu_hook = lulu_step;
        return Native.fn<Fn.Void>(0x00891b80).hook(m, _lulu_hook);
    }
}
