// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.BugFixes;

/// <summary>
/// Bug fixes (mod yuta.bug-fixes): Fixes of the PC version: NPC mouths that stay open, the Farplane video that never plays, Lulu's Fury spins that are not counted.
/// The settings and their defaults are those of settings.default.toml in this folder (embedded in the DLL),
/// the one list of them (the converter reads the same file).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class BugFixesModule : FhModule {
    public static readonly CodePatch[] PATCHES = [
        new("fixes.npc_mouths",     "false", "NPC mouth textures close after talking", FixPatches.npc_mouths),
        new("fixes.farplane_video", "false", "Farplane background video plays", FixPatches.farplane_video),
        new("fixes.lulu_fury",      "false", "Lulu's Fury spins register", FixPatches.lulu_fury),
    ];

    public static readonly ModInfo INFO = new("bug-fixes", "Bug fixes", SettingInfo.defaults_of(typeof(BugFixesModule)), PATCHES, Builds: false);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
