//! Character textures: PS2 texels painted into the HD pages of every
//! character whose HD model kept the PS2 mesh.

use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::bake;
use crate::disc::Disc;
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
    let chr = Chr::parse(&chr).with_context(|| format!("{name}.chr"))?;
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
