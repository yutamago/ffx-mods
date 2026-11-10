// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Movies;

/// <summary>
/// PS2 movies (mod yuta.ps2-movies): The PS2 movies (FMVs) in their 4:3 picture with the PS2 sound, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class MoviesModule : FhModule {
    public static readonly CodePatch[] PATCHES = [];

    public static readonly ModInfo INFO = new("ps2-movies", "PS2 movies", SettingInfo.defaults_of(typeof(MoviesModule)), PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
