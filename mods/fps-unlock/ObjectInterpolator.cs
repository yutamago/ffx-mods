// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Numerics;
using System.Runtime.InteropServices;

namespace Yuta.FfxMods.FpsUnlock;

/// <summary>
///     What the game last asked for a map object (a display object or map group, by its id in the scene's object tree):
///     its position (<c>graphicObjectSetTranslation(id, pos)</c>) and its angles in their order
///     (<c>graphicObjectSetRotation(id, angles, order)</c>). Either can be missing: an object the game only moves keeps the
///     turn its file gave it.
/// </summary>
/// <param name="HasPosition">The game set the position.</param>
/// <param name="Position">Field units, as the game passes them (the meshes' matrices get <c>(-x, -y, z)</c>).</param>
/// <param name="HasAngles">The game set the angles.</param>
/// <param name="Angles">Radians about x, y and z.</param>
/// <param name="Order">The order of the turns (0 z y x … 5 x y z, anything else none).</param>
public readonly record struct ObjectPose(bool HasPosition, Vector3 Position, bool HasAngles, Vector3 Angles, int Order) {
    /// <summary>This pose with the position set.</summary>
    public ObjectPose WithPosition(Vector3 p) => this with { HasPosition = true, Position = p };

    /// <summary>This pose with the angles set.</summary>
    public ObjectPose WithAngles(Vector3 a, int order) => this with { HasAngles = true, Angles = a, Order = order };

    /// <summary>Whether two poses can be blended: the same parts set and the same order of turns.</summary>
    public bool SameShape(in ObjectPose o) => HasPosition == o.HasPosition && HasAngles == o.HasAngles && (!HasAngles || Order == o.Order);

    /// <summary>
    ///     The pose at <paramref name="t"/> from <paramref name="a"/> (0) to <paramref name="b"/> (1): the position lerped,
    ///     each angle turned the short way (a wrap from π to −π is a small step, not a turn back). The ends are exact; two
    ///     poses of another shape give <paramref name="b"/>.
    /// </summary>
    public static ObjectPose Interpolate(in ObjectPose a, in ObjectPose b, float t) {
        if (t <= 0) return a;
        if (t >= 1 || !a.SameShape(b)) return b;
        return b with {
            Position = a.HasPosition ? Vector3.Lerp(a.Position, b.Position, t) : b.Position,
            Angles = a.HasAngles
                ? new Vector3(LerpAngle(a.Angles.X, b.Angles.X, t), LerpAngle(a.Angles.Y, b.Angles.Y, t), LerpAngle(a.Angles.Z, b.Angles.Z, t))
                : b.Angles,
        };
    }

    /// <summary>
    ///     Whether going from <paramref name="a"/> to <paramref name="b"/> in one tick is a jump to draw as it is: another
    ///     shape (<see cref="SameShape"/>), a move further than <paramref name="cutDistance"/>, an angle turned more than
    ///     <paramref name="cutDegrees"/> (the short way), or a value that is not finite.
    /// </summary>
    public static bool IsCut(in ObjectPose a, in ObjectPose b, double cutDistance, double cutDegrees) {
        if (!a.SameShape(b)) return true;
        if (a.HasPosition) {
            if (!Finite(a.Position) || !Finite(b.Position)) return true;
            if (Vector3.Distance(a.Position, b.Position) > cutDistance) return true;
        }
        if (a.HasAngles) {
            if (!Finite(a.Angles) || !Finite(b.Angles)) return true;
            double limit = cutDegrees * Math.PI / 180;
            if (Math.Abs(Wrap(b.Angles.X - a.Angles.X)) > limit || Math.Abs(Wrap(b.Angles.Y - a.Angles.Y)) > limit
                || Math.Abs(Wrap(b.Angles.Z - a.Angles.Z)) > limit) return true;
        }
        return false;
    }

    /// <summary>An angle step into (−π, π].</summary>
    public static double Wrap(double d) {
        d %= 2 * Math.PI;
        if (d > Math.PI) d -= 2 * Math.PI;
        else if (d <= -Math.PI) d += 2 * Math.PI;
        return d;
    }

    private static float LerpAngle(float a, float b, float t) => a == b ? b : (float)(a + Wrap(b - a) * t);

    private static bool Finite(Vector3 v) => float.IsFinite(v.X + v.Y + v.Z);
}

