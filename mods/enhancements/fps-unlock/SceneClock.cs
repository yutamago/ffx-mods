// SPDX-License-Identifier: MIT

using System;

namespace Yuta.FfxMods.FpsUnlock;

/// <summary>
///     The scene's own clock: the game time a drawn frame hands the renderer's animations, which FFX.exe steps with the
///     frame's real time instead of the world's ticks (docs/mods/fps-unlock.md, "The scene's clock").
/// </summary>
/// <remarks>
///     <para>
///         <c>FFXApplication_animate</c> (<c>0x0042f520</c>) passes the application's step (<c>app +0x24</c>, the
///         frame's real time, set by <c>PApplication::frame</c> from <c>QueryPerformanceCounter</c>) to <c>updateFFX</c>,
///         and after it to <c>updateGlobalTime</c> (<c>0x0042fa00</c>: the shaders' <c>GlobalTime</c>, which the water,
///         the UV-animation shaders (scrolling clouds), ice and wave shaders multiply their scroll speeds by) and to
///         <c>PhyreScene_animate</c> (<c>0x00657140</c>: the texture animations, the light flash, the radial blur, the
///         character fog fade). The mod's tick plan only reaches <c>updateFFX</c>, so these kept real time.
///     </para>
///     <para>
///         The step this gives them is the accumulator's own game time: the ticks the frame ran plus the change of the
///         carry (the part of a tick between ticks), so it is smooth between ticks, follows the time scale and the booster,
///         and slows down with the world when the CPU cannot keep up (dropped ticks are not counted).
///     </para>
/// </remarks>
public static class SceneClock {
    /// <summary>
    ///     The game time a frame advanced, in seconds: <paramref name="ticks"/> at <paramref name="rate"/> a second plus the
    ///     carry's change (<paramref name="carryBefore"/> to <paramref name="carryAfter"/>, seconds). Never below 0 (a reset
    ///     carry is not time going back) and 0 for a rate that is not above 0.
    /// </summary>
    public static double Step(int ticks, double rate, double carryBefore, double carryAfter) {
        if (!(rate > 0)) return 0;
        double step = Math.Max(0, ticks) / rate + carryAfter - carryBefore;
        return double.IsFinite(step) && step > 0 ? step : 0;
    }

    /// <summary>The game's 30 ticks a second in the world: what the per-frame steps of the PS2 code were written for.</summary>
    public const double WorldRate = 30;

    /// <summary><c>BlurEffect_update</c> (<c>0x006c3d40</c>) adds this to its time <c>+0x84</c> on every call, 30 calls a second at the game's 30 fps (f64 <c>0x00b494c8</c>).</summary>
    public const double RadialBlurPerFrame = 0.098;

    /// <summary>
    ///     The radial blur's time to put in before <c>BlurEffect_update</c> runs, so that its own +0.098 leaves it at
    ///     <paramref name="time"/> + 0.098 × (<paramref name="step"/> × 30): the blur then lasts as long in game time at any
    ///     frame rate (the game's is 103 calls past 10.0, 3.4 s at 30 fps) and stands still when no game time passes.
    /// </summary>
    public static float RadialBlurBefore(float time, double step) =>
        (float)(time + RadialBlurPerFrame * (Math.Max(0, step) * WorldRate - 1));
}

/// <summary>
///     The shaders' <c>GlobalTime</c> (<c>FFXApplication +0x3b0</c>, copied to <c>+0x120</c>), kept in double precision:
///     the game adds each step to an <c>f32</c>, and past 2048 s one <c>f32</c> step is 0.24 ms, more than a slow-motion
///     frame's step at a high frame rate (×0.05 at 360 fps is 0.14 ms), which then rounds to a whole 0.24 ms (×1.8 too
///     fast) or, under 0.12 ms, to nothing (the water stands still).
/// </summary>
/// <remarks>
///     The game still decides whether the time runs (<c>updateGlobalTime</c>: not while paused, loading, or the free camera
///     holds it): the mod calls it with a probe step and looks whether the value moved. The mod only decides how far.
/// </remarks>
public sealed class GlobalTimeClock {
    /// <summary><c>updateGlobalTime</c> sets the time back to 0 from 2400.0 on (f32 <c>0x00b0db50</c>).</summary>
    public const double Wrap = 2400;

    private double _time;
    private float _written;
    private bool _synced;

    /// <summary>The time as the mod keeps it.</summary>
    public double Time => _time;

    /// <summary>
    ///     The value to write after the game's update: <paramref name="held"/> is what the game held before it (a value the
    ///     mod did not write, such as the 0 of a map change, is taken over), <paramref name="runs"/> whether the game's
    ///     update moved it, <paramref name="step"/> the frame's game time.
    /// </summary>
    public float Next(float held, bool runs, double step) {
        if (!_synced || BitConverter.SingleToInt32Bits(held) != BitConverter.SingleToInt32Bits(_written)) _time = float.IsFinite(held) ? held : 0;
        if (runs && double.IsFinite(step) && step > 0) _time += step;
        if (!(_time < Wrap)) _time = 0;
        _written = (float)_time;
        _synced = true;
        return _written;
    }

    /// <summary>Forgets the kept value: the next <see cref="Next"/> starts from what the game holds (the mod was off for a frame).</summary>
    public void Forget() => _synced = false;
}
