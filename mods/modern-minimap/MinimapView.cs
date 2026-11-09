// SPDX-License-Identifier: MIT

using System.Diagnostics;

namespace Yuta.FfxMods.ModernMinimap;

/// <summary>
/// The Modern Minimap, drawn with ImGui over the game's picture: always centred on the player,
/// turned so that where the camera looks is up; the walkable area of the map's minimap mesh
/// over a grid fixed to the world, the player as an arrow of the character's facing, the
/// game's markers in their shapes and colours; markers beyond the edge sit on the rim, fading
/// in as they come nearer; destinations stay on the rim. "crt" adds a small CRT screen
/// (scanlines, a slow refresh band, a darker rim, a sheen). docs/minimap.md.
/// </summary>
internal static unsafe class MinimapView {
    public enum Corner { TopRight, TopLeft, BottomRight, BottomLeft }

    /// <summary>A marker of AtelGetGideMapActorInfo, positions in the game's world (x, y, z).</summary>
    public sealed record Marker(uint Flags, Vector4 Colour, Vector4 Ring, Vector3 Pos, float Heading, Vector2? LineA, Vector2? LineB);

    /// <summary>The camera's forward and right from its view matrix, and eye to look-at.</summary>
    public readonly record struct Camera(Vector3 Forward, Vector3 Right, Vector3 EyeToLookAt);

    public sealed record Frame(float Shown, Marker? Player, List<Marker> Markers, Camera Camera, float Blink);

    /// <summary>The mesh on the ground (x, z): its triangles and its outline (edges of one triangle).</summary>
    private sealed record Ground(nint Mesh, Vector2[] Triangles, Vector2[] Outline, int Seams);

    /// <summary>World units from the centre to the edge (a character is about 20 tall).</summary>
    private const float RANGE = 120f;
    /// <summary>The frame is stale (no field, a battle, a menu) after this long.</summary>
    private const long STALE_MS = 150;

    private static readonly object _lock = new();
    private static Frame? _frame;
    private static long _frame_at;
    private static nint _mesh_at;
    private static Ground? _ground;
    private static nint _logged_mesh;
    private static readonly Stopwatch _clock = Stopwatch.StartNew();

    // ---- the game's thread ----------------------------------------------------------------

    public static void hide() {
        lock (_lock) _frame = null;
    }

    public static void update(Frame f, nint mesh, FhLogger? log) {
        Ground? ground = null;
        bool rebuild;
        lock (_lock) rebuild = mesh != _mesh_at || (_ground is null && f.Player is not null);
        if (rebuild && f.Player is not null && mesh != 0) {
            ground = read_ground(mesh, new Vector2(f.Player.Pos.X, f.Player.Pos.Z), log);
            if (_logged_mesh != mesh && log is not null) {
                _logged_mesh = mesh;
                Camera c = f.Camera;
                log.Info($"minimap: mesh 0x{mesh:x8}: {ground?.Triangles.Length / 3 ?? 0} triangles, {ground?.Outline.Length / 2 ?? 0} outline pieces ({ground?.Seams ?? 0} inner seam edges left out);"
                    + $" player ({f.Player.Pos.X:0.0}, {f.Player.Pos.Y:0.0}, {f.Player.Pos.Z:0.0}) facing {f.Player.Heading:0.00};"
                    + $" camera forward {fmt(c.Forward)} right {fmt(c.Right)} eye->look-at {fmt(c.EyeToLookAt)}; {f.Markers.Count} markers: {string.Join(" ", f.Markers.Select(k => $"0x{k.Flags:x}"))}");
            }
        }
        lock (_lock) {
            if (rebuild && (ground is not null || mesh == 0)) {
                _ground = ground;
                _mesh_at = mesh;
            }
            _frame = f;
            _frame_at = _clock.ElapsedMilliseconds;
        }
    }

    private static string fmt(Vector3 v) => $"({v.X:0.00}, {v.Y:0.00}, {v.Z:0.00})";

    /// <summary>Bytes per primitive record of each packet type (g_GuidePacketStrides 0x00c5e790).</summary>
    private static readonly int[] STRIDES = [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];

