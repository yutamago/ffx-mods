// SPDX-License-Identifier: MIT

using System.Reflection;

namespace Yuta.FfxMods;

/// <summary>
/// The shared library's own module (mod yuta.ffx-mods): every mod of the family depends on it
/// and finds this one copy of it (Fahrenheit loads a mod DLL once and shares it with the mods
/// that reference it). It draws the family's F8 window.
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class FfxModsModule : FhModule {
    private string _version = "";
    private bool _ui_failed;

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        _version = typeof(FfxModsModule).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "?";
        _logger.Info($"FFX mods shared library {_version}, {mod_context.Paths.ModDir.FullName}");
        foreach (FhModContext m in FhApi.Mods.get_mods())
            if (ModFiles.is_family(m)) _logger.Info($"  {m.Manifest.Id} {m.Manifest.Version}");
        return true;
    }

    public override void render_imgui() {
        // Fahrenheit calls this without a try/catch, so an exception would end the game.
        if (_ui_failed) return;
        try {
            FamilyWindow.draw(_version, _logger);
        }
        catch (Exception e) {
            _ui_failed = true;
            _logger.Error($"The window failed and is now off: {e}");
        }
    }
}
