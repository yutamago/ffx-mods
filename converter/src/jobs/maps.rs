//! Field and battle map textures: every HD map texture named after a PS2
//! GS region (`TBP_PSM_x_y_bpp_w_h`) is cut out of the emulated PS2 GS
//! memory of its map, with the CLUT the map's TEX0 writes bind to it, and
//! written at PS2 size. Texture-swap frames (`tex/texswap`) come from the
//! `.vpa`'s swap records. Everything else stays HD. Format: `docs/maps.md`.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};

use rayon::prelude::*;

use crate::config::Config;
use crate::disc::Disc;
use crate::formats::gsmem::{self, Gs};
use crate::jobs::{Ctx, Job, Output};
use crate::formats::phyretex::{self, Rgba};
use crate::formats::ps2map::{self, SwapFrame, Tex0, Txa};

const HD_ROOT: &str = "ffx_data/gamedata/ps3data";

/// An unbound result (a guessed CLUT or a relocated region) is only used
/// when its mean colour
/// distance to the HD texture is at most this (0..255 per channel).
pub const MAX_GUESS_ERROR: f32 = 35.0;

/// A TEX0-bound result this far from the HD texture is checked against
/// relocated regions too.
pub const POOR_ERROR: f32 = 30.0;

/// Unbound sources need this luminance correlation with the HD texture.
pub const MIN_CORRELATION: f32 = 0.6;

/// A bound result this far off in colour may be replaced by an unbound one
/// closer than `CLOSE_COLOUR`.
pub const FAR_COLOUR: f32 = 60.0;
pub const CLOSE_COLOUR: f32 = 20.0;

/// Below this correlation a result further than `FAR_COLOUR` is rejected.
pub const MISMATCH_CORRELATION: f32 = 0.5;

/// HD alpha at or above this counts as opaque.
const OPAQUE: u8 = 250;

/// One HD map: `kind` `map` or `btlmap`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MapId {
    pub kind: String,
    pub area: String,
    pub name: String,
}

impl MapId {
    pub fn ps2_dir(&self) -> String {
        format!("jppc/{}/{}/{}/bin", self.kind, self.area, self.name)
    }
}

/// The HD maps and their textures (archive paths), from the PC archive.
pub fn hd_maps(pc: &ffx_vbf::Archive) -> BTreeMap<MapId, Vec<String>> {
    let mut maps: BTreeMap<MapId, Vec<String>> = BTreeMap::new();
    for e in pc.entries() {
        let Some(rest) = e
            .path
            .strip_prefix(HD_ROOT)
            .and_then(|r| r.strip_prefix('/'))
        else {
            continue;
        };
        let p: Vec<&str> = rest.split('/').collect();
        let tex = match p.as_slice() {
            [k, _, _, "tex", "d3d11", f] | [k, _, _, "tex", "texswap", "d3d11", f]
                if (*k == "map" || *k == "btlmap") && f.ends_with(".dds.phyre") =>
            {
                true
            }
            _ => false,
        };
        if tex {
            maps.entry(MapId {
                kind: p[0].into(),
                area: p[1].into(),
                name: p[2].into(),
            })
            .or_default()
            .push(e.path.clone());
        }
    }
    maps
}

/// An HD texture name: `TBP_PSM_x_y_bpp_w_h[_frame]` (all decimal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub tbp: u32,
    pub psm: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub frame: Option<u32>,
}

impl Region {
    pub fn parse(file: &str) -> Option<Self> {
        let stem = file.strip_suffix(".dds.phyre")?;
        let n: Vec<u32> = stem
            .split('_')
            .map(|s| s.parse().ok())
            .collect::<Option<_>>()?;
        let (frame, n) = match n.len() {
            7 => (None, &n[..]),
            8 => (Some(n[7]), &n[..7]),
            _ => return None,
        };
        let bits = gsmem::index_bits(n[1])?;
        if n[4] != bits || n[5] == 0 || n[6] == 0 || n[0] >= 0x4000 {
            return None;
        }
        Some(Self {
            tbp: n[0],
            psm: n[1],
            x: n[2],
            y: n[3],
            w: n[5],
            h: n[6],
            frame,
        })
    }
}

