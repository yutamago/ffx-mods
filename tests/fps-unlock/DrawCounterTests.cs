// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;

namespace Yuta.FfxMods.FpsUnlock.Tests;

/// <summary>
///     The title's water and the dream noise step their counters in the draw, once per <c>Sg_MainLoop</c>. Modelled as
///     the game does it: <c>PhyreScene_drawMainMenuWaveEffect</c> draws <c>Water_[frame]</c>, then steps the frame (0..69) and
///     a byte count; <c>graphicDrawDream</c> draws its speckles when the cycle is 0, then steps it (0, 1, 2).
/// </summary>
public class DrawCounterTests {
    /// <summary>The bytes as the mod reads them: water frame, water count, the dream cycle (16 bits).</summary>
    private sealed class Model {
        public readonly byte[] Memory = new byte[4];

        /// <summary>One <c>Sg_MainLoop</c>'s draw: what it showed (water frame, speckles).</summary>
        public (int Water, bool Speckles) Draw() {
            int water = Memory[0];
            Memory[0] = (byte)(water + 1 < 0x46 ? water + 1 : 0);
            Memory[1]++;
            int cycle = BitConverter.ToInt16(Memory, 2);
            bool speckles = cycle == 0;
            BitConverter.GetBytes((short)(cycle == 2 ? 0 : cycle + 1)).CopyTo(Memory, 2);
            return (water, speckles);
        }
    }

    /// <summary>The title at <paramref name="fps"/> (30 Hz world, Redraw between ticks), for <paramref name="seconds"/>.</summary>
    private static List<(FrameKind Kind, int Water, bool Speckles)> Title(double fps, double seconds, bool hold, FrameSettings? settings = null) {
        Model game = new();
        DrawCounters counters = new(game.Memory.Length);
        FrameDirector d = new();
        List<(FrameKind, int, bool)> drawn = [];
        for (int i = 0; i < (int)(fps * seconds); i++) {
            FrameDecision f = d.Next(new FrameInput(1 / fps, 2, true, false, false), settings ?? new FrameSettings());
            int updates = f.Kind == FrameKind.Tick ? f.Plan.Loops : 1;
            for (int u = 0; u < updates; u++) {
                if (hold && f.Kind == FrameKind.Redraw) counters.BeginRedraw(game.Memory);
                else if (hold) counters.BeginTick(game.Memory);
                (int water, bool speckles) = game.Draw();
                if (hold && f.Kind == FrameKind.Redraw) counters.EndRedraw(game.Memory);
                if (u == updates - 1) drawn.Add((f.Kind, water, speckles));
            }
        }
        return drawn;
    }

    [Fact]
    public void Without_the_hold_the_water_runs_at_the_frame_rate() {
        // What main did: at 200 fps the 70 frames went round in 0.35 s instead of 2.3 s.
        var drawn = Title(200, 1, hold: false);
        Assert.Equal(200 % 70, drawn[^1].Water + 1);
    }

    [Theory]
    [InlineData(60)]
    [InlineData(144)]
    [InlineData(200)]
    [InlineData(1000)]
    public void The_water_steps_once_a_tick_whatever_the_frame_rate(double fps) {
        var drawn = Title(fps, 2, hold: true);
        int ticks = drawn.Count(x => x.Kind == FrameKind.Tick);
        Assert.InRange(ticks, 59, 61);
        // Every frame shows the frame of the tick before it: the water steps 30 times a second.
        int last = -1;
        foreach ((FrameKind kind, int water, _) in drawn) {
            if (kind == FrameKind.Tick) {
                Assert.Equal(last < 0 ? 0 : (last + 1) % 0x46, water);
                last = water;
            }
            else if (last >= 0) Assert.Equal(last, water);
        }
        Assert.Equal((ticks - 1) % 0x46, last);
    }

    [Theory]
    [InlineData(1.0)]
    [InlineData(0.25)]
    public void At_the_game_target_the_water_keeps_the_tick_speed(double scale) {
        // Target Game (#66's pass-through): at ×1 every frame ticks, the counters are never held, and the water steps
        // once a frame as in the game; in slow motion the frames between ticks are Redraws again and hold it.
        FrameSettings s = new() { Target = FpsTarget.Game, TimeScale = scale };
        var drawn = Title(30, 4, hold: true, s);
        int ticks = drawn.Count(x => x.Kind == FrameKind.Tick);
        Assert.InRange(ticks, 120 * scale - 2, 120 * scale + 2);
        if (scale == 1) Assert.All(drawn, x => Assert.Equal(FrameKind.Tick, x.Kind));
        int last = -1;
        foreach ((FrameKind kind, int water, _) in drawn) {
            if (kind == FrameKind.Tick) last = water;
            else if (last >= 0) Assert.Equal(last, water);
        }
        Assert.Equal((ticks - 1) % 0x46, last);
    }

    [Fact]
    public void The_dream_noise_keeps_its_cadence() {
        var drawn = Title(200, 2, hold: true);
        bool tickSpeckles = false;
        int tick = 0;
        foreach ((FrameKind kind, _, bool speckles) in drawn) {
            if (kind == FrameKind.Tick) {
                Assert.Equal(tick++ % 3 == 0, speckles);
                tickSpeckles = speckles;
            }
            else if (tick > 0) Assert.Equal(tickSpeckles, speckles);
        }
    }

