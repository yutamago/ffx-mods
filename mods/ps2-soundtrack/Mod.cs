// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Soundtrack;

/// <summary>
/// PS2 soundtrack (mod yuta.ps2-soundtrack): The original soundtrack rendered from your PS2 disc by the PS2 sound driver, and new games start with it (install.cmd).
/// The settings and their defaults are those of settings.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class SoundtrackModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("audio.music",      AppliedBy.Converter, "ps2"),
        new("audio.soundtrack", AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("audio.soundtrack", "hd", "original soundtrack for new games", SoundtrackPatches.soundtrack),
    ];

    public static readonly ModInfo INFO = new("ps2-soundtrack", "PS2 soundtrack", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
