// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;

namespace Yuta.FfxMods.MusicEngine;

/// <summary>
/// audio.music_engine = "live": loads demaster_live.dll (next to this mod's DLL) and installs it.
/// The DLL wraps FFX.exe's FMOD event imports; while the "Original" soundtrack is chosen, every
/// music event's start, stop, pause and volume goes to the PS2 sound driver (built from the
/// disc files demaster.exe put into the mod's music folder) instead of the bank, and its output
/// goes into FMOD as one user stream (docs/music.md, "Phase 2"). Its own log: music/live.log.
/// Anything missing leaves the game on the rendered bank.
/// </summary>
internal static class LiveMusic {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern nint LoadLibraryW(string path);

    [DllImport("kernel32.dll", CharSet = CharSet.Ansi)]
    private static extern nint GetProcAddress(nint module, string name);

    [UnmanagedFunctionPointer(CallingConvention.StdCall, CharSet = CharSet.Unicode)]
    private delegate uint Install([MarshalAs(UnmanagedType.LPWStr)] string mod_dir);

    public static bool install(ModHost m, string value) {
        string dir = m.mod_dir;
        string dll = Path.Combine(dir, "demaster_live.dll");
        if (!File.Exists(dll)) {
            m.log.Warning($"audio.music_engine = live: {dll} is missing; the rendered bank plays");
            return false;
        }
        nint h = LoadLibraryW(dll);
        if (h == 0) {
            m.log.Warning($"audio.music_engine = live: could not load {dll} ({Marshal.GetLastWin32Error()})");
            return false;
        }
        nint f = GetProcAddress(h, "demaster_live_install");
        if (f == 0) {
            m.log.Warning("audio.music_engine = live: demaster_live_install not found");
            return false;
        }
        uint r = Marshal.GetDelegateForFunctionPointer<Install>(f)(dir);
        string what = r switch {
            0 => "installed",
            1 => "the music files are missing (run demaster.exe build); the rendered bank plays",
            2 => "FFX.exe is not the expected build; the rendered bank plays",
            _ => $"failed ({r}); the rendered bank plays",
        };
        m.log.Info($"audio.music_engine = live: {what} (log: {Path.Combine(dir, "music", "live.log")})");
        return r == 0;
    }
}
