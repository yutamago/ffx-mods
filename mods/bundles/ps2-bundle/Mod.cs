// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.Ps2Bundle;

/// <summary>
/// PS2 bundle (mod yuta.ps2-bundle): every PS2 mod of the family in one zip with one install
/// step. It changes nothing in the game itself and has no settings: its manifest lists the mods
/// of the bundle (Dependencies, from its csproj), and its section of the F8 window shows which
/// of them are on and the state of each.
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class Ps2BundleModule : FhModule {
    public static readonly ModInfo INFO = new("ps2-bundle", "PS2 bundle", [], [], Builds: false, Section: draw);

    private const string COMMON = "yuta.ffx-mods";
    private static string[] _members = [];
    private static string _mods_dir = "";
    private static HashSet<string> _off = [];

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        _members  = [.. mod_context.Manifest.Dependencies.Where(id => id != COMMON)];
        _mods_dir = mod_context.Paths.ModDir.Parent?.FullName ?? "";
        _off      = ModFiles.switched_off(_mods_dir);
        ModHost.start(this, _logger, mod_context, INFO);
        HashSet<string> loaded = [.. FhApi.Mods.get_mods().Select(m => m.Manifest.Id)];
        string[] off = [.. _members.Where(id => !loaded.Contains(id))];
        _logger.Info($"{_members.Length - off.Length} of the bundle's {_members.Length} mods are loaded");
        foreach (string id in off) _logger.Info($"  off: {id} ({why_off(id)})");
        return true;
    }

    /// <summary>
    /// Why a mod of the bundle is off: switched off (install.cmd -Disable, kept in
    /// mods\ffx-mods-off.txt), not in mods\loadorder (install.cmd not run since it was unpacked),
    /// or no folder.
    /// </summary>
    private static string why_off(string id) =>
        !File.Exists(Path.Combine(_mods_dir, id, $"{id}.manifest.json")) ? "not installed"
        : _off.Contains(id) ? $"switched off (install.cmd -Enable {id["yuta.".Length..]})"
        : "not in mods\\loadorder (run install.cmd)";

    /// <summary>The bundle's part of the F8 window: each of its mods and its state.</summary>
    private static void draw(ModHost bundle) {
        ModHost[] started = [.. FamilyWindow.mods];
        HashSet<string> loaded = [.. FhApi.Mods.get_mods().Select(m => m.Manifest.Id)];
        int on = 0;
        foreach (string id in _members) {
            ModHost? h = started.FirstOrDefault(x => $"yuta.{x.Info.Name}" == id);
            (Vector4 colour, string state) = h is not null ? FamilyWindow.state(h)
                : loaded.Contains(id) ? (new Vector4(1.0f, 0.4f, 0.4f, 1), "loaded, did not start (see its log)")
                : (new Vector4(0.6f, 0.6f, 0.6f, 1), $"off: {why_off(id)}");
            if (h is not null) on++;
            ImGui.TextColored(colour, $"{(h?.Info.Title ?? id),-22}");
            ImGui.SameLine();
            ImGui.TextUnformatted(state);
        }
        ImGui.PushTextWrapPos(0);
        ImGui.TextDisabled($"{on} of {_members.Length} on. Each mod has its own section in this window and its settings.toml in its folder. "
                         + "To switch a whole mod off: mods\\yuta.ps2-bundle\\install.cmd -Disable <mod> (it stays off; -Enable <mod> switches it on again).");
        ImGui.PopTextWrapPos();
    }
}