/// <summary>
///     The map's moving objects of the last two ticks (field animated parts, objects the event scripts move: lifts, gates,
///     the crane, boats and the like), keyed by object id, and what to draw between them. The game places them with
///     <c>graphicObjectSetTranslation</c>/<c>Rotation</c>, which rewrite the meshes' world matrices; the mod records the
///     game's last call in each game update that ran a tick (<see cref="EndUpdate"/>) and, after the game's update of every
///     drawn frame, places each object that moved in both of its last two ticks between them.
/// </summary>
/// <remarks>
///     <para>An object is drawn between ticks only when it was set in two consecutive tick updates, with the same identity
///     (its node in the scene's object tree and that node's mesh list: a new map, a battle's scene or a reload makes new
///     ones) and the same shape, and the step is not a warp (the characters' warp thresholds). <see cref="CutAll"/> (a
///     camera cut) draws every object as the tick left it until the next tick; <see cref="Clear"/> on a new map.</para>
///     <para>The game's own last pose (<see cref="Truth"/>) is kept for every object, so the mod can put it back before the
///     next update: the game reads nothing back from these matrices, but an object it stops moving must not stay where
///     the mod drew it.</para>
/// </remarks>
public sealed class ObjectHistory {
    private sealed class Sample {
        public ulong Identity;
        public ObjectPose Prev, Cur;
        public int PrevSerial = -1, CurSerial = -1;
        public bool Cut;
    }

    private readonly Dictionary<int, Sample> _samples = [];
    private readonly Dictionary<int, (ulong Identity, ObjectPose Pose)> _truth = [];
    private readonly HashSet<int> _touched = [];
    private readonly List<int> _stale = [];
    private int _allCut = -1;
    private bool _cutPending;

    /// <summary>Tick updates recorded (the current one's number).</summary>
    public int Serial { get; private set; }
    /// <summary>Objects with a sample.</summary>
    public int Objects => _samples.Count;
    /// <summary>Warps seen.</summary>
    public int Cuts { get; private set; }

    /// <summary>Whether every object is drawn as it is at this tick (<see cref="CutAll"/>).</summary>
    public bool AllCut => _allCut == Serial;

    /// <summary>Forgets everything (a new map, the setting off).</summary>
    public void Clear() {
        _samples.Clear();
        _truth.Clear();
        _touched.Clear();
        _allCut = -1;
        _cutPending = false;
    }

    /// <summary>Forgets the samples but not the game's last poses (the setting switched off).</summary>
    public void ClearSamples() {
        _samples.Clear();
        _allCut = -1;
        _cutPending = false;
    }

    /// <summary>
    ///     The camera cut in this game update (a new shot): every object is drawn as the tick has it until the next tick.
    ///     Found while the update runs, before <see cref="EndUpdate"/> records its tick, so it holds for that tick too.
    /// </summary>
    public void CutAll() {
        _allCut = Serial;
        _cutPending = true;
    }

    /// <summary>A position call of the game's (not the mod's own) for the object with this identity.</summary>
    public void SawPosition(int id, ulong identity, Vector3 p) {
        ref (ulong Identity, ObjectPose Pose) t = ref Saw(id, identity);
        t.Pose = t.Pose.WithPosition(p);
    }

    /// <summary>An angles call of the game's (not the mod's own) for the object with this identity.</summary>
    public void SawAngles(int id, ulong identity, Vector3 a, int order) {
        ref (ulong Identity, ObjectPose Pose) t = ref Saw(id, identity);
        t.Pose = t.Pose.WithAngles(a, order);
    }

    private ref (ulong Identity, ObjectPose Pose) Saw(int id, ulong identity) {
        ref (ulong Identity, ObjectPose Pose) t = ref CollectionsMarshal.GetValueRefOrAddDefault(_truth, id, out bool had);
        if (!had || t.Identity != identity) t = (identity, default);
        _touched.Add(id);
        return ref t;
    }

