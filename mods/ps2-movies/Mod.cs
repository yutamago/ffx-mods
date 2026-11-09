// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Movies;

/// <summary>
/// PS2 movies (mod yuta.ps2-movies): The PS2 movies (FMVs) in their 4:3 picture with the PS2 sound, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class MoviesModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("movies", AppliedBy.Converter, "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [];

    public static readonly ModInfo INFO = new("ps2-movies", "PS2 movies", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
