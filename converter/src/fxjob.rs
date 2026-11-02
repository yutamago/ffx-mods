//! Effect textures: spells, summons, overdrives (`magic/magic_NNNN/`) and
//! the particle effects of field and battle maps (`<map>/fp/`). Each HD
//! texture is named after the PS2 upload it replaces, so the PS2 image
//! goes in under the same name (`docs/effects.md`).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};

use crate::disc::Disc;
use crate::phyretex::{self, Rgba};
use crate::ps2fx::{self, Gs};

const HD: &str = "ffx_data/gamedata/ps3data";
const EUROPEAN: [&str; 5] = ["tex_de/", "tex_fr/", "tex_it/", "tex_es/", "tex_us/"];

/// One effect: a PS2 file and the HD folder of its textures.
#[derive(Debug, Clone)]
pub struct Source {
    /// `magic_0007`, `map/bika/bika00`, `btlmap/bika/bika00_a`.
    pub id: String,
    /// PS2 file (`_host0/magic/mag_0007.bin`, `jppc/map/.../bin/mapout.txa`).
    pub ps2: String,
    /// HD folder holding `tex*/d3d11/*.dds.phyre`.
    pub hd_dir: String,
}

impl Source {
    /// `7`, `0007`, `magic_0007`, `map/bika/bika00`, `btlmap/bika/bika00_a`,
    /// `chr/mon/m028`.
    pub fn parse(spec: &str) -> Option<Self> {
        let spec = spec.trim_matches('/');
        if let Some(rest) = spec.strip_prefix("chr/") {
            let (kind, name) = rest.split_once('/')?;
            return Some(Self {
                id: format!("chr/{kind}/{name}"),
                ps2: format!("jppc/chr/{kind}/{name}/mdl/{name}.chr"),
                hd_dir: format!("{HD}/chr/{kind}/{name}/fp"),
            });
        }
        if let Some(rest) = spec
            .strip_prefix("map/")
            .map(|r| ("map", r))
            .or_else(|| spec.strip_prefix("btlmap/").map(|r| ("btlmap", r)))
        {
            let (kind, path) = rest;
            return Some(Self {
                id: format!("{kind}/{path}"),
                ps2: format!("jppc/{kind}/{path}/bin/mapout.txa"),
                hd_dir: format!("{HD}/{kind}/{path}/fp"),
            });
        }
        let spec = spec.strip_prefix("magic/").unwrap_or(spec);
        let n: u32 = spec.strip_prefix("magic_").unwrap_or(spec).parse().ok()?;
        Some(Self {
            id: format!("magic_{n:04}"),
            ps2: format!("_host0/magic/mag_{n:04}.bin"),
            hd_dir: format!("{HD}/magic/magic_{n:04}"),
        })
    }
}

/// The HD archive's effect textures: folder -> files under it
/// (`tex/d3d11/13312_19_0_0_256_256.dds.phyre`).
pub struct HdIndex {
    dirs: BTreeMap<String, BTreeSet<String>>,
}

