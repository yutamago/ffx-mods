//! Paints PS2 texels into an HD model's texture pages.
//!
//! For a model that kept its PS2 mesh, every HD triangle has a PS2 twin
//! with the same three corners (HD position × 1000 = PS2 `i16`, after one
//! axis permutation, sign flip and offset per model). The HD triangle is
//! rasterised in its own UV space and each pixel reads the PS2 image through
//! the PS2 triangle's palette at the interpolated PS2 UV. Pages that were
//! re-laid out in the remaster (atlases merged or cropped) come out right
//! without knowing the layout.

use std::collections::HashMap;

use crate::phyretex::Rgba;
use crate::ps2chr::{Chr, Triangle};

type P = [i32; 3];

/// An axis permutation and sign flip, then an offset: PS2 = R(HD × 1000) + t.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    perm: [usize; 3],
    sign: [i32; 3],
    t: P,
}

impl Frame {
    fn apply(&self, hd: [f32; 3]) -> P {
        let s = hd.map(|c| (c * 1000.0).round() as i32);
        [0, 1, 2].map(|k| self.sign[k] * s[self.perm[k]] + self.t[k])
    }
}

/// Snaps a rounded position to a PS2 vertex at most 1 unit away on each
/// axis (rotated HD floats round off by one now and then).
fn snap(set: &std::collections::HashSet<P>, q: P) -> Option<P> {
    if set.contains(&q) {
        return Some(q);
    }
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let p = [q[0] + dx, q[1] + dy, q[2] + dz];
                if set.contains(&p) {
                    return Some(p);
                }
            }
        }
    }
    None
}

const PERMS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// Finds the frame under which the most HD vertices land on a PS2 vertex.
/// Returns it with the share of HD vertices that match.
pub fn find_frame(ps2: &[P], hd: &[[f32; 3]]) -> Option<(Frame, f32)> {
    if ps2.is_empty() || hd.is_empty() {
        return None;
    }
    let set: std::collections::HashSet<P> = ps2.iter().copied().collect();
    // Vote with unique positions: meshes repeat a position for every UV seam
    // and normal, and repeated pairs would outvote the real offset.
    let mut hd_unique: Vec<P> = hd
        .iter()
        .map(|p| p.map(|c| (c * 1000.0).round() as i32))
        .collect();
    hd_unique.sort_unstable();
    hd_unique.dedup();
    let sample: Vec<[f32; 3]> = hd_unique
        .iter()
        .step_by((hd_unique.len() / 150).max(1))
        .map(|p| p.map(|c| c as f32 / 1000.0))
        .collect();
    let mut ps2_unique: Vec<P> = set.iter().copied().collect();
    ps2_unique.sort_unstable();
    let ps2_sample: Vec<P> = ps2_unique
        .iter()
        .step_by((ps2_unique.len() / 600).max(1))
        .copied()
        .collect();
    let mut best: Option<(Frame, f32)> = None;
    for perm in PERMS {
        for bits in 0..8 {
            let sign = [0, 1, 2].map(|k| if bits >> k & 1 == 1 { -1 } else { 1 });
            let zero = Frame {
                perm,
                sign,
                t: [0; 3],
            };
            // vote for the offset
            let mut votes: HashMap<P, u32> = HashMap::new();
            for &h in &sample {
                let r = zero.apply(h);
                for &p in &ps2_sample {
                    *votes
                        .entry([p[0] - r[0], p[1] - r[1], p[2] - r[2]])
                        .or_default() += 1;
                }
            }
            let Some((&t, _)) = votes.iter().max_by_key(|&(_, &n)| n) else {
                continue;
            };
            let f = Frame { perm, sign, t };
            let hits = hd
                .iter()
                .filter(|&&h| snap(&set, f.apply(h)).is_some())
                .count();
            let share = hits as f32 / hd.len() as f32;
            if best.is_none_or(|(_, s)| share > s) {
                best = Some((f, share));
            }
        }
    }
    best
}

fn key(mut c: [P; 3]) -> [P; 3] {
    c.sort_unstable();
    c
}

/// The PS2 triangles by their (sorted) corner positions, and every PS2
/// corner (material, UV) by position.
pub struct Index<'a> {
    tris: HashMap<[P; 3], Vec<&'a Triangle>>,
    corners: HashMap<P, Vec<(usize, [f32; 2])>>,
    vertices: std::collections::HashSet<P>,
}

