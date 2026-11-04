// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Reflection;

namespace Yuta.Demaster;

/// <summary>
/// FFX Demaster. demaster.exe builds the PS2 files into this mod's efl/x folder, where
/// Fahrenheit's file loader puts them in place of the HD ones. This module checks at startup
/// that the build matches demaster.toml, applies the code patches the settings ask for, and
/// shows the result in a small window (F8).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class DemasterModule : FhModule {
    private const ImGuiKey TOGGLE_KEY = ImGuiKey.F8;

    private static readonly Vector4 GREEN  = new(0.55f, 0.9f, 0.55f, 1);
    private static readonly Vector4 YELLOW = new(1.0f, 0.85f, 0.3f, 1);
    private static readonly Vector4 RED    = new(1.0f, 0.4f, 0.4f, 1);

    private string _mod_dir = "";
    private string _version = "";
    private DemasterSettings? _settings;
    private BuildStatus? _status;
    private List<(CodePatch Patch, string Value, PatchState State)> _patches = [];
    private bool _open;
    private bool _ui_failed;

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        _mod_dir = mod_context.Paths.ModDir.FullName;
        _version = typeof(DemasterModule).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "?";
        _logger.Info($"FFX Demaster {_version}, mod folder {_mod_dir}");

        _settings = DemasterSettings.load(Path.Combine(_mod_dir, "demaster.toml"));
        if (!_settings.Found) _logger.Warning($"{_settings.Path} is missing; using the defaults. Running demaster.exe creates it.");
        foreach (string p in _settings.Problems) _logger.Warning($"demaster.toml: {p}");
        foreach (string k in _settings.Unknown)  _logger.Warning($"demaster.toml: unknown setting {k}, ignored");

        _status = BuildStatus.check(_mod_dir, _settings, _version);
        switch (_status.State) {
            case BuildState.Ok: _logger.Info(_status.Summary); break;
            default:
                _logger.Warning($"{_status.Summary} To rebuild: {rebuild_command()}, then restart the game.");
                foreach (var c in _status.Changed) _logger.Warning($"  {c.Key}: built {c.Built}, now {c.Now}");
                break;
        }
        foreach (string j in _status.Jobs) _logger.Info($"  {j}");

        _patches = Patches.apply(this, _settings, _logger);
        // Say so at once when the PS2 files don't match the settings.
        _open = _status.State != BuildState.Ok;
        return true;
    }

    /// <summary>The module's log, for the code patches.</summary>
    internal FhLogger log => _logger;

    /// <summary>The mod folder (demaster.toml, efl, music, demaster_live.dll).</summary>
    internal string mod_dir => _mod_dir;

    private string rebuild_command() => $"\"{Path.Combine(_mod_dir, "demaster.exe")}\" build";

    public override void render_imgui() {
        // Fahrenheit calls this without a try/catch, so an exception would end the game.
        if (_ui_failed) return;
        try {
            draw();
        }
        catch (Exception e) {
            _ui_failed = true;
            _logger.Error($"The window failed and is now off: {e}");
        }
    }

    private static void text(string s) => ImGui.TextUnformatted(s);
    private static void coloured(Vector4 c, string s) => ImGui.TextColored(c, s.Replace("%", "%%"));
    private static void wrapped(string s) => ImGui.TextWrapped(s.Replace("%", "%%"));

    private void draw() {
        if (ImGui.IsKeyPressed(TOGGLE_KEY, false)) _open = !_open;
        if (!_open || _status is null || _settings is null) return;

        ImGui.SetNextWindowSize(new Vector2(560, 0), ImGuiCond.FirstUseEver);
        if (ImGui.Begin($"FFX Demaster {_version.Split('+')[0]} (F8)", ref _open)) {
            Vector4 colour = _status.State switch {
                BuildState.Ok         => GREEN,
                BuildState.Incomplete => YELLOW,
                _                     => RED,
            };
            ImGui.PushTextWrapPos(0);
            coloured(colour, _status.Summary);
            ImGui.PopTextWrapPos();
            if (_status.BuiltAt is DateTime at)
                ImGui.TextDisabled($"built {at:yyyy-MM-dd HH:mm} by converter {_status.ConverterVersion}, {_status.FilesFound} files in efl\\x");

            foreach (var c in _status.Changed) text($"  {c.Key}: built as {c.Built}, now {c.Now}");

            if (_status.State != BuildState.Ok) {
                ImGui.Separator();
                wrapped("Quit the game, run this, then start the game again:");
                text(rebuild_command());
                if (ImGui.Button("Copy command")) ImGui.SetClipboardText(rebuild_command());
                ImGui.SameLine();
                if (ImGui.Button("Open mod folder")) open_folder();
            }
            if (!_settings.Found) coloured(YELLOW, "demaster.toml is missing: the defaults are used.");
            foreach (string p in _settings.Problems) coloured(YELLOW, $"demaster.toml: {p}");
            foreach (string k in _settings.Unknown)  coloured(YELLOW, $"demaster.toml: unknown setting {k}");

            if (_status.Jobs.Count > 0 && ImGui.CollapsingHeader("Converter jobs")) {
                foreach (string j in _status.Jobs) wrapped(j);
            }
            if (ImGui.CollapsingHeader("Code patches")) {
                foreach (var (p, value, state) in _patches) {
                    (Vector4 c, string s) = state switch {
                        PatchState.On             => (GREEN, "on"),
                        PatchState.Failed         => (RED, "failed"),
                        PatchState.NotImplemented => (YELLOW, "not yet"),
                        _                         => (new Vector4(0.6f, 0.6f, 0.6f, 1), "off"),
                    };
                    coloured(c, $"{s,-8}");
                    ImGui.SameLine();
                    text($"{p.Key} = {value}: {p.What}");
                }
            }
            ImGui.TextDisabled("Settings: demaster.toml in the mod folder. Changes need a rebuild and a restart.");
        }
        ImGui.End();
    }

    private void open_folder() {
        try { Process.Start(new ProcessStartInfo("explorer.exe", $"\"{_mod_dir}\"") { UseShellExecute = true }); }
        catch (Exception e) { _logger.Warning($"could not open {_mod_dir}: {e.Message}"); }
    }
}
