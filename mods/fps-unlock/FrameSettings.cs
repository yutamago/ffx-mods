// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;

namespace Yuta.FfxMods.FpsUnlock;

/// <summary>How fast the mod lets the game draw.</summary>
public enum FpsTarget {
    /// <summary>The game's own limiter: 30 fps in the world, 60 in the main menu.</summary>
    Game,
    /// <summary>The refresh rate of the monitor the game's window is on.</summary>
    Monitor,
    /// <summary><see cref="FrameSettings.FixedFps"/>.</summary>
    Fixed,
    /// <summary>No limiter: as fast as the machine draws (vsync, when on, still waits for the monitor).</summary>
    Unlimited,
}

/// <summary>Whether a present waits for the monitor's refresh.</summary>
public enum VsyncMode {
    /// <summary>As the game's renderer has it (its own setting).</summary>
    Game,
    On,
    Off,
}

/// <summary>What a drawn frame without a tick does.</summary>
public enum DrawOnlyMode {
    /// <summary>
    ///     Runs the game's update with no tick (exec frames 0) and holds what would still advance, so the frame draws
    ///     again and the camera can move between ticks (interpolation).
    /// </summary>
    Redraw,
    /// <summary>
    ///     Skips the game's update, as the PC's Esc menu does, and lets the renderer show the last frame again. Nothing
    ///     of the game runs; only the camera can still move (when interpolating).
    /// </summary>
    Repeat,
}

/// <summary>
///     The mod's settings. Immutable: the window makes a changed copy and swaps it in, so the game thread always reads
///     one consistent set.
/// </summary>
/// <remarks>
///     Kept in the mod's settings.toml (<see cref="Keys"/>: the keys of <c>Mod.cs</c>'s SETTINGS, in the same order).
///     Missing keys keep their defaults; values out of range are clamped (<see cref="Clamped"/>).
/// </remarks>
public sealed record FrameSettings {
    /// <summary>The whole mod: off gives the game back its own timing, limiter and vsync.</summary>
    public bool Enabled { get; init; } = true;

    public FpsTarget Target { get; init; } = FpsTarget.Monitor;
    /// <summary>The frame rate of <see cref="FpsTarget.Fixed"/>, 10 to 1000.</summary>
    public double FixedFps { get; init; } = 120;
    public VsyncMode Vsync { get; init; } = VsyncMode.Game;
    /// <summary>A small always-on counter in the corner: drawn fps, world ticks a second, vsync, time scale.</summary>
    public bool ShowOverlay { get; init; } = true;

    /// <summary>Game time per real second, 0.05 to 100; 1 is the game's speed.</summary>
    public double TimeScale { get; init; } = 1;
    /// <summary>The scale Ctrl+F9 switches to and back.</summary>
    public double FastScale { get; init; } = 10;
    /// <summary>The scale Shift+F9 switches to and back.</summary>
    public double SlowScale { get; init; } = 0.25;
    /// <summary>Sound off while the game runs faster than ×1, as the booster does.</summary>
    public bool MuteWhenFast { get; init; } = true;
    /// <summary>Also keep ×1 where the PC booster refuses to speed up (its maps, spells, the sphere grid, a load).</summary>
    public bool RespectBoosterLimits { get; init; } = true;
    /// <summary>
    ///     The scene's own animations follow game time (<see cref="SceneClock"/>): the shaders' <c>GlobalTime</c> (water,
    ///     scrolling clouds and the other UV-animated, ice and wave surfaces), the PC's texture animations, the radial blur,
    ///     the light flash and the character fog fade. Off: the frame's real time, as the game has it.
    /// </summary>
    public bool SceneTime { get; init; } = true;

    /// <summary>The most real time one frame counts, in seconds (the game's own is 0.3).</summary>
    public double CapSeconds { get; init; } = 0.3;
    /// <summary>The most ticks one frame runs.</summary>
    public int MaxTicksPerFrame { get; init; } = 100;
    /// <summary>
    ///     The most whole game updates one frame runs; more ticks than this go through exec frames, in every update but the
    ///     last (which draws and runs one tick; with exec frames a frame has at least 2 updates, <see cref="TickPlan"/>).
    /// </summary>
    public int MaxUpdatesPerFrame { get; init; } = 4;
    /// <summary>
    ///     The most real time one frame's game update may take, in seconds: past it the frame runs fewer ticks (the game
    ///     then runs slower than the scale asks, and the overlay says so) instead of each frame taking longer than the last.
    /// </summary>
    public double FrameBudgetSeconds { get; init; } = 0.04;

