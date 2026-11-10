// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Effects;

/// <summary>
/// PS2 effects (mod yuta.ps2-effects): The PS2 textures of spells, summons and the other effects, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class EffectsModule : FhModule {
    public static readonly CodePatch[] PATCHES = [];

    public static readonly ModInfo INFO = new("ps2-effects", "PS2 effects", SettingInfo.defaults_of(typeof(EffectsModule)), PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
