//! `demaster materials-audit`: checks the character materials of a built
//! output against what their segments sample (`docs/mods/characters.md`, "Alpha",
//! "Auditing the output").
//!
//! It reads only the written files (models, the textures their materials
//! bind, asset lists) and the HD originals, and does not call the
//! converter's decisions (`alpha::blend`, `coverage`), so it checks
//! them instead of repeating them. Per drawn segment (one with a triangle
//! of non-zero area: hidden HD segments keep one degenerate triangle):
//!
//! 1. opaque texels, blended material: every texel its triangles sample in
//!    the bound texture is opaque (alpha >= [`OPAQUE`]) but the material's
//!    variant blends (`ALPHA_ENABLED`). Split by whether the HD original
//!    material blended already (HD cloth, fur: drawn the same) or the
//!    output made it blend (suspect: the Wakka case, opaque parts in a
//!    blended hair material).
//! 2. see-through texels, opaque material: [`MIN_SHARE`] or more of its
//!    texels see-through, but the variant neither blends nor alpha tests
//!    (or tests at threshold 0, which keeps every texel): solid cards.
//! 3. every material's variant resolves: its import index is one of the HD
//!    original's effect variant imports (an appended import is not found by
//!    the game, which then draws nothing: Lulu's collar, Tidus's pant
//!    lattice), the variant exists in the archive and the character's
//!    asset list names it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ffx_phyre::edit::ClusterEdit;

use crate::characters::alpha::{self, Variants};

/// A texel counts as opaque from this alpha up (PS2 `0x78` of `0x80`).
pub const OPAQUE: u8 = 240;

/// The share of see-through texels from which a segment must not be drawn
/// solid (rule 2).
pub const MIN_SHARE: f64 = 0.01;

/// A written texel within this of the mean of the HD texels it covers
/// (each of R, G, B, A) is taken for an unpainted HD texel.
pub const UNPAINTED: u8 = 8;

/// Texels a segment samples.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Texels {
    pub total: u64,
    /// Alpha under [`OPAQUE`].
    pub see: u64,
    /// Alpha 0.
    pub zero: u64,
}

impl Texels {
    pub fn add(&mut self, o: Texels) {
        self.total += o.total;
        self.see += o.see;
        self.zero += o.zero;
    }

    pub fn share(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.see as f64 / self.total as f64
        }
    }
}

/// The texels of a `w` x `h` image (wrapped) whose centres lie inside the
/// triangle `uv` (in pixels, rows top first), or the texel under its
/// centroid when none does. `alpha` reads texel (x, y), 0 <= x < w,
/// 0 <= y < h.
pub fn sample_triangle(
    uv: [[f64; 2]; 3],
    w: u32,
    h: u32,
    alpha: impl Fn(u32, u32) -> u8,
) -> Texels {
    let (wi, hi) = (i64::from(w.max(1)), i64::from(h.max(1)));
    let mut t = Texels::default();
    let mut count = |x: i64, y: i64| {
        let a = alpha(x.rem_euclid(wi) as u32, y.rem_euclid(hi) as u32);
        t.total += 1;
        t.see += u64::from(a < OPAQUE);
        t.zero += u64::from(a == 0);
    };
    let min = |k: usize| uv.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
    let max = |k: usize| uv.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
    let (x0, x1, y0, y1) = (min(0), max(0), min(1), max(1));
    let cross = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    };
    let area = cross(uv[0], uv[1], uv[2]);
    let mut inside = 0;
    if area.abs() > 1e-9 && (x1 - x0) * (y1 - y0) < 1.0e7 {
        for y in y0.floor() as i64..=y1.ceil() as i64 {
            for x in x0.floor() as i64..=x1.ceil() as i64 {
                let p = [x as f64 + 0.5, y as f64 + 0.5];
                let e = [
                    cross(uv[0], uv[1], p),
                    cross(uv[1], uv[2], p),
                    cross(uv[2], uv[0], p),
                ];
                if e.iter().all(|&e| e * area >= 0.0) {
                    inside += 1;
                    count(x, y);
                }
            }
        }
    }
    if inside == 0 {
        let c = [
            (uv[0][0] + uv[1][0] + uv[2][0]) / 3.0,
            (uv[0][1] + uv[1][1] + uv[2][1]) / 3.0,
        ];
        count(c[0].floor() as i64, c[1].floor() as i64);
    }
    t
}

