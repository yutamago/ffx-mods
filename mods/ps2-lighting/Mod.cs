// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Lighting;

/// <summary>
/// PS2 lighting (mod yuta.ps2-lighting): Characters lit the PS2 way: its light levels, falloff and point lights. The event lights need your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class LightingModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("render.lighting", AppliedBy.Both,      "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("render.lighting", "hd", "PS2-like character lighting", LightingPatches.lighting),
    ];

    public static readonly ModInfo INFO = new("ps2-lighting", "PS2 lighting", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