pub struct Converted {
    pub path: String,
    pub region: Region,
    /// HD texture (box-filtered to the PS2 size) | PS2, if asked for.
    pub preview: Option<Rgba>,
    /// Where the pixels and CLUT came from.
    pub how: String,
    /// Mean colour distance to the HD texture.
    pub error: f32,
    /// The runner-up's distance when several CLUTs were possible.
    pub second: Option<f32>,
    pub file: Vec<u8>,
}

#[derive(Default)]
pub struct Report {
    pub converted: Vec<Converted>,
    /// HD textures with no PS2 region (HD-only art, `_s` maps, ...).
    pub hd_only: Vec<String>,
    /// Named regions that could not be filled, with the reason.
    pub failed: Vec<(String, String)>,
}

/// A candidate for one HD texture.
#[derive(Clone)]
struct Cand {
    how: String,
    image: Rgba,
    /// Bound by the map's own data (a TEX0 write or a swap frame's CLUT),
    /// not guessed.
    bound: bool,
    /// The block address the indices were read from.
    tbp: u32,
}

fn to_rgba(w: u32, h: u32, idx: &[u8], clut: &[[u8; 4]]) -> Rgba {
    let mut img = Rgba::new(w, h);
    for (i, &k) in idx.iter().enumerate() {
        let [r, g, b, a] = clut.get(usize::from(k)).copied().unwrap_or([0; 4]);
        img.pixels[4 * i..4 * i + 4].copy_from_slice(&[r, g, b, gsmem::alpha(a)]);
    }
    img
}

/// Area-average resample of `src` to `w` x `h`.
pub fn resize(src: &Rgba, w: u32, h: u32) -> Rgba {
    if (src.width, src.height) == (w, h) {
        return src.clone();
    }
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        let (y0, y1) = (
            y * src.height / h,
            ((y + 1) * src.height / h).max(y * src.height / h + 1),
        );
        for x in 0..w {
            let (x0, x1) = (
                x * src.width / w,
                ((x + 1) * src.width / w).max(x * src.width / w + 1),
            );
            let mut acc = [0u32; 4];
            let mut n = 0;
            for sy in y0..y1.min(src.height) {
                for sx in x0..x1.min(src.width) {
                    let c = src.get(sx, sy);
                    for k in 0..4 {
                        acc[k] += u32::from(c[k]);
                    }
                    n += 1;
                }
            }
            out.put(x, y, acc.map(|v| (v / n.max(1)) as u8));
        }
    }
    out
}

/// Mean colour distance per channel (0..255); colour counts by the lower
/// of the two alphas, alpha by itself.
pub fn is_opaque(img: &Rgba) -> bool {
    img.pixels.chunks_exact(4).all(|p| p[3] >= OPAQUE)
}

/// `img` with alpha 255 everywhere when `hd` is opaque: the HD material
/// then draws without alpha, and the PS2 alpha (often 0x40 or 0x7f under a
/// PS2 blend mode that ignores it) must not make it see-through.
pub fn opaque_like(mut img: Rgba, hd: &Rgba) -> Rgba {
    if is_opaque(hd) {
        for p in img.pixels.chunks_exact_mut(4) {
            p[3] = 255;
        }
    }
    img
}

pub fn distance(a: &Rgba, b: &Rgba) -> f32 {
    // compare colours, not detail: both reduced to at most 32 texels a side
    let (w, h) = (a.width.min(b.width), a.height.min(b.height));
    let f = (w.max(h) / 32).max(1);
    let (w, h) = ((w / f).max(1), (h / f).max(1));
    let (a, b) = (&opaque_like(resize(a, w, h), b), &resize(b, w, h));
    let step = 1;
    let (mut sum, mut n) = (0.0f64, 0u32);
    for y in (0..h).step_by(step as usize) {
        for x in (0..w).step_by(step as usize) {
            let (p, q) = (a.get(x, y), b.get(x, y));
            let wa = f64::from(p[3].min(q[3])) / 255.0;
            let dc: f64 = (0..3)
                .map(|k| f64::from((i32::from(p[k]) - i32::from(q[k])).abs()))
                .sum::<f64>()
                / 3.0;
            let da = f64::from((i32::from(p[3]) - i32::from(q[3])).abs());
            sum += dc * wa + da * 0.5;
            n += 1;
        }
    }
    (sum / f64::from(n.max(1))) as f32
}

