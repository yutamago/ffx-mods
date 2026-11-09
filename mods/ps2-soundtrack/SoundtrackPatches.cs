// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Soundtrack;

/// <summary>audio.soundtrack: new games start with the original soundtrack (docs/patches.md).</summary>
internal static class SoundtrackPatches {
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

    public static bool soundtrack(ModHost m, string value) {
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

}
