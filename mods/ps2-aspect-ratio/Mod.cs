// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.AspectRatio;

/// <summary>
/// PS2 aspect ratio (mod yuta.ps2-aspect-ratio): The PS2 picture: 4:3 with black bars and the PS2 projection.
/// The settings and their defaults are those of settings.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class AspectRatioModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("render.aspect", AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("render.aspect", "hd", "4:3 with the PS2 projection", AspectPatches.aspect),
    ];

    public static readonly ModInfo INFO = new("ps2-aspect-ratio", "PS2 aspect ratio", SETTINGS, PATCHES, Builds: false);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
