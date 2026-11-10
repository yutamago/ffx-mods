// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.GameFeel;

/// <summary>
/// The code patches of PS2 game feel (docs/mods/patches.md). Each Apply function returns whether
/// everything it needs is in place; a byte patch only goes in when the bytes are those of the
/// FFX.exe it was written for (ffx.sym's sha256).
/// </summary>
internal static class GameFeelPatches {
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

    public static bool battle_menu_delay(ModHost m, string value) {
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
    // something else (docs/mods/patches.md).
    // The game is relocated (its absolute operands move with the load address), so the expected
    // operand is g_SaveMenu's address in this process.
    public static bool battle_subtitles(ModHost m, string value) {
        byte[] original = [0xa1, .. BitConverter.GetBytes(Native.reloc(0x0113079c))];
        return Native.patch_code(0x0078521c, original, [0xb8, 0x00, 0x02, 0x00, 0x00], m.log);
    }

    // ---- timing.run_speed --------------------------------------------------------------------
    // Sg_PlayerInitDefault (0x0082d8d0, run at every event jump) sets the player's walk, run and
    // swim speeds: Sg_PlayerSetSpeed(2, f32 [0x00b59830] = 38.0). The PS2 disc's has 34.0
    // (SLPM_675.13 0x001164d0: lui 0x4208), and the PC's own scripts still restore 34 after
    // their special runs (Bsil0100, Mihn*, Nagi0000, ...). The fld at 0x0082d8e6 reads the .rdata
    // 34.0 at 0x00b656c4 instead (the only 34.0 there; .rdata is read-only, so it stays 34).
    // docs/mods/timing.md, "Run speed".
    public static bool run_speed(ModHost m, string value) {
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
}