    /// <summary>
    /// The minimap mesh (rcbgModel, docs/formats/field.md "Guide map" and
    /// updateGuildMapRenderData 0x00717620): vertex to world matrix at +0xc0, the blob at +0x14
    /// (+4 packet list, +8 vertices of 3 i16, +0x12 vertex count); packets of type 0 hold
    /// triangles, three u16 vertex indices at +0xc. Of the matrix's two orders the one whose
    /// mesh holds the player (else comes nearer) wins.
    /// </summary>
    private static Ground? read_ground(nint mesh, Vector2 player, FhLogger? log) {
        float* m = (float*)(mesh + 0xc0);
        byte* blob = *(byte**)(mesh + 0x14);
        if (blob == null) return null;
        int count = Math.Min((int)*(ushort*)(blob + 0x12), 0x4000);
        short* vb = (short*)(blob + *(uint*)(blob + 8));
        List<(ushort, ushort, ushort)> tris = [];
        byte* packet = blob + *(uint*)(blob + 4);
        for (int guard = 0; guard < 64; guard++) {
            if (*(ushort*)packet == 0xffff) break;
            int kind = packet[1];
            int n = Math.Min((int)*(ushort*)(packet + 2), 0x4000);
            if (kind >= STRIDES.Length) {
                log?.Warning($"minimap: mesh 0x{mesh:x8} has a packet of type {kind}; read up to it");
                break;
            }
            packet += 16;
            if (kind == 0) {
                for (int i = 0; i < n; i++) {
                    ushort* r = (ushort*)(packet + i * STRIDES[0] + 0xc);
                    if (r[0] < count && r[1] < count && r[2] < count) tris.Add((r[0], r[1], r[2]));
                }
            }
            packet += n * STRIDES[kind];
        }
        Vector2[] place(bool rows) {
            Vector2[] pts = new Vector2[count];
            for (int i = 0; i < count; i++) {
                float x = vb[3 * i], y = vb[3 * i + 1], z = vb[3 * i + 2];
                float at(int r, int c) => rows ? m[r * 4 + c] : m[c * 4 + r];
                pts[i] = new(at(0, 0) * x + at(0, 1) * y + at(0, 2) * z + at(0, 3),
                             at(2, 0) * x + at(2, 1) * y + at(2, 2) * z + at(2, 3));
            }
            return pts;
        }
        float distance(Vector2[] pts) {
            if (pts.Length == 0) return float.MaxValue;
            Vector2 lo = new(float.MaxValue), hi = new(float.MinValue);
            foreach (Vector2 q in pts) { lo = Vector2.Min(lo, q); hi = Vector2.Max(hi, q); }
            Vector2 d = Vector2.Max(lo - player, Vector2.Zero) + Vector2.Max(player - hi, Vector2.Zero);
            return d.X + d.Y;
        }
        Vector2[] a = place(true), b = place(false);
        Vector2[] p = distance(a) <= distance(b) ? a : b;
        Vector2[] triangles = new Vector2[tris.Count * 3];
        // Edges of one triangle only (by vertex index), with that triangle.
        Dictionary<(ushort, ushort), (int Count, int Tri)> edges = [];
        for (int i = 0; i < tris.Count; i++) {
            var (t0, t1, t2) = tris[i];
            triangles[3 * i] = p[t0]; triangles[3 * i + 1] = p[t1]; triangles[3 * i + 2] = p[t2];
            foreach (var (u, v) in new[] { (t0, t1), (t1, t2), (t2, t0) }) {
                var key = (Math.Min(u, v), Math.Max(u, v));
                edges[key] = (edges.GetValueOrDefault(key).Count + 1, i);
            }
        }
        // Not every such edge is the walkable area's edge: where the mesh has a seam (its
        // pieces meet with vertices of their own, or one long edge against two short ones),
        // other triangles lie on the far side. An edge stays only where the ground just
        // outside it is uncovered; it is checked in pieces, so a seam that runs on into a
        // real edge keeps its real part.
        List<Vector2> outline = [];
        int seams = 0;
        foreach (var ((u, v), e) in edges.OrderBy(e => e.Key)) {
            if (e.Count != 1) continue;
            int before = outline.Count;
            outer_parts(p[u], p[v], e.Tri, triangles, outline);
            if (outline.Count == before) seams++;
        }
        return new Ground(mesh, triangles, [.. outline], seams);
    }