fn side_by_side(hd: &Rgba, ps2: &Rgba) -> Rgba {
    let s = (256 / ps2.width.max(ps2.height)).clamp(1, 4);
    let (w, h) = (ps2.width * s, ps2.height * s);
    let left = resize(hd, w, h);
    let mut out = Rgba::new(2 * w + 4, h);
    for y in 0..h {
        for x in 0..w {
            out.put(x, y, left.get(x, y));
            out.put(w + 4 + x, y, ps2.get(x / s, y / s));
        }
        for x in w..w + 4 {
            out.put(x, y, [255, 0, 255, 255]);
        }
    }
    out
}

struct MapData {
    txa: Txa,
    gs: Gs,
    tex0: BTreeMap<Tex0, u32>,
    frames: Vec<SwapFrame>,
    cbps: Vec<u32>,
}

impl MapData {
    /// TEX0s that read `(tbp, psm)` with a CLUT the map uploads.
    fn tex0s(&self, tbp: u32, psm: u32) -> Vec<Tex0> {
        let mut v: Vec<Tex0> = self
            .tex0
            .keys()
            .filter(|t| t.tbp == tbp && t.psm == psm && self.cbps.contains(&t.cbp))
            .copied()
            .collect();
        let bits = gsmem::index_bits(psm).unwrap_or(8);
        if bits == 8 {
            // the CSA does not matter for 8-bit textures
            v.sort_by_key(|t| (t.cbp, t.tbw));
            v.dedup_by_key(|t| (t.cbp, t.tbw));
        }
        v
    }

    /// The buffer width a region is read with: the TEX0's, else that of the
    /// texture uploaded there, else 256 texels.
    fn tbw(&self, tbp: u32, psm: u32) -> u32 {
        self.tex0
            .keys()
            .find(|t| t.tbp == tbp && t.psm == psm && t.tbw > 0)
            .map(|t| t.tbw)
            .or_else(|| {
                self.txa
                    .textures
                    .iter()
                    .find(|t| t.dbp == tbp && t.psm == psm)
                    .map(|t| t.dbw)
            })
            .unwrap_or(4)
    }

