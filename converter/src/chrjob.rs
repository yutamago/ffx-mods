//! Character textures: PS2 texels painted into the HD pages of every
//! character whose HD model kept the PS2 mesh.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::bake;
use crate::config::Config;
use crate::disc::Disc;
use crate::jobs::{Ctx, Job, Output};
use crate::phyretex::{self, Rgba};
use crate::ps2chr::Chr;
use crate::texanim::{self, TexAnim};

/// The share of an HD page's triangles that must have a PS2 twin; pages
/// below it belong to remodelled meshes and are left alone. Above it, the
/// triangles without a twin keep the HD pixels (scaled down).
pub const MIN_MATCHED: f32 = 0.5;

pub struct Page {
    /// Archive path of the HD texture.
    pub path: String,
    pub image: Rgba,
    pub matched: f32,
    pub covered: f32,
    pub scale: u32,
    /// The finished `*.dds.phyre`.
    pub file: Vec<u8>,
}

pub struct Report {
    pub frame_share: f32,
    pub pages: Vec<Page>,
    /// The HD model with the materials of the segments that sample
    /// see-through PS2 texels blended (`chralpha`) and its asset list with
    /// the shader variants added: (archive path, bytes) each, and a note.
    /// Only for models whose mesh was kept (not remodelled).
    pub model: Option<(Vec<(String, Vec<u8>)>, String)>,
    /// HD pages skipped, with the share of matched triangles.
    pub skipped: Vec<(String, f32)>,
}

/// `kind`: `pc`, `mon`, `npc`, `sum`, `wep`, `obj`, `skl`; `name`: `m001`.
pub fn bake(
    disc: &mut Disc,
    pc: &ffx_vbf::Archive,
    kind: &str,
    name: &str,
) -> Result<Option<Report>> {
    let Some(chr) = disc.read(&format!("jppc/chr/{kind}/{name}/mdl/{name}.chr"))? else {
        return Ok(None);
    };
    bake_chr(&chr, pc, kind, name)
}

/// [`bake`] with the PS2 `.chr` already read.
pub fn bake_chr(chr: &[u8], pc: &ffx_vbf::Archive, kind: &str, name: &str) -> Result<Option<Report>> {
    bake_chr_with(chr, pc, kind, name, true)
}

/// The HD pages [`bake_chr`] would skip (remodelled meshes), without
/// painting.
pub fn skipped_chr(chr: &[u8], pc: &ffx_vbf::Archive, kind: &str, name: &str) -> Result<Option<Report>> {
    bake_chr_with(chr, pc, kind, name, false)
}

