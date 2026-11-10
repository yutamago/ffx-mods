// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.PostEffects;

/// <summary>
/// No HD post-effects (mod yuta.no-hd-post-effects): No HD glow and no shadow maps: the picture without the HD bloom and with the PS2 ground shadows.
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (demaster/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class PostEffectsModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("render.glow",    AppliedBy.Mod,       "ps2"),
        new("render.shadows", AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("render.glow", "hd", "glow/blow-out pass off", PostEffectPatches.glow),
        new("render.shadows", "hd", "ground shadows only, no shadow maps", PostEffectPatches.shadows),
    ];

    public static readonly ModInfo INFO = new("no-hd-post-effects", "No HD post-effects", SETTINGS, PATCHES, Builds: false);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
