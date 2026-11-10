//! PS2 character meshes as Phyre models: the PS2 mesh, skinned to the
//! PS2 skeleton the game animates (PS2 bone i = node `SkeletonNode_i`),
//! with the PS2 textures, written into the HD model in place of its
//! remodelled geometry. Notes: `docs/models.md`.
//!
//! Per character:
//! 1. Every PS2 vertex is put in the bind pose: its model's base bone's
//!    world matrix (taken from the HD nodes, which equal the PS2 bind pose
//!    / 1000) times the stored vertex / 1000. Its bones and weights come
//!    from the skin lists (at most 4, the largest, renormalised).
//! 2. The PS2 textures used (image + palette pairs) are packed into one
//!    ARGB8 atlas at PS2 size, which replaces the base texture of the HD
//!    segment chosen as template.
//! 3. The triangles go into the template segment and into new segments
//!    copied from it (bone palettes are kept under [`MAX_PALETTE`]),
//!    through `ffx_phyre::import_into_cluster`.
//! 4. All other HD segments keep their vertices but lose their triangles
//!    (cloth segments get vertex data uploaded by index every frame, so
//!    their vertex counts stay).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result, bail, ensure};

use crate::characters::alpha;
use crate::config::Config;
use crate::disc::Disc;
use crate::jobs::{Ctx, Job, Output};
use crate::formats::glb::{Glb, Primitive};
use crate::formats::phyretex::{self, Rgba};
use crate::formats::ps2chr::Chr;
use crate::formats::ps2mesh::{self, Corner, Mesh};
use crate::characters::texanim::{self, Rect, TexAnim};

/// Most bones one new segment's palette may name (the HD models go up to
/// 75).
pub const MAX_PALETTE: usize = 64;
/// Edge texels repeated around each atlas page against filtering bleed.
const PAD: u32 = 2;

/// World matrices of the HD model's nodes (column-major, f64).
pub fn node_worlds(model: &ffx_phyre::Model) -> Vec<[f64; 16]> {
    let mut out: Vec<Option<[f64; 16]>> = vec![None; model.nodes.len()];
    fn get(model: &ffx_phyre::Model, out: &mut [Option<[f64; 16]>], i: usize) -> [f64; 16] {
        if let Some(m) = out[i] {
            return m;
        }
        let local = model.nodes[i].local.map(f64::from);
        let m = match model.nodes[i].parent {
            Some(p) if p != i => ps2mesh::mul(&get(model, out, p), &local),
            _ => local,
        };
        out[i] = Some(m);
        m
    }
    (0..model.nodes.len()).map(|i| get(model, &mut out, i)).collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn normalize(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 1e-12 { [a[0] / l, a[1] / l, a[2] / l] } else { [0.0, 1.0, 0.0] }
}

/// Share of triangles whose face normal (by winding) points along their
/// vertex normals.
fn winding_agreement(tris: impl Iterator<Item = ([[f64; 3]; 3], [f64; 3])>) -> f64 {
    let (mut yes, mut all) = (0usize, 0usize);
    for (p, n) in tris {
        let g = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let d = dot(g, n);
        if d.abs() < 1e-12 {
            continue;
        }
        all += 1;
        if d > 0.0 {
            yes += 1;
        }
    }
    yes as f64 / all.max(1) as f64
}

/// A placed atlas page.
#[derive(Debug, Clone, Copy)]
struct Placed {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Shelf-packs pages (with [`PAD`] around each) into an atlas no wider
/// than the widest page or 512.
fn pack(sizes: &[(u32, u32)]) -> (u32, u32, Vec<Placed>) {
    let width = sizes.iter().map(|s| s.0 + 2 * PAD).max().unwrap_or(1).max(512 + 2 * PAD);
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse((sizes[i].1, sizes[i].0)));
    let mut placed = vec![Placed { x: 0, y: 0, w: 0, h: 0 }; sizes.len()];
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for i in order {
        let (w, h) = (sizes[i].0 + 2 * PAD, sizes[i].1 + 2 * PAD);
        if x + w > width {
            y += shelf;
            x = 0;
            shelf = 0;
        }
        placed[i] = Placed { x: x + PAD, y: y + PAD, w: sizes[i].0, h: sizes[i].1 };
        x += w;
        shelf = shelf.max(h);
    }
    let used_w = placed.iter().map(|p| p.x + p.w + PAD).max().unwrap_or(1);
    (used_w, y + shelf, placed)
}

/// One finished character.
pub struct Converted {
    /// (archive path, file) to write under `efl/x`.
    pub files: Vec<(String, Vec<u8>)>,
    pub atlas: Rgba,
    /// Texture animation frames (name tag, image), for previews.
    pub anim: Vec<(String, Rgba)>,
    pub glb: Vec<u8>,
    pub report: String,
}

/// What a PS2 vertex becomes.
#[derive(Clone, Copy)]
struct Vert {
    pos: [f64; 3],
    normal: [f64; 3],
    bones: [(u16, f32); 4],
}

/// Whether a character's HD model is a remodel this converter replaces:
/// the texture bake skips at least one of its pages and under 60 % of its
/// HD vertices sit on a PS2 vertex (`docs/models.md`, "What gets
/// converted").
pub fn remodelled(chr: &[u8], pc: &ffx_vbf::Archive, kind: &str, name: &str) -> Result<bool> {
    Ok(crate::jobs::character_textures::skipped_chr(chr, pc, kind, name)?
        .is_some_and(|r| !r.skipped.is_empty() && r.frame_share < 0.6))
}

/// One build job per character kind (`models.party`, ...): the PS2 mesh
/// into every remodelled HD model of that kind. The texture-swap jobs
/// (`chr.*`) cover the models whose mesh was kept.
struct ModelsJob {
    name: &'static str,
    settings: &'static [&'static str],
    kinds: &'static [&'static str],
    enabled: fn(&Config) -> bool,
}

pub fn jobs() -> Vec<Box<dyn Job>> {
    fn job(
        name: &'static str,
        settings: &'static [&'static str],
        kinds: &'static [&'static str],
        enabled: fn(&Config) -> bool,
    ) -> Box<dyn Job> {
        Box::new(ModelsJob { name, settings, kinds, enabled })
    }
    vec![
        job("models.party", &["models.party"], &["pc"], |c| c.models.party.ps2()),
        job("models.monster", &["models.monster"], &["mon"], |c| c.models.monster.ps2()),
        job("models.npc", &["models.npc"], &["npc"], |c| c.models.npc.ps2()),
        job("models.aeon", &["models.aeon"], &["sum"], |c| c.models.aeon.ps2()),
        job("models.weapon", &["models.weapon"], &["wep"], |c| c.models.weapon.ps2()),
        job("models.object", &["models.object"], &["obj", "skl"], |c| c.models.object.ps2()),
    ]
}

impl Job for ModelsJob {
    /// 2: textures with mip chains (phyretex::mip_chain). 3: the body in a
    /// template material that does not blend. 4: textures in one level (no
    /// quality drop), see-through parts blended with the PS2's alpha test,
    /// PS2 cloth anchors, pieces the HD has another shape for skinned.
    /// 5: the `.cdf` lists the PS2 cloth triangles; the PS2 pins the first
    /// run of a cloth entry, not the second. 6: a blended twin the HD model
    /// does not import takes over an import of a material no segment uses
    /// (appended imports are not resolved: those parts were not drawn); new
    /// `.ahwin32` entries go before the model, not before the animation
    /// textures.
    /// 7: cloth triangles the PS2 draws after another piece whose segment
    /// comes later go into a tail segment after it (Kimahri's mane's
    /// fringe over the fur collar).
    fn version(&self) -> u32 {
        7
    }
    fn name(&self) -> &'static str {
        self.name
    }

    fn settings(&self) -> &'static [&'static str] {
        self.settings
    }

    fn enabled(&self, config: &Config) -> bool {
        (self.enabled)(config)
    }

    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        use rayon::prelude::*;
        let names: Vec<(String, String)> = ctx.disc(|d| {
            Ok(crate::jobs::character_textures::models(d)
                .into_iter()
                .filter(|(k, _)| self.kinds.contains(&k.as_str()))
                .collect())
        })?;
        let (mut done, mut failed) = (0, 0);
        // in chunks, written in name order: every build writes the same files
        for chunk in names.chunks(16) {
            let results: Vec<Result<Option<Converted>>> = chunk
                .par_iter()
                .map(|(kind, name)| {
                    let Some(chr) =
                        ctx.disc(|d| d.read(&format!("jppc/chr/{kind}/{name}/mdl/{name}.chr")))?
                    else {
                        return Ok(None);
                    };
                    if !remodelled(&chr, ctx.pc, kind, name).unwrap_or(false) {
                        return Ok(None);
                    }
                    let hd_path =
                        format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
                    let original = ctx.pc.read(ctx.pc.find(&hd_path).context("no HD model")?)?;
                    let variants = alpha::Variants::get(ctx.pc)?;
                    convert_bytes(&chr, &original, &hd_path, variants, |p| {
                        ctx.pc.find(p).map(|e| ctx.pc.read(e)).transpose().map_err(Into::into)
                    })
                    .map(Some)
                })
                .collect();
            for ((kind, name), r) in chunk.iter().zip(results) {
                match r {
                    Ok(None) => {}
                    Ok(Some(c)) => {
                        ctx.note(format!("{kind}/{name}:\n{}", c.report.trim_end()));
                        for (path, bytes) in &c.files {
                            out.put(path, bytes)?;
                        }
                        done += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        ctx.note(format!("{kind}/{name}: left HD: {e:#}"));
                    }
                }
            }
        }
        let mut s = format!("{done} remodelled models given the PS2 mesh");
        if failed > 0 {
            s += &format!(", {failed} failed and left HD (--verbose lists them)");
        }
        Ok(s)
    }
}

pub fn convert(disc: &mut Disc, pc: &ffx_vbf::Archive, kind: &str, name: &str) -> Result<Option<Converted>> {
    let Some(chr_bytes) = disc.read(&format!("jppc/chr/{kind}/{name}/mdl/{name}.chr"))? else {
        return Ok(None);
    };
    let hd_path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
    let Some(entry) = pc.find(&hd_path) else {
        return Ok(None);
    };
    let original = pc.read(entry)?;
    let variants = alpha::Variants::get(pc)?;
    convert_bytes(&chr_bytes, &original, &hd_path, variants, |p| {
        pc.find(p).map(|e| pc.read(e)).transpose().map_err(Into::into)
    })
    .map(Some)
}