    private const int EDGE_PIECES = 8;

    /// <summary>The parts of edge a-b of triangle <paramref name="own"/> with no other triangle just outside them, into <paramref name="outline"/>.</summary>
    private static void outer_parts(Vector2 a, Vector2 b, int own, Vector2[] triangles, List<Vector2> outline) {
        Vector2 d = b - a;
        float length = d.Length();
        if (length < 1e-4f) return;
        // Outward: away from the triangle's third corner.
        Vector2 c = triangles[3 * own] + triangles[3 * own + 1] + triangles[3 * own + 2] - a - b;
        Vector2 n = new(-d.Y / length, d.X / length);
        if (Vector2.Dot(c - a, n) > 0) n = -n;
        float step = Math.Clamp(length * 0.01f, 0.05f, 0.5f);
        int start = -1;
        for (int k = 0; k <= EDGE_PIECES; k++) {
            bool outer = k < EDGE_PIECES && !covered(a + d * ((k + 0.5f) / EDGE_PIECES) + n * step, own, triangles);
            if (outer && start < 0) start = k;
            if (!outer && start >= 0) {
                outline.Add(a + d * ((float)start / EDGE_PIECES));
                outline.Add(a + d * ((float)k / EDGE_PIECES));
                start = -1;
            }
        }
    }

    /// <summary>Whether a triangle other than <paramref name="own"/> holds the point.</summary>
    private static bool covered(Vector2 q, int own, Vector2[] triangles) {
        for (int i = 0; i + 2 < triangles.Length; i += 3) {
            if (i == 3 * own) continue;
            Vector2 t0 = triangles[i], t1 = triangles[i + 1], t2 = triangles[i + 2];
            if (q.X < MathF.Min(t0.X, MathF.Min(t1.X, t2.X)) || q.X > MathF.Max(t0.X, MathF.Max(t1.X, t2.X))
                || q.Y < MathF.Min(t0.Y, MathF.Min(t1.Y, t2.Y)) || q.Y > MathF.Max(t0.Y, MathF.Max(t1.Y, t2.Y))) continue;
            float d0 = cross(t1 - t0, q - t0), d1 = cross(t2 - t1, q - t1), d2 = cross(t0 - t2, q - t2);
            bool neg = d0 < 0 || d1 < 0 || d2 < 0, pos = d0 > 0 || d1 > 0 || d2 > 0;
            if (!(neg && pos)) return true;
        }
        return false;
    }

    private static float cross(Vector2 a, Vector2 b) => a.X * b.Y - a.Y * b.X;

    // ---- ImGui (render_imgui) -------------------------------------------------------------

    // The game's minimap colours: its dark green panel, light green grid, steel blue walkable
    // area, the rim, the player's yellow in a magenta ring.
    private static readonly Vector3 PANEL  = new(8, 30, 22);
    private static readonly Vector3 GRID   = new(92, 150, 110);
    private static readonly Vector3 GROUND = new(64, 100, 138);
    private static readonly Vector3 EDGE   = new(150, 190, 222);
    private static readonly Vector3 RIM    = new(110, 172, 132);
    private static readonly Vector3 PLAYER = new(226, 214, 72);
    private static readonly Vector3 RING   = new(232, 92, 232);

    private static Vector2[] _circle = [];
    private static readonly List<Vector2> _poly_a = [], _poly_b = [];

    /// <summary>The part of the screen the game's 16:9 picture covers (bars on wider or narrower screens).</summary>
    public static (Vector2 Min, Vector2 Size) picture(Vector2 screen) {
        const float ASPECT = 16f / 9f;
        if (screen.X > screen.Y * ASPECT * 1.01f) {
            float w = screen.Y * ASPECT;
            return (new((screen.X - w) / 2, 0), new(w, screen.Y));
        }
        if (screen.X < screen.Y * ASPECT * 0.99f) {
            float h = screen.X / ASPECT;
            return (new(0, (screen.Y - h) / 2), new(screen.X, h));
        }
        return (Vector2.Zero, screen);
    }