impl<'a> Index<'a> {
    pub fn new(chr: &'a Chr) -> Self {
        let mut tris: HashMap<[P; 3], Vec<&Triangle>> = HashMap::new();
        let mut corners: HashMap<P, Vec<(usize, [f32; 2])>> = HashMap::new();
        for t in &chr.triangles {
            let c = t.corners.map(|c| c.pos.map(i32::from));
            if c[0] == c[1] || c[1] == c[2] || c[0] == c[2] {
                continue; // strip degenerates
            }
            tris.entry(key(c)).or_default().push(t);
            for k in &t.corners {
                let e = corners.entry(k.pos.map(i32::from)).or_default();
                let uv = [f32::from(k.uv[0]), f32::from(k.uv[1])];
                if !e.contains(&(t.material, uv)) {
                    e.push((t.material, uv));
                }
            }
        }
        let vertices = corners.keys().copied().collect();
        Self {
            tris,
            corners,
            vertices,
        }
    }

    /// The PS2 triangle with these corners: its material and, per HD
    /// corner, the PS2 corner's UV.
    fn find(&self, corners: [P; 3]) -> Option<(usize, [[f32; 2]; 3])> {
        let t = *self.tris.get(&key(corners))?.first()?;
        let uv = corners.map(|p| {
            let c = t
                .corners
                .iter()
                .find(|c| c.pos.map(i32::from) == p)
                .expect("same corners");
            [f32::from(c.uv[0]), f32::from(c.uv[1])]
        });
        Some((t.material, uv))
    }
}

/// An HD triangle to paint: its UVs (as stored, v up), the PS2 material
/// and the PS2 UVs at its corners.
pub struct Job {
    pub hd_uv: [[f32; 2]; 3],
    pub material: usize,
    pub ps2_uv: [[f32; 2]; 3],
}

/// An HD triangle whose corners are PS2 vertices but which has no PS2
/// twin (the remaster split a quad the other way).
pub struct Pending {
    pub hd_uv: [[f32; 2]; 3],
    pub corners: [P; 3],
}

/// Matches HD triangles (`positions`, `uvs`, triangle `indices`) to PS2
/// ones. Returns the matches, the triangles left for [`second_pass`] and
/// how many HD triangles there were.
pub fn match_triangles(
    index: &Index,
    frame: &Frame,
    positions: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u16],
) -> (Vec<Job>, Vec<Pending>, usize) {
    let mut jobs = Vec::new();
    let mut pending = Vec::new();
    let tris = indices.len() / 3;
    for t in indices.as_chunks::<3>().0 {
        let i = t.map(usize::from);
        let Some(c0) = snap(&index.vertices, frame.apply(positions[i[0]])) else {
            continue;
        };
        let Some(c1) = snap(&index.vertices, frame.apply(positions[i[1]])) else {
            continue;
        };
        let Some(c2) = snap(&index.vertices, frame.apply(positions[i[2]])) else {
            continue;
        };
        let hd_uv = i.map(|k| uvs[k]);
        match index.find([c0, c1, c2]) {
            Some((material, ps2_uv)) => jobs.push(Job {
                hd_uv,
                material,
                ps2_uv,
            }),
            None => pending.push(Pending {
                hd_uv,
                corners: [c0, c1, c2],
            }),
        }
    }
    (jobs, pending, tris)
}

/// A page's UV map for one PS2 material: HD = s * PS2 / 4096 + o, per axis.
#[derive(Debug, Clone, Copy, PartialEq)]
struct UvMap {
    material: usize,
    s: [f32; 2],
    o: [f32; 2],
}

impl UvMap {
    fn apply(&self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.s[0] * uv[0] / 4096.0 + self.o[0],
            self.s[1] * uv[1] / 4096.0 + self.o[1],
        ]
    }
}

