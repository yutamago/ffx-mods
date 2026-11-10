// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Interface;

/// <summary>
/// ui.loading: a clean start-up, the code half (docs/mods/patches.md, "ui.loading"); the converter's
/// ui.loading job makes the loading screen's textures transparent.
/// </summary>
internal static class LoadingPatches {
    // ---- ui.loading (splash videos) ----------------------------------------------------------
    // graphicInitScene loops on PhyreScene_isOpeningScreenPlaying (0x00660400, its only caller)
    // while OpeningScreen.swf plays the Square Enix / Virtuos / CRI / Dolby logos, then deletes
    // the player and goes on loading. Answering "not playing" skips the loop; the rest of the
    // start-up (the loading wait, asyncFFXInitialize) runs as after the logos.
    private static Fn.RetByte? _opening_hook;

    // The window's backdrop: the PhyreFrameworkClass window class (registered at 0x00626cb0)
    // has hbrBackground COLOR_WINDOW + 1, the system's window colour (white), which fills the
    // window until the first frame is presented. The PS2 starts from black: the immediate of
    // `mov dword [ebp-0xf4], 6` (0x00626d48) becomes the stock black brush. Fahrenheit's stage 1
    // loads (and runs this) before FFX.exe's entry point, so before the class is registered.
    private const uint WINDOW_BRUSH = 0x00626d4e;
    private const int BLACK_BRUSH = 4;

    [System.Runtime.InteropServices.DllImport("gdi32.dll")]
    private static extern nint GetStockObject(int i);

    public static bool skip_splash(ModHost m, string value) {
        FhLogger log = m.log;
        bool logged = false;
        _opening_hook = () => {
            if (!logged) { logged = true; log.Info("ui.loading: start-up logos skipped"); }
            return 0;
        };
        nint black = GetStockObject(BLACK_BRUSH);
        bool ok = black != 0
            && Native.patch_code(WINDOW_BRUSH, BitConverter.GetBytes(6), BitConverter.GetBytes((uint)black), log);
        if (ok) log.Info("ui.loading: window backdrop black");
        return ok & Native.fn<Fn.RetByte>(0x00660400).hook(m, _opening_hook);
    }
}