fn bake_chr_with(
    chr: &[u8],
    pc: &ffx_vbf::Archive,
    kind: &str,
    name: &str,
    paint: bool,
) -> Result<Option<Report>> {
    let raw = chr;
    let chr = Chr::parse(chr).with_context(|| format!("{name}.chr"))?;
    let hd_path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
    let Some(entry) = pc.find(&hd_path) else {
        return Ok(None);
    };
    let hd_bytes = pc.read(entry)?;
    let model = ffx_phyre::Model::parse(&hd_bytes).with_context(|| hd_path.clone())?;

    let ps2_pos: Vec<[i32; 3]> = chr
        .triangles
        .iter()
        .flat_map(|t| t.corners.map(|c| c.pos.map(i32::from)))
        .collect();
    let hd_pos: Vec<[f32; 3]> = model
        .segments
        .iter()
        .flat_map(|s| s.positions.iter().copied())
        .collect();
    let Some((frame, frame_share)) = bake::find_frame(&ps2_pos, &hd_pos) else {
        return Ok(None);
    };
    let index = bake::Index::new(&chr);

    // HD triangles per base-colour texture.
    let mut by_page: BTreeMap<String, (Vec<bake::Job>, Vec<bake::Pending>, usize)> =
        BTreeMap::new();
    // Segments with a triangle whose PS2 twin samples see-through texels,
    // with their page.
    let mut see_through: Vec<(usize, String)> = Vec::new();
    for (s, seg) in model.segments.iter().enumerate() {
        let Some(tex) = seg
            .material
            .and_then(|m| model.materials.get(m))
            .and_then(|m| m.base_color())
        else {
            continue;
        };
        if seg.uvs.is_empty() {
            continue;
        }
        let (jobs, pending, total) =
            bake::match_triangles(&index, &frame, &seg.positions, &seg.uvs, &seg.indices);
        if paint {
            let (mut n, mut all) = (0, 0);
            for j in &jobs {
                let (a, b) = see_through_job(&chr, j);
                n += a;
                all += b;
            }
            let share = n as f32 / all.max(1) as f32;
            if std::env::var_os("DM_ALPHA").is_some() {
                eprintln!("{name} seg {s}: {} tris, see-through {n} of {all} texels ({:.2}%)", jobs.len(), share * 100.0);
            }
            if share >= crate::chralpha::MIN_SHARE {
                see_through.push((s, tex.archive_path()));
            }
        }
        let e = by_page.entry(tex.archive_path()).or_default();
        e.0.extend(jobs);
        e.1.extend(pending);
        e.2 += total;
    }

    let mut report = Report {
        frame_share,
        pages: Vec::new(),
        model: None,
        skipped: Vec::new(),
    };
    for (path, (mut jobs, pending, total)) in by_page {
        // within a quarter of a PS2 texel of a 256 page
        let more = bake::second_pass(&index, &jobs, &pending, 1.0 / 1024.0);
        if !paint {
            let matched = (jobs.len() + more.len()) as f32 / total.max(1) as f32;
            if matched < MIN_MATCHED {
                report.skipped.push((path, matched));
            }
            continue;
        }
        if std::env::var_os("DM_DEBUG").is_some() {
            let count = |js: &[bake::Job]| {
                let mut c = std::collections::BTreeMap::new();
                for j in js {
                    *c.entry(j.material).or_insert(0) += 1;
                }
                c
            };
            eprintln!(
                "{path}: exact {:?} second {:?} pending {}",
                count(&jobs),
                count(&more),
                pending.len()
            );
            let mut ps2 = std::collections::BTreeMap::new();
            for t in &chr.triangles {
                *ps2.entry(t.material).or_insert(0) += 1;
            }
            eprintln!("  ps2 triangles by material {ps2:?}");
        }
        jobs.extend(more);
        let matched = jobs.len() as f32 / total.max(1) as f32;
        if matched < MIN_MATCHED {
            report.skipped.push((path, matched));
            continue;
        }
        let Some(e) = pc.find(&path) else {
            report.skipped.push((path, matched));
            continue;
        };
        let template = pc.read(e)?;
        let hd = ffx_phyre::Texture::parse(template.clone()).with_context(|| path.clone())?;
        let scale = bake::density_scale(&jobs, (hd.width, hd.height));
        let size = ((hd.width / scale).max(1), (hd.height / scale).max(1));
        let hd_rgba = Rgba {
            width: hd.width,
            height: hd.height,
            pixels: hd.to_rgba(),
        };
        let (image, covered) = bake::paint(&chr, &jobs, size, Some(&hd_rgba));
        let file = phyretex::argb8_from(&template, &image).with_context(|| path.clone())?;
        // The HD's animation frames over this page, painted from the PS2
        // patches (eyes, mouths).
        let frames = anim_frames(raw, &chr, &model, pc, kind, name, &path, &jobs, &image)
            .with_context(|| format!("{path}: texture animation"))?;
        report.pages.push(Page {
            path,
            image,
            matched,
            covered,
            scale,
            file,
        });
        report.pages.extend(frames);
    }
    // The PS2 drew those texels see-through (`chralpha`); where the HD
    // material does not blend, it gets its blended twin. A remodelled
    // model is the model converter's (`models::remodelled`).
    let remodelled = !report.skipped.is_empty() && report.frame_share < 0.6;
    let painted: std::collections::BTreeSet<&str> = report.pages.iter().map(|p| p.path.as_str()).collect();
    let segs: Vec<u32> = see_through
        .iter()
        .filter(|(_, page)| painted.contains(page.as_str()))
        .map(|&(s, _)| u32::try_from(s))
        .collect::<Result<_, _>>()?;
    if !remodelled && !segs.is_empty() {
        let variants = crate::chralpha::Variants::get(pc)?;
        let mut edit = ffx_phyre::edit::ClusterEdit::parse(&hd_bytes).context("edit")?;
        let note = crate::chralpha::blend_in_place(&mut edit, &segs, variants)?;
        if note.switched + note.lowered > 0 {
            let mut files = vec![(hd_path.clone(), edit.write().context("writing")?)];
            if note.switched > 0 {
                let list_path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/{name}.ahwin32");
                let list = pc.read(pc.find(&list_path).with_context(|| format!("no {list_path}"))?)?;
                let entries = crate::chralpha::list_entries(&edit)?;
                files.push((list_path, ffx_phyre::assets::add_files(&list, &entries)?));
            }
            report.model = Some((files, note.text));
        }
    }
    Ok(Some(report))
}

