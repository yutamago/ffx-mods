// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.MusicEngine;

/// <summary>
/// PS2 music engine (mod yuta.ps2-music-engine): The PS2 sound driver plays the original soundtrack inside the game from your PS2 disc. Needs the PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class MusicEngineModule : FhModule {
    public static readonly CodePatch[] PATCHES = [
        new("audio.music_engine", "hd", "PS2 music played live by the game's commands", LiveMusic.install, Only: "live"),
    ];

    public static readonly ModInfo INFO = new("ps2-music-engine", "PS2 music engine", SettingInfo.defaults_of(typeof(MusicEngineModule)), PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