    private static uint col(Vector3 c, float a) =>
        (uint)c.X | (uint)c.Y << 8 | (uint)c.Z << 16 | (uint)Math.Clamp(MathF.Round(a * 255f), 0f, 255f) << 24;
    private static uint col(Vector4 c, float a) => col(new Vector3(c.X, c.Y, c.Z) * 255f, c.W * a);
    private static uint black(float a) => col(Vector3.Zero, a);

    /// <summary>Draws it when the game shows its minimap; returns whether it drew.</summary>
    public static bool draw(ImDrawListPtr draw, Vector2 screen) {
        Frame? f;
        Ground? ground;
        lock (_lock) {
            f = _frame;
            ground = _ground;
            if (_clock.ElapsedMilliseconds - _frame_at > STALE_MS) f = null;
        }
        if (f is null || ground is null || f.Player is null || f.Shown <= 0f) return false;
        float alpha = f.Shown;
        var (pic_min, pic_size) = picture(screen);
        float radius = MathF.Max(pic_size.Y * MinimapPatches.diameter * 0.5f, 20f);
        float margin = pic_size.Y * 0.04f;
        if (pic_size.X < 2 * (radius + margin) || pic_size.Y < 2 * (radius + margin)) return false;
        Corner corner = MinimapPatches.place;
        bool left = corner is Corner.TopLeft or Corner.BottomLeft, top = corner is Corner.TopLeft or Corner.TopRight;
        Vector2 centre = new(
            left ? pic_min.X + margin + radius : pic_min.X + pic_size.X - margin - radius,
            top  ? pic_min.Y + margin + radius : pic_min.Y + pic_size.Y - margin - radius);

        // Up is where the camera looks, on the ground; right is its right.
        Vector2 fwd = flat(f.Camera.EyeToLookAt, flat(f.Camera.Forward, new Vector2(0, 1)));
        Vector2 right = new(fwd.Y, -fwd.X);
        Vector2 me = new(f.Player.Pos.X, f.Player.Pos.Z);
        float scale = radius / RANGE;
        Vector2 to_px(Vector2 p) {
            Vector2 d = p - me;
            return new(Vector2.Dot(d, right) * scale, -Vector2.Dot(d, fwd) * scale);
        }
        float edge = radius - 1.5f;
        bool crt = MinimapPatches.crt;

        // The panel.
        draw.AddCircleFilled(centre, radius, col(PANEL, 0.74f * alpha), 64);

        // The grid, fixed to the world: four cells to the edge.
        float cell = RANGE / 4f;
        uint grid = col(GRID, 0.28f * alpha);
        for (int axis = 0; axis < 2; axis++) {
            float c0 = axis == 0 ? me.X : me.Y, c1 = axis == 0 ? me.Y : me.X;
            int first = (int)MathF.Floor((c0 - RANGE * 1.5f) / cell);
            for (int k = first; k < first + 13; k++) {
                float v = k * cell;
                Vector2 a = axis == 0 ? new(v, c1 - RANGE * 2) : new(c1 - RANGE * 2, v);
                Vector2 b = axis == 0 ? new(v, c1 + RANGE * 2) : new(c1 + RANGE * 2, v);
                if (inside(to_px(a), to_px(b), edge, out Vector2 p, out Vector2 q))
                    draw.AddLine(centre + p, centre + q, grid, 1f);
            }
        }

        // The walkable area, cut to the disc. Without anti-aliasing the triangles meet without seams.
        if (_circle.Length == 0 || _circle[0].X != edge) {
            _circle = new Vector2[64];
            for (int k = 0; k < 64; k++) {
                float t = k / 64f * MathF.Tau;
                _circle[k] = new(edge * MathF.Cos(t), edge * MathF.Sin(t));
            }
        }
        uint fill = col(GROUND, 0.66f * alpha);
        ImDrawListFlags old_flags = draw.Flags;
        draw.Flags = old_flags & ~ImDrawListFlags.AntiAliasedFill;
        Vector2* poly = stackalloc Vector2[80];
        for (int i = 0; i + 2 < ground.Triangles.Length; i += 3) {
            Vector2 t0 = to_px(ground.Triangles[i]), t1 = to_px(ground.Triangles[i + 1]), t2 = to_px(ground.Triangles[i + 2]);
            Vector2 lo = Vector2.Min(t0, Vector2.Min(t1, t2)), hi = Vector2.Max(t0, Vector2.Max(t1, t2));
            if (lo.X > edge || lo.Y > edge || hi.X < -edge || hi.Y < -edge) continue;
            // Counter-clockwise on screen (y down), as the circle.
            if ((t1.X - t0.X) * (t2.Y - t0.Y) - (t1.Y - t0.Y) * (t2.X - t0.X) < 0) (t1, t2) = (t2, t1);
            float e2 = edge * edge;
            int n;
            if (t0.LengthSquared() <= e2 && t1.LengthSquared() <= e2 && t2.LengthSquared() <= e2) {
                poly[0] = t0; poly[1] = t1; poly[2] = t2;
                n = 3;
            } else {
                cut([t0, t1, t2], _circle);
                n = Math.Min(_poly_a.Count, 80);
                for (int k = 0; k < n; k++) poly[k] = _poly_a[k];
            }
            if (n < 3) continue;
            for (int k = 0; k < n; k++) poly[k] += centre;
            draw.AddConvexPolyFilled(poly, n, fill);
        }
        draw.Flags = old_flags;

        // Its edge, with a phosphor glow.
        uint glow = col(EDGE, 0.16f * alpha), line = col(EDGE, 0.85f * alpha);
        for (int i = 0; i + 1 < ground.Outline.Length; i += 2) {
            if (!inside(to_px(ground.Outline[i]), to_px(ground.Outline[i + 1]), edge, out Vector2 p, out Vector2 q)) continue;
            if (crt) draw.AddLine(centre + p, centre + q, glow, 3f);
            draw.AddLine(centre + p, centre + q, line, 1f);
        }

        if (crt) screen_glass(draw, centre, radius, alpha);

        // The rim.
        draw.AddCircle(centre, radius + 1f, black(0.63f * alpha), 64, 1f);
        draw.AddCircle(centre, radius, col(RIM, 0.9f * alpha), 64, 2f);

        // The markers: inside at their place, beyond on the rim. Those with the blink flag
        // (0x1000) fade in and out as on the game's minimap (f.Blink).
        float icon = MathF.Max(radius * 0.075f, 4f);
        foreach (Marker k in f.Markers) {
            float a = (k.Flags & MinimapPatches.BLINK) != 0 ? alpha * f.Blink : alpha;
            if (a > 0f) marker(draw, k, to_px, centre, radius - icon, radius, icon, a);
        }

        // The player: an arrow of the character's facing in a ring.
        Vector2 facing = new(MathF.Cos(f.Player.Heading), MathF.Sin(f.Player.Heading));
        Vector2 dir = new(Vector2.Dot(facing, right), -Vector2.Dot(facing, fwd));
        dir = dir.LengthSquared() > 1e-8f ? Vector2.Normalize(dir) : new Vector2(0, -1);
        Vector2 side = new(-dir.Y, dir.X);
        float s = radius * 0.1f;
        draw.AddCircle(centre, s * 1.45f, col(RING, 0.18f * alpha), 32, 3f);
        draw.AddCircle(centre, s * 1.45f, col(RING, 0.95f * alpha), 32, 1.2f);
        Vector2 tip = centre + dir * s * 1.1f, back = centre - dir * s * 0.75f;
        Vector2* arrow = stackalloc Vector2[4];
        arrow[0] = tip;
        arrow[1] = back + side * s * 0.8f;
        arrow[2] = centre - dir * s * 0.3f;
        arrow[3] = back - side * s * 0.8f;
        uint yellow = f.Player.Colour.W > 0 ? col(f.Player.Colour, alpha / f.Player.Colour.W) : col(PLAYER, alpha);
        // Concave: two triangles.
        draw.AddTriangleFilled(arrow[0], arrow[1], arrow[2], yellow);
        draw.AddTriangleFilled(arrow[0], arrow[2], arrow[3], yellow);
        draw.AddPolyline(arrow, 4, black(0.78f * alpha), 1f, ImDrawFlags.Closed);
        return true;
    }