    public DrawOnlyMode DrawOnly { get; init; } = DrawOnlyMode.Redraw;
    /// <summary>
    ///     Redraw frames: what the effects' programs do (<see cref="EffectsMode"/>). Replay holds them and hands the
    ///     renderer their last tick's draws again, so they keep the game's speed and are drawn every frame.
    /// </summary>
    public EffectsMode Effects { get; init; } = EffectsMode.Replay;
    /// <summary>Redraw frames: skip the cloth step (<c>Ch_DrawClothClear</c> and the cloth jobs <c>Ch_DrawMain</c> queues); the cloth keeps its last tick's shape.</summary>
    public bool HoldCloth { get; init; } = true;
    /// <summary>Redraw frames: skip <c>Ch_CalcMiddle</c> (the characters' bones and motion events).</summary>
    public bool HoldCharacters { get; init; } = true;
    /// <summary>Redraw frames: skip <c>yiAnimMain</c> (the field's animation).</summary>
    public bool HoldFieldAnimation { get; init; } = true;
    /// <summary>Redraw frames: put the fade counters back after <c>Sg_DrawFilter</c> drew them.</summary>
    public bool HoldFades { get; init; } = true;

    /// <summary>Draw the camera between its last two ticks.</summary>
    public bool Interpolate { get; init; } = true;
    /// <summary>Also move the camera on <see cref="DrawOnlyMode.Repeat"/> frames.</summary>
    public bool InterpolateRepeats { get; init; } = true;
    /// <summary>A camera that moved further than this in one tick cut: it is not interpolated (world units).</summary>
    public double CutDistance { get; init; } = 100;
    /// <summary>A camera that turned more than this in one tick cut (degrees).</summary>
    public double CutDegrees { get; init; } = 45;
    /// <summary>
    ///     A camera the game sets at once (the scripts' camera setters through <c>MsSetCheckChange</c>, <c>MsCameraResetMove</c>:
    ///     the cut natives, the magic files' camera) is a cut when its step jumps, however far under the thresholds. A cut draws the camera, every character and their cloth as the tick has them.
    /// </summary>
    public bool ScriptCameraCuts { get; init; } = true;

    /// <summary>
    ///     Draw each character's place and facing between its last two ticks (with <see cref="InterpolateBones"/> its
    ///     animation, with <see cref="InterpolateCloth"/> its cloth and hair). On since its in-game check (2026-11-08).
    /// </summary>
    public bool InterpolateCharacters { get; init; } = true;
    /// <summary>Also draw the characters' bones between their last two ticks (their animation), not only their place.</summary>
    public bool InterpolateBones { get; init; } = true;
    /// <summary>
    ///     Also draw the characters' cloth (hair, Tidus's shorts, Auron's coat) between their last two ticks: its vertices
    ///     blended and carried with the character. Needs <see cref="HoldCloth"/>; without it the cloth keeps its last tick's
    ///     shape and place while the body moves on.
    /// </summary>
    public bool InterpolateCloth { get; init; } = true;
    /// <summary>
    ///     Draw the map's moving objects (field animated parts, objects the event scripts move: lifts, gates, boats, the
    ///     crane; everything placed through <c>graphicObjectSetTranslation</c>/<c>Rotation</c>) between their last two ticks.
    ///     Warps use the character thresholds.
    /// </summary>
    public bool InterpolateObjects { get; init; } = true;
    /// <summary>A character or map object that moved further than this in one tick warped: it is drawn as it is (world units).</summary>
    public double CharacterCutDistance { get; init; } = 50;
    /// <summary>A character or map object that turned more than this in one tick warped (degrees).</summary>
    public double CharacterCutDegrees { get; init; } = 120;

    /// <summary>The settings with every number in its range.</summary>
    public FrameSettings Clamped() => this with {
        Target = Enum.IsDefined(Target) ? Target : FpsTarget.Monitor,
        Vsync = Enum.IsDefined(Vsync) ? Vsync : VsyncMode.Game,
        DrawOnly = Enum.IsDefined(DrawOnly) ? DrawOnly : DrawOnlyMode.Redraw,
        Effects = Enum.IsDefined(Effects) ? Effects : EffectsMode.Replay,
        FixedFps = Clamp(FixedFps, MinFps, MaxFps, 120),
        TimeScale = Clamp(TimeScale, MinScale, MaxScale, 1),
        FastScale = Clamp(FastScale, MinScale, MaxScale, 10),
        SlowScale = Clamp(SlowScale, MinScale, MaxScale, 0.25),
        CapSeconds = Clamp(CapSeconds, 0.05, 2, 0.3),
        MaxTicksPerFrame = Math.Clamp(MaxTicksPerFrame, 1, MaxTicks),
        MaxUpdatesPerFrame = Math.Clamp(MaxUpdatesPerFrame, 1, 10),
        FrameBudgetSeconds = Clamp(FrameBudgetSeconds, MinBudget, MaxBudget, 0.04),
        CutDistance = Clamp(CutDistance, 1, 100000, 100),
        CutDegrees = Clamp(CutDegrees, 1, 180, 45),
        CharacterCutDistance = Clamp(CharacterCutDistance, 1, 100000, 50),
        CharacterCutDegrees = Clamp(CharacterCutDegrees, 1, 180, 120),
    };