/// The face of a character with texture animation: its triangles go into
/// the HD's animated material (`Mat_CTA_…`), which shows the PS2 patches
/// over the rectangles the PS2 blits them into (`characters/texanim.rs`).
struct Face {
    /// HD instance and segment of the animated material, and the material.
    instance: usize,
    segment: usize,
    material: usize,
    /// Texture-list entry of the animated image.
    page: u8,
    /// Where patches go, in that page's texels.
    rects: Vec<Rect>,
    /// `_MAYA` shaders: the one rectangle (T, S) every layer uses.
    maya: Option<([f32; 2], [f32; 2])>,
    /// Import id of the texture the HD material samples before any frame
    /// is set (`…_anim_n0_p0.dds`), and its tag (`n0_p0`).
    anim_id: String,
    anim_tag: String,
    /// The animation textures, by id.
    textures: Vec<Rgba>,
    records: Vec<texanim::Record>,
}

impl Face {
    /// Whether triangle `t` samples a rectangle a patch goes into.
    fn covers(&self, t: &ps2mesh::Tri) -> bool {
        if t.texture != self.page {
            return false;
        }
        let u = t.corners.map(|c| f64::from(c.uv[0]) / 16.0);
        let v = t.corners.map(|c| f64::from(c.uv[1]) / 16.0);
        let (u0, u1) = (u.iter().copied().fold(f64::MAX, f64::min), u.iter().copied().fold(f64::MIN, f64::max));
        let (v0, v1) = (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
        self.rects.iter().any(|r| {
            u0 < f64::from(r.x + r.w) && u1 > f64::from(r.x) && v0 < f64::from(r.y + r.h) && v1 > f64::from(r.y)
        })
    }
}

/// Where a PS2 cloth piece goes: the HD segment tagged with it (or a new
/// copy of an HD cloth segment with `material`), tagged `(model, piece)`.
struct ClothTarget {
    tag: (u32, u32),
    segment: Option<usize>,
    material: usize,
    /// Kept skinned, not simulated: the `.cdf` cannot list its triangles
    /// (none, or more than [`CDF_MAX_TRIANGLES`]).
    skinned: bool,
}

/// The most triangles of a piece the PC searches when it binds a vertex
/// (`InsideTriangle` `0x00828c10`: its scratch arrays hold 0x300).
const CDF_MAX_TRIANGLES: usize = 0x300;

/// A `.cdf` (`Ch_ClothMeshMapping` `0x00826340`): the bounding box (6
/// `f32`), then per piece `u16 model, piece, count, pad` and `count`
/// triangles of 32 bytes; a model `ffff` ends it.
struct Cdf {
    head: Vec<u8>,
    /// ((model, piece), pad, triangle records)
    pieces: Vec<((u16, u16), u16, Vec<u8>)>,
    tail: Vec<u8>,
}

impl Cdf {
    fn parse(b: &[u8]) -> Result<Self> {
        let u = |o: usize| -> Result<u16> { Ok(u16::from_le_bytes(b.get(o..o + 2).context("cdf: short")?.try_into()?)) };
        let mut pieces = Vec::new();
        let mut o = 24;
        while u(o)? != 0xffff {
            let n = usize::from(u(o + 4)?);
            let tris = b.get(o + 8..o + 8 + 32 * n).context("cdf: short")?.to_vec();
            pieces.push(((u(o)?, u(o + 2)?), u(o + 6)?, tris));
            o += 8 + 32 * n;
        }
        Ok(Self { head: b[..24].to_vec(), pieces, tail: b[o..].to_vec() })
    }