/// The texels the PS2 triangle of `job` samples and how many are
/// see-through.
fn see_through_job(chr: &Chr, job: &bake::Job) -> (usize, usize) {
    let uv = job.ps2_uv.map(|[u, v]| [f64::from(u) / 16.0, f64::from(v) / 16.0]);
    crate::chralpha::coverage(uv, |x, y| {
        chr.sample(job.material, (x * 16 + 8) as f32, (y * 16 + 8) as f32).map_or(255, |c| c[3])
    })
}

/// The animation textures of a kept mesh whose animated material
/// (`AnimSampler`) draws over `page`: each HD frame painted like the page,
/// from the PS2 image with the frame's PS2 patch put in, and cut to the
/// rectangle the shader shows it in (`texanim.rs`). The HD `.ags` stays.
#[allow(clippy::too_many_arguments)]
fn anim_frames(
    raw: &[u8],
    chr: &Chr,
    model: &ffx_phyre::Model,
    pc: &ffx_vbf::Archive,
    kind: &str,
    name: &str,
    page: &str,
    jobs: &[bake::Job],
    painted: &Rgba,
) -> Result<Vec<Page>> {
    let Some(mat) = model.materials.iter().find(|m| {
        m.texture("AnimSampler").is_some() && m.base_color().is_some_and(|t| t.archive_path() == page)
    }) else {
        return Ok(Vec::new());
    };
    let Ok(Some(anim)) = TexAnim::parse(raw) else {
        return Ok(Vec::new());
    };
    let dir = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}");
    let read = |p: &str| -> Result<Option<Vec<u8>>> { pc.find(p).map(|e| pc.read(e)).transpose().map_err(Into::into) };
    let Some(ags) = read(&format!("{dir}/mdl/d3d11/textureanimationchar.ags.phyre"))? else {
        return Ok(Vec::new());
    };
    let Some(list) = read(&format!("{dir}/{name}.ahwin32"))? else {
        return Ok(Vec::new());
    };
    let records = texanim::read_ags(&ags)?;
    let files = texanim::anim_files(&list)?;
    let maya = mat.parameter("AnimTranslate_MAYA").zip(mat.parameter("AnimScale_MAYA"));
    let (w, h) = (painted.width, painted.height);
    let mut out = Vec::new();
    for (n, file) in files.iter().enumerate() {
        // the first frame that shows texture n: its patch and rectangle
        let found = records.iter().find_map(|r| {
            let i = r.ids.iter().position(|&id| id as usize == n)?;
            let e = anim.entries.iter().find(|e| r.group.strip_prefix('G').and_then(|g| g.parse().ok()) == Some(e.group))?;
            let s: usize = r.animation.strip_prefix("Animation")?.parse().ok()?;
            let f = e.states.iter().find(|x| x.0 == s)?.1.get(i)?;
            let (t, sc) = match maya {
                Some((t, sc)) => ([t[0], t[1]], [sc[0], sc[1]]),
                None => ([r.rect[2], r.rect[3]], [r.rect[0], r.rect[1]]),
            };
            Some((e, f.patch, t, sc))
        });
        let Some((e, patch, t, sc)) = found else { continue };
        let Some(template) = read(file)? else { continue };
        // the rectangle in page pixels, rows top first (v up in the file)
        let px = |u: f32| (u * w as f32).clamp(0.0, w as f32);
        let py = |v: f32| ((1.0 - v) * h as f32).clamp(0.0, h as f32);
        let (x0, x1) = (px(t[0]).floor() as u32, px(t[0] + sc[0]).ceil() as u32);
        let (y0, y1) = (py(t[1] + sc[1]).floor() as u32, py(t[1]).ceil() as u32);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let mut with = chr.clone();
        let p = &anim.patches[patch];
        let Some(img) = with.images.get_mut(e.image) else { continue };
        for y in 0..p.height {
            for x in 0..p.width {
                let (dx, dy) = (e.x + x, e.y + y);
                if dx < img.width && dy < img.height {
                    img.texels[(dy * img.width + dx) as usize] = p.texels[(y * p.width + x) as usize];
                }
            }
        }
        let near: Vec<bake::Job> = jobs
            .iter()
            .filter(|j| {
                let xs = j.hd_uv.map(|q| q[0] * w as f32);
                let ys = j.hd_uv.map(|q| (1.0 - q[1]) * h as f32);
                xs.iter().copied().fold(f32::MIN, f32::max) >= x0 as f32
                    && xs.iter().copied().fold(f32::MAX, f32::min) <= x1 as f32
                    && ys.iter().copied().fold(f32::MIN, f32::max) >= y0 as f32
                    && ys.iter().copied().fold(f32::MAX, f32::min) <= y1 as f32
            })
            .cloned()
            .collect();
        let (full, _) = bake::paint(&with, &near, (w, h), Some(painted));
        let mut image = Rgba::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            for x in x0..x1 {
                image.put(x - x0, y - y0, full.get(x.min(w - 1), y.min(h - 1)));
            }
        }
        let file_bytes = phyretex::argb8_from(&template, &image).with_context(|| file.clone())?;
        out.push(Page { path: file.clone(), image, matched: 1.0, covered: 1.0, scale: 1, file: file_bytes });
    }
    Ok(out)
}