    public const double MinFps = 10, MaxFps = 1000;
    public const double MinScale = 0.05, MaxScale = 100;
    /// <summary>10 updates: 9 × 99 exec frames and the drawing update's 1 (<see cref="TickPlan.MaxTicks"/>).</summary>
    public const int MaxTicks = 9 * TickPlan.MaxExec + 1;
    /// <summary>The range of <see cref="FrameBudgetSeconds"/>.</summary>
    public const double MinBudget = 0.005, MaxBudget = 0.3;

    private static double Clamp(double v, double min, double max, double fallback) =>
        double.IsNaN(v) ? fallback : Math.Clamp(v, min, max);

    /// <summary>One setting of settings.toml: its key, how to write it as text, how to read it back.</summary>
    private sealed record Entry(string Key, Func<FrameSettings, string> Get, Func<FrameSettings, string, FrameSettings?> Set, bool Quoted);

    private static Entry Flag(string key, Func<FrameSettings, bool> get, Func<FrameSettings, bool, FrameSettings> set) =>
        new(key, s => get(s) ? "true" : "false", (s, v) => v switch { "true" => set(s, true), "false" => set(s, false), _ => null }, false);

    private static Entry Number(string key, Func<FrameSettings, double> get, Func<FrameSettings, double, FrameSettings> set) =>
        new(key, s => Float(get(s)), (s, v) => double.TryParse(v.Replace("_", ""), NumberStyles.Float, CultureInfo.InvariantCulture, out double d) ? set(s, d) : null, false);

    private static Entry Integer(string key, Func<FrameSettings, int> get, Func<FrameSettings, int, FrameSettings> set) =>
        new(key, s => get(s).ToString(CultureInfo.InvariantCulture), (s, v) => int.TryParse(v.Replace("_", ""), NumberStyles.Integer, CultureInfo.InvariantCulture, out int n) ? set(s, n) : null, false);

    private static Entry Choice<T>(string key, Func<FrameSettings, T> get, Func<FrameSettings, T, FrameSettings> set) where T : struct, Enum =>
        new(key, s => get(s).ToString().ToLowerInvariant(),
            (s, v) => v.Length > 0 && char.IsLower(v[0]) && Enum.TryParse(v, ignoreCase: true, out T e) && Enum.IsDefined(e) ? set(s, e) : null, true);

    /// <summary>
    ///     A float as the converter writes it (TOML): "120.0", "0.25", in the shortest text that reads back as the same value.
    ///     A value that is exactly a <c>float</c> (the window's sliders give floats) is written as that float's shortest text:
    ///     the slider's 0.05f is 0.05000000074505806 as a double, and is written "0.05".
    /// </summary>
    public static string Float(double d) {
        string t = d.ToString("R", CultureInfo.InvariantCulture);
        if (double.IsFinite(d) && (double)(float)d == d) {
            string f = ((float)d).ToString("R", CultureInfo.InvariantCulture);
            if (f.Length < t.Length) t = f;
        }
        return t.Contains('.') || t.Contains('E') || !double.IsFinite(d) ? t : t + ".0";
    }

    /// <summary>A slider's float as the double it shows: its shortest text read back (0.05f gives 0.05, not 0.05000000074505806).</summary>
    public static double FromFloat(float f) =>
        float.IsFinite(f) ? double.Parse(f.ToString("R", CultureInfo.InvariantCulture), NumberStyles.Float, CultureInfo.InvariantCulture) : f;

