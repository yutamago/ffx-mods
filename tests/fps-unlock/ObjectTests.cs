// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Numerics;

namespace Yuta.FfxMods.FpsUnlock.Tests;

public class ObjectPoseTests {
    private static ObjectPose At(float x, float yaw = 0, int order = 0) =>
        new ObjectPose().WithPosition(new Vector3(x, 0, 0)).WithAngles(new Vector3(0, yaw, 0), order);

    [Fact]
    public void A_move_is_lerped_and_a_turn_turned() {
        ObjectPose mid = ObjectPose.Interpolate(At(0, 0), At(10, 1), 0.5f);
        Assert.Equal(5, mid.Position.X, 5);
        Assert.Equal(0.5, mid.Angles.Y, 5);
    }

    [Fact]
    public void A_turn_over_the_wrap_goes_the_short_way() {
        // From just under π to just over −π: a small step forward, not almost a full turn back.
        ObjectPose a = At(0, MathF.PI - 0.1f), b = At(0, -MathF.PI + 0.1f);
        float mid = ObjectPose.Interpolate(a, b, 0.5f).Angles.Y;
        Assert.Equal(0, ObjectPose.Wrap(mid - MathF.PI), 4);
        Assert.False(ObjectPose.IsCut(a, b, 50, 120));
    }

    [Fact]
    public void The_ends_are_exact() {
        ObjectPose a = At(1, 0.25f), b = At(2, 0.5f);
        Assert.Equal(a, ObjectPose.Interpolate(a, b, 0));
        Assert.Equal(b, ObjectPose.Interpolate(a, b, 1));
        Assert.Equal(a, ObjectPose.Interpolate(a, b, -1));
        Assert.Equal(b, ObjectPose.Interpolate(a, b, 2));
    }

    [Fact]
    public void Only_the_parts_the_game_set_are_blended() {
        ObjectPose a = new ObjectPose().WithPosition(new Vector3(0, 0, 0)), b = new ObjectPose().WithPosition(new Vector3(4, 0, 0));
        ObjectPose mid = ObjectPose.Interpolate(a, b, 0.25f);
        Assert.False(mid.HasAngles);
        Assert.Equal(1, mid.Position.X, 5);
    }

    [Fact]
    public void Warps_are_long_moves_big_turns_and_another_shape() {
        Assert.True(ObjectPose.IsCut(At(0), At(60), 50, 120));
        Assert.False(ObjectPose.IsCut(At(0), At(40), 50, 120));
        Assert.True(ObjectPose.IsCut(At(0, 0), At(0, 2.5f), 50, 120));
        Assert.True(ObjectPose.IsCut(At(0, 0, order: 0), At(0, 0, order: 3), 50, 120));
        Assert.True(ObjectPose.IsCut(new ObjectPose().WithPosition(Vector3.Zero), At(0), 50, 120));
        Assert.True(ObjectPose.IsCut(At(0), At(float.NaN), 50, 120));
    }
}

public class ObjectHistoryTests {
    private const ulong Id1 = 0x1234_0000_5678;
    private static ObjectPose At(float x, float yaw = 0) => new ObjectPose().WithPosition(new Vector3(x, 0, 0)).WithAngles(new Vector3(0, yaw, 0), 0);

    /// <summary>One tick update in which the game set object 7 to <paramref name="x"/> (both setters, as yiRender_draw does).</summary>
    private static void Tick(ObjectHistory h, float x, float yaw = 0, ulong identity = Id1, int id = 7) {
        h.SawAngles(id, identity, new Vector3(0, yaw, 0), 0);
        h.SawPosition(id, identity, new Vector3(x, 0, 0));
        h.EndUpdate(tick: true, 50, 120);
    }

    /// <summary>A redrawn frame's update: the field's draw sets the tick's pose again, no tick.</summary>
    private static void Redraw(ObjectHistory h, float x, float yaw = 0, ulong identity = Id1, int id = 7) {
        h.SawAngles(id, identity, new Vector3(0, yaw, 0), 0);
        h.SawPosition(id, identity, new Vector3(x, 0, 0));
        h.EndUpdate(tick: false, 50, 120);
    }

