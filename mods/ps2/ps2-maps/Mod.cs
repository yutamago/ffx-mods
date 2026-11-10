// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Maps;

/// <summary>
/// PS2 maps (mod yuta.ps2-maps): The PS2 textures of the field and battle maps and the PS2 painted backgrounds, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class MapsModule : FhModule {
    public static readonly CodePatch[] PATCHES = [];

    public static readonly ModInfo INFO = new("ps2-maps", "PS2 maps", SettingInfo.defaults_of(typeof(MapsModule)), PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
