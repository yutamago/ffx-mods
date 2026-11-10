// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Characters;

/// <summary>
/// PS2 characters (mod yuta.ps2-characters): PS2 models and textures for the party, NPCs, monsters, aeons, weapons and objects, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (demaster/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class CharactersModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("models.party",           AppliedBy.Converter, "ps2"),
        new("models.npc",             AppliedBy.Converter, "ps2"),
        new("models.monster",         AppliedBy.Converter, "ps2"),
        new("models.aeon",            AppliedBy.Converter, "ps2"),
        new("models.weapon",          AppliedBy.Converter, "ps2"),
        new("models.object",          AppliedBy.Converter, "ps2"),
        new("render.texture_filter",  AppliedBy.Mod,       "game"),
        new("render.texture_quality", AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("render.texture_filter",  "game", "nearest-neighbour texture filtering", TexturePatches.texture_filter),
        new("render.texture_quality", "hd", "PS2 textures at full size at every texture quality", TexturePatches.texture_quality),
    ];

    public static readonly ModInfo INFO = new("ps2-characters", "PS2 characters", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