    [Fact]
    public void Two_consecutive_ticks_are_drawn_between() {
        ObjectHistory h = new();
        Tick(h, 0);
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        Tick(h, 10, 0.2f);
        Assert.True(h.TryAt(7, Id1, 0.5, out ObjectPose p));
        Assert.Equal(5, p.Position.X, 5);
        Assert.Equal(0.1, p.Angles.Y, 5);
        // On the redrawn frames after the tick the game sets the tick's pose again: still drawn between.
        Redraw(h, 10, 0.2f);
        Assert.True(h.TryAt(7, Id1, 0.75, out p));
        Assert.Equal(7.5, p.Position.X, 5);
        List<int> moving = [];
        h.Moving(moving);
        Assert.Equal([7], moving);
    }

    [Fact]
    public void A_tick_the_object_was_not_set_in_breaks_the_pair() {
        ObjectHistory h = new();
        Tick(h, 0);
        h.EndUpdate(tick: true, 50, 120);
        Tick(h, 10);
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        Tick(h, 20);
        Assert.True(h.TryAt(7, Id1, 0.5, out _));
    }

    [Fact]
    public void An_object_that_stops_being_set_is_dropped() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 1);
        h.EndUpdate(tick: true, 50, 120);
        h.EndUpdate(tick: true, 50, 120);
        Assert.Equal(0, h.Objects);
        // Its last pose stays, to be put back.
        Assert.True(h.Truth(7, out ulong identity, out ObjectPose truth));
        Assert.Equal(Id1, identity);
        Assert.Equal(1, truth.Position.X);
    }

    [Fact]
    public void Another_identity_starts_again() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 1, identity: 99);
        Assert.False(h.TryAt(7, 99, 0.5, out _));
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        Tick(h, 2, identity: 99);
        Assert.True(h.TryAt(7, 99, 0.5, out _));
        // A new identity forgets what the old one had set.
        Assert.True(h.Truth(7, out ulong id, out _));
        Assert.Equal(99ul, id);
    }

    [Fact]
    public void A_warp_is_drawn_as_it_is() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 500);
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        Assert.Equal(1, h.Cuts);
        Tick(h, 501);
        Assert.True(h.TryAt(7, Id1, 0.5, out _));
    }

    [Fact]
    public void A_camera_cut_found_during_the_update_holds_for_its_tick() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 1);
        // The cut is found in the next tick's update (the camera), before the update ends.
        h.SawPosition(7, Id1, new Vector3(2, 0, 0));
        h.SawAngles(7, Id1, Vector3.Zero, 0);
        h.CutAll();
        h.EndUpdate(tick: true, 50, 120);
        Assert.True(h.AllCut);
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        Redraw(h, 2);
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
        // The next tick blends from the cut's.
        Tick(h, 3);
        Assert.False(h.AllCut);
        Assert.True(h.TryAt(7, Id1, 0.5, out ObjectPose p));
        Assert.Equal(2.5, p.Position.X, 5);
    }

    [Fact]
    public void A_pose_the_game_set_since_the_tick_is_drawn_as_it_is() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 1);
        h.SawPosition(7, Id1, new Vector3(5, 0, 0));
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
    }

    [Fact]
    public void Clear_forgets_everything() {
        ObjectHistory h = new();
        Tick(h, 0);
        Tick(h, 1);
        h.Clear();
        Assert.Equal(0, h.Objects);
        Assert.False(h.Truth(7, out _, out _));
        Assert.False(h.TryAt(7, Id1, 0.5, out _));
    }

    [Fact]
    public void The_setting_is_on_by_default_and_round_trips() {
        Assert.True(new FrameSettings().InterpolateObjects);
        Assert.False(Toml.Settings(Toml.Text(new FrameSettings { InterpolateObjects = false })).InterpolateObjects);
        Assert.True(Toml.Settings("[frames.interpolate]\ncharacters = true\n").InterpolateObjects);
    }
}