    [Fact]
    public void A_redraw_before_any_tick_holds_the_counters() {
        DrawCounters c = new(4);
        byte[] memory = [5, 6, 1, 0];
        c.BeginRedraw(memory);
        Assert.Equal([5, 6, 1, 0], memory);
        memory[0] = 9;
        c.EndRedraw(memory);
        Assert.Equal([5, 6, 1, 0], memory);
        Assert.False(c.HaveTick);
        c.BeginTick([1, 2, 3, 4]);
        c.Clear();
        Assert.False(c.HaveTick);
    }

    /// <summary>
    ///     The field minimap, modelled as <c>rcbgGuideUpdateEx</c> steps it once per drawn frame: the fade level walks
    ///     0..6 (after a delay of 2 frames) with <c>g_GuideFade</c> = level * 128 / 6, and the blink counter's remainder
    ///     over 30 ping-pongs 0..15..0 — the pulsing circle over the player arrow.
    /// </summary>
    private sealed class GuideModel {
        /// <summary>The bytes as the mod holds them: the fade words (<c>0x0193bf6c</c>..<c>0x0193bf7b</c>) then the blink counter.</summary>
        public readonly byte[] Memory = new byte[20];

        /// <summary>One <c>rcbgGuideUpdateEx</c>: what the minimap showed (the pulse level 0..15, the fade alpha).</summary>
        public (int Pulse, int Alpha) Draw() {
            int delay = BitConverter.ToInt32(Memory, 4);
            int level = BitConverter.ToInt32(Memory, 8);
            int counter = BitConverter.ToInt32(Memory, 16);
            int pulse = counter % 30;
            if (pulse > 15) pulse = 30 - pulse;
            if (delay < 2) BitConverter.GetBytes(delay + 1).CopyTo(Memory, 4);
            else if (level < 6) {
                BitConverter.GetBytes(level + 1).CopyTo(Memory, 8);
                BitConverter.GetBytes((level + 1) * 128 / 6).CopyTo(Memory, 12);
            }
            BitConverter.GetBytes(counter + 1).CopyTo(Memory, 16);
            return (pulse, BitConverter.ToInt32(Memory, 12));
        }
    }

    /// <summary>The field at <paramref name="fps"/> (30 Hz world, Redraw between ticks), for <paramref name="seconds"/>.</summary>
    private static List<(FrameKind Kind, int Pulse, int Alpha)> Field(double fps, double seconds, bool hold) {
        GuideModel game = new();
        DrawCounters counters = new(game.Memory.Length);
        FrameDirector d = new();
        List<(FrameKind, int, int)> drawn = [];
        for (int i = 0; i < (int)(fps * seconds); i++) {
            FrameDecision f = d.Next(new FrameInput(1 / fps, 2, true, false, false), new FrameSettings());
            int updates = f.Kind == FrameKind.Tick ? f.Plan.Loops : 1;
            for (int u = 0; u < updates; u++) {
                if (hold && f.Kind == FrameKind.Redraw) counters.BeginRedraw(game.Memory);
                else if (hold) counters.BeginTick(game.Memory);
                (int pulse, int alpha) = game.Draw();
                if (hold && f.Kind == FrameKind.Redraw) counters.EndRedraw(game.Memory);
                if (u == updates - 1) drawn.Add((f.Kind, pulse, alpha));
            }
        }
        return drawn;
    }

    [Fact]
    public void Without_the_hold_the_minimap_pulse_runs_at_the_frame_rate() {
        // What the game did: at 120 fps the circle over the player arrow pulsed 4 times a second instead of once.
        var drawn = Field(120, 1, hold: false);
        Assert.Equal(4, drawn.Count(x => x.Pulse == 0));   // 120 steps of the 30-count cycle in one second
    }

    [Theory]
    [InlineData(60)]
    [InlineData(144)]
    [InlineData(200)]
    [InlineData(1000)]
    public void The_minimap_pulse_steps_once_a_tick_whatever_the_frame_rate(double fps) {
        var drawn = Field(fps, 2, hold: true);
        int ticks = drawn.Count(x => x.Kind == FrameKind.Tick);
        Assert.InRange(ticks, 59, 61);
        int last = -1;
        int steps = 0;
        foreach ((FrameKind kind, int pulse, _) in drawn) {
            if (kind == FrameKind.Tick) {
                if (last >= 0 && pulse != last) steps++;
                last = pulse;
            }
            else if (last >= 0) Assert.Equal(last, pulse);   // a Redraw shows the tick's circle
        }
        Assert.Equal(ticks - 1, steps);                      // one count per tick: 30 a second
    }

    [Fact]
    public void The_minimap_fade_completes_over_the_same_ticks_at_any_frame_rate() {
        foreach (double fps in new[] {30.0, 200.0}) {
            var drawn = Field(fps, 1, hold: true);
            int ticks = drawn.Count(x => x.Kind == FrameKind.Tick);
            Assert.InRange(ticks, 29, 31);
            int tick = 0, shownAt = -1;
            foreach ((FrameKind kind, _, int alpha) in drawn) {
                if (kind == FrameKind.Tick) tick++;
                if (alpha >= 128 && shownAt < 0) shownAt = tick;
            }
            Assert.InRange(shownAt, 8, 9);   // 2 delay frames and the 6 levels of g_GuideFadeSteps, in ticks
        }
    }
}