/// How a material draws texture alpha, from its variant's switches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Neither `ALPHA_ENABLED` nor `UATFOPQ`: alpha ignored.
    Opaque,
    /// `UATFOPQ`: alpha tested at the threshold (to coverage with MSAA).
    Test(f32),
    /// `ALPHA_ENABLED`: blended, texels under the threshold dropped.
    Blend(f32),
}

impl Mode {
    pub fn of(alpha_enabled: bool, uatfopq: bool, threshold: f32) -> Self {
        if alpha_enabled {
            Self::Blend(threshold)
        } else if uatfopq {
            Self::Test(threshold)
        } else {
            Self::Opaque
        }
    }

    /// Whether see-through texels show as see-through.
    pub fn shows_alpha(self) -> bool {
        match self {
            Self::Opaque => false,
            Self::Test(t) => t > 0.0,
            Self::Blend(_) => true,
        }
    }
}

/// Rule 1 or 2 for one segment: `Some(1)` opaque texels in a blended
/// material, `Some(2)` see-through texels drawn solid.
pub fn judge(texels: Texels, mode: Mode) -> Option<u8> {
    if texels.total == 0 {
        return None;
    }
    match mode {
        Mode::Blend(_) if texels.see == 0 => Some(1),
        _ if !mode.shows_alpha() && texels.share() >= MIN_SHARE => Some(2),
        _ => None,
    }
}

/// Whether a triangle list has a triangle of non-zero area.
pub fn drawn(positions: &[[f32; 3]], indices: &[u16]) -> bool {
    indices.as_chunks::<3>().0.iter().any(|t| {
        let [a, b, c] = [0, 1, 2].map(|k| {
            positions
                .get(usize::from(t[k]))
                .copied()
                .unwrap_or_default()
        });
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        n.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() > 1e-14
    })
}

/// Which job wrote a model: from `build/files.json` next to `efl/`.
fn writers(efl: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(mod_dir) = efl.parent().and_then(Path::parent) else {
        return out;
    };
    let Ok(text) = std::fs::read_to_string(mod_dir.join("build/files.json")) else {
        return out;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return out;
    };
    if let Some(jobs) = v["jobs"].as_object() {
        for (job, j) in jobs {
            for f in j["files"].as_array().into_iter().flatten() {
                if let Some(p) = f[0].as_str().and_then(|p| p.strip_prefix("efl/x/")) {
                    out.insert(p.to_owned(), job.clone());
                }
            }
        }
    }
    out
}

/// The characters (`kind/name`) the output touches: a model or a texture of
/// theirs is in it.
pub fn characters(efl: &Path) -> Vec<String> {
    let root = efl.join("ffx_data/gamedata/ps3data/chr");
    let mut out = Vec::new();
    let Ok(kinds) = std::fs::read_dir(&root) else {
        return out;
    };
    let mut kinds: Vec<PathBuf> = kinds
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    kinds.sort();
    for k in kinds {
        let Ok(names) = std::fs::read_dir(&k) else {
            continue;
        };
        let mut names: Vec<PathBuf> = names
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        names.sort();
        for n in names {
            let has =
                |sub: &str| std::fs::read_dir(n.join(sub)).is_ok_and(|mut d| d.next().is_some());
            if has("mdl/d3d11") || has("tex/d3d11") {
                let kind = k.file_name().unwrap_or_default().to_string_lossy();
                let name = n.file_name().unwrap_or_default().to_string_lossy();
                out.push(format!("{kind}/{name}"));
            }
        }
    }
    out
}

/// One finding.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule: u8,
    /// HD behaviour (rule 1: the HD material blended already; rule 3: the HD
    /// original has the same), counted, not a suspect.
    pub hd: bool,
    pub text: String,
}

