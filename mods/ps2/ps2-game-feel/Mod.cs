// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.GameFeel;

/// <summary>
/// PS2 game feel (mod yuta.ps2-game-feel): The PS2 run speed, a battle menu without input delays and battle dialogue subtitles.
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (demaster/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class GameFeelModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("timing.run_speed",     AppliedBy.Mod,       "ps2"),
        new("ui.battle_menu_delay", AppliedBy.Mod,       "ps2"),
        new("ui.battle_subtitles",  AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("timing.run_speed",     "hd", "PS2 run speed (34)", GameFeelPatches.run_speed),
        new("ui.battle_menu_delay", "hd", "battle menu without input delays", GameFeelPatches.battle_menu_delay),
        new("ui.battle_subtitles",  "hd", "battle dialogue subtitles", GameFeelPatches.battle_subtitles),
    ];

    public static readonly ModInfo INFO = new("ps2-game-feel", "PS2 game feel", SETTINGS, PATCHES, Builds: false);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