    private static readonly Entry[] Entries = [
        Flag("frames.enabled", s => s.Enabled, (s, v) => s with { Enabled = v }),
        Choice("frames.target", s => s.Target, (s, v) => s with { Target = v }),
        Number("frames.fixed_fps", s => s.FixedFps, (s, v) => s with { FixedFps = v }),
        Choice("frames.vsync", s => s.Vsync, (s, v) => s with { Vsync = v }),
        Flag("frames.overlay", s => s.ShowOverlay, (s, v) => s with { ShowOverlay = v }),
        Choice("frames.between_ticks", s => s.DrawOnly, (s, v) => s with { DrawOnly = v }),
        Choice("frames.effects", s => s.Effects, (s, v) => s with { Effects = v }),
        Number("frames.speed.time_scale", s => s.TimeScale, (s, v) => s with { TimeScale = v }),
        Number("frames.speed.fast", s => s.FastScale, (s, v) => s with { FastScale = v }),
        Number("frames.speed.slow", s => s.SlowScale, (s, v) => s with { SlowScale = v }),
        Flag("frames.speed.mute_when_fast", s => s.MuteWhenFast, (s, v) => s with { MuteWhenFast = v }),
        Flag("frames.speed.booster_limits", s => s.RespectBoosterLimits, (s, v) => s with { RespectBoosterLimits = v }),
        Flag("frames.speed.scene_time", s => s.SceneTime, (s, v) => s with { SceneTime = v }),
        Flag("frames.interpolate.camera", s => s.Interpolate, (s, v) => s with { Interpolate = v }),
        Flag("frames.interpolate.characters", s => s.InterpolateCharacters, (s, v) => s with { InterpolateCharacters = v }),
        Flag("frames.interpolate.bones", s => s.InterpolateBones, (s, v) => s with { InterpolateBones = v }),
        Flag("frames.interpolate.cloth", s => s.InterpolateCloth, (s, v) => s with { InterpolateCloth = v }),
        Flag("frames.interpolate.objects", s => s.InterpolateObjects, (s, v) => s with { InterpolateObjects = v }),
        Flag("frames.interpolate.repeats", s => s.InterpolateRepeats, (s, v) => s with { InterpolateRepeats = v }),
        Number("frames.cuts.camera_distance", s => s.CutDistance, (s, v) => s with { CutDistance = v }),
        Number("frames.cuts.camera_degrees", s => s.CutDegrees, (s, v) => s with { CutDegrees = v }),
        Flag("frames.cuts.game_sets", s => s.ScriptCameraCuts, (s, v) => s with { ScriptCameraCuts = v }),
        Number("frames.cuts.character_distance", s => s.CharacterCutDistance, (s, v) => s with { CharacterCutDistance = v }),
        Number("frames.cuts.character_degrees", s => s.CharacterCutDegrees, (s, v) => s with { CharacterCutDegrees = v }),
        Flag("frames.hold.characters", s => s.HoldCharacters, (s, v) => s with { HoldCharacters = v }),
        Flag("frames.hold.field_animation", s => s.HoldFieldAnimation, (s, v) => s with { HoldFieldAnimation = v }),
        Flag("frames.hold.fades", s => s.HoldFades, (s, v) => s with { HoldFades = v }),
        Flag("frames.hold.cloth", s => s.HoldCloth, (s, v) => s with { HoldCloth = v }),
        Number("frames.budget.max_real_time", s => s.CapSeconds, (s, v) => s with { CapSeconds = v }),
        Integer("frames.budget.max_ticks", s => s.MaxTicksPerFrame, (s, v) => s with { MaxTicksPerFrame = v }),
        Number("frames.budget.max_update_ms", s => Math.Round(s.FrameBudgetSeconds * 1000, 6), (s, v) => s with { FrameBudgetSeconds = v / 1000 }),
        Integer("frames.budget.whole_updates", s => s.MaxUpdatesPerFrame, (s, v) => s with { MaxUpdatesPerFrame = v }),
    ];

    /// <summary>The settings.toml keys, in the order of <c>Mod.cs</c>'s SETTINGS.</summary>
    public static IEnumerable<string> Keys {
        get { foreach (Entry e in Entries) yield return e.Key; }
    }

    /// <summary>Every setting as settings.toml has it as text ("monitor", "true", "120.0"), in <see cref="Keys"/>' order.</summary>
    public IReadOnlyList<KeyValuePair<string, string>> ToValues() {
        List<KeyValuePair<string, string>> values = new(Entries.Length);
        foreach (Entry e in Entries) values.Add(new(e.Key, e.Get(this)));
        return values;
    }

    /// <summary>The value of <paramref name="key"/> as a TOML literal: quoted for the choices, plain for flags and numbers.</summary>
    public static string TomlLiteral(string key, string value) {
        foreach (Entry e in Entries) {
            if (e.Key == key) return e.Quoted ? $"\"{value}\"" : value;
        }
        throw new ArgumentException($"no setting {key}", nameof(key));
    }

    /// <summary>
    ///     Reads the settings from <paramref name="get"/> (a key's text, or null when the file does not set it). A value
    ///     that cannot be read keeps its default and is reported in <paramref name="problems"/>.
    /// </summary>
    public static FrameSettings FromValues(Func<string, string?> get, List<string>? problems = null) {
        FrameSettings s = new();
        foreach (Entry e in Entries) {
            string? v = get(e.Key);
            if (v is null) continue;
            FrameSettings? next = e.Set(s, v);
            if (next is null) problems?.Add($"{e.Key} = {v}: not a value this setting takes; using {e.Get(s)}");
            else s = next;
        }
        return s.Clamped();
    }
}
