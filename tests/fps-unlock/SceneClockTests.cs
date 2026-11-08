// SPDX-License-Identifier: MIT

using System.Linq;

namespace Yuta.FfxMods.FpsUnlock.Tests;

/// <summary>
///     The scene's clock (<see cref="SceneClock"/>): the game time the water, the scrolling clouds and the other scene
///     animations get per drawn frame instead of the frame's real time (the bug of 2026-11-08: at ×0.1 they kept moving at
///     full speed).
/// </summary>
public class SceneClockTests {
    private static readonly FrameSettings Defaults = new();

    private static FrameInput World(double dt, bool excluded = false, double booster = 1) => new(dt, 2, true, false, excluded, booster);

    /// <summary>The game time <paramref name="frames"/> frames of <paramref name="dt"/> give, frame by frame.</summary>
    private static double[] Steps(FrameDirector d, FrameSettings s, double dt, int frames, bool excluded = false) =>
        Enumerable.Range(0, frames).Select(_ => d.Next(World(dt, excluded), s).GameStep).ToArray();

    [Theory]
    [InlineData(30, 1.0)]
    [InlineData(144, 1.0)]
    [InlineData(240, 0.1)]
    [InlineData(360, 0.05)]
    [InlineData(144, 10)]
    public void A_second_gives_the_scale_in_game_time(double fps, double scale) {
        FrameDirector d = new();
        double sum = Steps(d, Defaults with { TimeScale = scale }, 1 / fps, (int)fps).Sum();
        Assert.Equal(scale, sum, 1e-6);
    }

    [Fact]
    public void Between_ticks_every_frame_moves_the_same_game_time() {
        // At ×0.1 and 240 fps a tick comes every 80 frames; the scene must move on every frame, not jump at the ticks.
        FrameDirector d = new();
        double[] steps = Steps(d, Defaults with { TimeScale = 0.1 }, 1 / 240.0, 400);
        // (the accumulator's rounding slack of 1e-6 tick can move one step by up to 3e-8 s)
        Assert.All(steps, s => Assert.Equal(0.1 / 240, s, 1e-7));
    }

    [Fact]
    public void The_clock_is_the_ticks_plus_the_part_of_a_tick() {
        FrameDirector d = new();
        FrameSettings s = Defaults with { TimeScale = 0.37 };
        double clock = 0;
        int ticks = 0;
        for (int i = 0; i < 1000; i++) {
            FrameDecision f = d.Next(World(1 / 165.0), s);
            clock += f.GameStep;
            ticks += f.Plan.Ticks;
            Assert.Equal((ticks + d.Accumulator.Fraction) / 30, clock, 1e-9);
        }
    }

    [Fact]
    public void Ticks_the_cpu_could_not_run_are_not_scene_time_either() {
        // ×100 with at most 2 ticks a frame: the world runs 2 ticks a frame, and so does the water.
        FrameDirector d = new();
        FrameSettings s = Defaults with { TimeScale = 100, MaxTicksPerFrame = 2 };
        double sum = 0;
        int ticks = 0;
        for (int i = 0; i < 60; i++) {
            FrameDecision f = d.Next(World(1 / 60.0), s);
            sum += f.GameStep;
            ticks += f.Plan.Ticks;
        }
        Assert.Equal(120, ticks);
        Assert.Equal((ticks + d.Accumulator.Fraction) / 30, sum, 1e-9);
    }

    [Fact]
    public void Where_the_scale_is_held_at_1_the_scene_runs_in_real_time() {
        FrameDirector d = new();
        double sum = Steps(d, Defaults with { TimeScale = 0.1 }, 1 / 120.0, 120, excluded: true).Sum();
        Assert.Equal(1, sum, 1e-6);
    }

    [Fact]
    public void Off_hands_the_real_time_through() {
        FrameDirector d = new();
        Assert.Equal(1 / 75.0, d.Next(World(1 / 75.0), Defaults with { Enabled = false, TimeScale = 0.1 }).GameStep);
    }

    [Fact]
    public void Switching_the_mod_on_again_never_steps_back() {
        FrameDirector d = new();
        FrameSettings on = Defaults with { TimeScale = 0.5 };
        for (int i = 0; i < 7; i++) d.Next(World(1 / 100.0), on);
        d.Next(World(1 / 100.0), on with { Enabled = false });
        Assert.All(Steps(d, on, 1 / 100.0, 50), s => Assert.True(s >= 0));
    }