impl HdIndex {
    pub fn new(pc: &ffx_vbf::Archive) -> Self {
        let mut dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for e in pc.entries() {
            let p = e.path.as_str();
            if !p.ends_with(".dds.phyre") {
                continue;
            }
            // .../<dir>/<tex folder>/d3d11/<name>.dds.phyre
            let mut parts = p.rsplitn(4, '/');
            let (Some(file), Some("d3d11"), Some(folder), Some(dir)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            if !folder.starts_with("tex") {
                continue;
            }
            let is_fx = dir.starts_with(&format!("{HD}/magic/")) || dir.ends_with("/fp");
            if is_fx {
                dirs.entry(dir.to_string())
                    .or_default()
                    .insert(format!("{folder}/d3d11/{file}"));
            }
        }
        Self { dirs }
    }

    /// Every effect the disc and the HD archive both have.
    pub fn sources(&self, disc: &Disc) -> Vec<Source> {
        let names: BTreeSet<&str> = disc.names().collect();
        let mut out = Vec::new();
        for dir in self.dirs.keys() {
            let rel = &dir[HD.len() + 1..];
            let spec = rel.strip_suffix("/fp").unwrap_or(rel);
            if let Some(s) = Source::parse(spec).filter(|s| names.contains(s.ps2.as_str())) {
                out.push(s);
            }
        }
        out
    }

    /// HD effect folders with no PS2 file on this disc.
    pub fn without_ps2(&self, disc: &Disc) -> Vec<String> {
        let names: BTreeSet<&str> = disc.names().collect();
        self.dirs
            .keys()
            .filter(|dir| {
                let rel = &dir[HD.len() + 1..];
                let spec = rel.strip_suffix("/fp").unwrap_or(rel);
                Source::parse(spec).is_none_or(|s| !names.contains(s.ps2.as_str()))
            })
            .cloned()
            .collect()
    }
}

/// The HD language folder for the disc's region (`docs/effects.md`,
/// "Languages"), and whether the base folder `tex/` is in that language too.
fn language_folder(region: &str) -> (Option<String>, bool) {
    match region {
        "de" | "fr" | "it" | "es" => (Some(format!("tex_{region}")), false),
        "us" | "uk" => (Some("tex_us".into()), true),
        _ => (None, false),
    }
}

pub struct Done {
    /// Archive path of the HD texture.
    pub path: String,
    pub image: Rgba,
    pub file: Vec<u8>,
    /// HD size.
    pub hd: (u32, u32),
    /// Correlation with the HD texture shrunk to the PS2 size, and the
    /// alpha difference (`difference`); `None` if the sizes don't divide.
    pub diff: Option<(f32, f32)>,
    /// How it was turned to match the HD texture (`ORIENTATIONS`).
    pub orientation: &'static str,
    /// The PS2 side-by-side preview (HD left, PS2 right).
    pub preview: Rgba,
}

#[derive(Default)]
pub struct Report {
    pub done: Vec<Done>,
    /// HD textures with no PS2 image of their name (kept HD).
    pub hd_only: Vec<String>,
    /// `trxtex_*`: copies of the frame the effect makes at run time.
    pub runtime: Vec<String>,
    /// Text textures of other languages (kept HD).
    pub other_language: Vec<String>,
    /// PS2 images no HD texture is named after.
    pub ps2_unused: Vec<String>,
    /// HD textures that failed.
    pub failed: Vec<(String, String)>,
    /// PS2 images without a palette (written grey).
    pub missing_clut: Vec<String>,
}

pub fn convert(
    disc: &mut Disc,
    pc: &ffx_vbf::Archive,
    hd: &HdIndex,
    src: &Source,
    gs: &mut Gs,
) -> Result<Report> {
    let mut report = Report::default();
    let Some(files) = hd.dirs.get(&src.hd_dir) else {
        return Ok(report);
    };
    let bytes = disc
        .read(&src.ps2)?
        .with_context(|| format!("{} is not on the disc", src.ps2))?;
    let tables = ps2fx::tables(&bytes);
    let images = ps2fx::images(&bytes, &tables, gs);

    let (lang, base_is_lang) = language_folder(&disc.region);
    // Names with a European language variant are text. (`tex_cn` has a
    // copy of every texture.)
    let localized: BTreeSet<&str> = files
        .iter()
        .filter(|f| EUROPEAN.iter().any(|l| f.starts_with(l)))
        .filter_map(|f| f.rsplit('/').next())
        .collect();

    // The HD textures to replace: (path, folder, set, name).
    let mut wanted = Vec::new();
    for rel in files {
        let path = format!("{}/{rel}", src.hd_dir);
        let mut parts = rel.split('/');
        let (Some(folder), _, Some(file)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let name = file.trim_end_matches(".dds.phyre");
        if name.starts_with("trxtex") {
            report.runtime.push(path);
            continue;
        }
        let set: Option<usize> = match folder {
            "tex" => {
                if localized.contains(file) && !base_is_lang {
                    report.other_language.push(path);
                    continue;
                }
                Some(0)
            }
            f if f.starts_with("tex_") => {
                if lang.as_deref() != Some(f) {
                    report.other_language.push(path);
                    continue;
                }
                Some(0)
            }
            f => f.strip_prefix("tex").and_then(|n| n.parse().ok()),
        };
        match set {
            Some(set) => wanted.push((path, folder.to_string(), set, name.to_string())),
            None => report.hd_only.push(path),
        }
    }

    // Group by set and base name; give each HD texture of a group the PS2
    // image that matches it best (`assign`).
    let mut groups: BTreeMap<(String, usize, String), Vec<(String, String)>> = BTreeMap::new();
    for (path, folder, set, name) in wanted {
        groups
            .entry((folder, set, base_name(&name).to_string()))
            .or_default()
            .push((path, name));
    }
    let mut used = vec![false; images.len()];
    for ((_, set, base), members) in groups {
        let candidates: Vec<usize> = images
            .iter()
            .enumerate()
            .filter(|(_, i)| (set == 0 || i.set == set) && i.names().any(|n| base_name(n) == base))
            .map(|(k, _)| k)
            .collect();
        if candidates.is_empty() {
            report.hd_only.extend(members.into_iter().map(|(p, _)| p));
            continue;
        }
        let mut hds = Vec::new();
        for (path, name) in members {
            match load_hd(pc, &path) {
                Ok(h) => hds.push((path, name, h)),
                Err(e) => report.failed.push((path, format!("{e:#}"))),
            }
        }
        let scores: Vec<Vec<Fit>> = hds
            .iter()
            .map(|(_, name, h)| {
                candidates
                    .iter()
                    .map(|&k| fit(&h.1, &images[k], name, set))
                    .collect()
            })
            .collect();
        for (h, c) in assign(&scores) {
            let (path, _, (template, hd_rgba)) = &hds[h];
            let Some(c) = c else {
                report.hd_only.push(path.clone());
                continue;
            };
            let k = candidates[c];
            used[k] = true;
            let img = &images[k];
            if img.missing_clut {
                report.missing_clut.push(path.clone());
            }
            match write_one(path, template, hd_rgba, &img.image, &scores[h][c]) {
                Ok(d) => report.done.push(d),
                Err(e) => report.failed.push((path.clone(), format!("{e:#}"))),
            }
        }
    }
    // An upload counts as used if any image made from it went in.
    let covered: BTreeSet<(usize, usize)> = images
        .iter()
        .enumerate()
        .filter(|&(i, _)| used[i])
        .flat_map(|(_, img)| img.parts.iter().map(move |&p| (img.set, p)))
        .collect();
    for (i, img) in images.iter().enumerate() {
        if !used[i] && !img.parts.iter().all(|&p| covered.contains(&(img.set, p))) {
            let n = img.names().next().unwrap_or_default();
            report.ps2_unused.push(format!("set {} {n}", img.set));
        }
    }
    Ok(report)
}

/// A name without the HD's variant suffix (`13440_19_0_0_256_256_2`,
/// `..._0`, `..._n`): the first six fields.
fn base_name(name: &str) -> &str {
    match name.match_indices('_').nth(5) {
        Some((i, _)) => &name[..i],
        None => name,
    }
}

/// How a PS2 image fits an HD texture.
#[derive(Debug, Clone, Copy)]
struct Fit {
    /// Correlation and alpha difference (`difference`) in each
    /// orientation (`ORIENTATIONS`).
    turns: [Option<(f32, f32)>; 4],
    /// The image goes by exactly the HD name.
    exact: bool,
}

/// Upright, upside down, mirrored, turned by 180 degrees.
const ORIENTATIONS: [&str; 4] = ["", "flipped", "mirrored", "turned"];

impl Fit {
    /// Many map-particle textures (and a few others) were stored upside
    /// down or mirrored in the HD version, and the PC draws them that way,
    /// so ours follow the HD when that clearly matches better.
    fn orientation(&self) -> usize {
        let Some((up, _)) = self.turns[0] else {
            return 0;
        };
        (1..4)
            .filter_map(|k| self.turns[k].map(|(c, _)| (k, c)))
            .filter(|&(_, c)| c > TURN_MIN && c - up > TURN_MARGIN)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(k, _)| k)
    }
    fn diff(&self) -> Option<(f32, f32)> {
        self.turns[self.orientation()]
    }
    fn correlation(&self) -> f32 {
        self.diff().map_or(0.0, |d| d.0)
    }
    /// What `assign` ranks by: the correlation, plus a margin for the
    /// image that has the HD texture's exact name.
    fn score(&self) -> f32 {
        self.correlation() + if self.exact { EXACT_BONUS } else { 0.0 }
    }
}

/// How much better a turned PS2 image must match the HD texture to be
/// written that way (correlations, `difference`).
const TURN_MARGIN: f32 = 0.2;
const TURN_MIN: f32 = 0.5;
/// How much better another image must match than the one with the exact
/// name to be taken instead.
const EXACT_BONUS: f32 = 0.15;
/// An image already given to another HD texture of the group is only used
/// again above this correlation.
const REUSE_MIN: f32 = 0.5;

fn fit(hd: &Rgba, img: &ps2fx::Image, name: &str, set: usize) -> Fit {
    let exact = if set == 0 {
        (img.set == 0 && img.names().any(|n| n == name)) || img.set_name.as_deref() == Some(name)
    } else {
        img.names().any(|n| n == name)
    };
    Fit {
        turns: std::array::from_fn(|k| difference(hd, &turn(&img.image, k))),
        exact,
    }
}

/// Greedy one-to-one assignment by score; HD textures left over (more of
/// them than PS2 images) take the best image again if it fits well.
fn assign(scores: &[Vec<Fit>]) -> Vec<(usize, Option<usize>)> {
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for (h, row) in scores.iter().enumerate() {
        for (c, f) in row.iter().enumerate() {
            pairs.push((f.score(), h, c));
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out: Vec<Option<usize>> = vec![None; scores.len()];
    let mut taken = vec![false; scores.first().map_or(0, Vec::len)];
    for &(_, h, c) in &pairs {
        if out[h].is_none() && !taken[c] {
            out[h] = Some(c);
            taken[c] = true;
        }
    }
    for (h, o) in out.iter_mut().enumerate() {
        if o.is_none() {
            *o = (0..scores[h].len())
                .max_by(|&a, &b| scores[h][a].score().total_cmp(&scores[h][b].score()))
                .filter(|&c| scores[h][c].correlation() > REUSE_MIN);
        }
    }
    out.into_iter().enumerate().collect()
}

/// An HD texture: the file and its pixels.
fn load_hd(pc: &ffx_vbf::Archive, path: &str) -> Result<(Vec<u8>, Rgba)> {
    let e = pc.find(path).context("not in the archive")?;
    let template = pc.read(e)?;
    let hd = ffx_phyre::Texture::parse(template.clone())?;
    let rgba = Rgba {
        width: hd.width,
        height: hd.height,
        pixels: hd.to_rgba(),
    };
    Ok((template, rgba))
}

fn write_one(path: &str, template: &[u8], hd: &Rgba, image: &Rgba, f: &Fit) -> Result<Done> {
    let k = f.orientation();
    let image = turn(image, k);
    let file = phyretex::argb8_from(template, &image)?;
    Ok(Done {
        path: path.to_string(),
        file,
        hd: (hd.width, hd.height),
        diff: f.diff(),
        orientation: ORIENTATIONS[k],
        preview: side_by_side(hd, &image),
        image,
    })
}

/// How well `ps2` matches `hd` box-filtered down to `ps2`'s size: the
/// correlation of their colours times alpha (what an additive draw adds),
/// all three channels together, and the mean alpha difference (0..255).
/// The HD often stores colour as white with the brightness in alpha, or
/// scales brightness, which the correlation ignores; a wrong palette or
/// layout drives it down.
pub fn difference(hd: &Rgba, ps2: &Rgba) -> Option<(f32, f32)> {
    if !hd.width.is_multiple_of(ps2.width)
        || !hd.height.is_multiple_of(ps2.height)
        || hd.width / ps2.width != hd.height / ps2.height
    {
        return None;
    }
    let (sx, sy) = (hd.width / ps2.width, hd.height / ps2.height);
    let n = f64::from(sx * sy);
    let (mut sa, mut sb, mut saa, mut sbb, mut sab, mut count) =
        (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let mut alpha = 0f64;
    for y in 0..ps2.height {
        for x in 0..ps2.width {
            let mut acc = [0f64; 4];
            for dy in 0..sy {
                for dx in 0..sx {
                    let c = hd.get(x * sx + dx, y * sy + dy);
                    let a = f64::from(c[3]) / 255.0;
                    for k in 0..3 {
                        acc[k] += f64::from(c[k]) * a;
                    }
                    acc[3] += f64::from(c[3]);
                }
            }
            let p = ps2.get(x, y);
            let pa = f64::from(p[3]) / 255.0;
            for k in 0..3 {
                let (a, b) = (acc[k] / n, f64::from(p[k]) * pa);
                sa += a;
                sb += b;
                saa += a * a;
                sbb += b * b;
                sab += a * b;
                count += 1.0;
            }
            alpha += (acc[3] / n - f64::from(p[3])).abs();
        }
    }
    let cov = sab / count - sa / count * sb / count;
    let va = saa / count - (sa / count).powi(2);
    let vb = sbb / count - (sb / count).powi(2);
    let corr = if va <= 1e-9 && vb <= 1e-9 {
        1.0
    } else if va <= 1e-9 || vb <= 1e-9 {
        0.0
    } else {
        cov / (va * vb).sqrt()
    };
    let px = f64::from(ps2.width * ps2.height);
    Some((corr as f32, (alpha / px) as f32))
}

/// `image` in orientation `k` of `ORIENTATIONS`.
pub fn turn(image: &Rgba, k: usize) -> Rgba {
    let (w, h) = (image.width, image.height);
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (tx, ty) = match k {
                1 => (x, h - 1 - y),
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                _ => (x, y),
            };
            out.put(tx, ty, image.get(x, y));
        }
    }
    out
}

/// HD on the left, PS2 (scaled up, nearest) on the right, both over black.
fn side_by_side(hd: &Rgba, ps2: &Rgba) -> Rgba {
    let (w, h) = (hd.width.max(ps2.width), hd.height.max(ps2.height));
    let gap = 4;
    let mut out = Rgba::new(w * 2 + gap, h);
    for p in out.pixels.chunks_mut(4) {
        p.copy_from_slice(&[48, 48, 48, 255]);
    }
    let over_black = |c: [u8; 4]| {
        let a = u32::from(c[3]);
        [
            (u32::from(c[0]) * a / 255) as u8,
            (u32::from(c[1]) * a / 255) as u8,
            (u32::from(c[2]) * a / 255) as u8,
            255,
        ]
    };
    for y in 0..h {
        for x in 0..w {
            out.put(
                x,
                y,
                over_black(hd.get(x * hd.width / w, y * hd.height / h)),
            );
            out.put(
                w + gap + x,
                y,
                over_black(ps2.get(x * ps2.width / w, y * ps2.height / h)),
            );
        }
    }
    out
}

/// `demaster build`'s job for `effects.textures`: every effect source
/// (spell files, map and battle-map particles, monster particles).
pub fn jobs() -> Vec<Box<dyn crate::jobs::Job>> {
    vec![Box::new(FxJob)]
}

struct FxJob;

impl crate::jobs::Job for FxJob {
    fn name(&self) -> &'static str {
        "fx.textures"
    }

    fn settings(&self) -> &'static [&'static str] {
        &["effects.textures"]
    }

    fn enabled(&self, config: &crate::config::Config) -> bool {
        config.effects.textures.ps2()
    }

    fn run(&self, ctx: &crate::jobs::Ctx, out: &crate::jobs::Output) -> Result<String> {
        let hd = HdIndex::new(ctx.pc);
        let (mut done, mut failed) = (0usize, 0usize);
        ctx.disc(|d| {
            let mut gs = Gs::default();
            for src in hd.sources(d) {
                match convert(d, ctx.pc, &hd, &src, &mut gs) {
                    Ok(r) => {
                        for t in &r.done {
                            if out.put(&t.path, &t.file)? {
                                done += 1;
                            }
                        }
                        for (p, e) in &r.failed {
                            ctx.note(format!("{p}: {e}"));
                        }
                        failed += r.failed.len();
                    }
                    Err(e) => {
                        ctx.note(format!("{}: {e:#}", src.id));
                        failed += 1;
                    }
                }
            }
            Ok(())
        })?;
        Ok(format!("{done} effect textures, {failed} failed"))
    }
}