/// The UV maps the exact matches agree on (scale and offset rounded to
/// 1/256), most used first.
fn learn_maps(jobs: &[Job]) -> Vec<UvMap> {
    let q = |x: f32| (x * 256.0).round() as i32;
    let mut votes: HashMap<(usize, [i32; 4]), u32> = HashMap::new();
    for j in jobs {
        let mut s = [0f32; 2];
        let mut ok = true;
        for a in 0..2 {
            // the corner pair with the largest PS2 extent along this axis
            let (p, h) = (j.ps2_uv.map(|u| u[a] / 4096.0), j.hd_uv.map(|u| u[a]));
            let (x, y) = [(0, 1), (0, 2), (1, 2)]
                .into_iter()
                .max_by(|&(a1, b1), &(a2, b2)| {
                    (p[b1] - p[a1]).abs().total_cmp(&(p[b2] - p[a2]).abs())
                })
                .expect("pairs");
            if (p[y] - p[x]).abs() < 1e-4 {
                ok = false;
                break;
            }
            s[a] = (h[y] - h[x]) / (p[y] - p[x]);
        }
        if !ok {
            continue;
        }
        let o = [0, 1].map(|a| j.hd_uv[0][a] - s[a] * j.ps2_uv[0][a] / 4096.0);
        *votes
            .entry((j.material, [q(s[0]), q(s[1]), q(o[0]), q(o[1])]))
            .or_default() += 1;
    }
    let mut maps: Vec<(u32, UvMap)> = votes
        .into_iter()
        .filter(|&(_, n)| n >= 3)
        .map(|((material, k), n)| {
            let f = |v: i32| v as f32 / 256.0;
            (
                n,
                UvMap {
                    material,
                    s: [f(k[0]), f(k[1])],
                    o: [f(k[2]), f(k[3])],
                },
            )
        })
        .collect();
    maps.sort_by_key(|m| std::cmp::Reverse(m.0));
    maps.into_iter().map(|(_, m)| m).collect()
}

/// Pairs the pending triangles with PS2 corners through the page's UV maps:
/// at each corner the PS2 (material, UV) whose mapped UV lands closest to
/// the HD UV, accepted when all three land within `tolerance` (in UV units).
pub fn second_pass(index: &Index, exact: &[Job], pending: &[Pending], tolerance: f32) -> Vec<Job> {
    let maps = learn_maps(exact);
    let mut out = Vec::new();
    for p in pending {
        let mut best: Option<(f32, Job)> = None;
        for m in &maps {
            let mut ps2_uv = [[0f32; 2]; 3];
            let mut err = 0f32;
            let mut ok = true;
            for (k, slot) in ps2_uv.iter_mut().enumerate() {
                let Some(cands) = index.corners.get(&p.corners[k]) else {
                    ok = false;
                    break;
                };
                let pick = cands
                    .iter()
                    .filter(|(mat, _)| *mat == m.material)
                    .map(|&(_, uv)| {
                        let h = m.apply(uv);
                        (
                            (h[0] - p.hd_uv[k][0])
                                .abs()
                                .max((h[1] - p.hd_uv[k][1]).abs()),
                            uv,
                        )
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                match pick {
                    Some((e, uv)) if e <= tolerance => {
                        err += e;
                        *slot = uv;
                    }
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok && best.as_ref().is_none_or(|(e, _)| err < *e) {
                best = Some((
                    err,
                    Job {
                        hd_uv: p.hd_uv,
                        material: m.material,
                        ps2_uv,
                    },
                ));
            }
        }
        if let Some((_, j)) = best {
            out.push(j);
        }
    }
    out
}

/// How many HD page pixels one PS2 texel covers along each axis, rounded
/// to a power of two: the HD page is painted at `size / scale`.
pub fn density_scale(jobs: &[Job], page: (u32, u32)) -> u32 {
    let area = |p: [[f32; 2]; 3]| {
        ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]))
            .abs()
            / 2.0
    };
    let mut ratios: Vec<f32> = jobs
        .iter()
        .filter_map(|j| {
            let ps2 = area(j.ps2_uv.map(|[u, v]| [u / 16.0, v / 16.0]));
            let hd = area(j.hd_uv.map(|[u, v]| [u * page.0 as f32, v * page.1 as f32]));
            (ps2 > 1.0).then(|| hd / ps2)
        })
        .collect();
    if ratios.is_empty() {
        return 1;
    }
    ratios.sort_by(f32::total_cmp);
    let r = ratios[ratios.len() / 2].sqrt().max(1.0);
    1 << r.log2().round() as u32
}

/// Box-filters `src` down to `size` (an integer factor smaller).
pub fn downscale(src: &Rgba, size: (u32, u32)) -> Rgba {
    let (w, h) = size;
    let (fx, fy) = ((src.width / w).max(1), (src.height / h).max(1));
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for dy in 0..fy {
                for dx in 0..fx {
                    let c = src.get(
                        (x * fx + dx).min(src.width - 1),
                        (y * fy + dy).min(src.height - 1),
                    );
                    for k in 0..4 {
                        acc[k] += u32::from(c[k]);
                    }
                }
            }
            out.put(x, y, acc.map(|v| (v / (fx * fy)) as u8));
        }
    }
    out
}