    /// <summary>The CRT screen: scanlines, a slow refresh band, darker toward the rim, a sheen on the glass.</summary>
    private static void screen_glass(ImDrawListPtr draw, Vector2 centre, float radius, float alpha) {
        uint lines = black(alpha * 70f / 255f);
        for (float y = -radius + 1f; y < radius; y += 3f) {
            float half = MathF.Sqrt(MathF.Max(radius * radius - y * y, 0f));
            draw.AddLine(centre + new Vector2(-half, y), centre + new Vector2(half, y), lines, 1f);
        }
        // One pass every four seconds.
        float band_at = _clock.ElapsedMilliseconds % 4000 / 4000f * 2.6f - 1.3f;
        for (int k = 0; k < 6; k++) {
            float y = band_at * radius + k * 2f;
            if (MathF.Abs(y) >= radius) continue;
            float half = MathF.Sqrt(radius * radius - y * y);
            float o = 0.035f * (1f - MathF.Abs(k - 2.5f) / 3f);
            draw.AddLine(centre + new Vector2(-half, y), centre + new Vector2(half, y), col(GRID, o * alpha), 2f);
        }
        for (int k = 0; k < 5; k++)
            draw.AddCircle(centre, radius - 1f - k * 2f, black((0.16f - k * 0.03f) * alpha), 64, 2f);
        Vector2* sheen = stackalloc Vector2[17];
        for (int k = 0; k <= 16; k++) {
            float t = MathF.PI * (1.05f + 0.4f * k / 16f);
            sheen[k] = centre + new Vector2(MathF.Cos(t), MathF.Sin(t)) * (radius - 5f);
        }
        draw.AddPolyline(sheen, 17, col(new Vector3(255, 255, 255), alpha * 22f / 255f), 2f, ImDrawFlags.None);
    }