/// One character's audit.
#[derive(Debug, Default)]
pub struct ModelAudit {
    pub spec: String,
    /// The job that wrote the model, "-" when the HD model is used.
    pub job: String,
    pub segments: usize,
    pub drawn: usize,
    pub findings: Vec<Finding>,
    pub notes: Vec<String>,
}

/// A decoded texture: width, height, RGBA rows, whether the output wrote it.
type Page = (u32, u32, Vec<u8>, bool);

struct Ctx<'a> {
    pc: &'a ffx_vbf::Archive,
    efl: &'a Path,
    variants: &'a Variants,
}

impl Ctx<'_> {
    /// A file of the output, else of the archive: (bytes, from the output).
    fn read(&self, path: &str) -> Result<Option<(Vec<u8>, bool)>> {
        let p = self.efl.join(path);
        if p.is_file() {
            return Ok(Some((
                std::fs::read(&p).with_context(|| p.display().to_string())?,
                true,
            )));
        }
        self.hd(path).map(|b| b.map(|b| (b, false)))
    }

    fn hd(&self, path: &str) -> Result<Option<Vec<u8>>> {
        self.pc
            .find(path)
            .map(|e| self.pc.read(e))
            .transpose()
            .map_err(Into::into)
    }
}

/// The D3D11 file names of an asset list, lower case.
fn list_names(list: &[u8]) -> BTreeSet<String> {
    let text = String::from_utf8_lossy(list).to_ascii_lowercase();
    let start = text
        .find("#ifdef phyre_rendering_platform_d3d11")
        .unwrap_or(0);
    let end = text[start..]
        .find("#endif")
        .map_or(text.len(), |e| start + e);
    text[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

fn short(id: &str) -> &str {
    id.rsplit('#').next().unwrap_or(id)
}

fn mode_text(m: Mode) -> String {
    match m {
        Mode::Opaque => "opaque".into(),
        Mode::Test(t) => format!("alpha test {t}"),
        Mode::Blend(t) => format!("blended {t}"),
    }
}

fn audit_one(ctx: &Ctx, spec: &str, writers: &BTreeMap<String, String>) -> Result<ModelAudit> {
    let (kind, name) = spec.split_once('/').context("kind/name")?;
    let path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
    let mut a = ModelAudit {
        spec: spec.to_owned(),
        job: writers.get(&path).cloned().unwrap_or_else(|| "-".into()),
        ..Default::default()
    };
    let Some((bytes, written)) = ctx.read(&path)? else {
        a.notes.push("no model".into());
        return Ok(a);
    };
    if written && a.job == "-" {
        a.job = "?".into();
    }
    let model = ffx_phyre::Model::parse(&bytes).context("model")?;
    let edit = ClusterEdit::parse(&bytes).context("model cluster")?;
    let hd_bytes = ctx.hd(&path)?;
    let hd_edit = hd_bytes
        .as_deref()
        .map(ClusterEdit::parse)
        .transpose()
        .context("HD model cluster")?;

    let ids = alpha::imports(&edit)?;
    let idx = alpha::material_variant_indices(&edit)?;
    let (hd_ids, hd_vars) = match &hd_edit {
        Some(h) => (alpha::imports(h)?, alpha::material_variants(h)?),
        None => (Vec::new(), Vec::new()),
    };
    let hd_set: BTreeSet<&str> = hd_vars.iter().map(String::as_str).collect();
    let list_path = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/{name}.ahwin32");
    let list = ctx.read(&list_path)?.map(|(b, _)| list_names(&b));
    let hd_list = ctx.hd(&list_path)?.map(|b| list_names(&b));

    // segment -> PMaterial index
    a.segments = model.segments.len();
    let seg_mat: Vec<Option<u32>> = (0..model.segments.len())
        .map(|s| edit.segment_material(s as u32).ok())
        .collect();
    let mut used: BTreeSet<u32> = BTreeSet::new();
    let mut tex_cache: BTreeMap<String, Option<Page>> = BTreeMap::new();
    let mut hd_tex: BTreeMap<String, Option<(u32, u32, Vec<u8>)>> = BTreeMap::new();

    for (s, seg) in model.segments.iter().enumerate() {
        if seg.indices.is_empty() || !drawn(&seg.positions, &seg.indices) {
            continue;
        }
        a.drawn += 1;
        let Some(m) = seg_mat[s] else {
            a.notes.push(format!(
                "seg {s}: material not resolved through its mesh's set"
            ));
            continue;
        };
        used.insert(m);
        let Some(&vi) = idx.get(m as usize) else {
            continue;
        };
        let Some(id) = ids.get(usize::from(vi)) else {
            continue;
        };
        let Some(v) = ctx.variants.of(id) else {
            continue;
        }; // rule 3 reports it
        let mat = seg.material.and_then(|k| model.materials.get(k));
        let threshold = mat
            .and_then(|x| x.parameter("AlphaThreshold"))
            .and_then(|p| p.first().copied())
            .unwrap_or(0.0);
        let mode = Mode::of(v.on("ALPHA_ENABLED"), v.on("UATFOPQ"), threshold);
        let Some(tex) = mat.and_then(|x| x.base_color()) else {
            a.notes.push(format!("seg {s}: no TextureSampler"));
            continue;
        };
        let tp = tex.archive_path();
        if !tex_cache.contains_key(&tp) {
            let t = match ctx.read(&tp)? {
                Some((b, w)) => {
                    let t = ffx_phyre::Texture::parse(b).with_context(|| tp.clone())?;
                    Some((t.width, t.height, t.to_rgba(), w))
                }
                None => None,
            };
            tex_cache.insert(tp.clone(), t);
        }
        let Some((w, h, px, tex_written)) = &tex_cache[&tp] else {
            a.notes.push(format!("seg {s}: texture {tp} not found"));
            continue;
        };
        let (w, h) = (*w, *h);
        // the HD texture under a written one: are its see-through texels
        // the HD's own (unpainted parts of the page keep the HD texels)?
        if *tex_written && !hd_tex.contains_key(&tp) {
            let t = match ctx.hd(&tp)? {
                Some(b) => ffx_phyre::Texture::parse(b)
                    .ok()
                    .map(|t| (t.width, t.height, t.to_rgba())),
                None => None,
            };
            hd_tex.insert(tp.clone(), t);
        }
        let hd_px = if *tex_written {
            hd_tex.get(&tp).and_then(Option::as_ref)
        } else {
            None
        };
        let (hd_too, unpainted) = (std::cell::Cell::new(0u64), std::cell::Cell::new(0u64));
        let read = |x: u32, y: u32| {
            let o = ((y * w + x) * 4) as usize;
            let a = px[o + 3];
            if a < OPAQUE
                && let Some((hw, hh, hp)) = hd_px
            {
                // the HD texels this one covers, averaged (pages are written
                // at 1/1 or 1/2 of the HD size)
                let (sx, sy) = ((hw / w).max(1), (hh / h).max(1));
                let (hx0, hy0) = ((x * hw / w).min(hw - 1), (y * hh / h).min(hh - 1));
                let mut sum = [0u32; 4];
                let mut n = 0;
                for hy in hy0..(hy0 + sy).min(*hh) {
                    for hx in hx0..(hx0 + sx).min(*hw) {
                        let q = ((hy * hw + hx) * 4) as usize;
                        (0..4).for_each(|k| sum[k] += u32::from(hp[q + k]));
                        n += 1;
                    }
                }
                let mean = sum.map(|v| (v / n.max(1)) as u8);
                if mean[3] < OPAQUE {
                    hd_too.set(hd_too.get() + 1);
                }
                if (0..4).all(|k| px[o + k].abs_diff(mean[k]) <= UNPAINTED) {
                    unpainted.set(unpainted.get() + 1);
                }
            }
            a
        };
        let mut texels = Texels::default();
        for t in seg.indices.as_chunks::<3>().0 {
            let uv = [0, 1, 2].map(|k| {
                let p = seg.uvs.get(usize::from(t[k])).copied().unwrap_or_default();
                [
                    f64::from(p[0]) * f64::from(w),
                    (1.0 - f64::from(p[1])) * f64::from(h),
                ]
            });
            texels.add(sample_triangle(uv, w, h, read));
        }
        let (hd_too, unpainted) = (hd_too.get(), unpainted.get());
        // see-through texels the converter wrote (not the HD's left over)
        let painted_see = if texels.total == 0 {
            0.0
        } else {
            (texels.see - unpainted) as f64 / texels.total as f64
        };
        if std::env::var_os("DM_AUDIT_SEGS").is_some() {
            a.notes.push(format!(
                "seg {s} ({} tris) material {m} {} {} {}: {} texels {:.2}% see-through ({hd_too} HD see-through too, {unpainted} unpainted)",
                seg.indices.len() / 3,
                short(id),
                mode_text(mode),
                tp.rsplit('/').next().unwrap_or(&tp),
                texels.total,
                texels.share() * 100.0
            ));
        }
        let Some(rule) = judge(texels, mode) else {
            continue;
        };
        if let Some(dir) = std::env::var_os("DM_AUDIT_PNG") {
            // the page (alpha dropped, darkened) with the segment's texels:
            // opaque ones bright, see-through ones magenta (alpha 0) or cyan
            let mut img: Vec<u8> = px
                .chunks(4)
                .flat_map(|c| [c[0] / 4, c[1] / 4, c[2] / 4, 255])
                .collect();
            for t in seg.indices.as_chunks::<3>().0 {
                let uv = [0, 1, 2].map(|k| {
                    let p = seg.uvs.get(usize::from(t[k])).copied().unwrap_or_default();
                    [
                        f64::from(p[0]) * f64::from(w),
                        (1.0 - f64::from(p[1])) * f64::from(h),
                    ]
                });
                let mark = std::cell::RefCell::new(&mut img);
                sample_triangle(uv, w, h, |x, y| {
                    let o = ((y * w + x) * 4) as usize;
                    let a = px[o + 3];
                    let c = match a {
                        0 => [255, 0, 255],
                        a if a < OPAQUE => [0, 255, 255],
                        _ => [px[o], px[o + 1], px[o + 2]],
                    };
                    mark.borrow_mut()[o..o + 3].copy_from_slice(&c);
                    a
                });
            }
            let f = PathBuf::from(dir).join(format!("{}-seg{s}.png", spec.replace('/', "-")));
            let _ = std::fs::write(&f, ffx_phyre::png(w, h, &img));
            // and the page as an opaque material shows it (alpha ignored)
            let rgb: Vec<u8> = px.chunks(4).flat_map(|c| [c[0], c[1], c[2], 255]).collect();
            let _ = std::fs::write(f.with_extension("rgb.png"), ffx_phyre::png(w, h, &rgb));
        }
        let tex_name = tp.rsplit('/').next().unwrap_or(&tp);
        let anim = if mat.is_some_and(|x| x.texture("AnimSampler").is_some()) {
            ", texture animation"
        } else {
            ""
        };
        let from = if *tex_written { "output" } else { "HD" };
        let what = format!(
            "{spec} seg {s} ({} tris): material {m} {} {}, {tex_name} ({from}{anim}): {} texels, {:.1}% see-through, {:.1}% alpha 0{}",
            seg.indices.len() / 3,
            short(id),
            mode_text(mode),
            texels.total,
            texels.share() * 100.0,
            texels.zero as f64 * 100.0 / texels.total as f64,
            if *tex_written && texels.see > 0 {
                format!(
                    "; of those, HD see-through too {:.0}%, unpainted (HD texel) {:.0}%; painted see-through {:.1}% of all",
                    hd_too as f64 * 100.0 / texels.see as f64,
                    unpainted as f64 * 100.0 / texels.see as f64,
                    painted_see * 100.0
                )
            } else {
                String::new()
            }
        );
        // Did the HD original draw this material the same? Its variant id is
        // the HD material's (same index) or one an HD material blends with.
        let hd_same = hd_vars.get(m as usize).is_some_and(|h| h == id)
            || (m as usize >= hd_vars.len() && hd_set.contains(id.as_str()));
        let hd = match rule {
            1 => hd_same,
            // see-through texels that are (nearly all) the HD's own, in an
            // HD material: drawn as the HD drew them
            _ => hd_same && (!*tex_written || painted_see < MIN_SHARE),
        };
        a.findings.push(Finding {
            rule,
            hd,
            text: what,
        });
    }

    // rule 3: every material's variant
    for (m, &vi) in idx.iter().enumerate() {
        let m32 = m as u32;
        let tag = if used.contains(&m32) {
            "drawn"
        } else {
            "unused"
        };
        let id = ids.get(usize::from(vi)).cloned().unwrap_or_default();
        let mut why = Vec::new();
        if usize::from(vi) >= ids.len() {
            why.push(format!("import {vi} outside the {} imports", ids.len()));
        } else if hd_edit.is_some()
            && (usize::from(vi) >= hd_ids.len() || !hd_ids[usize::from(vi)].contains(".fx#"))
        {
            why.push(format!("import {vi} is not one of the HD model's {} effect variant imports (appended: not found by the game)", hd_ids.len()));
        }
        if !id.is_empty()
            && ctx.variants.of(&id).is_none()
            && ctx.read(&alpha::variant_path(&id))?.is_none()
        {
            why.push("variant not in the archive".into());
        }
        let entry = ffx_phyre::assets::texture_entry(&id).to_ascii_lowercase();
        let hd_lacks_too =
            hd_set.contains(id.as_str()) && hd_list.as_ref().is_some_and(|l| !l.contains(&entry));
        if !id.is_empty() && list.as_ref().is_some_and(|l| !l.contains(&entry)) {
            why.push(format!(
                "not in {name}.ahwin32{}",
                if hd_lacks_too {
                    " (nor in the HD list, which the HD model uses it with)"
                } else {
                    ""
                }
            ));
        }
        if why.is_empty() {
            continue;
        }
        let only_list = why.len() == 1 && hd_lacks_too;
        a.findings.push(Finding {
            rule: 3,
            hd: only_list || !used.contains(&m32),
            text: format!(
                "{spec} material {m} ({tag}) {}: {}",
                short(&id),
                why.join("; ")
            ),
        });
    }
    Ok(a)
}

/// Audits `specs` (`kind/name`; every character the output touches when
/// empty). `known`: characters whose see-through leftovers the converter
/// reported ("left as they were", "left:"): their rule 2 findings count as
/// known, not as suspects.
pub fn run(
    pc: &ffx_vbf::Archive,
    efl: &Path,
    specs: &[String],
    known: &BTreeSet<String>,
) -> Result<String> {
    use rayon::prelude::*;
    let variants = Variants::get(pc)?;
    let ctx = Ctx { pc, efl, variants };
    let writers = writers(efl);
    let specs: Vec<String> = if specs.is_empty() {
        characters(efl)
    } else {
        specs.to_vec()
    };
    let audits: Vec<(String, Result<ModelAudit>)> = specs
        .par_iter()
        .map(|s| (s.clone(), audit_one(&ctx, s, &writers)))
        .collect();

    let mut out = String::new();
    // [rule][kind]: (suspects, hd, suspects of known leftovers)
    let mut counts: BTreeMap<(u8, String), [usize; 3]> = BTreeMap::new();
    let mut models_with: BTreeMap<u8, BTreeSet<String>> = BTreeMap::new();
    let (mut n_models, mut n_drawn, mut errors) = (0, 0, 0);
    let group = |job: &str| -> String {
        match job.split('.').next() {
            Some("models") => "remodelled".into(),
            Some("chr") => "kept".into(),
            _ => "hd model".into(),
        }
    };
    let mut lines: BTreeMap<(u8, bool), Vec<String>> = BTreeMap::new();
    for (spec, r) in &audits {
        let a = match r {
            Ok(a) => a,
            Err(e) => {
                errors += 1;
                let _ = writeln!(out, "error {spec}: {e:#}");
                continue;
            }
        };
        n_models += 1;
        n_drawn += a.drawn;
        let g = group(&a.job);
        for f in &a.findings {
            let k = f.rule == 2 && known.contains(spec);
            let c = counts.entry((f.rule, g.clone())).or_default();
            if f.hd {
                c[1] += 1;
            } else if k {
                c[2] += 1;
            } else {
                c[0] += 1;
            }
            if !f.hd {
                models_with.entry(f.rule).or_default().insert(spec.clone());
            }
            let mark = if f.hd {
                "hd"
            } else if k {
                "known"
            } else {
                "SUSPECT"
            };
            lines
                .entry((f.rule, f.hd))
                .or_default()
                .push(format!("  [{mark}] [{}] {}", a.job, f.text));
        }
        if std::env::var_os("DM_AUDIT_NOTES").is_some()
            || std::env::var_os("DM_AUDIT_SEGS").is_some()
        {
            for n in &a.notes {
                let _ = writeln!(out, "note {spec}: {n}");
            }
        }
    }
    let title = |rule: u8, hd: bool| match (rule, hd) {
        (1, false) => "rule 1, opaque texels in a blended material, made blended in the output",
        (1, true) => "rule 1, opaque texels in a blended material, blended in the HD model already",
        (2, false) => "rule 2, see-through texels drawn solid",
        (2, true) => {
            "rule 2, see-through texels drawn solid, HD material on the HD's own texels (HD behaviour)"
        }
        (3, false) => "rule 3, variant the game does not resolve (drawn materials)",
        _ => "rule 3, unresolved variant on unused materials, or the HD's own asset list",
    };
    for ((rule, hd), ls) in &lines {
        let _ = writeln!(out, "\n{} ({}):", title(*rule, *hd), ls.len());
        for l in ls {
            let _ = writeln!(out, "{l}");
        }
    }
    let _ = writeln!(
        out,
        "\nsummary: {n_models} models, {n_drawn} drawn segments, {errors} errors"
    );
    for ((rule, g), c) in &counts {
        let _ = writeln!(
            out,
            "  rule {rule} {g:<10}: {} suspect, {} known leftover, {} hd",
            c[0], c[2], c[1]
        );
    }
    for (rule, ms) in &models_with {
        let _ = writeln!(
            out,
            "  rule {rule} models: {}",
            ms.iter().cloned().collect::<Vec<_>>().join(" ")
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_texel_centres_inside() {
        // a 4x4 image, alpha 0 in column 0, opaque elsewhere
        let alpha = |x: u32, _y: u32| if x == 0 { 0 } else { 255 };
        // triangle covering columns 1..4, rows 0..4 (lower right half)
        let t = sample_triangle([[1.0, 4.0], [4.0, 4.0], [4.0, 1.0]], 4, 4, alpha);
        assert_eq!(t.see, 0);
        assert!(t.total >= 3);
        // the whole image: 4 of 16 see-through
        let a = sample_triangle([[0.0, 0.0], [8.0, 0.0], [0.0, 8.0]], 4, 4, alpha);
        let b = sample_triangle([[0.0, 0.0], [4.0, 0.0], [0.0, 4.0]], 4, 4, alpha);
        assert!(a.total > b.total);
        assert!(b.see > 0 && b.zero == b.see);
        // tiny: the centroid's texel
        let c = sample_triangle([[0.1, 0.1], [0.2, 0.1], [0.1, 0.2]], 4, 4, alpha);
        assert_eq!((c.total, c.see), (1, 1));
        // wraps
        let d = sample_triangle([[4.1, 0.1], [4.2, 0.1], [4.1, 0.2]], 4, 4, alpha);
        assert_eq!((d.total, d.see), (1, 1));
        // either winding
        let e = sample_triangle([[4.0, 1.0], [4.0, 4.0], [1.0, 4.0]], 4, 4, alpha);
        assert_eq!(e, t);
    }

    #[test]
    fn judges() {
        let opaque = Texels {
            total: 100,
            see: 0,
            zero: 0,
        };
        let fringe = Texels {
            total: 100,
            see: 5,
            zero: 1,
        };
        let speck = Texels {
            total: 1000,
            see: 2,
            zero: 0,
        };
        assert_eq!(judge(opaque, Mode::Blend(0.0078)), Some(1));
        assert_eq!(judge(opaque, Mode::Opaque), None);
        assert_eq!(judge(fringe, Mode::Opaque), Some(2));
        assert_eq!(judge(fringe, Mode::Test(0.0)), Some(2));
        assert_eq!(judge(fringe, Mode::Test(0.5)), None);
        assert_eq!(judge(fringe, Mode::Blend(0.1)), None);
        assert_eq!(judge(speck, Mode::Opaque), None);
        assert_eq!(judge(Texels::default(), Mode::Blend(0.1)), None);
        assert_eq!(Mode::of(true, true, 0.5), Mode::Blend(0.5));
        assert_eq!(Mode::of(false, true, 0.5), Mode::Test(0.5));
        assert_eq!(Mode::of(false, false, 0.5), Mode::Opaque);
    }

    #[test]
    fn hidden_segments() {
        let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert!(drawn(&p, &[0, 1, 2]));
        assert!(!drawn(&p, &[0, 0, 0]));
        assert!(!drawn(&p, &[0, 1, 1]));
    }

    #[test]
    fn asset_list_names() {
        let list = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11\nconst char *g_fileNames[] = {\n\"A/B.phyre\"\n,\"c.phyre\"\n};\n#endif\n#ifdef X\n\"d\"\n#endif";
        let n = list_names(list);
        assert_eq!(
            n,
            BTreeSet::from(["a/b.phyre".to_owned(), "c.phyre".to_owned()])
        );
    }
    /// Rule 3 on a real model: an effect variant import appended to Tidus's
    /// HD model (the converter's old way) is reported. Needs the archive:
    /// `DM_VBF=<FFX_Data.vbf> cargo test -p demaster --release -- --ignored appended`.
    #[test]
    #[ignore = "needs FFX_Data.vbf (DM_VBF)"]
    fn appended_variant_import() {
        let pc = ffx_vbf::Archive::open(std::env::var("DM_VBF").expect("DM_VBF")).expect("archive");
        let path = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
        let hd = pc.read(pc.find(path).expect("c001")).expect("read");
        let mut edit = ClusterEdit::parse(&hd).expect("parse");
        let ids = alpha::imports(&edit).expect("imports");
        let vars = alpha::material_variants(&edit).expect("variants");
        let variants = Variants::get(&pc).expect("variants");
        // a character variant the model does not import
        let other = variants
            .iter()
            .map(|(p, _)| {
                p.rsplit('#')
                    .next()
                    .unwrap_or(p)
                    .trim_end_matches(".phyre")
                    .to_ascii_uppercase()
            })
            .find(|h| h.len() == 32 && !ids.iter().any(|i| i.ends_with(h.as_str())))
            .expect("a variant");
        let (prefix, _) = vars[0].rsplit_once('#').expect("id");
        let id = format!("{prefix}#{other}");
        let like = ids.iter().position(|i| *i == vars[0]).expect("import") as u32;
        let i = edit.add_import(like, &id).expect("append");
        assert_eq!(
            alpha::set_material_variant(&mut edit, 0, &id, &mut vec![i]).expect("set"),
            Ok(())
        );
        let dir = std::env::temp_dir().join(format!("mataudit-{}", std::process::id()));
        let out = dir.join(path);
        std::fs::create_dir_all(out.parent().expect("dir")).expect("mkdir");
        std::fs::write(&out, edit.write().expect("write")).expect("write");
        let ctx = Ctx {
            pc: &pc,
            efl: &dir,
            variants,
        };
        let a = audit_one(&ctx, "pc/c001", &BTreeMap::new()).expect("audit");
        std::fs::remove_dir_all(&dir).expect("cleanup");
        assert!(
            a.findings
                .iter()
                .any(|f| f.rule == 3 && f.text.contains("appended")),
            "{:?}",
            a.findings
        );
    }
}
