// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Threading;

namespace Yuta.FfxMods.FpsUnlock;

/// <summary>
/// The mod at run time: its live settings, the hooks, the hotkeys, the fps counter and its part of
/// the F8 window (docs/fps-unlock.md, "What the mod does"). The settings come from settings.toml when
/// the game starts; the window and the hotkeys change them at once, and they are written back into
/// settings.toml half a second after the last change (only the values that changed, the comments
/// stay).
/// </summary>
internal static class FpsUnlock {
    private static FrameSettings _settings = new();
    private static FrameHooks? _hooks;
    private static ModHost? _host;
    private static string _hook_problem = "";
    private static readonly Stopwatch _clock = Stopwatch.StartNew();
    private static double _save_at = double.NaN;
    /// <summary>What settings.toml holds now (as far as this mod wrote or read it).</summary>
    private static Dictionary<string, string> _saved = [];
    private static double _saved_scale = 1;
    private static bool _scale_short;
    private static bool _section_failed;

    private static FrameSettings Settings => Volatile.Read(ref _settings);

    /// <summary>The code patch of frames.enabled: reads the settings, checks the game's code and installs every hook.</summary>
    public static bool install(ModHost host, string value) {
        _host = host;
        FhLogger log = host.log;
        List<string> problems = [];
        FrameSettings loaded = FrameSettings.FromValues(k => host.Settings.Values.TryGetValue(k, out string? v) ? v : null, problems);
        foreach (string p in problems) log.Warning($"{ModSettings.FILE}: {p}");
        Volatile.Write(ref _settings, loaded);
        _saved = loaded.ToValues().ToDictionary(kv => kv.Key, kv => kv.Value);
        _saved_scale = loaded.TimeScale;

        nint base_addr = Win32.ModuleBase("FFX.exe");
        if (base_addr == 0) {
            log.Error("FPS unlock: FFX.exe is not loaded; the mod does nothing.");
            return false;
        }
        Game game = new(base_addr);
        FrameHooks hooks = new(game, log, () => Settings);
        if (!hooks.CheckGame()) {
            log.Error("FPS unlock: a hook cannot go in safely (see above); the mod installs nothing and the game runs as without it.");
            return false;
        }
        _hooks = hooks;
        bool ok = hooks.Install(host);
        if (!ok) {
            _hook_problem = "Some hooks failed (see the log); the mod may not work.";
            log.Error("FPS unlock: not every hook went in; the frame timing may be wrong. Switch the mod off in the F8 window or remove it from the load order.");
        }
        hooks.PatchLimiter();
        FrameSettings s = Settings;
        log.Info($"FPS unlock: {(s.Enabled ? "on" : "off")}, target {s.Target}{(s.Target == FpsTarget.Fixed ? $" {s.FixedFps}" : "")}, vsync {s.Vsync}, "
            + $"time scale ×{s.TimeScale}, draw-only frames {s.DrawOnly}, interpolation {(s.Interpolate ? "on" : "off")}. Its settings are in the F8 window.");
        return ok;
    }

    /// <summary>The code patch of every other setting: read live by the hooks, so it is on when they are in.</summary>
    public static bool live(ModHost host, string value) => _hooks is not null;

    /// <summary>Swaps in new settings at once; the file is written half a second after the last change (a slider being dragged changes them every frame).</summary>
    private static void set(FrameSettings next) {
        Volatile.Write(ref _settings, next.Clamped());
        _save_at = _clock.Elapsed.TotalSeconds + 0.5;
    }

    private static void save_when_due() {
        // A scale saved lower than asked (not yet checked) is saved again once the game has run at it.
        if (double.IsNaN(_save_at) && _scale_short && _hooks is not null && _hooks.Guard.Persisted(Settings.TimeScale) > _saved_scale) _save_at = 0;
        if (double.IsNaN(_save_at) || _clock.Elapsed.TotalSeconds < _save_at) return;
        _save_at = double.NaN;
        save(Settings);
    }