/// Every `(kind, name)` with a PS2 model on the disc, sorted.
pub fn models(disc: &Disc) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = disc
        .names()
        .filter_map(|n| {
            let rest = n.strip_prefix("jppc/chr/")?;
            let mut p = rest.split('/');
            let (kind, name, mdl, file) = (p.next()?, p.next()?, p.next()?, p.next()?);
            (mdl == "mdl" && file == format!("{name}.chr"))
                .then(|| (kind.to_string(), name.to_string()))
        })
        .collect();
    v.sort();
    v
}

/// One build job per character kind: the PS2 textures painted into the HD
/// pages of the models whose mesh the HD version kept.
struct ChrJob {
    name: &'static str,
    settings: &'static [&'static str],
    /// Folders under `chr/` (`mon`).
    kinds: &'static [&'static str],
    enabled: fn(&Config) -> bool,
}

/// The party's remodelled meshes are the model converter's; this job only
/// covers the `pc` models whose mesh was kept (one test model). `skl`
/// (skeletons and dummies, 4 textured) goes with the objects.
pub fn jobs() -> Vec<Box<dyn Job>> {
    fn job(
        name: &'static str,
        settings: &'static [&'static str],
        kinds: &'static [&'static str],
        enabled: fn(&Config) -> bool,
    ) -> Box<dyn Job> {
        Box::new(ChrJob {
            name,
            settings,
            kinds,
            enabled,
        })
    }
    vec![
        job("chr.party", &["models.party"], &["pc"], |c| c.models.party.ps2()),
        job("chr.monster", &["models.monster"], &["mon"], |c| c.models.monster.ps2()),
        job("chr.npc", &["models.npc"], &["npc"], |c| c.models.npc.ps2()),
        job("chr.aeon", &["models.aeon"], &["sum"], |c| c.models.aeon.ps2()),
        job("chr.weapon", &["models.weapon"], &["wep"], |c| c.models.weapon.ps2()),
        job("chr.object", &["models.object"], &["obj", "skl"], |c| c.models.object.ps2()),
    ]
}

