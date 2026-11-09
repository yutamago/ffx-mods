// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.ModernMinimap;

/// <summary>
/// Modern Minimap (mod yuta.modern-minimap): the field minimap round, always centred on the
/// player and turned with the camera, in the game's colours on a small CRT screen
/// (docs/minimap.md). The settings and their defaults are those of settings.default.toml in this
/// folder; the converter's list of this mod (converter/src/mods.rs) must agree (a converter
/// test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class ModernMinimapModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("minimap.style",  AppliedBy.Mod, "crt"),
        new("minimap.size",   AppliedBy.Mod, "medium"),
        new("minimap.corner", AppliedBy.Mod, "top_right"),
    ];

    // Size and corner have no "off": they only place the Modern Minimap.
    public static readonly CodePatch[] PATCHES = [
        new("minimap.size",   "", "minimap size",  MinimapPatches.size),
        new("minimap.corner", "", "minimap place", MinimapPatches.corner),
        new("minimap.style",  "game", "the Modern Minimap in place of the game's", MinimapPatches.style),
    ];

    public static readonly ModInfo INFO = new("modern-minimap", "Modern Minimap", SETTINGS, PATCHES, Builds: false);

    private bool _draw_failed, _drawn;

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }

    public override void render_imgui() {
        // Fahrenheit calls this without a try/catch, so an exception would end the game.
        if (_draw_failed || !MinimapPatches.active) return;
        try {
            Vector2 screen = FhApi.Gui.display_size;
            if (MinimapView.draw(ImGui.GetBackgroundDrawList(), screen) && !_drawn) {
                _drawn = true;
                _logger.Info($"minimap: first drawn, screen {screen.X} x {screen.Y}");
            }
        }
        catch (Exception e) {
            _draw_failed = true;
            _logger.Error($"Drawing the minimap failed and is now off: {e}");
        }
    }
}