    fn write(&self) -> Vec<u8> {
        let mut b = self.head.clone();
        for ((m, p), pad, tris) in &self.pieces {
            for x in [*m, *p, (tris.len() / 32) as u16, *pad] {
                b.extend_from_slice(&x.to_le_bytes());
            }
            b.extend_from_slice(tris);
        }
        b.extend_from_slice(&self.tail);
        b
    }
}

/// The PS2 triangles of a cloth piece as `.cdf` records: the model's
/// triangles whose three corners are vertices of the piece (not
/// degenerate), each `u16 v0, v1, v2`, the tag `100 + piece` (sub-part 1:
/// `passThisTriangle` `0x00829870` lets every sub-part through to a
/// segment tagged with the piece itself, and some characters' pieces only
/// sub-part 1), the corners' PS2 UVs (1/16 texel) as `f32` pairs.
fn cdf_triangles(m: &ps2mesh::Model, c: &ps2mesh::Cloth, piece: u16) -> Vec<u8> {
    let set: BTreeSet<u16> = c.vertices.iter().copied().collect();
    let mut b = Vec::new();
    for t in &m.tris {
        let v = t.corners.map(|c| c.vertex);
        if v[0] == v[1] || v[1] == v[2] || v[0] == v[2] || !v.iter().all(|x| set.contains(x)) {
            continue;
        }
        for x in v.into_iter().chain([100 + piece]) {
            b.extend_from_slice(&x.to_le_bytes());
        }
        for c in &t.corners {
            for x in c.uv {
                b.extend_from_slice(&f32::from(x).to_le_bytes());
            }
        }
    }
    b
}

/// Per segment, its cloth piece (`m_clothmodelIndex`, `m_clothIndex`;
/// `None` for -1).
fn cloth_tags(cluster: &[u8]) -> Result<Vec<Option<(u32, u32)>>> {
    let edit = ffx_phyre::edit::ClusterEdit::parse(cluster).context("model")?;
    let list = edit.list_of("PMeshSegment").context("no segments")?;
    let c = &edit.cluster;
    let (Some(m), Some(p)) = (c.member_offset("PMeshSegment", "m_clothmodelIndex"), c.member_offset("PMeshSegment", "m_clothIndex"))
    else {
        return Ok(vec![None; edit.lists[list].count() as usize]);
    };
    let l = &edit.lists[list];
    Ok((0..l.count())
        .map(|i| {
            let o = l.object(i);
            let at = |k: u32| u32::from_le_bytes(o[k as usize..k as usize + 4].try_into().expect("4 bytes"));
            let (a, b) = (at(m), at(p));
            (a != u32::MAX && b != u32::MAX).then_some((a, b))
        })
        .collect())
}

fn set_cloth_tag(edit: &mut ffx_phyre::edit::ClusterEdit, segment: usize, (a, b): (u32, u32)) -> Result<()> {
    let list = edit.list_of("PMeshSegment").context("no segments")?;
    let m = edit.cluster.member_offset("PMeshSegment", "m_clothmodelIndex").context("no m_clothmodelIndex")? as usize;
    let p = edit.cluster.member_offset("PMeshSegment", "m_clothIndex").context("no m_clothIndex")? as usize;
    let o = edit.lists[list].object_mut(u32::try_from(segment)?);
    o[m..m + 4].copy_from_slice(&a.to_le_bytes());
    o[p..p + 4].copy_from_slice(&b.to_le_bytes());
    Ok(())
}

/// Plans the texture animation of a character (`None` when it has none,
/// or the HD model has no animated material to show it with).
#[allow(clippy::too_many_arguments)]
fn plan_face(
    mesh: &Mesh,
    tex: &Chr,
    chr_bytes: &[u8],
    model: &ffx_phyre::Model,
    original: &[u8],
    target: &[usize],
    pages: &[(u8, Rgba)],
    page_of: &HashMap<u8, Placed>,
    (aw, ah): (u32, u32),
    report: &mut String,
) -> Result<Option<Face>> {
    let anim = match TexAnim::parse(chr_bytes) {
        Ok(Some(a)) if !a.entries.is_empty() => a,
        Ok(_) => return Ok(None),
        Err(e) => {
            *report += &format!("  texture animation left out: {e:#}\n");
            return Ok(None);
        }
    };
    // The HD's animated material: named Mat_CTA_G0G1…, on a segment of an
    // instance the PS2 triangles go into.
    let edit = ffx_phyre::edit::ClusterEdit::parse(original).context("HD model")?;
    let names = texanim::material_names(&edit)?;
    let mut best: Option<(usize, usize, usize, String)> = None;
    for (inst, i) in model.instances.iter().enumerate() {
        if !target.contains(&inst) {
            continue;
        }
        for &(s, _) in &i.segments {
            let Ok(pm) = edit.segment_material(u32::try_from(s)?) else { continue };
            let Some(name) = names.get(pm as usize).filter(|n| n.contains("Mat_CTA")) else { continue };
            let Some(m) = model.segments[s].material else { continue };
            let size = model.segments[s].indices.len();
            if best.as_ref().is_none_or(|b| size > model.segments[b.1].indices.len()) {
                best = Some((inst, s, m, name.clone()));
            }
        }
    }
    let Some((instance, segment, material, name)) = best else {
        *report += &format!("  texture animation: {} groups, but the HD model has no animated material; faces stay static\n", anim.entries.len());
        return Ok(None);
    };
    let mat = &model.materials[material];
    let maya_shader = mat.parameter("AnimTranslate_MAYA").is_some();
    if !maya_shader && mat.parameter("AnimTranslate_GAME").is_none() {
        *report += &format!("  texture animation: material {name} has neither _GAME nor _MAYA rectangles; faces stay static\n");
        return Ok(None);
    }
    let anim_id = mat.texture("AnimSampler").map(|t| t.path.clone()).unwrap_or_default();
    // `…_anim_n<k % 10>_p<k / 10>.dds`: texture k (`texanim::anim_tag`)
    let initial = anim_id.rsplit_once("_anim_").and_then(|(_, t)| {
        let t = t.split('.').next()?;
        let (n, p) = t.strip_prefix('n')?.split_once("_p")?;
        (n.len() == 1 && p.len() == 1).then_some((t.to_owned(), n.parse::<usize>().ok()? + 10 * p.parse::<usize>().ok()?))
    });
    let Some((anim_tag, initial)) = initial else {
        *report += &format!("  texture animation: material {name} samples {anim_id:?}; faces stay static\n");
        return Ok(None);
    };
    // Groups the material does not name are never drawn by the PC.
    let named: BTreeSet<usize> = name
        .split('G')
        .skip(1)
        .filter_map(|g| g.parse().ok())
        .collect();
    let image = anim.entries[0].image;
    let entries: Vec<&texanim::Entry> =
        anim.entries.iter().filter(|e| e.image == image && named.contains(&e.group)).collect();
    if entries.len() < anim.entries.len() {
        *report += &format!(
            "  texture animation: {} of {} groups left out (other image or not in {name})\n",
            anim.entries.len() - entries.len(),
            anim.entries.len()
        );
    }
    let rect_of = |e: &texanim::Entry| -> Option<Rect> {
        e.states
            .iter()
            .map(|(s, _)| {
                let (x, y, w, h) = e.rect(&anim.patches, *s);
                Rect { x, y, w, h }
            })
            .reduce(Rect::union)
    };
    let rects: Vec<Rect> = entries.iter().filter_map(|e| rect_of(e)).collect();
    // The page: the texture-list entry of that image whose triangles sample
    // the rectangles most.
    let hits = |t: u8| -> usize {
        mesh.models
            .iter()
            .flat_map(|m| m.tris.iter())
            .filter(|tr| tr.texture == t)
            .flat_map(|tr| tr.corners.iter())
            .filter(|c| {
                let (u, v) = (u32::from(c.uv[0]) / 16, u32::from(c.uv[1]) / 16);
                rects.iter().any(|r| (r.x..=r.x + r.w).contains(&u) && (r.y..=r.y + r.h).contains(&v))
            })
            .count()
    };
    let Some((page, _)) = pages
        .iter()
        .map(|p| p.0)
        .filter(|&t| tex.materials.get(usize::from(t)).is_some_and(|m| m.image == image))
        .map(|t| (t, hits(t)))
        .filter(|x| x.1 > 0)
        .max_by_key(|x| (x.1, std::cmp::Reverse(x.0)))
    else {
        *report += "  texture animation: no triangle samples the animated rectangles\n";
        return Ok(None);
    };
    let base = &pages.iter().find(|p| p.0 == page).context("face page")?.1;
    let pl = page_of[&page];
    let clut = tex.cluts.get(tex.materials[usize::from(page)].clut).context("face clut")?;
    ensure!(
        rects.iter().all(|r| r.x + r.w <= base.width && r.y + r.h <= base.height),
        "texture animation rectangle outside the {}x{} page",
        base.width,
        base.height
    );
    let union = rects.iter().copied().reduce(Rect::union).context("no rectangle")?;

    // Textures by what they show: (patch, area, offset in it).
    let mut keys: Vec<(usize, Rect, u32, u32)> = Vec::new();
    let mut textures: Vec<Rgba> = Vec::new();
    let mut records = Vec::new();
    for e in &entries {
        for (s, frames) in &e.states {
            let (x, y, w, h) = e.rect(&anim.patches, *s);
            let r = if maya_shader { union } else { Rect { x, y, w, h } };
            let mut ids = Vec::new();
            for f in frames {
                let p = &anim.patches[f.patch];
                let key = (f.patch, r, e.x - r.x, e.y - r.y);
                let id = match keys.iter().position(|k| *k == key) {
                    Some(i) => i,
                    None => {
                        keys.push(key);
                        textures.push(texanim::compose(base, r, p, clut, key.2, key.3));
                        keys.len() - 1
                    }
                };
                ids.push(u32::try_from(id)?);
            }
            let (t, sc) = r.shader(pl.x, pl.y, aw, ah);
            records.push(texanim::Record {
                animation: format!("Animation{s}"),
                group: format!("G{}", e.group),
                interval: 0.5,
                ids,
                rect: [sc[0], sc[1], t[0], t[1]],
            });
        }
    }
    // The texture the material starts with must be listed.
    while textures.len() <= initial {
        textures.push(textures[0].clone());
    }
    ensure!(textures.len() <= 100, "{} animation textures (at most 100 have names)", textures.len());
    let maya = maya_shader.then(|| union.shader(pl.x, pl.y, aw, ah));
    Ok(Some(Face { instance, segment, material, page, rects, maya, anim_id, anim_tag, textures, records }))
}

#[allow(clippy::too_many_lines)]
pub fn convert_bytes(
    chr_bytes: &[u8],
    original: &[u8],
    hd_path: &str,
    variants: &alpha::Variants,
    read: impl Fn(&str) -> Result<Option<Vec<u8>>>,
) -> Result<Converted> {
    let mut report = String::new();
    let mesh = Mesh::parse(chr_bytes).context("PS2 mesh")?;
    let tex = Chr::parse(chr_bytes).context("PS2 textures")?;
    let model = ffx_phyre::Model::parse(original).context("HD model")?;

    // The HD skeleton: bone i's bind world matrix from node SkeletonNode_i.
    let worlds = node_worlds(&model);
    let node_of: HashMap<&str, usize> =
        model.nodes.iter().enumerate().map(|(i, n)| (n.name.as_str(), i)).collect();
    let ps2_world = mesh.bind_world();
    let hd_world = |b: u16| -> Option<[f64; 16]> {
        node_of.get(format!("SkeletonNode_{b}").as_str()).map(|&n| worlds[n])
    };
    // Vertices are placed with the PS2 bind pose (HD units: / 1000). It
    // equals the HD nodes for every bone that skins; the nodes of other
    // bones can be missing or degenerate (c101's base bone 150 has a zero
    // matrix).
    let bone_world = |b: u16| -> Option<[f64; 16]> {
        let mut m = *ps2_world.get(usize::from(b))?;
        for k in 12..15 {
            m[k] /= 1000.0;
        }
        Some(m)
    };

    // Where each PS2 model goes. A model the skin lists move goes into the
    // HD's skinned instance (the largest). A rigid one (no skin entry) is
    // drawn by the game with its base bone's matrix by model number
    // (Ch_DrawMainForSituation): HD instance i, unskinned, holds PS2 model
    // i in its base bone's frame; without such an instance it is skinned
    // rigidly to its base bone instead.
    let skinned_instance = model
        .instances
        .iter()
        .enumerate()
        .filter(|(_, i)| i.skeleton.is_some())
        .max_by_key(|(_, i)| i.segments.iter().map(|&(s, _)| model.segments[s].indices.len()).sum::<usize>())
        .map(|(k, _)| k);
    let rigid: Vec<bool> = mesh
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            m.influences.iter().all(Vec::is_empty)
                && model.instances.get(i).is_some_and(|x| x.skeleton.is_none() && !x.segments.is_empty())
        })
        .collect();
    // A skinned model goes into the skinned instance whose skeleton names
    // most of its bones (m070 is three fish, each its own instance and
    // skeleton), the largest on a tie.
    let in_skeleton = |k: usize, b: u16| {
        let name = format!("SkeletonNode_{b}");
        model.instances[k].skeleton.is_some_and(|s| model.skeletons[s].names.iter().any(|n| *n == name))
    };
    let target: Vec<usize> = rigid
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            if r {
                return Ok(i);
            }
            let bones: BTreeSet<u16> =
                mesh.models[i].influences.iter().flatten().map(|x| x.0).chain([mesh.models[i].base]).collect();
            let largest = skinned_instance.context("no skinned instance")?;
            Ok((0..model.instances.len())
                .filter(|&k| model.instances[k].skeleton.is_some())
                .max_by_key(|&k| (bones.iter().filter(|&&b| in_skeleton(k, b)).count(), k == largest, std::cmp::Reverse(k)))
                .unwrap_or(largest))
        })
        .collect::<Result<_>>()?;
    let plain = |m: usize| {
        let mat = &model.materials[m];
        mat.parameter("ClothWorld").is_none() && mat.texture("AnimSampler").is_none() && mat.base_color().is_some()
    };
    // Materials that blend (`ALPHA_ENABLED`: the `Transparent` pass only).
    // The HD draws bodies opaque and only hair and fringes blended; a body
    // in a blended material fades with the character's mesh alpha
    // (`FakeTransparent`, cut scenes and battle fades) where the HD's stays
    // solid, and its alpha threshold (Wakka's hair: 0.8) cuts it away.
    // Per segment (the cluster's material list is not `model.materials`'
    // order: `segment_material` resolves it).
    let blended: Vec<bool> = {
        let edit = ffx_phyre::edit::ClusterEdit::parse(original).context("HD model")?;
        let ids = alpha::material_variants(&edit)?;
        (0..model.segments.len())
            .map(|s| {
                let m = u32::try_from(s).ok().and_then(|s| edit.segment_material(s).ok());
                m.and_then(|m| ids.get(m as usize)).and_then(|id| variants.of(id)).is_some_and(|v| v.on("ALPHA_ENABLED"))
            })
            .collect()
    };
    let score = |s: usize| {
        let seg = &model.segments[s];
        let m = seg.material.unwrap_or(usize::MAX);
        let alpha = model.materials.get(m).and_then(|x| x.parameter("AlphaThreshold")).map_or(0.0, |v| v[0]);
        (
            u8::from(m < model.materials.len() && plain(m)),
            u8::from(!blended.get(s).copied().unwrap_or(false)),
            u8::from(seg.bones.len() > 1),
            u8::from(alpha > 0.0),
            seg.indices.len(),
        )
    };
    // Per target instance, the template segment: one whose material has no
    // cloth and no texture animation, preferably one that does not blend (the
    // body is drawn opaque, see-through texels go to blended twins),
    // skinned to several bones (c101's palette-1 segments are hair with its
    // own shader) and alpha tested, the largest.
    struct Slot {
        template: usize,
        material: usize,
        texture: String,
        color: [u8; 4],
    }
    let mut slots: BTreeMap<usize, Slot> = BTreeMap::new();
    for &k in &target {
        if slots.contains_key(&k) {
            continue;
        }
        let template = model.instances[k]
            .segments
            .iter()
            .map(|&(s, _)| s)
            .max_by_key(|&s| score(s))
            .context("no segments")?;
        let material = model.segments[template].material.context("template without material")?;
        ensure!(plain(material), "no plain material to copy (segment {template})");
        let texture = model.materials[material].base_color().context("no base texture")?.archive_path();
        let color = {
            // BTreeMap: a tie must not be settled by HashMap order
            let mut c: BTreeMap<[u8; 4], usize> = BTreeMap::new();
            for x in &model.segments[template].colors {
                *c.entry(*x).or_default() += 1;
            }
            c.into_iter().max_by_key(|e| e.1).map_or([255; 4], |e| e.0)
        };
        report += &format!(
            "  instance {k}: template segment {template} (material {material}), texture {texture}, colour {color:?}\n"
        );
        slots.insert(k, Slot { template, material, texture, color });
    }
    if rigid.iter().any(|&r| r) {
        report += &format!("  rigid models (own instance): {}\n", rigid.iter().filter(|&&r| r).count());
    }

    // Vertices in the bind pose.
    let mut verts: Vec<Vec<Vert>> = Vec::new();
    let mut missing = BTreeSet::new();
    let mut off_skeleton = BTreeSet::new();
    let mut dropped_weight = 0f32;
    let skinnable = |k: usize, b: u16| hd_world(b).is_some() && in_skeleton(k, b);
    let mut mismatch = 0f64;
    for (mi, m) in mesh.models.iter().enumerate() {
        if rigid[mi] {
            // in the base bone's frame, as stored
            verts.push(
                m.vertices
                    .iter()
                    .map(|raw| Vert {
                        pos: raw.map(|c| f64::from(c) / 1000.0),
                        normal: [0.0; 3],
                        bones: [(0, 0.0); 4],
                    })
                    .collect(),
            );
            continue;
        }
        let base = bone_world(m.base).with_context(|| format!("no bone {}", m.base))?;
        // the HD node pose against the PS2 bind pose, for the bones used
        let used: BTreeSet<u16> = m.influences.iter().flatten().map(|x| x.0).collect();
        for &b in &used {
            if let Some(w) = hd_world(b).filter(|_| skinnable(target[mi], b)) {
                let p = ps2_world[usize::from(b)];
                mismatch = mismatch.max((12..15).map(|k| (w[k] * 1000.0 - p[k]).abs()).fold(0.0, f64::max));
            }
        }
        let mut out = Vec::with_capacity(m.vertices.len());
        for (v, raw) in m.vertices.iter().enumerate() {
            let p = ps2mesh::transform(&base, raw.map(|c| f64::from(c) / 1000.0));
            let mut inf: Vec<(u16, f32)> = Vec::new();
            for &(b, w, _) in &m.influences[v] {
                match inf.iter_mut().find(|x| x.0 == b) {
                    Some(x) => x.1 += w,
                    None => inf.push((b, w)),
                }
            }
            if inf.is_empty() {
                inf.push((m.base, 1.0));
            }
            // Bones the HD mesh cannot skin to (no node, or not among its
            // joints) hand their weight to the nearest ancestor it can.
            let mut merged: Vec<(u16, f32)> = Vec::new();
            for (b, w) in inf {
                let mut x = b;
                let mut steps = 0;
                while !skinnable(target[mi], x) {
                    if hd_world(x).is_none() {
                        missing.insert(x);
                    } else {
                        off_skeleton.insert(x);
                    }
                    let p = mesh.bones.get(usize::from(x)).map_or(x, |bn| bn.parent);
                    steps += 1;
                    if p == x || steps > mesh.bones.len() {
                        break;
                    }
                    x = p;
                }
                if !skinnable(target[mi], x) {
                    continue;
                }
                match merged.iter_mut().find(|e| e.0 == x) {
                    Some(e) => e.1 += w,
                    None => merged.push((x, w)),
                }
            }
            let mut inf = merged;
            ensure!(!inf.is_empty(), "vertex {v} has no bone the HD mesh can skin to");
            inf.sort_by(|a, b| b.1.total_cmp(&a.1));
            let total: f32 = inf.iter().map(|x| x.1).sum();
            let kept: f32 = inf.iter().take(4).map(|x| x.1).sum();
            dropped_weight = dropped_weight.max(1.0 - kept / total.max(1e-6));
            let mut bones = [(0u16, 0f32); 4];
            for (k, &(b, w)) in inf.iter().take(4).enumerate() {
                bones[k] = (b, w / kept.max(1e-6));
            }
            out.push(Vert { pos: p, normal: [0.0; 3], bones });
        }
        // normals: per corner from the model's normal array (rotated like
        // the vertices), else the face normals averaged per vertex
        verts.push(out);
    }
    if !missing.is_empty() || !off_skeleton.is_empty() {
        report += &format!("  bones without HD node {missing:?}, not in the skeleton {off_skeleton:?} (moved to the base bone)\n");
    }
    report += &format!(
        "  HD vs PS2 bind pose: max translation diff {mismatch:.1} PS2 units; most weight dropped by the 4-bone limit {:.0}%\n",
        dropped_weight * 100.0
    );

    // Atlas pages: every texture-list entry the triangles use.
    let used_tex: BTreeSet<u8> = mesh.models.iter().flat_map(|m| m.tris.iter().map(|t| t.texture)).collect();
    let mut pages: Vec<(u8, Rgba)> = Vec::new();
    for &t in &used_tex {
        let Some(mat) = tex.materials.get(usize::from(t)) else {
            bail!("texture {t} is not in the list");
        };
        let img = tex.images.get(mat.image).context("image")?;
        let pal = tex.cluts.get(mat.clut).context("clut")?;
        let mut page = Rgba::new(img.width, img.height);
        for y in 0..img.height {
            for x in 0..img.width {
                page.put(x, y, pal[usize::from(img.texels[(y * img.width + x) as usize])]);
            }
        }
        pages.push((t, page));
    }
    let (aw, ah, placed) = pack(&pages.iter().map(|p| (p.1.width, p.1.height)).collect::<Vec<_>>());
    let mut atlas = Rgba::new(aw, ah);
    for ((_, page), pl) in pages.iter().zip(&placed) {
        for y in 0..pl.h + 2 * PAD {
            for x in 0..pl.w + 2 * PAD {
                let sx = x.saturating_sub(PAD).min(pl.w - 1);
                let sy = y.saturating_sub(PAD).min(pl.h - 1);
                atlas.put(pl.x - PAD + x, pl.y - PAD + y, page.get(sx, sy));
            }
        }
    }
    let page_of: HashMap<u8, Placed> = pages.iter().map(|p| p.0).zip(placed.iter().copied()).collect();
    report += &format!("  atlas {aw}x{ah}: textures {:?}\n", used_tex);

    // Texture animation (eyes, mouths): the face triangles go into the HD's
    // animated material (`characters/texanim.rs`).
    let face = plan_face(&mesh, &tex, chr_bytes, &model, original, &target, &pages, &page_of, (aw, ah), &mut report)?;

    // Winding: the PS2 strips against the HD convention.
    let hd_agree = winding_agreement(model.segments.iter().filter(|s| !s.normals.is_empty()).flat_map(|s| {
        s.indices.chunks_exact(3).map(move |t| {
            let p = [0, 1, 2].map(|k| s.positions[usize::from(t[k])].map(f64::from));
            let n = [0, 1, 2].map(|k| s.normals[usize::from(t[k])].map(f64::from));
            (p, [n[0][0] + n[1][0] + n[2][0], n[0][1] + n[1][1] + n[2][1], n[0][2] + n[1][2] + n[2][2]])
        })
    }));

    // Corner normals in the bind pose.
    let corner_normal = |mi: usize, c: &Corner| -> Option<[f64; 3]> {
        let m = &mesh.models[mi];
        let n = m.normals.get(usize::from(c.normal))?;
        let n = n.map(f64::from);
        if rigid[mi] {
            return Some(normalize(n));
        }
        let base = bone_world(m.base)?;
        Some(normalize(ps2mesh::rotate(&base, n)))
    };
    let ps2_agree = winding_agreement(mesh.models.iter().enumerate().flat_map(|(mi, m)| {
        let verts = &verts;
        m.tris.iter().filter_map(move |t| {
            let p = t.corners.map(|c| verts[mi][usize::from(c.vertex)].pos);
            let mut n = [0f64; 3];
            for c in &t.corners {
                let cn = corner_normal(mi, c)?;
                for k in 0..3 {
                    n[k] += cn[k];
                }
            }
            Some((p, n))
        })
    }));
    let flip = (hd_agree > 0.5) != (ps2_agree > 0.5);
    report += &format!(
        "  winding: HD faces along normals {:.0}%, PS2 {:.0}%{}\n",
        hd_agree * 100.0,
        ps2_agree * 100.0,
        if flip { ", flipped" } else { "" }
    );

    // Face normals per vertex for models without normals.
    let mut face_normals: Vec<Vec<[f64; 3]>> = verts.iter().map(|v| vec![[0.0; 3]; v.len()]).collect();
    for (mi, m) in mesh.models.iter().enumerate() {
        for t in &m.tris {
            let p = t.corners.map(|c| verts[mi][usize::from(c.vertex)].pos);
            let mut g = cross(sub(p[1], p[0]), sub(p[2], p[0]));
            if ps2_agree < 0.5 {
                g = g.map(|x| -x);
            }
            for c in &t.corners {
                let f = &mut face_normals[mi][usize::from(c.vertex)];
                for k in 0..3 {
                    f[k] += g[k];
                }
            }
        }
    }
    for (mi, v) in verts.iter_mut().enumerate() {
        for (k, x) in v.iter_mut().enumerate() {
            x.normal = normalize(face_normals[mi][k]);
        }
    }

    // Triangles into primitives with bounded bone palettes.
    let mut joints: Vec<u16> = Vec::new();
    let mut joint_index: HashMap<u16, u16> = HashMap::new();
    let mut prims: Vec<Primitive> = Vec::new();
    let mut prim_bones: Vec<BTreeSet<u16>> = Vec::new();
    let mut prim_keys: Vec<HashMap<(usize, u16, u16, [u16; 2], u8), u32>> = Vec::new();
    let mut prim_slot: Vec<(usize, usize)> = Vec::new();
    // See-through texels (`chralpha`): template triangles that sample them
    // go into segments of their own (blended); cloth and face segments are
    // blended whole. Per primitive: blended, (see-through, sampled) texels.
    let mut prim_blend: Vec<bool> = Vec::new();
    let mut prim_cov: Vec<(usize, usize)> = Vec::new();
    let page_image: HashMap<u8, &Rgba> = pages.iter().map(|p| (p.0, &p.1)).collect();
    // Order: by model, then texture, keeping the strip order.
    let mut order: Vec<(usize, usize)> = Vec::new();
    for (mi, m) in mesh.models.iter().enumerate() {
        let mut ts: Vec<usize> = (0..m.tris.len()).collect();
        ts.sort_by_key(|&k| m.tris[k].texture);
        order.extend(ts.into_iter().map(|k| (mi, k)));
    }
    // PS2 cloth: the PC runs the PS2 cloth simulation for every piece the
    // `.cdf` lists (all of them) and moves the vertices of each HD segment
    // tagged with a piece (`PMeshSegment.m_clothmodelIndex`,
    // `m_clothIndex`) with the simulated PS2 triangle they lie in
    // (`Ch_PreCalcActiveClothMesh`, `InsideTriangle`). So each PS2 piece's
    // triangles go into the segment tagged with it, or into a copy of an HD
    // cloth segment tagged anew; they lie on the simulated triangles
    // exactly. Without HD cloth segments (no cloth shader) they stay
    // skinned.
    let tags = cloth_tags(original)?;
    let hd_cloth = |s: usize| {
        model.segments[s]
            .material
            .and_then(|m| model.materials.get(m))
            .is_some_and(|m| m.parameter("ClothWorld").is_some())
            && tags.get(s).is_some_and(Option::is_some)
    };
    let mut cloth_targets: Vec<ClothTarget> = Vec::new();
    let cdf_path = {
        let dir = hd_path.split("/mdl/").next().context("model path")?;
        format!("{dir}/{}.cdf", dir.rsplit('/').next().context("model path")?)
    };
    let mut cdf = read(&cdf_path)?.as_deref().map(Cdf::parse).transpose().context("HD .cdf")?;
    // per model: PS2 vertex -> piece (the first piece that has it)
    let mut piece_of: Vec<HashMap<u16, usize>> = vec![HashMap::new(); mesh.models.len()];
    for (mi, m) in mesh.models.iter().enumerate() {
        if rigid[mi] {
            continue;
        }
        let k = target[mi];
        let instance_cloth: Vec<usize> =
            model.instances[k].segments.iter().map(|&(s, _)| s).filter(|&s| hd_cloth(s)).collect();
        // the template for new cloth segments: the largest HD cloth segment
        let Some(&template) = instance_cloth.iter().max_by_key(|&&s| (model.segments[s].indices.len(), std::cmp::Reverse(s)))
        else {
            continue;
        };
        for (ci, c) in m.cloths.iter().enumerate() {
            let tag = (u32::try_from(mi)?, u32::try_from(ci)?);
            let own = instance_cloth
                .iter()
                .copied()
                .filter(|&s| tags[s] == Some(tag))
                .max_by_key(|&s| (model.segments[s].indices.len(), std::cmp::Reverse(s)));
            let seg = own.unwrap_or(template);
            let Some(material) = model.segments[seg].material else { continue };
            // The PC binds each vertex of the segment to the nearest
            // triangle the `.cdf` lists for the piece. The HD's lists hold
            // the HD's shape (Wakka's crest (0, 3): 54 triangles, 24 cards,
            // where the PS2 has 526): vertices off them were projected
            // onto those cards and the crest lay flat. So the piece's own
            // PS2 triangles go first; they lie on them, and the search
            // keeps the first of equally near triangles. The HD's follow
            // for the hidden HD segments, some of which pass only their
            // own sub-part's triangles (`passThisTriangle`: Kimahri's
            // piece (0, 10)).
            let mut tris = cdf_triangles(m, c, u16::try_from(ci)?);
            let n = tris.len() / 32;
            let skinned = match cdf.as_mut() {
                Some(cdf) if (1..=CDF_MAX_TRIANGLES).contains(&n) => {
                    let key = (u16::try_from(mi)?, u16::try_from(ci)?);
                    match cdf.pieces.iter_mut().find(|p| p.0 == key) {
                        Some(p) => {
                            let keep = p.2.len().min(32 * (CDF_MAX_TRIANGLES - n));
                            if keep < p.2.len() {
                                report += &format!(
                                    "  PS2 cloth piece {tag:?}: {} of the HD's {} triangles left out of the .cdf\n",
                                    (p.2.len() - keep) / 32,
                                    p.2.len() / 32
                                );
                            }
                            tris.extend_from_slice(&p.2[..keep]);
                            p.2 = tris;
                        }
                        None => cdf.pieces.push((key, 0, tris)),
                    }
                    false
                }
                _ => true,
            };
            if skinned {
                report += &format!("  PS2 cloth piece {tag:?} kept skinned: {n} triangles, the .cdf cannot list them\n");
            }
            let t = cloth_targets.len();
            cloth_targets.push(ClothTarget { tag, segment: own, material, skinned });
            for &v in &c.vertices {
                piece_of[mi].entry(v).or_insert(t);
            }
        }
    }
    if std::env::var_os("DM_DEBUG").is_some() {
        for (s, t) in tags.iter().enumerate() {
            let Some((m, c)) = *t else { continue };
            let Some(piece) = mesh.models.get(m as usize).and_then(|x| x.cloths.get(c as usize)) else { continue };
            let mi = m as usize;
            let raw: Vec<[f64; 3]> = piece.vertices.iter().filter_map(|&v| mesh.models[mi].vertices.get(usize::from(v))).map(|r| r.map(|c| f64::from(c) / 1000.0)).collect();
            let bind: Vec<[f64; 3]> = piece.vertices.iter().filter_map(|&v| verts[mi].get(usize::from(v)).map(|x| x.pos)).collect();
            let med = |set: &[[f64; 3]]| {
                let mut d: Vec<f64> = model.segments[s].positions.iter().map(|p| set.iter().map(|q| (0..3).map(|k| (f64::from(p[k]) - q[k]).powi(2)).sum::<f64>().sqrt()).fold(f64::MAX, f64::min)).collect();
                d.sort_by(f64::total_cmp);
                d.get(d.len() / 2).copied().unwrap_or(-1.0)
            };
            eprintln!("HD cloth segment {s} ({m}, {c}): median distance to PS2 raw {:.3}, to bind pose {:.3}", med(&raw), med(&bind));
            // the other way: each PS2 vertex to its nearest HD vertex
            let mut d: Vec<f64> = bind
                .iter()
                .map(|q| {
                    model.segments[s]
                        .positions
                        .iter()
                        .map(|p| (0..3).map(|k| (f64::from(p[k]) - q[k]).powi(2)).sum::<f64>().sqrt())
                        .fold(f64::MAX, f64::min)
                })
                .collect();
            d.sort_by(f64::total_cmp);
            let q = |f: f64| d.get(((d.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(-1.0);
            eprintln!(
                "  PS2 piece vertices ({}) to the HD segment: min {:.3} median {:.3} 90% {:.3} max {:.3}",
                d.len(),
                q(0.0),
                q(0.5),
                q(0.9),
                q(1.0)
            );
        }
    }
    // Cloth vertex colours steer the PC's binding (`Ch_PreCalcActiveClothMesh`):
    // all channels at most 0.3 keep the vertex skinned, larger ones bind it
    // to the simulated triangle it lies in (red with its distance off the
    // triangle, white on it: 0 for the PS2 vertices, which lie on their
    // triangles). The PS2 piece says which of its vertices the simulation
    // moves; the pinned ones it copies from the skinned model, so they keep
    // the skin colour and are skinned like the body around them. A moving
    // vertex gets the colour of the nearest free vertex of the HD segment
    // of its piece (the HD's binding mode), else the HD's usual free colour.
    let skin_like = |c: &[u8; 4]| c[..3].iter().all(|&v| v <= 77);
    let free_color = {
        let mut c: BTreeMap<[u8; 4], usize> = BTreeMap::new();
        for s in (0..model.segments.len()).filter(|&s| hd_cloth(s)) {
            for x in &model.segments[s].colors {
                if !skin_like(x) {
                    *c.entry(*x).or_default() += 1;
                }
            }
        }
        c.into_iter().max_by_key(|e| e.1).map_or([255; 4], |e| e.0)
    };
    // per model: the vertices some piece moves
    let moving: Vec<BTreeSet<u16>> =
        mesh.models.iter().map(|m| m.cloths.iter().flat_map(|c| c.moving().iter().copied()).collect()).collect();
    let cloth_color = |mi: usize, v: u16, pos: [f64; 3], target: &ClothTarget, skin: [u8; 4]| -> [u8; 4] {
        if !moving[mi].contains(&v) || target.skinned {
            return skin;
        }
        let Some(s) = target.segment else { return free_color };
        let seg = &model.segments[s];
        seg.positions
            .iter()
            .zip(&seg.colors)
            .filter(|(_, c)| !skin_like(c))
            .map(|(p, c)| ((0..3).map(|k| (f64::from(p[k]) - pos[k]).powi(2)).sum::<f64>(), *c))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(free_color, |x| x.1)
    };
    // Draw order between cloth pieces. The PS2 draws a model's groups in
    // order; the PC draws a cloth piece's segment where its segment stands
    // in the list: the HD's own segment of the piece, or a new one after
    // all of them. A triangle the PS2 draws after every triangle of another
    // piece whose segment comes later goes into a tail segment of its
    // piece, drawn after them: Kimahri's mane (c004 (0, 6), HD segment 7)
    // has its bottom row in groups 59-60, after the fur collar ((0, 10),
    // groups 26-58, a new segment). Drawn before the collar, the row's
    // half-transparent fringe hid the collar behind it (depth written) and
    // showed the openings of the head and the fire there.
    let tri_cloth = |mi: usize, t: &ps2mesh::Tri| -> Option<usize> {
        let c = t.corners.map(|c| piece_of[mi].get(&c.vertex).copied());
        c[0].filter(|&x| c.iter().all(|y| *y == Some(x)))
    };
    let mut last_group: Vec<Option<u16>> = vec![None; cloth_targets.len()];
    let mut rank: Vec<Option<usize>> = cloth_targets.iter().map(|c| c.segment).collect();
    for (n, &(mi, k)) in order.iter().enumerate() {
        let t = &mesh.models[mi].tris[k];
        if let Some(c) = tri_cloth(mi, t) {
            last_group[c] = last_group[c].max(Some(t.group));
            // a new segment: after the HD's, in the order of first use
            rank[c].get_or_insert(model.segments.len() + n);
        }
    }
    // (pieces of one model: their groups compare)
    let tail = |c: usize, group: u16| -> bool {
        (0..cloth_targets.len()).any(|b| {
            b != c
                && cloth_targets[b].tag.0 == cloth_targets[c].tag.0
                && rank[b] > rank[c]
                && last_group[b].is_some_and(|g| g < group)
        })
    };
    let mut prim_tail: Vec<bool> = Vec::new();
    // per primitive: the first PS2 group of its triangles (model, group)
    let mut prim_group: Vec<(usize, u16)> = Vec::new();
    let mut tail_triangles = 0;
    let mut degenerate = 0;
    let mut two_sided = 0;
    for (mi, k) in order {
        let t = &mesh.models[mi].tris[k];
        let vs = t.corners.map(|c| &verts[mi][usize::from(c.vertex)]);
        if t.corners[0].vertex == t.corners[1].vertex
            || t.corners[1].vertex == t.corners[2].vertex
            || t.corners[0].vertex == t.corners[2].vertex
        {
            degenerate += 1;
            continue;
        }
        let cloth = t.corners.map(|c| piece_of[mi].get(&c.vertex).copied());
        let cloth = cloth[0].filter(|&c| cloth.iter().all(|x| *x == Some(c)));
        let bones: BTreeSet<u16> = vs.iter().flat_map(|v| v.bones.iter().filter(|b| b.1 > 0.0).map(|b| b.0)).collect();
        let on_face = cloth.is_none() && face.as_ref().is_some_and(|f| f.instance == target[mi] && f.covers(t));
        // what the triangle goes into: 0 the template, 1 the face, 2 + c cloth target c
        let k = (target[mi], cloth.map_or(usize::from(on_face), |c| 2 + c));
        let slot = &slots[&k.0];
        let cov = {
            let img = page_image[&t.texture];
            alpha::coverage(t.corners.map(|c| c.uv.map(|x| f64::from(x) / 16.0)), |x, y| {
                img.get(x.clamp(0, i64::from(img.width) - 1) as u32, y.clamp(0, i64::from(img.height) - 1) as u32)[3]
            })
        };
        let blend = k.1 == 0 && cov.0 > 0;
        let is_tail = cloth.is_some_and(|c| tail(c, t.group));
        tail_triangles += usize::from(is_tail);
        let fits = |p: usize| {
            prim_slot[p] == k
                && prim_blend[p] == blend
                && prim_tail[p] == is_tail
                && prim_bones[p].union(&bones).count() <= MAX_PALETTE
                && prims[p].positions.len() + 3 <= 0xffff
        };
        let p = match (0..prims.len()).rev().find(|&p| fits(p)) {
            Some(p) => p,
            None => {
                let first = !(0..prims.len()).any(|p| prim_slot[p] == k && prims[p].segment.is_some());
                let (segment, material) = match (&face, k.1) {
                    (_, c @ 2..) => (cloth_targets[c - 2].segment, cloth_targets[c - 2].material),
                    (Some(f), 1) => (Some(f.segment), f.material),
                    _ => (Some(slot.template), slot.material),
                };
                prims.push(Primitive {
                    segment: segment.filter(|_| first && !is_tail),
                    material,
                    ..Primitive::default()
                });
                prim_bones.push(BTreeSet::new());
                prim_keys.push(HashMap::new());
                prim_slot.push(k);
                prim_blend.push(blend);
                prim_tail.push(is_tail);
                prim_group.push((mi, t.group));
                prim_cov.push((0, 0));
                prims.len() - 1
            }
        };
        prim_bones[p].extend(bones.iter().copied());
        prim_cov[p].0 += cov.0;
        prim_cov[p].1 += cov.1;
        let pl = page_of[&t.texture];
        let mut idx = [0u32; 3];
        for (j, c) in t.corners.iter().enumerate() {
            let key = (mi, c.vertex, c.normal, c.uv, t.texture);
            let prim = &mut prims[p];
            let n = prim.positions.len() as u32;
            let i = *prim_keys[p].entry(key).or_insert(n);
            if i == n {
                let v = vs[j];
                prim.positions.push(v.pos.map(|x| x as f32));
                let normal = corner_normal(mi, c).unwrap_or(v.normal);
                prim.normals.push(normal.map(|x| x as f32));
                let u = (f64::from(c.uv[0]) / 16.0).clamp(0.0, f64::from(pl.w));
                let w = (f64::from(c.uv[1]) / 16.0).clamp(0.0, f64::from(pl.h));
                prim.uvs.push([
                    ((f64::from(pl.x) + u) / f64::from(aw)) as f32,
                    ((f64::from(pl.y) + w) / f64::from(ah)) as f32,
                ]);
                prim.colors.push(if k.1 >= 2 {
                    cloth_color(mi, c.vertex, v.pos, &cloth_targets[k.1 - 2], slot.color)
                } else {
                    slot.color
                });
                if !rigid[mi] {
                    let mut jo = [0u16; 4];
                    let mut we = [0f32; 4];
                    for (q, &(b, wt)) in v.bones.iter().enumerate() {
                        if wt <= 0.0 {
                            continue;
                        }
                        let ji = *joint_index.entry(b).or_insert_with(|| {
                            joints.push(b);
                            (joints.len() - 1) as u16
                        });
                        jo[q] = ji;
                        we[q] = wt;
                    }
                    prim.joints.push(jo);
                    prim.weights.push(we);
                }
            }
            idx[j] = i;
        }
        if flip {
            idx.swap(1, 2);
        }
        prim_group[p] = prim_group[p].min((mi, t.group));
        prims[p].indices.extend(idx);
        // Group flag bit 2: the PS2 draws the group's polygons from both
        // sides. Its packet builder (`Ch_PacketCreateMain_008423c0`) picks
        // the culling VU1 entry only for groups without the bit while the
        // character's cull flag (chr `+0x194` bit 19, `Ch_PacketSetCull`) is
        // on, which it is for the whole party in the boot intro. The PC
        // culls back faces: the turned-away side of these polygons gets its
        // own triangle, with the same vertices and normals (lit like the
        // front, as the PS2's per-vertex lighting does).
        if t.flags & 4 != 0 {
            prims[p].indices.extend([idx[0], idx[2], idx[1]]);
            two_sided += 1;
        }
    }
    // DM_DROP_BODY=1 (debugging): the opaque body left out, one triangle
    // kept per primitive, to see what the blended and cloth segments draw
    // (DM_DROP_BODY=2: the cloth too)
    if let Ok(v) = std::env::var("DM_DROP_BODY") {
        for p in 0..prims.len() {
            if (prim_slot[p].1 == 0 && !prim_blend[p]) || (v == "2" && prim_slot[p].1 >= 2) {
                prims[p].indices.truncate(3);
            }
        }
    }
    report += &format!(
        "  {} triangles ({degenerate} degenerate left out, {two_sided} drawn from both sides, {tail_triangles} cloth triangles in tail segments) in {} segments, palettes {:?}\n",
        prims.iter().map(|p| p.indices.len() / 3).sum::<usize>(),
        prims.len(),
        prim_bones.iter().map(BTreeSet::len).collect::<Vec<_>>()
    );

    let face_triangles: usize = prims.iter().zip(&prim_slot).filter(|(_, k)| k.1 == 1).map(|(p, _)| p.indices.len() / 3).sum();
    let cloth_triangles: usize = prims.iter().zip(&prim_slot).filter(|(_, k)| k.1 >= 2).map(|(p, _)| p.indices.len() / 3).sum();
    // the glTF order (by instance, then primitive): the order of the
    // segments the import writes
    let mut meshes: BTreeMap<usize, Vec<(Primitive, usize, bool)>> = BTreeMap::new();
    // the tail segments after the others, in the PS2's group order (new
    // segments follow the glTF order)
    let mut by_order: Vec<usize> = (0..prims.len()).collect();
    by_order.sort_by_key(|&p| (prim_tail[p], if prim_tail[p] { prim_group[p] } else { (0, 0) }));
    let mut taken: Vec<Option<Primitive>> = prims.into_iter().map(Some).collect();
    let prims: Vec<Primitive> = by_order.iter().map(|&p| taken[p].take().expect("each once")).collect();
    let prim_slot: Vec<(usize, usize)> = by_order.iter().map(|&p| prim_slot[p]).collect();
    let prim_blend: Vec<bool> = by_order.iter().map(|&p| prim_blend[p]).collect();
    let prim_cov: Vec<(usize, usize)> = by_order.iter().map(|&p| prim_cov[p]).collect();
    for (((p, k), b), c) in prims.into_iter().zip(&prim_slot).zip(&prim_blend).zip(&prim_cov) {
        // blended: a template primitive of see-through triangles, a cloth
        // piece with any see-through texel, a face with 1 % or more
        let b = *b
            || (k.1 >= 2 && c.0 > 0)
            || (k.1 == 1 && c.0 as f32 >= alpha::MIN_SHARE * c.1 as f32 && c.0 > 0);
        meshes.entry(k.0).or_default().push((p, k.1, b));
    }
    // (vertices, indices, kind, blended) per primitive: to find the segment each became
    let first = |p: &[[f32; 3]]| p.first().map(|v| v.map(f32::to_bits));
    let kinds: Vec<(usize, usize, Option<[u32; 3]>, usize, bool)> = meshes
        .values()
        .flatten()
        .map(|x| (x.0.positions.len(), x.0.indices.len(), first(&x.0.positions), x.1, x.2))
        .collect();
    let meshes: BTreeMap<usize, Vec<Primitive>> =
        meshes.into_iter().map(|(k, v)| (k, v.into_iter().map(|x| x.0).collect())).collect();
    let glb = Glb {
        joints: joints.iter().map(|b| format!("SkeletonNode_{b}")).collect(),
        meshes: meshes.into_values().collect(),
        materials: model.materials.len(),
    }
    .write();
    let imported = ffx_phyre::import_into_cluster(original, &glb).context("import")?;
    ensure!(imported.new_textures.is_empty(), "import added textures {:?}", imported.new_textures);
    ensure!(imported.segments.len() == kinds.len(), "import wrote {} segments for {} primitives", imported.segments.len(), kinds.len());

    // Hide every other segment: no triangles, vertices kept.
    let mut out_model = ffx_phyre::Model::parse(&imported.bytes).context("re-reading")?;
    // What each written segment holds (the import lists them sorted).
    let mut unused = kinds.clone();
    let mut seg_kind: Vec<(usize, usize)> = Vec::new();
    let mut blended: Vec<u32> = Vec::new();
    for &s in &imported.segments {
        let seg = &out_model.segments[s];
        let i = unused
            .iter()
            .position(|k| k.0 == seg.positions.len() && k.1 == seg.indices.len() && k.2 == first(&seg.positions))
            .with_context(|| format!("segment {s} matches no primitive"))?;
        let k = unused.remove(i);
        ensure!(
            !unused.iter().any(|u| u.0 == k.0 && u.1 == k.1 && u.2 == k.2 && (u.3 != k.3 || u.4 != k.4)),
            "segment {s} matches primitives of two kinds"
        );
        seg_kind.push((s, k.3));
        if k.4 {
            blended.push(u32::try_from(s)?);
        }
    }
    let mut hidden = 0;
    for (s, seg) in out_model.segments.iter_mut().enumerate() {
        if imported.segments.contains(&s) {
            continue;
        }
        seg.indices = vec![0, 0, 0];
        hidden += 1;
    }
    let bytes = out_model.write(&imported.bytes).context("writing")?;

    // The atlas is a new texture, `<name>_ps2.dds`, so that HD segments
    // still drawn (cloth) keep theirs; the templates' materials (and the
    // segments copied from them) point to it.
    let dir = hd_path.split("/mdl/").next().context("model path")?;
    let name = dir.rsplit('/').next().context("model path")?;
    let first = slots.values().next().context("no slot")?;
    let old_id = model.materials[first.material].base_color().context("no base texture")?.path.clone();
    // In the model's own folder (c906 uses c106's texture), and as long as
    // the HD id, which is patched in place: c001.dds -> p001.dds.
    let rel = dir.strip_prefix("ffx_data/gamedata/ps3data/").context("model path")?;
    // (n238 uses n238_hair.dds: p238_ps2x.dds)
    let stem = old_id
        .len()
        .checked_sub(format!("PS3Data/{rel}/tex/.dds").len())
        .filter(|&n| n >= name.len())
        .with_context(|| format!("no texture name of the length of {old_id}"))?;
    let file: String = format!("p{}_ps2", name.get(1..).context("name")?)
        .chars()
        .chain(std::iter::repeat('x'))
        .take(stem)
        .collect();
    let new_id = format!("PS3Data/{rel}/tex/{file}.dds");
    ensure!(new_id.len() == old_id.len(), "texture id {old_id} and {new_id} differ in length");
    let new_path = format!("{dir}/tex/d3d11/{file}.dds.phyre");
    let mut edit = ffx_phyre::edit::ClusterEdit::parse(&bytes).context("edit")?;
    let mut materials = BTreeSet::new();
    for &s in &imported.segments {
        materials.insert(edit.segment_material(u32::try_from(s)?).context("segment material")?);
    }
    for &m in &materials {
        edit.set_texture(m, "TextureSampler", &new_id).context("set texture")?;
    }
    // Cloth segments carry their piece.
    let mut cloth_segments = Vec::new();
    for &(s, kind) in &seg_kind {
        if kind >= 2 {
            let tag = cloth_targets[kind - 2].tag;
            set_cloth_tag(&mut edit, s, tag)?;
            cloth_segments.push((s, tag));
        }
    }
    if !cloth_segments.is_empty() {
        report += &format!(
            "  PS2 cloth: {cloth_triangles} triangles in cloth segments (segment, (model, piece)) {cloth_segments:?}\n"
        );
    }
    let face = face.filter(|_| face_triangles > 0);
    if let Some(f) = &face {
        // the face segment as numbered after the import (new segments shift it)
        let seg = seg_kind.iter().find(|x| x.1 == 1).map(|x| x.0).context("face segment")?;
        let m = edit.segment_material(u32::try_from(seg)?).context("face material")?;
        if let Some((t, s)) = f.maya {
            let mut set = 0;
            for n in ["", "2", "3", "4", "5", "6"] {
                set += u32::from(texanim::set_value(&mut edit, m, &format!("AnimTranslate_MAYA{n}"), &t)?);
                set += u32::from(texanim::set_value(&mut edit, m, &format!("AnimScale_MAYA{n}"), &s)?);
            }
            ensure!(set >= 2, "the animated material has no AnimTranslate_MAYA to set");
        }
        report += &format!(
            "  texture animation: {face_triangles} face triangles in segment {}, {} frames, {} records{}\n",
            seg,
            f.textures.len(),
            f.records.len(),
            if f.maya.is_some() { " (MAYA rectangle)" } else { "" }
        );
    }
    // Segments with see-through texels draw blended like the PS2 (after the
    // face's rectangle is set: a copied material keeps it).
    // A variant the HD model did not import is not found when the game
    // resolves the model's imports (`PAssetReferenceImport::resolve`:
    // "Failed to find asset", the fallback draws nothing): the blended twin
    // takes over an import that only materials no segment uses any more
    // refer to (the hidden segments take drawn materials of their kind).
    let drawn: BTreeSet<u32> = imported.segments.iter().map(|&s| u32::try_from(s)).collect::<Result<_, _>>()?;
    let spare_materials = alpha::free_hidden_materials(&mut edit, &drawn, variants)?;
    let mut spare = alpha::spare_imports(&edit, &spare_materials)?;
    report += &alpha::blend_segments(&mut edit, &blended, variants, alpha::Draw::Blend, &mut spare)?;
    let shader_entries = alpha::list_entries(&edit)?;
    let bytes = edit.write().context("writing")?;
    let check = ffx_phyre::Model::parse(&bytes).context("re-reading")?;
    for &s in &imported.segments {
        let t = check.segments[s].material.and_then(|m| check.materials[m].base_color()).map(|t| t.path.clone());
        ensure!(t.as_deref() == Some(new_id.as_str()), "segment {s} reads texture {t:?}");
    }
    report += &format!("  segments written {:?}, {hidden} HD segments hidden\n", imported.segments);

    let mut files = vec![(hd_path.to_owned(), bytes)];
    // the PS2 triangles of the simulated pieces
    if let Some(cdf) = cdf.filter(|_| cloth_targets.iter().any(|t| !t.skinned)) {
        files.push((cdf_path, cdf.write()));
    }
    let template_tex = read(&first.texture)?.with_context(|| format!("no {}", first.texture))?;
    files.push((new_path, phyretex::argb8_one_level(&template_tex, &atlas, Some((&old_id, &new_id)))?));
    let list_path = format!("{dir}/{name}.ahwin32");
    let list = read(&list_path)?.with_context(|| format!("no {list_path}"))?;
    let atlas_entry = ffx_phyre::assets::texture_entry(&new_id);
    let mut anim = Vec::new();
    let list = match &face {
        Some(f) => {
            let ags_path = format!("{dir}/mdl/d3d11/textureanimationchar.ags.phyre");
            let ags = read(&ags_path)?.with_context(|| format!("no {ags_path}"))?;
            files.push((ags_path, texanim::write_ags(&ags, &f.records)?));
            let template_id = &f.anim_id;
            let template_path = texanim::archive_path(template_id);
            let template = read(&template_path)?.with_context(|| format!("no {template_path}"))?;
            let mut entries = Vec::new();
            for (k, image) in f.textures.iter().enumerate() {
                let tag = texanim::anim_tag(k);
                let id = template_id.replace(&format!("_anim_{}.", f.anim_tag), &format!("_anim_{tag}."));
                ensure!(id.len() == template_id.len(), "animation texture id {id}");
                let rename = (id != *template_id).then_some((template_id.as_str(), id.as_str()));
                files.push((texanim::archive_path(&id), phyretex::argb8_one_level(&template, image, rename)?));
                entries.push(ffx_phyre::assets::texture_entry(&id));
                anim.push((tag, image.clone()));
            }
            texanim::rewrite_list(&list, &entries, &[atlas_entry])?
        }
        None => texanim::insert_files(&list, &[atlas_entry])?,
    };
    // the blended twins' shader variants
    let list = texanim::insert_files(&list, &shader_entries)?;
    files.push((list_path, list));
    Ok(Converted {
        files,
        atlas,
        anim,
        glb,
        report,
    })
}

pub fn debug(chr: &[u8], hd: &[u8]) -> Result<()> {
    let mesh = Mesh::parse(chr)?;
    let model = ffx_phyre::Model::parse(hd).context("HD model")?;
    println!("version {:#x}, {} bones, {} models", mesh.version, mesh.bones.len(), mesh.models.len());
    let ps2w = mesh.bind_world();
    let hdw = node_worlds(&model);
    for (i, w) in ps2w.iter().enumerate() {
        let Some(n) = model.nodes.iter().position(|n| n.name == format!("SkeletonNode_{i}")) else {
            println!("bone {i}: no HD node");
            continue;
        };
        let h = hdw[n];
        let d = (12..15).map(|k| (h[k] * 1000.0 - w[k]).abs()).fold(0.0, f64::max);
        let r = (0..12).map(|k| (h[k] - w[k]).abs()).fold(0.0, f64::max);
        if d > 1.0 || r > 1e-3 {
            println!("bone {i}: trans diff {d:.2}, rot diff {r:.4}");
        }
    }
    for (mi, m) in mesh.models.iter().enumerate() {
        let mut by_tex = BTreeMap::new();
        for t in &m.tris {
            *by_tex.entry((t.texture, t.flags)).or_insert(0) += 1;
        }
        let skinned = m.influences.iter().filter(|i| !i.is_empty()).count();
        let maxinf = m.influences.iter().map(Vec::len).max().unwrap_or(0);
        println!(
            "model {mi}: base {}, {} verts, {} normals, {} tris {:?}, bad corners {}, skinned {skinned} (max {maxinf}), cloths {:?}",
            m.base,
            m.vertices.len(),
            m.normals.len(),
            m.tris.len(),
            by_tex,
            m.bad_corners,
            m.cloths.iter().map(|c| c.vertices.len()).collect::<Vec<_>>()
        );
    }
    // The normal skin lists against the normal array, in the bind pose.
    for (mi, m) in mesh.models.iter().enumerate() {
        if m.normals.is_empty() {
            continue;
        }
        let base = ps2w[usize::from(m.base)];
        let mut ang: Vec<f64> = Vec::new();
        let mut none = 0;
        for (k, n) in m.normals.iter().enumerate() {
            let a = normalize(ps2mesh::rotate(&base, n.map(f64::from)));
            let inf = &m.normal_influences[k];
            if inf.is_empty() {
                none += 1;
                continue;
            }
            let mut s = [0f64; 3];
            for &(bn, w, p) in inf {
                let r = ps2mesh::rotate(&ps2w[usize::from(bn)], p.map(f64::from));
                for j in 0..3 {
                    s[j] += f64::from(w) * r[j];
                }
            }
            let s = normalize(s);
            ang.push((a[0] * s[0] + a[1] * s[1] + a[2] * s[2]).clamp(-1.0, 1.0).acos().to_degrees());
        }
        ang.sort_by(f64::total_cmp);
        let q = |f: f64| ang.get(((ang.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(0.0);
        println!(
            "model {mi}: normal skin lists vs normal array (bind pose), {} skinned, {none} without list: angle p50 {:.1} p90 {:.1} p99 {:.1} max {:.1} deg",
            ang.len(),
            q(0.5),
            q(0.9),
            q(0.99),
            q(1.0)
        );
    }
    // Per corner: the normal's bones against its vertex's (sum of |w| differences / 2).
    for (mi, m) in mesh.models.iter().enumerate() {
        let sum = |inf: &[(u16, f32, [i16; 3])]| {
            let mut s: BTreeMap<u16, f32> = BTreeMap::new();
            for &(b, w, _) in inf {
                *s.entry(b).or_default() += w;
            }
            s
        };
        let mut d: Vec<f32> = Vec::new();
        for t in &m.tris {
            for c in &t.corners {
                let (Some(vi), Some(ni)) = (m.influences.get(usize::from(c.vertex)), m.normal_influences.get(usize::from(c.normal))) else { continue };
                let (a, b) = (sum(vi), sum(ni));
                let keys: BTreeSet<u16> = a.keys().chain(b.keys()).copied().collect();
                let x = keys.iter().map(|k| (a.get(k).copied().unwrap_or(0.0) - b.get(k).copied().unwrap_or(0.0)).abs()).sum::<f32>() / 2.0;
                d.push(x);
            }
        }
        d.sort_by(f32::total_cmp);
        let q = |f: f32| d.get(((d.len() as f32 - 1.0) * f) as usize).copied().unwrap_or(0.0);
        println!("model {mi}: corner weight difference normal vs vertex p50 {:.3} p90 {:.3} p99 {:.3} max {:.3}", q(0.5), q(0.9), q(0.99), q(1.0));
    }
    // Per model and texture entry: UV box (texels), position box and the
    // bones that skin the triangles (most weight first).
    // (triangles, UV box, position box, weight per bone)
    type Group = (usize, [u16; 4], [i16; 6], BTreeMap<u16, f32>);
    for (mi, m) in mesh.models.iter().enumerate() {
        let mut groups: BTreeMap<u8, Group> = BTreeMap::new();
        for t in &m.tris {
            let g = groups.entry(t.texture).or_insert((0, [u16::MAX, u16::MAX, 0, 0], [i16::MAX, i16::MAX, i16::MAX, i16::MIN, i16::MIN, i16::MIN], BTreeMap::new()));
            g.0 += 1;
            for c in &t.corners {
                let uv = c.uv.map(|x| x / 16);
                g.1 = [g.1[0].min(uv[0]), g.1[1].min(uv[1]), g.1[2].max(uv[0]), g.1[3].max(uv[1])];
                if let Some(p) = m.vertices.get(usize::from(c.vertex)) {
                    for (k, &x) in p.iter().enumerate() {
                        g.2[k] = g.2[k].min(x);
                        g.2[k + 3] = g.2[k + 3].max(x);
                    }
                }
                for &(b, w, _) in m.influences.get(usize::from(c.vertex)).map_or(&[][..], Vec::as_slice) {
                    *g.3.entry(b).or_default() += w;
                }
            }
        }
        for (t, (n, uv, pos, bones)) in groups {
            let mut b: Vec<(u16, f32)> = bones.into_iter().collect();
            b.sort_by(|x, y| y.1.total_cmp(&x.1));
            let b: Vec<String> = b.iter().take(12).map(|(b, w)| format!("{b}:{w:.0}")).collect();
            println!("model {mi} texture {t}: {n} tris, uv {uv:?}, pos {pos:?}, bones {}", b.join(" "));
        }
    }
    // DM_BONES=110,116: per bone its parent chain and the triangles it skins
    // (any corner), by texture with their UV box.
    for b in std::env::var("DM_BONES").unwrap_or_default().split(',').filter_map(|x| x.parse::<u16>().ok()) {
        let mut chain = vec![b];
        while let Some(p) = mesh.bones.get(usize::from(*chain.last().unwrap_or(&b))).map(|x| x.parent) {
            if chain.contains(&p) || chain.len() > 64 {
                break;
            }
            chain.push(p);
        }
        let hd: Vec<bool> = chain.iter().map(|x| model.nodes.iter().any(|n| n.name == format!("SkeletonNode_{x}"))).collect();
        println!("bone {b}: parents {chain:?}, HD node {hd:?}");
        for (mi, m) in mesh.models.iter().enumerate() {
            let mut g: BTreeMap<u8, (usize, [u16; 4])> = BTreeMap::new();
            for t in &m.tris {
                if t.corners.iter().any(|c| m.influences.get(usize::from(c.vertex)).is_some_and(|i| i.iter().any(|x| x.0 == b))) {
                    let e = g.entry(t.texture).or_insert((0, [u16::MAX, u16::MAX, 0, 0]));
                    e.0 += 1;
                    for c in &t.corners {
                        let uv = c.uv.map(|x| x / 16);
                        e.1 = [e.1[0].min(uv[0]), e.1[1].min(uv[1]), e.1[2].max(uv[0]), e.1[3].max(uv[1])];
                    }
                }
            }
            for (t, (n, uv)) in g {
                println!("  model {mi} texture {t}: {n} tris, uv {uv:?}");
            }
        }
    }
    for (mi, m) in mesh.models.iter().enumerate() {
        for (ci, c) in m.cloths.iter().enumerate() {
            let mut bones: BTreeMap<u16, usize> = BTreeMap::new();
            let mut none = 0;
            for &v in &c.vertices {
                let inf = m.influences.get(usize::from(v)).cloned().unwrap_or_default();
                if inf.is_empty() {
                    none += 1;
                }
                for (b, _, _) in inf {
                    *bones.entry(b).or_default() += 1;
                }
            }
            let set: BTreeSet<u16> = c.vertices.iter().copied().collect();
            let mut tex: BTreeMap<u8, usize> = BTreeMap::new();
            for t in m.tris.iter().filter(|t| t.corners.iter().all(|c| set.contains(&c.vertex))) {
                *tex.entry(t.texture).or_default() += 1;
            }
            println!(
                "model {mi} cloth {ci}: {} vertices, unskinned {none}, bones {bones:?}, triangles by texture {tex:?}",
                c.vertices.len()
            );
        }
    }
    // DM_CLOTH=1: each cloth piece's pinned and moving vertices (bind
    // pose, PS2 units, y down).
    if std::env::var_os("DM_CLOTH").is_some() {
        for (mi, m) in mesh.models.iter().enumerate() {
            let base = ps2w.get(usize::from(m.base)).copied();
            for (ci, c) in m.cloths.iter().enumerate() {
                for (name, run) in [("pinned", &c.vertices[..c.pinned.min(c.vertices.len())]), ("moving", c.moving())] {
                    let mut lo = [f64::MAX; 3];
                    let mut hi = [f64::MIN; 3];
                    for &v in run {
                        let Some(p) = m.vertices.get(usize::from(v)) else { continue };
                        let p = p.map(f64::from);
                        let w = base.map_or(p, |b| crate::formats::ps2mesh::transform(&b, p));
                        for k in 0..3 {
                            lo[k] = lo[k].min(w[k]);
                            hi[k] = hi[k].max(w[k]);
                        }
                    }
                    println!("model {mi} cloth {ci} {name}: {} vertices, bind pose from {lo:.0?} to {hi:.0?}", run.len());
                }
            }
        }
    }
    for (i, inst) in model.instances.iter().enumerate() {
        println!("instance {i} {:?}: skeleton {:?}, segments {:?}", inst.name, inst.skeleton, inst.segments);
    }
    for (i, m) in model.materials.iter().enumerate() {
        let anim: Vec<String> = m
            .parameters
            .iter()
            .filter(|p| p.name.contains("Anim") || p.name.contains("Cloth"))
            .map(|p| format!("{}={:?}", p.name, p.values))
            .collect();
        println!(
            "material {i}: {:?} {}",
            m.textures.iter().map(|t| format!("{}={}", t.sampler, t.path)).collect::<Vec<_>>(),
            anim.join(" ")
        );
    }
    for (s, seg) in model.segments.iter().enumerate() {
        let mat = seg.material.and_then(|m| model.materials.get(m));
        println!(
            "hd seg {s}: {} verts, {} tris, palette {}, material {:?} {:?} alpha {:?} cloth {} color0 {:?}",
            seg.positions.len(),
            seg.indices.len() / 3,
            seg.bones.len(),
            seg.material,
            mat.map(|m| m.textures.iter().map(|t| format!("{}={}", t.sampler, t.path)).collect::<Vec<_>>()),
            mat.and_then(|m| m.parameter("AlphaThreshold")),
            mat.is_some_and(|m| m.parameter("ClothWorld").is_some()),
            seg.colors.first()
        );
        // DM_SEGS=1,2: the segment's vertices (position, colour)
        if std::env::var("DM_SEGS").unwrap_or_default().split(',').any(|x| x.parse() == Ok(s)) {
            for (p, c) in seg.positions.iter().zip(seg.colors.iter().map(Some).chain(std::iter::repeat(None))) {
                println!("  {:8.3} {:8.3} {:8.3}  {:?}", p[0], p[1], p[2], c);
            }
        }
    }
    Ok(())
}