/// Paints the jobs into a page of `size`. Pixels no triangle covers keep
/// `background` (the HD page scaled down), or without one are filled from
/// their neighbours. Returns the image and the share painted from the PS2.
pub fn paint(chr: &Chr, jobs: &[Job], size: (u32, u32), background: Option<&Rgba>) -> (Rgba, f32) {
    let (w, h) = size;
    let mut img = background.map_or_else(|| Rgba::new(w, h), |b| downscale(b, size));
    let mut done = vec![false; (w * h) as usize];
    for j in jobs {
        // page pixel coordinates, rows top first (v up in the file)
        let p = j.hd_uv.map(|[u, v]| [u * w as f32, (1.0 - v) * h as f32]);
        let (x0, x1) = (
            p.iter().map(|q| q[0]).fold(f32::MAX, f32::min),
            p.iter().map(|q| q[0]).fold(f32::MIN, f32::max),
        );
        let (y0, y1) = (
            p.iter().map(|q| q[1]).fold(f32::MAX, f32::min),
            p.iter().map(|q| q[1]).fold(f32::MIN, f32::max),
        );
        let d =
            (p[1][1] - p[2][1]) * (p[0][0] - p[2][0]) + (p[2][0] - p[1][0]) * (p[0][1] - p[2][1]);
        if d.abs() < 1e-9 {
            continue;
        }
        // half a pixel of slack so seams between triangles get covered
        let pad = 0.5;
        for y in ((y0 - pad).floor().max(0.0) as u32)..((y1 + pad).ceil().min(h as f32) as u32) {
            for x in ((x0 - pad).floor().max(0.0) as u32)..((x1 + pad).ceil().min(w as f32) as u32)
            {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let a = ((p[1][1] - p[2][1]) * (px - p[2][0])
                    + (p[2][0] - p[1][0]) * (py - p[2][1]))
                    / d;
                let b = ((p[2][1] - p[0][1]) * (px - p[2][0])
                    + (p[0][0] - p[2][0]) * (py - p[2][1]))
                    / d;
                let c = 1.0 - a - b;
                let inside = a >= 0.0 && b >= 0.0 && c >= 0.0;
                let i = (y * w + x) as usize;
                if !inside && (done[i] || a.min(b).min(c) < -0.05) {
                    continue;
                }
                let u = a * j.ps2_uv[0][0] + b * j.ps2_uv[1][0] + c * j.ps2_uv[2][0];
                let v = a * j.ps2_uv[0][1] + b * j.ps2_uv[1][1] + c * j.ps2_uv[2][1];
                if let Some(col) = chr.sample(j.material, u, v) {
                    img.put(x, y, col);
                    done[i] = true;
                }
            }
        }
    }
    let covered = done.iter().filter(|&&d| d).count() as f32 / done.len() as f32;
    if background.is_none() {
        fill_holes(&mut img, &mut done);
    }
    (img, covered)
}

/// Grows painted pixels into unpainted ones until none are left.
fn fill_holes(img: &mut Rgba, done: &mut [bool]) {
    let (w, h) = (img.width as i64, img.height as i64);
    if !done.iter().any(|&d| d) {
        return;
    }
    loop {
        let mut next = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if done[(y * w + x) as usize] {
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h && done[(ny * w + nx) as usize] {
                        next.push((x, y, img.get(nx as u32, ny as u32)));
                        break;
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        for (x, y, c) in next {
            img.put(x as u32, y as u32, c);
            done[(y * w + x) as usize] = true;
        }
    }
}