    /// <summary>
    /// One marker: the game's shape and colour, restyled; exits as a bar where they are (an
    /// arrow on the rim beyond the edge); destinations always on the rim when away.
    /// </summary>
    private static void marker(ImDrawListPtr draw, Marker k, Func<Vector2, Vector2> to_px, Vector2 centre,
                               float inner, float rim, float s, float alpha) {
        uint colour(float o) => col(new Vector3(k.Colour.X, k.Colour.Y, k.Colour.Z) * 255f, o * alpha);
        uint dark(float o) => col(new Vector3(6, 18, 12), o * alpha * 0.9f);
        Vector2 p = to_px(new Vector2(k.Pos.X, k.Pos.Z));
        if (k.LineA is Vector2 la && k.LineB is Vector2 lb && inside(to_px(la), to_px(lb), inner, out Vector2 l0, out Vector2 l1)) {
            draw.AddLine(centre + l0, centre + l1, colour(0.95f), 3f);
            return;
        }
        Vector2 q;
        float o;
        if ((k.Flags & MinimapPatches.DESTINATION) != 0) {
            float d = MathF.Max(p.Length(), 1e-3f);
            (q, o) = d <= inner ? (p, 1f) : (p * (rim / d), 1f);
        } else {
            (q, o) = place(p, inner, rim, rim);
        }
        if (o <= 0.01f) return;
        Vector2 pos = centre + q;
        bool on_rim = q.Length() >= rim - 0.5f;
        if (k.LineA is not null || on_rim && (k.Flags & MinimapPatches.DESTINATION) != 0) {
            // An arrow pointing out, toward it.
            Vector2 out_ = q.LengthSquared() > 1e-6f ? Vector2.Normalize(q) : new Vector2(0, -1);
            Vector2 side = new(-out_.Y, out_.X);
            Vector2 a = pos + out_ * s * 1.2f, b = pos - out_ * s * 0.6f + side * s, c = pos - out_ * s * 0.6f - side * s;
            draw.AddTriangleFilled(a, b, c, colour(o));
            draw.AddTriangle(a, b, c, dark(o), 1f);
            return;
        }
        switch (k.Flags & MinimapPatches.SHAPE) {
            case MinimapPatches.DIAMOND: {
                Vector2 a = pos + new Vector2(0, -s), b = pos + new Vector2(s, 0), c = pos + new Vector2(0, s), d = pos + new Vector2(-s, 0);
                draw.AddQuadFilled(a, b, c, d, colour(o));
                draw.AddQuad(a, b, c, d, dark(o), 1f);
                break;
            }
            case MinimapPatches.SQUARE: {
                Vector2 h = new(s * 0.7f);
                draw.AddRectFilled(pos - h, pos + h, colour(o), 1f);
                draw.AddRect(pos - h - Vector2.One, pos + h + Vector2.One, dark(o), 1f);
                break;
            }
            case MinimapPatches.TRIANGLE: {
                Vector2 a = pos + new Vector2(0, -s), b = pos + new Vector2(s * 0.9f, s * 0.7f), c = pos + new Vector2(-s * 0.9f, s * 0.7f);
                draw.AddTriangleFilled(a, b, c, colour(o));
                draw.AddTriangle(a, b, c, dark(o), 1f);
                break;
            }
            default:
                draw.AddCircleFilled(pos, s * 0.75f, colour(o), 16);
                draw.AddCircle(pos, s * 0.75f, dark(o), 16, 1f);
                break;
        }
        if ((k.Flags & MinimapPatches.RING) != 0)
            draw.AddCircle(pos, s * 1.4f, col(k.Ring, o * 0.8f * alpha), 24, 1.5f);
    }

