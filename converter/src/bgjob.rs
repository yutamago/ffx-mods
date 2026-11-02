//! `maps.backgrounds`: the PS2 paintings of the 2D-background rooms
//! (`docs/screen.md`, "2D backgrounds").
//!
//! A PC 2D room draws its layers from `map/<area>/<map>/2d/tex/d3d11/
//! <layer>_<part>_<w>_<h>_<frame>.dds.phyre`, full-screen quads in PS2
//! units (512 x 448). The PS2 pictures are in the disc's
//! `jppc/map/<area>/<map>/bin/mapout.vpa`, section 0 (`YNSC`): one PSMT8
//! image transfer per part with its CLUT after it ([`uijob::bg_layers`]),
//! zeroed in the PC's copy. The parts come in the same order as the HD
//! names sorted by (layer, part), with sizes a few pixels apart (the PS2
//! pads them). Parts with several frames (texture animation) stay HD: the
//! PS2 transfer holds one picture.

use anyhow::Result;

use crate::config::Config;
use crate::disc::Disc;
use crate::jobs::{Ctx, Job, Output};
use crate::phyretex::{self, Rgba};
use crate::uijob::{self, Done};

const PS3: &str = "ffx_data/gamedata/ps3data";

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(BgJob)]
}

struct BgJob;

impl Job for BgJob {
    fn name(&self) -> &'static str {
        "maps.backgrounds"
    }
    fn settings(&self) -> &'static [&'static str] {
        &["maps.backgrounds"]
    }
    fn enabled(&self, config: &Config) -> bool {
        config.maps.backgrounds.ps2()
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        let (done, notes) = ctx.disc(|d| convert(d, ctx.pc, &[]))?;
        let mut n = 0;
        for d in &done {
            if out.put(&d.path, &d.file)? {
                n += 1;
            }
            ctx.note(&d.note);
        }
        for line in &notes {
            ctx.note(line);
        }
        Ok(format!("{n} layers ({} notes, see --verbose)", notes.len()))
    }
}

/// `<layer>_<part>_<w>_<h>_<frame>`.
fn parse_name(file: &str) -> Option<[u32; 5]> {
    let s = file.strip_suffix(".dds.phyre")?;
    let v: Vec<u32> = s.split('_').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    v.try_into().ok()
}

/// The HD 2D rooms: (`<area>/<map>`, the texture paths under it).
fn hd_rooms(pc: &ffx_vbf::Archive) -> Vec<(String, Vec<String>)> {
    let mut rooms: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for e in pc.entries() {
        let Some(rest) = e.path.strip_prefix(&format!("{PS3}/map/")) else { continue };
        let Some((room, file)) = rest.split_once("/2d/tex/d3d11/") else { continue };
        if room.starts_with("titl/") || parse_name(file).is_none() {
            continue; // the title screen is ui.title's
        }
        rooms.entry(room.to_string()).or_default().push(e.path.clone());
    }
    rooms.into_iter().collect()
}

/// Converts the rooms whose `<area>/<map>` or map name is in `only` (all
/// when empty).
pub fn convert(d: &mut Disc, pc: &ffx_vbf::Archive, only: &[String]) -> Result<(Vec<Done>, Vec<String>)> {
    let mut done = Vec::new();
    let mut notes = Vec::new();
    for (room, paths) in hd_rooms(pc) {
        let map = room.rsplit('/').next().unwrap_or(&room);
        if !only.is_empty() && !only.iter().any(|o| o == &room || o == map || room.starts_with(&format!("{o}/"))) {
            continue;
        }
        let Some(vpa) = d.read(&format!("jppc/map/{room}/bin/mapout.vpa"))? else {
            notes.push(format!("{room}: no PS2 map"));
            continue;
        };
        let layers = uijob::bg_layers(&vpa);
        // HD parts in (layer, part) order, with their frames.
        let mut parts: Vec<([u32; 2], u32, u32, Vec<String>)> = Vec::new();
        let mut named: Vec<([u32; 5], String)> = paths
            .iter()
            .filter_map(|p| parse_name(p.rsplit('/').next()?).map(|n| (n, p.clone())))
            .collect();
        named.sort();
        for (n, p) in named {
            match parts.last_mut() {
                Some(last) if last.0 == [n[0], n[1]] => last.3.push(p),
                _ => parts.push(([n[0], n[1]], n[2], n[3], vec![p])),
            }
        }
        if parts.len() != layers.len() {
            notes.push(format!("{room}: {} HD parts, {} PS2 pictures; kept HD", parts.len(), layers.len()));
            continue;
        }
        if let Some((k, p)) = parts
            .iter()
            .enumerate()
            .find(|(k, p)| p.1.abs_diff(layers[*k].width) > 8 || p.2.abs_diff(layers[*k].height) > 8)
        {
            notes.push(format!(
                "{room}: part {:?} is {}x{}, PS2 picture {k} {}x{}; kept HD",
                p.0, p.1, p.2, layers[k].width, layers[k].height
            ));
            continue;
        }
        // Some rooms were re-framed for 16:9 (bika01: the HD shows more at
        // the sides, the PS2 picture is a zoomed crop). Their PS2 pictures
        // would not line up with the 3D scene, so the whole room stays HD
        // unless its full-screen pictures match the HD ones closely.
        let mut framing = 1.0f32;
        for (k, (_, w, h, frames)) in parts.iter().enumerate() {
            if *w == 512 && *h == 448 && frames.len() == 1 {
                if let Some((_, hd)) = uijob::load_hd(pc, &frames[0])? {
                    let img = &layers[k];
                    let small = crate::mapjob::resize(&hd, img.width, img.height);
                    framing = framing.min(crate::mapjob::correlation(&small, img));
                }
            }
        }
        if framing < 0.85 {
            notes.push(format!("{room}: framed differently from the HD (correlation {framing:.2}); kept HD"));
            continue;
        }
        for (k, (lp, _, _, frames)) in parts.iter().enumerate() {
            if frames.len() > 1 {
                notes.push(format!("{room}: part {lp:?} has {} frames, kept HD", frames.len()));
                continue;
            }
            let path = &frames[0];
            let Some((template, hd)) = uijob::load_hd(pc, path)? else { continue };
            let img = &layers[k];
            let small = crate::mapjob::resize(&hd, img.width, img.height);
            let c = crate::mapjob::correlation(&small, img);
            if c < 0.5 {
                notes.push(format!("{path}: PS2 picture {k} looks different (correlation {c:.2}), kept HD"));
                continue;
            }
            let img = alpha_like(img, &hd);
            done.push(Done {
                file: phyretex::argb8_from(&template, &img)?,
                preview: uijob::side_by_side(&hd, &img),
                note: format!("{path}: PS2 picture {k} (correlation {c:.2})"),
                path: path.clone(),
            });
        }
    }
    Ok((done, notes))
}

/// The PS2 picture with the HD layer's opacity where the HD is fully
/// opaque (the base layer); otherwise the PS2 alpha.
fn alpha_like(img: &Rgba, hd: &Rgba) -> Rgba {
    crate::mapjob::opaque_like(img.clone(), hd)
}
