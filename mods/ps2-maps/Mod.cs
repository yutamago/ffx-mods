// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Maps;

/// <summary>
/// PS2 maps (mod yuta.ps2-maps): The PS2 textures of the field and battle maps and the PS2 painted backgrounds, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class MapsModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("maps.field",       AppliedBy.Converter, "ps2"),
        new("maps.battle",      AppliedBy.Converter, "ps2"),
        new("maps.backgrounds", AppliedBy.Converter, "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [];

    public static readonly ModInfo INFO = new("ps2-maps", "PS2 maps", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
