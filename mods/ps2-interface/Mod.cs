// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Interface;

/// <summary>
/// PS2 interface (mod yuta.ps2-interface): The PS2 menus, windows, font, portraits, title screen and button glyphs, and a clean start-up, built from your PS2 disc (install.cmd).
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (converter/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class InterfaceModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("ui.menus",         AppliedBy.Both,      "ps2"),
        new("ui.portraits",     AppliedBy.Converter, "ps2"),
        new("ui.font",          AppliedBy.Converter, "ps2"),
        new("ui.title",         AppliedBy.Both,      "ps2"),
        new("ui.loading",       AppliedBy.Both,      "ps2"),
        new("ui.button_glyphs", AppliedBy.Converter, "ps2"),
        new("ui.boot_logo",     AppliedBy.Converter, "hd"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("ui.menus",   "hd", "PS2 message, battle command windows, main menu, items and abilities", MenuPatches.menus),
        new("ui.title",   "hd", "title menu cursor, no water band", TitlePatches.title),
        new("ui.loading", "hd", "splash videos skipped, black window at start-up", LoadingPatches.skip_splash),
    ];

    public static readonly ModInfo INFO = new("ps2-interface", "PS2 interface", SETTINGS, PATCHES, Builds: true);

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }
}
