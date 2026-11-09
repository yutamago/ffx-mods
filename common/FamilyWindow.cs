// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Threading;

namespace Yuta.FfxMods;

/// <summary>
/// The one F8 window of the family: a section per installed mod with its build status (whether
/// its files match its settings.toml, and how to rebuild them) and its code patches. Drawn by
/// <see cref="FfxModsModule"/>, the module of this shared library, so however many mods are
/// installed there is one window and one key. It opens by itself when a mod's files don't match
/// its settings, a patch failed or a settings.toml has errors (not for a settings.toml that
/// install.cmd has not made yet: the defaults apply).
/// </summary>
public static class FamilyWindow {
    public const ImGuiKey TOGGLE_KEY = ImGuiKey.F8;

    private static readonly Vector4 GREEN  = new(0.55f, 0.9f, 0.55f, 1);
    private static readonly Vector4 YELLOW = new(1.0f, 0.85f, 0.3f, 1);
    private static readonly Vector4 RED    = new(1.0f, 0.4f, 0.4f, 1);
    private static readonly Vector4 GREY   = new(0.6f, 0.6f, 0.6f, 1);

    private static readonly List<ModHost> _mods = [];
    private static readonly Lock _lock = new();
    private static bool _open;
    /// <summary>The mod whose section <see cref="toggle"/> asked to open and show.</summary>
    private static ModHost? _show;
    private static readonly HashSet<string> _failed_sections = [];

    /// <summary>The mods that started, in load order.</summary>
    public static IReadOnlyList<ModHost> mods { get { lock (_lock) return [.. _mods]; } }

    internal static void register(ModHost h) {
        lock (_lock) _mods.Add(h);
        if (problem(h)) _open = true;
    }

    /// <summary>
    /// Opens the window with <paramref name="h"/>'s section open and in view, or closes the window
    /// when it is open. Render thread. (No mod binds a key to it: F8 is the family's only window key.)
    /// </summary>
    public static void toggle(ModHost h) {
        if (_open) {
            _open = false;
            return;
        }
        _open = true;
        _show = h;
    }

    private static bool problem(ModHost h) =>
        (h.Status is { } s && s.State != BuildState.Ok)
        || h.Patches.Any(p => p.State == PatchState.Failed)
        || h.Settings.Problems.Count > 0;

    /// <summary>A mod's state as its header in the window shows it: "built", "rebuild needed", "on", ...</summary>
    public static (Vector4 Colour, string State) state(ModHost h) => h.Status?.State switch {
        null when h.Patches.Any(p => p.State == PatchState.Failed) => (RED, "a patch failed"),
        null                  => (GREEN, "on"),
        BuildState.Ok         => (GREEN, "built"),
        BuildState.Incomplete => (YELLOW, "build incomplete"),
        BuildState.NotBuilt   => (RED, "not built"),
        _                     => (RED, "rebuild needed"),
    };

    private static void text(string s) => ImGui.TextUnformatted(s);
    private static void coloured(Vector4 c, string s) => ImGui.TextColored(c, s.Replace("%", "%%"));
    private static void wrapped(string s) => ImGui.TextWrapped(s.Replace("%", "%%"));

    internal static void draw(string version, FhLogger log) {
        if (ImGui.IsKeyPressed(TOGGLE_KEY, false)) _open = !_open;
        if (!_open) return;
        ModHost[] mods = [.. FamilyWindow.mods];

        ImGui.SetNextWindowSize(new Vector2(600, 0), ImGuiCond.FirstUseEver);
        if (ImGui.Begin($"FFX mods {version.Split('+')[0]} (F8)", ref _open)) {
            if (mods.Length == 0) wrapped("No mod of the family has started (see the game's logs).");
            foreach (ModHost h in mods) {
                (Vector4 c, string state) = FamilyWindow.state(h);
                ImGui.PushID(h.Info.Name);
                bool show = _show == h;
                if (show) ImGui.SetNextItemOpen(true);
                bool open = ImGui.CollapsingHeader($"{h.Info.Title}  ({state})###{h.Info.Name}", problem(h) ? ImGuiTreeNodeFlags.DefaultOpen : 0);
                if (show) {
                    ImGui.SetScrollHereY(0);
                    _show = null;
                }
                if (open) draw_mod(h, c, log);
                ImGui.PopID();
            }
            ImGui.PushTextWrapPos(0);
            ImGui.TextDisabled("Settings: settings.toml in each mod's folder. Changes need install.cmd (mods with PS2 files) and a restart.");
            ImGui.PopTextWrapPos();
        }
        ImGui.End();
    }