    // ---- geometry ---------------------------------------------------------------------------

    /// <summary>A world direction on the ground (x, z), normalised; <paramref name="or"/> when it has none.</summary>
    private static Vector2 flat(Vector3 v, Vector2 or) {
        Vector2 g = new(v.X, v.Z);
        float l = g.Length();
        return l < 1e-4f || !float.IsFinite(l) ? or : g / l;
    }

    /// <summary>
    /// Where a marker shows: within <paramref name="inner"/> of the centre at its place; further
    /// out on the rim, fading out over 1.5 radii beyond it (smoothstep).
    /// </summary>
    public static (Vector2 At, float Opacity) place(Vector2 p, float inner, float rim, float radius) {
        float d = p.Length();
        if (d <= inner) return (p, 1f);
        float fade = Math.Clamp(1f - MathF.Max(d - rim, 0f) / (radius * 1.5f), 0f, 1f);
        return (p * (rim / d), fade * fade * (3f - 2f * fade));
    }

    /// <summary>The part of the segment a-b inside the circle of radius r at the origin.</summary>
    public static bool inside(Vector2 a, Vector2 b, float r, out Vector2 p, out Vector2 q) {
        p = q = default;
        Vector2 d = b - a;
        float qa = d.LengthSquared(), qb = 2f * Vector2.Dot(a, d), qc = a.LengthSquared() - r * r;
        if (qa < 1e-9f) {
            if (qc > 0f) return false;
            (p, q) = (a, b);
            return true;
        }
        float disc = qb * qb - 4f * qa * qc;
        if (disc < 0f) return false;
        float sq = MathF.Sqrt(disc);
        float t0 = MathF.Max((-qb - sq) / (2f * qa), 0f), t1 = MathF.Min((-qb + sq) / (2f * qa), 1f);
        if (t0 >= t1) return false;
        (p, q) = (a + d * t0, a + d * t1);
        return true;
    }

    /// <summary>The convex polygon cut to the inside of the convex <paramref name="clip"/> (same winding), into _poly_a.</summary>
    public static void cut(ReadOnlySpan<Vector2> poly, Vector2[] clip) {
        _poly_a.Clear();
        foreach (Vector2 v in poly) _poly_a.Add(v);
        for (int k = 0; k < clip.Length && _poly_a.Count > 0; k++) {
            Vector2 a = clip[k], b = clip[(k + 1) % clip.Length];
            float side(Vector2 v) => (b.X - a.X) * (v.Y - a.Y) - (b.Y - a.Y) * (v.X - a.X);
            _poly_b.Clear();
            for (int i = 0; i < _poly_a.Count; i++) {
                Vector2 p = _poly_a[i], q = _poly_a[(i + 1) % _poly_a.Count];
                float sp = side(p), sq = side(q);
                if (sp >= 0f) _poly_b.Add(p);
                if (sp >= 0f != sq >= 0f) _poly_b.Add(p + (q - p) * (sp / (sp - sq)));
            }
            _poly_a.Clear();
            _poly_a.AddRange(_poly_b);
        }
    }
}