    [Theory]
    [InlineData(0, 30, 0, 0, 0)]
    [InlineData(1, 30, 0.01, 0, 1 / 30.0 - 0.01)]
    [InlineData(-3, 30, 0, 0.002, 0.002)]
    [InlineData(2, 0, 0, 0.002, 0)]
    [InlineData(0, 30, 0.02, 0, 0)]
    public void Step_is_the_ticks_and_the_carrys_change_never_below_0(int ticks, double rate, double before, double after, double want) =>
        Assert.Equal(want, SceneClock.Step(ticks, rate, before, after), 1e-12);
}

/// <summary>The shaders' <c>GlobalTime</c> as the mod keeps it (<see cref="GlobalTimeClock"/>).</summary>
public class GlobalTimeClockTests {
    [Fact]
    public void Adds_the_step_while_the_game_lets_it_run() {
        GlobalTimeClock c = new();
        float t = c.Next(10, runs: true, 0.5);
        Assert.Equal(10.5f, t);
        Assert.Equal(10.5f, c.Next(t, runs: false, 0.5));
        Assert.Equal(11f, c.Next(10.5f, runs: true, 0.5));
    }

    [Fact]
    public void Takes_over_a_value_the_game_set_itself() {
        // A map change sets GlobalTime to 0 (PhyreScene_fieldMapShutdown).
        GlobalTimeClock c = new();
        c.Next(100, true, 1);
        Assert.Equal(0.25f, c.Next(0, true, 0.25));
    }

    [Fact]
    public void Wraps_to_0_at_2400_as_the_game_does() {
        GlobalTimeClock c = new();
        Assert.Equal(0f, c.Next(2399.75f, true, 0.5));
        Assert.Equal(0.5f, c.Next(0, true, 0.5));
    }

    [Fact]
    public void Slow_motion_steps_at_a_high_frame_rate_are_not_lost_late_in_the_wrap() {
        // ×0.05 at 360 fps for 10 s, starting at 2100 s: 0.5 s of game time. Added to an f32 as the game does, each 0.14 ms
        // step rounds to a whole 0.24 ms, 1.75 times too fast.
        GlobalTimeClock c = new();
        float t = 2100, naive = 2100;
        const double step = 0.05 / 360;
        for (int i = 0; i < 3600; i++) {
            t = c.Next(t, true, step);
            naive = (float)(naive + step);
        }
        Assert.Equal(2100.5, c.Time, 1e-6);
        Assert.Equal(2100.5f, t, 0.001f);
        Assert.True(naive - 2100 > 0.8, $"the f32 sum gave {naive - 2100}");
    }

    [Fact]
    public void Starts_again_from_the_game_after_Forget() {
        GlobalTimeClock c = new();
        c.Next(5, true, 1);
        c.Forget();
        Assert.Equal(6f, c.Next(5, true, 1));
    }
}

/// <summary>The radial blur's time, stepped by game time instead of per call (<see cref="SceneClock.RadialBlurBefore"/>).</summary>
public class RadialBlurTests {
    /// <summary>What <c>BlurEffect_update</c> does to its time: f32 + 0.098 (f64).</summary>
    private static float Game(float t) => (float)(t + 0.098);

    /// <summary>The real seconds until the blur's time passes 10.0 (where it ends) at <paramref name="fps"/> and <paramref name="scale"/>.</summary>
    private static double SecondsToEnd(double fps, double scale, bool mod) {
        float t = 0;
        int frames = 0;
        while (t <= 10f && frames < 1_000_000) {
            if (mod) t = SceneClock.RadialBlurBefore(t, scale / fps);
            t = Game(t);
            frames++;
        }
        return frames / fps;
    }

    [Fact]
    public void The_game_ends_it_after_3_4_seconds_at_its_30_fps() => Assert.Equal(3.4, SecondsToEnd(30, 1, mod: false), 0.05);

    [Fact]
    public void Without_the_clock_it_ends_7_times_sooner_at_200_fps() => Assert.Equal(0.51, SecondsToEnd(200, 1, mod: false), 0.01);

    [Theory]
    [InlineData(30, 1, 3.4)]
    [InlineData(144, 1, 3.4)]
    [InlineData(240, 1, 3.4)]
    [InlineData(240, 0.1, 34)]
    [InlineData(60, 10, 0.34)]
    public void With_the_clock_it_lasts_the_same_game_time_at_any_frame_rate(double fps, double scale, double seconds) =>
        Assert.Equal(seconds, SecondsToEnd(fps, scale, mod: true), seconds * 0.02 + 1.5 / fps);

    [Fact]
    public void No_game_time_holds_it_still() {
        float t = 4.2f;
        for (int i = 0; i < 1000; i++) t = Game(SceneClock.RadialBlurBefore(t, 0));
        Assert.Equal(4.2f, t, 0.001f);
    }
}