    private static void draw_mod(ModHost h, Vector4 colour, FhLogger log) {
        ImGui.Indent();
        ImGui.TextDisabled($"{h.Info.Name} {h.Version.Split('+')[0]}, {h.mod_dir}");
        if (h.Status is { } s) {
            ImGui.PushTextWrapPos(0);
            coloured(colour, s.Summary);
            ImGui.PopTextWrapPos();
            if (s.BuiltAt is DateTime at)
                ImGui.TextDisabled($"built {at:yyyy-MM-dd HH:mm} by converter {s.ConverterVersion}, {s.FilesFound} files");
            foreach (var c in s.Changed) text($"  {c.Key}: built as {c.Built}, now {c.Now}");
            if (s.State != BuildState.Ok) {
                wrapped("Quit the game, run this, then start the game again:");
                text(h.rebuild_command);
                if (ImGui.Button("Copy command")) ImGui.SetClipboardText(h.rebuild_command);
                ImGui.SameLine();
                if (ImGui.Button("Open mod folder")) open_folder(h.mod_dir, log);
            }
            if (s.Jobs.Count > 0 && ImGui.TreeNode("Converter jobs")) {
                foreach (string j in s.Jobs) wrapped(j);
                ImGui.TreePop();
            }
        }
        if (h.Settings.Missing) ImGui.TextDisabled($"No {ModSettings.FILE} yet: the defaults are used (install.cmd makes it from {ModSettings.DEFAULTS}).");
        foreach (string p in h.Settings.Problems) coloured(YELLOW, $"{ModSettings.FILE}: {p}");
        foreach (string k in h.Settings.Unknown)  coloured(YELLOW, $"{ModSettings.FILE}: unknown setting {k}");
        if (h.Info.Section is { } section && !_failed_sections.Contains(h.Info.Name)) {
            try { section(h); }
            catch (Exception e) {
                _failed_sections.Add(h.Info.Name);
                log.Error($"{h.Info.Name}: its part of the window failed and is now off: {e}");
            }
        }
        // A mod with its own section shows its live settings there; its patches list the values the game started with.
        bool patches_open = h.Info.Section is null || h.Patches.Any(p => p.State == PatchState.Failed);
        string patches_label = h.Info.Section is null ? "Code patches" : "Code patches (settings as the game started)";
        if (h.Patches.Count > 0 && ImGui.TreeNodeEx(patches_label, patches_open ? ImGuiTreeNodeFlags.DefaultOpen : 0)) {
            foreach (var (p, value, state) in h.Patches) {
                (Vector4 c, string label) = state switch {
                    PatchState.On             => (GREEN, "on"),
                    PatchState.Failed         => (RED, "failed"),
                    PatchState.NotImplemented => (YELLOW, "not yet"),
                    _                         => (GREY, "off"),
                };
                coloured(c, $"{label,-8}");
                ImGui.SameLine();
                text($"{p.Key} = {value}: {p.What}");
            }
            ImGui.TreePop();
        }
        ImGui.Unindent();
    }

    private static void open_folder(string dir, FhLogger log) {
        try { Process.Start(new ProcessStartInfo("explorer.exe", $"\"{dir}\"") { UseShellExecute = true }); }
        catch (Exception e) { log.Warning($"could not open {dir}: {e.Message}"); }
    }
}
