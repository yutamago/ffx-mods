// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Lighting;

/// <summary>
/// PS2 lighting (mod yuta.ps2-lighting): Characters lit the PS2 way: its light levels, falloff and point lights. The event lights need your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class LightingModule : FhModule {
    public static readonly CodePatch[] PATCHES = [
        new("render.lighting", "hd", "PS2-like character lighting", LightingPatches.lighting),
    ];

    public static readonly ModInfo INFO = new("ps2-lighting", "PS2 lighting", SettingInfo.defaults_of(typeof(LightingModule)), PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