    /// <summary>
    /// Writes the changed settings into settings.toml. A fast-forward scale goes in only once the game has run at it for
    /// a few seconds (<see cref="ScaleGuard.Persisted"/>): one that crashes the game is never saved, so it cannot crash the
    /// next start-up.
    /// </summary>
    private static void save(FrameSettings s) {
        if (_host is not { } host) return;
        double scale = _hooks?.Guard.Persisted(s.TimeScale) ?? s.TimeScale;
        _saved_scale = scale;
        _scale_short = scale < s.TimeScale;
        s = s with { TimeScale = scale };
        List<KeyValuePair<string, string>> changed = [];
        foreach ((string key, string value) in s.ToValues()) {
            if (_saved.TryGetValue(key, out string? old) && old == value) continue;
            changed.Add(new(key, FrameSettings.TomlLiteral(key, value)));
        }
        if (changed.Count == 0) return;
        try {
            ModSettings.write(host.Settings.Path, changed);
            foreach ((string key, string value) in s.ToValues()) _saved[key] = value;
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) {
            host.log.Error($"FPS unlock: cannot save the settings into {host.Settings.Path}: {e.Message}");
        }
    }

    private static readonly Vector4 Grey = new(0.7f, 0.7f, 0.7f, 1f);
    private static readonly Vector4 Red = new(1f, 0.45f, 0.4f, 1f);
    private static readonly Vector4 Yellow = new(1f, 0.85f, 0.3f, 1f);
    private static readonly string[] Targets = ["Game (30 fps)", "Monitor refresh rate", "Fixed", "Unlimited"];
    private static readonly string[] Vsyncs = ["As the game has it", "On", "Off"];
    private static readonly string[] DrawOnlys = ["Redraw (camera moves between ticks)", "Repeat the last frame"];
    private static readonly string[] EffectModes = ["Replay the last tick's (right speed)", "Run every frame (too fast)", "Hold (missing between ticks)"];

    /// <summary>Hotkeys, saving and the fps counter, every drawn frame (render thread). The settings themselves are drawn in the F8 window (<see cref="section"/>).</summary>
    public static void render() {
        if (_host is null) return;
        hotkeys(_host);
        save_when_due();
        _hooks?.Stats.Sample(_clock.Elapsed.TotalSeconds);
        overlay(Settings);
    }

    private static void hotkeys(ModHost host) {
        ImGuiIOPtr io = ImGui.GetIO();
        if (ImGui.IsKeyPressed(ImGuiKey.F9, false)) {
            FrameSettings s = Settings;
            if (io.KeyCtrl && io.KeyShift) set(s with { TimeScale = 1 });
            else if (io.KeyCtrl) set(s with { TimeScale = TimeScaleSteps.Toggle(s.TimeScale, s.FastScale) });
            else if (io.KeyShift) set(s with { TimeScale = TimeScaleSteps.Toggle(s.TimeScale, s.SlowScale) });
            else if (io.KeyAlt) set(s with { Vsync = s.Vsync == VsyncMode.On ? VsyncMode.Off : VsyncMode.On });
        }
        if (io.KeyCtrl && ImGui.IsKeyPressed(ImGuiKey.PageUp, true)) set(Settings with { TimeScale = TimeScaleSteps.Up(Settings.TimeScale) });
        if (io.KeyCtrl && ImGui.IsKeyPressed(ImGuiKey.PageDown, true)) set(Settings with { TimeScale = TimeScaleSteps.Down(Settings.TimeScale) });
    }

    /// <summary>The corner counter: what to look at while trying settings, without opening the window.</summary>
    private static void overlay(FrameSettings s) {
        FrameHooks? h = _hooks;
        if (!s.ShowOverlay || h is null) return;
        FrameStats st = h.Stats;
        ImGui.SetNextWindowPos(new Vector2(ImGui.GetIO().DisplaySize.X - 10, 10), ImGuiCond.Always, new Vector2(1, 0));
        ImGui.SetNextWindowBgAlpha(0.55f);
        const ImGuiWindowFlags flags = ImGuiWindowFlags.NoDecoration | ImGuiWindowFlags.AlwaysAutoResize | ImGuiWindowFlags.NoSavedSettings
            | ImGuiWindowFlags.NoFocusOnAppearing | ImGuiWindowFlags.NoNav | ImGuiWindowFlags.NoMove | ImGuiWindowFlags.NoInputs;
        if (ImGui.Begin("###yuta.fps-unlock.overlay", flags)) {
            string vsync = s.Vsync switch { VsyncMode.On => "on", VsyncMode.Off => "off", _ => "game" };
            ImGui.Text($"{st.Rate(Counter.Presents):0} fps   {st.Rate(Counter.WorldTicks):0.0} ticks/s");
            ImGui.TextColored(Grey, $"vsync {vsync} (Alt+F9)   ×{h.LastScale:0.##}{(s.Enabled ? "" : "   mod off")}");
            if (s.Enabled && s.TimeScale > h.Guard.Limit) ImGui.TextColored(Grey, $"starting up: ×{h.Guard.Limit:0.#} of ×{s.TimeScale:0.##}");
            else if (s.Enabled && reached(h) is double r) ImGui.TextColored(Yellow, $"reached ×{r:0.#} of ×{h.LastScale:0.##} (CPU)");
            ImGui.TextColored(Grey, "F8 settings");
        }
        ImGui.End();
    }