    fn indices(gs: &Gs, r: &Region, tbp: u32, tbw: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((r.w * r.h) as usize);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                v.push(gs.index(r.psm, tbp, tbw, x, y));
            }
        }
        v
    }

    /// Whether any uploaded texture covers block `tbp` with format `psm`
    /// (the region lies in data this map put there).
    fn covered(&self, tbp: u32, psm: u32) -> bool {
        self.txa
            .textures
            .iter()
            .any(|t| t.psm == psm && t.dbp <= tbp && tbp < t.dbp + t.blocks())
    }

    /// Candidates for a main texture region.
    fn candidates(&self, r: &Region) -> Vec<Cand> {
        let bits = gsmem::index_bits(r.psm).unwrap_or(8);
        let mut out = Vec::new();
        // a texture-swap target shows its frames; frame 0 is the start
        for f in self.frames.iter().filter(|f| f.frame == 0) {
            if let Some(c) = self.frame_cand(f, r) {
                out.push(c);
            }
        }
        // a TEX0 of another format on the same address only loads a CLUT
        let tex0s = if self.covered(r.tbp, r.psm) {
            self.tex0s(r.tbp, r.psm)
        } else {
            Vec::new()
        };
        for t in tex0s {
            let idx = Self::indices(&self.gs, r, r.tbp, t.tbw);
            let clut = self.gs.clut(t.cbp, t.csa, bits);
            out.push(Cand {
                how: format!(
                    "TEX0 cbp {:04x} (clut {}) csa {}",
                    t.cbp,
                    self.cbps.iter().position(|&c| c == t.cbp).unwrap_or(99),
                    t.csa
                ),
                image: to_rgba(r.w, r.h, &idx, &clut),
                bound: true,
                tbp: r.tbp,
            });
        }
        out
    }

    /// Unbound candidates: every uploaded CLUT (and every CSA slice).
    fn guesses(&self, r: &Region, tbp: u32, tbw: u32) -> Vec<Cand> {
        let bits = gsmem::index_bits(r.psm).unwrap_or(8);
        let idx = Self::indices(&self.gs, r, tbp, tbw);
        let mut out = Vec::new();
        let palettes: Vec<(String, Vec<[u8; 4]>)> = if self.cbps.is_empty() {
            // CLUTs not uploaded (allocation mode 0): the bank as is
            self.txa
                .bank
                .iter()
                .enumerate()
                .flat_map(|(i, raw)| {
                    let full: Vec<[u8; 4]> = (0..256)
                        .map(|k| {
                            let j = gsmem::csm1(k) as usize * 4;
                            [raw[j], raw[j + 1], raw[j + 2], raw[j + 3]]
                        })
                        .collect();
                    let n = if bits == 8 { 1 } else { 16 };
                    (0..n)
                        .map(|csa| {
                            let p = if bits == 8 {
                                full.clone()
                            } else {
                                full[csa * 16..csa * 16 + 16].to_vec()
                            };
                            (format!("bank clut {i} csa {csa}"), p)
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            self.cbps
                .iter()
                .enumerate()
                .flat_map(|(i, &cbp)| {
                    let n = if bits == 8 { 1 } else { 16 };
                    (0..n)
                        .map(|csa| {
                            (
                                format!("guess cbp {cbp:04x} (clut {i}) csa {csa}"),
                                self.gs.clut(cbp, csa, bits),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        for (how, clut) in palettes {
            out.push(Cand {
                how,
                image: to_rgba(r.w, r.h, &idx, &clut),
                bound: false,
                tbp,
            });
        }
        out
    }

    /// The region `r` with swap frame `f` uploaded, if the frame covers it.
    fn frame_cand(&self, f: &SwapFrame, r: &Region) -> Option<Cand> {
        // the frame must be uploaded to the region's page in a format
        // that writes it
        if f.dbp != r.tbp {
            return None;
        }
        let mut gs = self.gs.clone();
        if !gs.upload(f.dpsm, f.dbp, f.dbw, f.pos, f.size, &f.pixels) {
            return None;
        }
        let bits = gsmem::index_bits(r.psm)?;
        let tbw = if f.dpsm == r.psm {
            f.dbw
        } else {
            self.tbw(r.tbp, r.psm)
        };
        // the frame's rectangle in texels of the region's format
        let (fx, fy, fw, fh) = if f.dpsm == gsmem::PSMCT32 && r.psm == gsmem::PSMT4 {
            (f.pos.0 * 2, f.pos.1 * 4, f.size.0 * 2, f.size.1 * 4)
        } else {
            (f.pos.0, f.pos.1, f.size.0, f.size.1)
        };
        if r.x < fx || r.y < fy || r.x + r.w > fx + fw || r.y + r.h > fy + fh {
            return None;
        }
        let idx = Self::indices(&gs, r, r.tbp, tbw);
        let (clut, how) = match &f.clut {
            Some(c) if c.len() >= 1 << bits => (c.clone(), "own CLUT"),
            _ => {
                let t = self.tex0s(r.tbp, r.psm).into_iter().next()?;
                (gs.clut(t.cbp, t.csa, bits), "TEX0 CLUT")
            }
        };
        Some(Cand {
            how: format!("swap anim {} frame {} ({how})", f.anim, f.frame),
            image: to_rgba(r.w, r.h, &idx, &clut),
            bound: true,
            tbp: r.tbp,
        })
    }
}

/// Picks the candidate closest to the HD texture. Returns it with its
/// distance and the runner-up's.
fn pick(cands: Vec<Cand>, hd: &Rgba) -> Option<(Cand, f32, Option<f32>)> {
    let mut scored: Vec<(f32, Cand)> = cands
        .into_iter()
        .map(|c| (distance(&c.image, hd), c))
        .collect();
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    if std::env::var_os("DM_DEBUG").is_some() {
        for (e, c) in scored.iter().take(6) {
            eprintln!("    {e:6.1} {}", c.how);
        }
    }
    let second = scored.get(1).map(|s| s.0);
    let (e, c) = scored.into_iter().next()?;
    Some((c, e, second))
}

/// Pearson correlation of the luminance of two images reduced to at most
/// 32 texels a side (alpha-weighted toward black): the same pattern scores
/// near 1 whatever the colour grading.
pub fn correlation(a: &Rgba, b: &Rgba) -> f32 {
    correlation_with(a, b, true)
}

/// `correlation`, with colour weighted by alpha or not (HD textures drawn
/// additively often have alpha 0 everywhere).
pub fn correlation_with(a: &Rgba, b: &Rgba, use_alpha: bool) -> f32 {
    let (w, h) = (a.width.min(b.width), a.height.min(b.height));
    let f = (w.max(h) / 32).max(1);
    let (w, h) = ((w / f).max(1), (h / f).max(1));
    let (a, b) = (opaque_like(resize(a, w, h), b), resize(b, w, h));
    let lum = |c: [u8; 4]| {
        (0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2]))
            * if use_alpha {
                f64::from(c[3]) / 255.0
            } else {
                1.0
            }
    };
    let xs: Vec<f64> = (0..w * h).map(|i| lum(a.get(i % w, i / w))).collect();
    let ys: Vec<f64> = (0..w * h).map(|i| lum(b.get(i % w, i / w))).collect();
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(&ys) {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
    }
    if sxx < 1e-6 || syy < 1e-6 {
        return 0.0;
    }
    (sxy / (sxx * syy).sqrt()) as f32
}

/// The candidate whose pattern correlates best with the HD texture; among
/// those within 0.02 of it, the closest in colour. Returns it with its
/// correlation.
fn pick_by_pattern(cands: Vec<Cand>, hd: &Rgba) -> Option<((Cand, f32, Option<f32>), f32)> {
    let mut scored: Vec<(f32, f32, Cand)> = cands
        .into_iter()
        .map(|c| (correlation(&c.image, hd), distance(&c.image, hd), c))
        .collect();
    let top = scored.iter().map(|s| s.0).fold(f32::MIN, f32::max);
    scored.retain(|s| s.0 >= top - 0.02);
    scored.sort_by(|a, b| a.1.total_cmp(&b.1));
    if std::env::var_os("DM_DEBUG").is_some() {
        for (k, e, c) in &scored {
            eprintln!("    corr {k:.2} {e:6.1} {}", c.how);
        }
    }
    let second = scored.get(1).map(|s| s.1);
    let (k, e, c) = scored.into_iter().next()?;
    Some(((c, e, second), k))
}

/// Converts one map's textures.
pub fn convert(
    disc: &mut Disc,
    pc: &ffx_vbf::Archive,
    id: &MapId,
    paths: &[String],
    previews: bool,
) -> Result<Report> {
    let files = read_ps2(disc, id)?;
    convert_files(files, pc, paths, previews)
}

/// The map's `mapout.txa` and `mapout.vpa`, if the disc has the map.
pub fn read_ps2(disc: &mut Disc, id: &MapId) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
    let dir = id.ps2_dir();
    let txa = disc.read(&format!("{dir}/mapout.txa"))?;
    let vpa = disc.read(&format!("{dir}/mapout.vpa"))?;
    Ok(txa.zip(vpa))
}

/// Converts one map's textures from its PS2 files (`None`: an HD-only map,
/// hiku28-30 and stbv04).
pub fn convert_files(
    files: Option<(Vec<u8>, Vec<u8>)>,
    pc: &ffx_vbf::Archive,
    paths: &[String],
    previews: bool,
) -> Result<Report> {
    let mut report = Report::default();
    let Some((txa_bytes, vpa_bytes)) = files else {
        report.hd_only.extend(paths.iter().cloned());
        return Ok(report);
    };
    let txa = Txa::parse(&txa_bytes).context("mapout.txa")?;
    let gs = txa.vram(&txa_bytes);
    let cbps = txa.clut_cbps();
    let data = MapData {
        tex0: ps2map::scan_tex0(&vpa_bytes),
        frames: ps2map::swap_frames(&vpa_bytes),
        txa,
        gs,
        cbps,
    };

    for path in paths {
        let file = path.rsplit('/').next().unwrap_or("");
        let Some(r) = Region::parse(file) else {
            report.hd_only.push(path.clone());
            continue;
        };
        let e = pc.find(path).context("listed texture missing")?;
        let template = pc.read(e)?;
        let hd = ffx_phyre::Texture::parse(template.clone()).with_context(|| path.clone())?;
        let hd_rgba = Rgba {
            width: hd.width,
            height: hd.height,
            pixels: hd.to_rgba(),
        };
        let hd_small = resize(&hd_rgba, r.w, r.h);
        if std::env::var_os("DM_DEBUG").is_some() {
            eprintln!("  {file}");
        }

        let chosen = if let Some(n) = r.frame {
            // texswap/<region>_<n>: frame n of the swap on that page
            let mut on_page: Vec<&SwapFrame> =
                data.frames.iter().filter(|f| f.dbp == r.tbp).collect();
            on_page.sort_by_key(|f| (f.anim, f.frame));
            let anims: BTreeSet<u32> = on_page.iter().map(|f| f.anim).collect();
            let cands: Vec<Cand> = anims
                .iter()
                .filter_map(|a| {
                    let f = on_page.iter().find(|f| f.anim == *a && f.frame == n)?;
                    data.frame_cand(f, &r)
                })
                .collect();
            pick(cands, &hd_small)
        } else {
            let cands = data.candidates(&r);
            let mut best = if cands.is_empty() {
                None
            } else {
                pick(cands, &hd_small)
            };
            // Nothing bound there, or nothing there looks like the HD
            // texture: a CLUT guessed for what is there, or the same region
            // of another texture of the format (textures at 0x3c00 and
            // above are moved at run time). The HD art is often regraded,
            // so the source is chosen by its luminance pattern.
            if best.as_ref().is_none_or(|b| b.1 > POOR_ERROR) {
                let mut cands = Vec::new();
                if best.is_none() && data.covered(r.tbp, r.psm) {
                    let tbw = data.tbw(r.tbp, r.psm);
                    cands.extend(data.guesses(&r, r.tbp, tbw));
                }
                for t in &data.txa.textures {
                    if t.psm != r.psm || t.dbp == r.tbp || r.x + r.w > t.w || r.y + r.h > t.h {
                        continue;
                    }
                    let moved = Region { tbp: t.dbp, ..r };
                    for mut c in data.candidates(&moved) {
                        c.how = format!("relocated from {:04x}: {}", t.dbp, c.how);
                        c.bound = false;
                        cands.push(c);
                    }
                }
                let bound_corr = best
                    .as_ref()
                    .map_or(-1.0, |b| correlation(&b.0.image, &hd_small));
                let bound_err = best.as_ref().map_or(f32::MAX, |b| b.1);
                // flat textures (sand) have no pattern: a very close colour
                // match may stand in for a bound result that is far off
                let by_colour = pick(cands.clone(), &hd_small)
                    .filter(|(_, e, _)| *e < CLOSE_COLOUR && bound_err > FAR_COLOUR);
                if let Some((mut m, corr)) = pick_by_pattern(cands, &hd_small) {
                    if corr >= MIN_CORRELATION && corr > bound_corr + 0.15 {
                        // a moved texture may be drawn with a CLUT no TEX0
                        // in the file names: try them all on the same source
                        if m.1 > POOR_ERROR {
                            let src = Region { tbp: m.0.tbp, ..r };
                            let all = data.guesses(&src, src.tbp, data.tbw(src.tbp, src.psm));
                            let all: Vec<Cand> = all
                                .into_iter()
                                .filter(|c| correlation(&c.image, &hd_small) >= corr - 0.05)
                                .map(|mut c| {
                                    c.how = format!("relocated from {:04x}: {}", src.tbp, c.how);
                                    c
                                })
                                .collect();
                            if let Some(g) = pick(all, &hd_small) {
                                if g.1 < 0.7 * m.1 {
                                    m = g;
                                }
                            }
                        }
                        if m.1 <= MAX_GUESS_ERROR {
                            best = Some(m);
                        }
                    } else if let Some(c) = by_colour {
                        best = Some(c);
                    }
                }
            }
            best
        };
        let Some((c, error, second)) = chosen else {
            report
                .failed
                .push((path.clone(), "no PS2 texture or CLUT found".into()));
            continue;
        };
        // far off in colour and in pattern: the HD texture shows something
        // else (VRAM reused at run time); keep it HD
        if r.frame.is_none()
            && error > FAR_COLOUR
            && correlation_with(&c.image, &hd_small, true)
                .max(correlation_with(&c.image, &hd_small, false))
                < MISMATCH_CORRELATION
        {
            report.failed.push((
                path.clone(),
                format!("PS2 region does not match the HD texture ({})", c.how),
            ));
            continue;
        }
        let how = if c.bound {
            c.how
        } else {
            format!("{} [unbound]", c.how)
        };
        let image = opaque_like(c.image, &hd_rgba);
        let file = phyretex::argb8_from(&template, &image).with_context(|| path.clone())?;
        report.converted.push(Converted {
            path: path.clone(),
            region: r,
            preview: previews.then(|| side_by_side(&hd_rgba, &image)),
            how,
            error,
            second,
            file,
        });
    }
    Ok(report)
}

/// `maps.field` and `maps.battle` for `demaster build`.
pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![
        Box::new(MapsJob {
            name: "maps.field",
            settings: &["maps.field"],
            kind: "map",
            enabled: |c| c.maps.field.ps2(),
        }),
        Box::new(MapsJob {
            name: "maps.battle",
            settings: &["maps.battle"],
            kind: "btlmap",
            enabled: |c| c.maps.battle.ps2(),
        }),
    ]
}

struct MapsJob {
    name: &'static str,
    settings: &'static [&'static str],
    /// `map` or `btlmap`.
    kind: &'static str,
    enabled: fn(&Config) -> bool,
}

impl Job for MapsJob {
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
        let maps: Vec<(MapId, Vec<String>)> = hd_maps(ctx.pc)
            .into_iter()
            .filter(|(id, _)| id.kind == self.kind)
            .collect();
        let (mut conv, mut hd_only, mut failed) = (0, 0, 0);
        // A few maps at a time (each holds a 4 MiB GS image and its HD
        // textures), written in map order so every build writes the same.
        for chunk in maps.chunks(8) {
            let files: Vec<_> = chunk
                .iter()
                .map(|(id, _)| ctx.disc(|d| read_ps2(d, id)))
                .collect::<Result<_>>()?;
            let reports: Vec<Result<Report>> = chunk
                .par_iter()
                .zip(files)
                .map(|((_, paths), f)| convert_files(f, ctx.pc, paths, false))
                .collect();
            for ((id, paths), r) in chunk.iter().zip(reports) {
                let r = match r {
                    Ok(r) => r,
                    Err(e) => {
                        failed += paths.len();
                        ctx.note(format!("{}: left HD: {e:#}", id.name));
                        continue;
                    }
                };
                for c in &r.converted {
                    out.put(&c.path, &c.file)?;
                }
                for (p, why) in &r.failed {
                    ctx.note(format!("{p}: left HD: {why}"));
                }
                conv += r.converted.len();
                hd_only += r.hd_only.len();
                failed += r.failed.len();
            }
        }
        Ok(format!(
            "{} maps: {conv} textures from the PS2, {hd_only} HD-only, {failed} left HD",
            maps.len()
        ))
    }
}
