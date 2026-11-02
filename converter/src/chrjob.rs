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
    let chr = Chr::parse(chr).with_context(|| format!("{name}.chr"))?;
    let hd_path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
    let Some(entry) = pc.find(&hd_path) else {
        return Ok(None);
    };
    let model = ffx_phyre::Model::parse(&pc.read(entry)?).with_context(|| hd_path.clone())?;

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
    for seg in &model.segments {
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
        let e = by_page.entry(tex.archive_path()).or_default();
        e.0.extend(jobs);
        e.1.extend(pending);
        e.2 += total;
    }

    let mut report = Report {
        frame_share,
        pages: Vec::new(),
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
        report.pages.push(Page {
            path,
            image,
            matched,
            covered,
            scale,
            file,
        });
    }
    Ok(Some(report))
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
        let (mut pages, mut kept, mut failed) = (0, 0, 0);
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
            "{} models, {pages} pages, {kept} pages of remodelled meshes left HD",
            names.len()
        );
        if failed > 0 {
            s += &format!(", {failed} models unreadable and left HD (--verbose lists them)");
        }
        Ok(s)
    }
}