    /// <summary>The scale the game reached over the last second, when ticks were dropped and it is short of the asked one; else null.</summary>
    private static double? reached(FrameHooks h) {
        FrameStats st = h.Stats;
        double asked = h.LastScale;
        double r = ReachedSpeed.Of(asked, st.Rate(Counter.PlannedTicks), st.Rate(Counter.DroppedTicks));
        return ReachedSpeed.Short(asked, r) ? r : null;
    }

    /// <summary>The mod's part of the F8 window: every setting, changed live, and what the mod does now.</summary>
    public static void section(ModHost host) {
        if (_section_failed) return;
        try {
            window(Settings);
        }
        catch (Exception e) {
            _section_failed = true;
            host.log.Error($"FPS unlock: its part of the F8 window failed and is now off: {e}");
        }
    }

    private static bool header(string label, bool open = true) =>
        ImGui.TreeNodeEx(label, ImGuiTreeNodeFlags.Framed | (open ? ImGuiTreeNodeFlags.DefaultOpen : 0));

    private static void window(FrameSettings s) {
        FrameHooks? h = _hooks;
        if (h is null) {
            ImGui.TextColored(Red, "Not running: FFX.exe was not found or is another version (see the log).");
            return;
        }
        if (h.Failed) ImGui.TextColored(Red, h.Status);
        if (_hook_problem.Length > 0) ImGui.TextColored(Red, _hook_problem);
        if (!h.LimiterPatched) ImGui.TextColored(Yellow, "The frame limiter is not patched: the game stays at its own 30 fps.");

        bool enabled = s.Enabled;
        if (ImGui.Checkbox("On", ref enabled)) set(s with { Enabled = enabled });
        ImGui.SameLine();
        ImGui.TextColored(Grey, "(off: the game's own timing)");

        if (header("Frame rate")) {
            int target = (int)s.Target;
            if (ImGui.Combo("Target", ref target, Targets, Targets.Length)) set(s with { Target = (FpsTarget)target });
            if (s.Target == FpsTarget.Fixed) {
                float fps = (float)s.FixedFps;
                if (ImGui.SliderFloat("fps", ref fps, (float)FrameSettings.MinFps, 360f, "%.0f")) set(s with { FixedFps = FrameSettings.FromFloat(fps) });
            }
            int vsync = (int)s.Vsync;
            if (ImGui.Combo("Vsync", ref vsync, Vsyncs, Vsyncs.Length)) set(s with { Vsync = (VsyncMode)vsync });
            bool overlay = s.ShowOverlay;
            if (ImGui.Checkbox("Show the fps counter", ref overlay)) set(s with { ShowOverlay = overlay });
            int draw_only = (int)s.DrawOnly;
            if (ImGui.Combo("Frames between ticks", ref draw_only, DrawOnlys, DrawOnlys.Length)) set(s with { DrawOnly = (DrawOnlyMode)draw_only });
            bool interp = s.Interpolate;
            if (ImGui.Checkbox("Interpolate the camera", ref interp)) set(s with { Interpolate = interp });
            ImGui.SameLine();
            bool chars = s.InterpolateCharacters;
            if (ImGui.Checkbox("the characters", ref chars)) set(s with { InterpolateCharacters = chars });
            int effects = (int)s.Effects;
            if (ImGui.Combo("Effects between ticks", ref effects, EffectModes, EffectModes.Length)) set(s with { Effects = (EffectsMode)effects });
            ImGui.TreePop();
        }

        if (header("Time scale")) {
            float scale = (float)s.TimeScale;
            if (ImGui.SliderFloat("×", ref scale, (float)FrameSettings.MinScale, (float)FrameSettings.MaxScale, "%.2f", ImGuiSliderFlags.Logarithmic)) set(s with { TimeScale = FrameSettings.FromFloat(scale) });
            if (ImGui.Button("×0.25")) set(s with { TimeScale = 0.25 });
            ImGui.SameLine();
            if (ImGui.Button("×0.5")) set(s with { TimeScale = 0.5 });
            ImGui.SameLine();
            if (ImGui.Button("×1")) set(s with { TimeScale = 1 });
            ImGui.SameLine();
            if (ImGui.Button("×2")) set(s with { TimeScale = 2 });
            ImGui.SameLine();
            if (ImGui.Button("×10")) set(s with { TimeScale = 10 });
            ImGui.SameLine();
            if (ImGui.Button("×100")) set(s with { TimeScale = 100 });
            bool mute = s.MuteWhenFast;
            if (ImGui.Checkbox("Mute above ×1", ref mute)) set(s with { MuteWhenFast = mute });
            ImGui.SameLine();
            bool booster = s.RespectBoosterLimits;
            if (ImGui.Checkbox("Keep the booster's limits", ref booster)) set(s with { RespectBoosterLimits = booster });
            bool scene = s.SceneTime;
            if (ImGui.Checkbox("Water, sky and other scene animations follow the scale", ref scene)) set(s with { SceneTime = scene });
            ImGui.PushTextWrapPos(0);
            ImGui.TextColored(Grey, $"Ctrl+F9 ×{s.FastScale:0.##} / ×1, Shift+F9 ×{s.SlowScale:0.##} / ×1, Ctrl+Shift+F9 ×1, Ctrl+PgUp/PgDn step, Alt+F9 vsync on/off.");
            ImGui.PopTextWrapPos();
            ImGui.TreePop();
        }

        if (header("Live", open: false)) {
            FrameStats st = h.Stats;
            ImGui.Text($"Drawn: {st.Rate(Counter.Presents):0.0} fps (monitor {h.MonitorHz} Hz)");
            ImGui.Text($"World ticks: {st.Rate(Counter.WorldTicks):0.0}/s (planned {st.Rate(Counter.PlannedTicks):0.0}/s; ×1 reads about 30)");
            ImGui.Text($"Game updates: {st.Rate(Counter.Updates):0.0}/s; frames with ticks {st.Rate(Counter.TickFrames):0.0}/s, redrawn {st.Rate(Counter.RedrawFrames):0.0}/s, repeated {st.Rate(Counter.RepeatFrames):0.0}/s");
            ImGui.Text($"Scale now: ×{h.LastScale:0.##}, ticks last frame {h.LastTicks}, between ticks {h.LastAlpha:0.00}");
            ImGui.Text($"Tick cost {h.TickMs:0.00} ms: the frame's budget fits {h.BudgetTicks} ticks; dropped {st.Rate(Counter.DroppedTicks):0}/s");
            string ramp = h.Guard.Done ? "done" : $"×{h.Guard.Limit:0.#} so far";
            ImGui.Text($"Start-up ramp {ramp}; settings.toml keeps ×{h.Guard.Persisted(s.TimeScale):0.##} (a faster scale after {ScaleGuard.CheckSeconds:0} s at it)");
            if (reached(h) is double r) {
                ImGui.TextColored(Yellow, $"Reached ×{r:0.#} of ×{h.LastScale:0.##}: the CPU cannot run more ticks within the budget (Advanced).");
            }
            string? why = h.ExcludedReason;
            if (why is not null) ImGui.TextColored(Yellow, $"Time scale held at ×1: {why}");
            CameraHistory cam = h.Camera;
            string step = cam.LastStep is (double d, double deg) ? $"{d:0.0} units, {deg:0.0}°" : "-";
            ImGui.Text($"Camera: last tick moved {step}; cuts {cam.Cuts} ({cam.SnapCuts} set by the game){(cam.Cut ? " (cut now)" : "")}");
            ImGui.Text($"Effects: {h.EffectDraws} draws recorded; replayed {st.Rate(Counter.EffectReplays):0}/s, gone {st.Rate(Counter.EffectsGone):0}/s");
            ImGui.Text($"Characters: {h.Characters} held, between ticks {st.Rate(Counter.CharactersInterpolated):0}/s; warps {h.CharacterCuts}");
            ImGui.Text($"Cloth: {h.ClothMeshesHeld} meshes held, blended {st.Rate(Counter.ClothInterpolated):0}/s");
            ImGui.TextColored(Grey, h.Status);
            ImGui.TreePop();
        }

        if (header("Advanced", open: false)) {
            float cap = (float)s.CapSeconds;
            if (ImGui.SliderFloat("Most real time per frame (s)", ref cap, 0.05f, 2f, "%.2f")) set(s with { CapSeconds = FrameSettings.FromFloat(cap) });
            int max_ticks = s.MaxTicksPerFrame;
            if (ImGui.SliderInt("Most ticks per frame", ref max_ticks, 1, FrameSettings.MaxTicks)) set(s with { MaxTicksPerFrame = max_ticks });
            float budget_ms = (float)(s.FrameBudgetSeconds * 1000);
            if (ImGui.SliderFloat("Most update time per frame (ms)", ref budget_ms, (float)(FrameSettings.MinBudget * 1000), (float)(FrameSettings.MaxBudget * 1000), "%.0f", ImGuiSliderFlags.Logarithmic)) {
                set(s with { FrameBudgetSeconds = FrameSettings.FromFloat(budget_ms) / 1000.0 });
            }
            int max_updates = s.MaxUpdatesPerFrame;
            if (ImGui.SliderInt("Whole updates per frame", ref max_updates, 1, 10)) set(s with { MaxUpdatesPerFrame = max_updates });
            float fast = (float)s.FastScale, slow = (float)s.SlowScale;
            if (ImGui.SliderFloat("Ctrl+F9 scale", ref fast, 1f, (float)FrameSettings.MaxScale, "%.1f", ImGuiSliderFlags.Logarithmic)) set(s with { FastScale = FrameSettings.FromFloat(fast) });
            if (ImGui.SliderFloat("Shift+F9 scale", ref slow, (float)FrameSettings.MinScale, 1f, "%.2f", ImGuiSliderFlags.Logarithmic)) set(s with { SlowScale = FrameSettings.FromFloat(slow) });
            ImGui.Separator();
            ImGui.TextColored(Grey, "Held on redrawn frames (things that would otherwise run once per drawn frame):");
            bool chars = s.HoldCharacters, field = s.HoldFieldAnimation, fades = s.HoldFades, cloth = s.HoldCloth;
            if (ImGui.Checkbox("Characters' bones and motion events (Ch_CalcMiddle)", ref chars)) set(s with { HoldCharacters = chars });
            if (ImGui.Checkbox("Field animation (yiAnimMain)", ref field)) set(s with { HoldFieldAnimation = field });
            if (ImGui.Checkbox("Fade counters (Sg_DrawFilter)", ref fades)) set(s with { HoldFades = fades });
            if (ImGui.Checkbox("Cloth (Sg_ClothStep)", ref cloth)) set(s with { HoldCloth = cloth });
            ImGui.Separator();
            bool bones = s.InterpolateBones;
            if (ImGui.Checkbox("Interpolate the characters' bones (their animation), not only their place", ref bones)) set(s with { InterpolateBones = bones });
            bool cloth_lerp = s.InterpolateCloth;
            if (ImGui.Checkbox("Interpolate the characters' cloth (hair, shorts, coats); needs the cloth held", ref cloth_lerp)) set(s with { InterpolateCloth = cloth_lerp });
            float cdist = (float)s.CharacterCutDistance, cdeg = (float)s.CharacterCutDegrees;
            if (ImGui.SliderFloat("Character warp: moved more than", ref cdist, 1f, 2000f, "%.0f units", ImGuiSliderFlags.Logarithmic)) set(s with { CharacterCutDistance = FrameSettings.FromFloat(cdist) });
            if (ImGui.SliderFloat("Character warp: turned more than", ref cdeg, 1f, 180f, "%.0f°")) set(s with { CharacterCutDegrees = FrameSettings.FromFloat(cdeg) });
            bool repeats = s.InterpolateRepeats;
            if (ImGui.Checkbox("Move the camera on repeated frames", ref repeats)) set(s with { InterpolateRepeats = repeats });
            float dist = (float)s.CutDistance, deg = (float)s.CutDegrees;
            if (ImGui.SliderFloat("Cut: moved more than", ref dist, 1f, 2000f, "%.0f units", ImGuiSliderFlags.Logarithmic)) set(s with { CutDistance = FrameSettings.FromFloat(dist) });
            if (ImGui.SliderFloat("Cut: turned more than", ref deg, 1f, 180f, "%.0f°")) set(s with { CutDegrees = FrameSettings.FromFloat(deg) });
            bool snaps = s.ScriptCameraCuts;
            if (ImGui.Checkbox("The game's instant camera sets are cuts", ref snaps)) set(s with { ScriptCameraCuts = snaps });
            if (ImGui.Button("Defaults")) set(new FrameSettings { Enabled = s.Enabled });
            ImGui.TreePop();
        }
        ImGui.TextColored(Grey, "Changes apply at once and are saved into settings.toml. F8 closes this window.");
    }
}