    /// <summary>The game's own last pose of an object, and the identity it was set for.</summary>
    public bool Truth(int id, out ulong identity, out ObjectPose pose) {
        bool had = _truth.TryGetValue(id, out (ulong Identity, ObjectPose Pose) t);
        (identity, pose) = t;
        return had;
    }

    /// <summary>The ids with a game pose (the test trace).</summary>
    public IEnumerable<int> Ids() => _truth.Keys;

    /// <summary>Forgets the game's last pose of an object (it is gone).</summary>
    public void Forget(int id) {
        _truth.Remove(id);
        _samples.Remove(id);
    }

    /// <summary>
    ///     The end of a game update. With <paramref name="tick"/> (it ran a tick) every object the game set in it goes into
    ///     the history as the new tick's sample, and what was not seen in the last two ticks is dropped. The marks are
    ///     cleared either way.
    /// </summary>
    public void EndUpdate(bool tick, double cutDistance, double cutDegrees) {
        if (tick) {
            Serial++;
            foreach (int id in _touched) {
                (ulong identity, ObjectPose pose) = _truth[id];
                Push(id, identity, pose, cutDistance, cutDegrees);
            }
            Prune();
            if (_cutPending) _allCut = Serial;
        }
        _cutPending = false;
        _touched.Clear();
    }

    /// <summary>A new tick, for <see cref="Push"/> without <see cref="EndUpdate"/> (the tests).</summary>
    public void BeginTick() => Serial++;

    /// <summary>Adds an object's pose at this tick.</summary>
    public void Push(int id, ulong identity, in ObjectPose pose, double cutDistance, double cutDegrees) {
        if (!_samples.TryGetValue(id, out Sample? s)) _samples[id] = s = new Sample();
        if (s.CurSerial == Serial) {
            // A second push in the same tick replaces the first.
            if (s.Identity != identity) s.PrevSerial = -1;
            s.Identity = identity;
            s.Cur = pose;
            s.Cut = s.PrevSerial >= 0 && ObjectPose.IsCut(s.Prev, s.Cur, cutDistance, cutDegrees);
            return;
        }
        bool follows = s.Identity == identity && s.CurSerial == Serial - 1;
        s.Identity = identity;
        s.Prev = s.Cur;
        s.PrevSerial = follows ? s.CurSerial : -1;
        s.Cur = pose;
        s.CurSerial = Serial;
        s.Cut = follows && ObjectPose.IsCut(s.Prev, s.Cur, cutDistance, cutDegrees);
        if (s.Cut) Cuts++;
    }

    /// <summary>Drops the samples not seen in the last two ticks (the game's last poses stay until <see cref="Clear"/>).</summary>
    public void Prune() {
        _stale.Clear();
        foreach ((int k, Sample s) in _samples) {
            if (s.CurSerial < Serial - 1) _stale.Add(k);
        }
        foreach (int k in _stale) _samples.Remove(k);
    }

    /// <summary>The ids <see cref="TryAt"/> may draw between ticks now (two consecutive, different samples up to this tick, no warp).</summary>
    public void Moving(List<int> into) {
        into.Clear();
        if (AllCut) return;
        foreach ((int k, Sample s) in _samples) {
            // Two equal samples: the object stood still over the tick; the game's own pose is already the one to draw.
            if (s.CurSerial == Serial && s.PrevSerial == Serial - 1 && !s.Cut && s.Prev != s.Cur) into.Add(k);
        }
    }

    /// <summary>
    ///     The pose to draw at <paramref name="alpha"/> for the object with this identity; false when it is drawn as it is
    ///     (one sample, not consecutive, another identity, a warp, a camera cut, or the game set it since the tick).
    /// </summary>
    public bool TryAt(int id, ulong identity, double alpha, out ObjectPose pose) {
        pose = default;
        if (AllCut || !_samples.TryGetValue(id, out Sample? s) || s.Identity != identity) return false;
        if (s.CurSerial != Serial || s.PrevSerial != Serial - 1 || s.Cut) return false;
        if (_truth.TryGetValue(id, out (ulong Identity, ObjectPose Pose) t) && (t.Identity != identity || t.Pose != s.Cur)) return false;
        pose = ObjectPose.Interpolate(s.Prev, s.Cur, (float)Math.Clamp(alpha, 0, 1));
        return true;
    }
}