impl Job for ChrJob {
    /// 2: textures with mip chains (phyretex::mip_chain).
    fn version(&self) -> u32 {
        2
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
        let names: Vec<(String, String)> = ctx.disc(|d| {
            Ok(models(d)
                .into_iter()
                .filter(|(k, _)| self.kinds.contains(&k.as_str()))
                .collect())
        })?;
        let (mut pages, mut kept, mut failed, mut blended) = (0, 0, 0, 0);
        // A model can use another model's page. The page's own model wins,
        // then the first in name order; pages of other models wait until
        // the end. Baking runs in parallel in chunks, writing in name order,
        // so every build writes the same files.
        let mut foreign: Vec<Page> = Vec::new();
        for chunk in names.chunks(32) {
            let results: Vec<Result<Option<Report>>> = chunk
                .par_iter()
                .map(|(kind, name)| {
                    let chr =
                        ctx.disc(|d| d.read(&format!("jppc/chr/{kind}/{name}/mdl/{name}.chr")))?;
                    Ok(chr
                        .map(|chr| bake_chr(&chr, ctx.pc, kind, name))
                        .transpose()?
                        .flatten())
                })
                .collect();
            for ((kind, name), r) in chunk.iter().zip(results) {
                match r {
                    // Dummies without a mesh or with an unsupported texture format.
                    Err(e) => {
                        failed += 1;
                        ctx.note(format!("{kind}/{name}: left HD: {e:#}"));
                    }
                    Ok(None) => ctx.note(format!("{kind}/{name}: no PS2 or HD model")),
                    Ok(Some(r)) => {
                        for (p, m) in &r.skipped {
                            ctx.note(format!(
                                "{p}: kept HD (remodelled, {:.0}% triangles matched)",
                                m * 100.0
                            ));
                        }
                        kept += r.skipped.len();
                        if let Some((files, note)) = &r.model {
                            ctx.note(format!("{kind}/{name}: see-through texels: {note}"));
                            blended += 1;
                            for (path, bytes) in files {
                                out.put(path, bytes)?;
                            }
                        }
                        let own = format!("/chr/{kind}/{name}/");
                        for p in r.pages {
                            ctx.note(format!(
                                "{kind}/{name}: {} -> {}x{}, {:.0}% triangles matched",
                                p.path,
                                p.image.width,
                                p.image.height,
                                p.matched * 100.0
                            ));
                            if p.path.contains(&own) {
                                pages += usize::from(out.put(&p.path, &p.file)?);
                            } else {
                                foreign.push(p);
                            }
                        }
                    }
                }
            }
        }
        for p in foreign {
            pages += usize::from(out.put(&p.path, &p.file)?);
        }
        let mut s = format!(
            "{} models, {pages} pages, {kept} pages of remodelled meshes left HD, {blended} models with see-through materials blended",
            names.len()
        );
        if failed > 0 {
            s += &format!(", {failed} models unreadable and left HD (--verbose lists them)");
        }
        Ok(s)
    }
}
