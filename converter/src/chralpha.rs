//! Character materials and texture alpha (`docs/models.md`, "Alpha").
//!
//! A `PMaterial` draws with the effect variant its `m_effectVariant`
//! names: a user fixup holding a big-endian index into the cluster's
//! `PAssetReferenceImport`s, whose id is
//! `PS3Data/Shaders/PhyreChrLitShader.fx#<md5>`
//! (`shaders/d3d11/phyrechrlitshader.fx#<md5>.phyre` in the archive). The
//! variant's switches (`PMaterialSwitch`) decide how it draws:
//! `ALPHA_ENABLED` variants only have a `Transparent` pass (blended by
//! texture alpha, far to near), `UATFOPQ` ones draw in the `Opaque` pass
//! with alpha to coverage on and the alpha test (`AlphaThreshold`), the
//! others opaque without either.
//!
//! The PS2 drew every character polygon with alpha blending on (GIF tag
//! `PRIM` ABE, context 2) and `ALPHA_2` = `0x44`, (Cs - Cd) × As + Cd, the
//! texture's alpha taken (TCC): the texture-list entry's bytes `+8`, `+9`
//! (`0x44 00` in every file), written as register `0x43` by the packet
//! builder (FFX.exe `FUN_008438b0`, from `FUN_008423c0`). So a texel with
//! GS alpha under `0x80` is see-through on the PS2: fur and hair fringes
//! fade out. The PC draws the same texels with the HD material's variant,
//! which may be opaque (alpha ignored: solid quads) or alpha tested at 0.5
//! (hard, ragged cut-outs). Materials that sample such texels are switched
//! to their `UATFOPQ` twin (the variant with the same switches and
//! `UATFOPQ` on) with `AlphaThreshold` lowered to [`THRESHOLD`]: with MSAA,
//! alpha to coverage turns texture alpha into partial coverage, so the
//! fringes fade out in the opaque pass. The blended (`ALPHA_ENABLED`)
//! twins were tried first: they look the same in cut scenes, but in battle
//! the parts that lie over the character's own body (Lulu's necklace and
//! collar, Kimahri's leg wraps, Tidus's wristband) were not drawn.
//! HD materials that blend already keep blending on kept meshes, with the
//! lower threshold.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use anyhow::{Context, Result, ensure};
use ffx_phyre::cluster::Cluster;
use ffx_phyre::edit::ClusterEdit;

/// Byte offset of the field a fixup's member word names.
fn field_offset(cluster: &Cluster, word: u32) -> Option<u32> {
    if word & 0x8000_0000 != 0 {
        Some(word & 0x7fff_ffff)
    } else {
        cluster.member_by_index(word).map(|(_, m)| m.offset)
    }
}

fn offset_of(edit: &ClusterEdit, list: usize, member: &str) -> Result<u32> {
    let class = &edit.cluster.classes[edit.lists[list].class].name;
    edit.cluster.member_offset(class, member).with_context(|| format!("{class} has no {member}"))
}

fn list(edit: &ClusterEdit, name: &str) -> Result<usize> {
    edit.list_of(name).with_context(|| format!("no {name} list"))
}

/// The string an array fixup of `obj` at `field` points to.
fn array_string(edit: &ClusterEdit, l: usize, obj: u32, field: u32) -> Option<String> {
    let f = edit.lists[l]
        .array_fixups
        .iter()
        .find(|f| f.object() == obj && field_offset(&edit.cluster, f.member()) == Some(field))?;
    let tail = edit.lists[l].arrays.get(f.0[3] as usize..)?;
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    Some(String::from_utf8_lossy(&tail[..end]).into_owned())
}

/// The ids of the cluster's asset imports, by index.
pub fn imports(edit: &ClusterEdit) -> Result<Vec<String>> {
    let l = list(edit, "PAssetReferenceImport")?;
    let field = offset_of(edit, l, "m_id")?;
    Ok((0..edit.lists[l].count()).map(|i| array_string(edit, l, i, field).unwrap_or_default()).collect())
}

/// The pointer fixup of material `m`'s `m_effectVariant`.
fn variant_fixup(edit: &ClusterEdit, m: u32) -> Result<(usize, usize)> {
    let l = list(edit, "PMaterial")?;
    let field = offset_of(edit, l, "m_effectVariant")?;
    let k = edit.lists[l]
        .pointer_fixups
        .iter()
        .position(|f| f.object() == m && field_offset(&edit.cluster, f.member()) == Some(field))
        .with_context(|| format!("material {m} has no effect variant"))?;
    Ok((l, k))
}

/// The effect variant import id of every `PMaterial`.
pub fn material_variants(edit: &ClusterEdit) -> Result<Vec<String>> {
    let ids = imports(edit)?;
    let l = list(edit, "PMaterial")?;
    (0..edit.lists[l].count())
        .map(|m| {
            let (l, k) = variant_fixup(edit, m)?;
            let r = edit.lists[l].pointer_fixups[k].user_fixup().context("effect variant is not a user fixup")?;
            let [_, len, at] = edit.user_records[r as usize];
            anyhow::ensure!(len == 2, "effect variant fixup of {len} bytes");
            let i = u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]);
            ids.get(usize::from(i)).cloned().with_context(|| format!("import {i}"))
        })
        .collect()
}

/// Points material `m` at effect variant `id` (an import id), adding the
/// import (a copy of the material's current one) when the cluster has none.
pub fn set_material_variant(edit: &mut ClusterEdit, m: u32, id: &str) -> Result<()> {
    let (l, k) = variant_fixup(edit, m)?;
    let old = edit.lists[l].pointer_fixups[k].user_fixup().context("effect variant is not a user fixup")?;
    let index = match edit.import_index(id)? {
        Some(i) => i,
        None => {
            let [_, _, at] = edit.user_records[old as usize];
            let current = u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]);
            edit.add_import(u32::from(current), id)?
        }
    };
    let index = u16::try_from(index).context("imports")?;
    let at = u32::try_from(edit.user_data.len())?;
    edit.user_data.extend_from_slice(&index.to_be_bytes());
    let kind = edit.user_records[old as usize][0];
    edit.user_records.push([kind, 2, at]);
    let record = u32::try_from(edit.user_records.len() - 1)?;
    edit.lists[l].pointer_fixups[k].0[6] = record;
    Ok(())
}

/// The parameter layout of material `m`'s buffer: (name, type, size << 16
/// | offset) per tweakable definition, in order. A material can only switch
/// to a variant whose own material has the same layout (its location
/// tables address the buffer by offset).
pub fn buffer_layout(edit: &ClusterEdit, m: u32) -> Result<Vec<(String, u8, u32)>> {
    let materials = list(edit, "PMaterial")?;
    let field = offset_of(edit, materials, "m_parameterBuffer")?;
    let f = edit.lists[materials]
        .pointer_fixups
        .iter()
        .find(|f| f.object() == m && field_offset(&edit.cluster, f.member()) == Some(field))
        .context("material without a parameter buffer")?;
    let buffer = f.target_list() as usize;
    let obj = f.target();
    let defs_field = offset_of(edit, buffer, "m_tweakableShaderParameterDefinitions")?;
    let d = edit.lists[buffer]
        .pointer_fixups
        .iter()
        .find(|f| f.object() == obj && field_offset(&edit.cluster, f.member()) == Some(defs_field + 4))
        .context("buffer without definitions")?;
    let dl = d.target_list() as usize;
    let (o_name, o_type, o_loc) = (
        offset_of(edit, dl, "m_name")?,
        offset_of(edit, dl, "m_parameterType")? as usize,
        offset_of(edit, dl, "m_bufferLoc")? as usize,
    );
    Ok((d.target()..d.target() + d.count())
        .map(|i| {
            let o = edit.lists[dl].object(i);
            (
                array_string(edit, dl, i, o_name).unwrap_or_default(),
                o[o_type],
                u32::from_le_bytes(o[o_loc..o_loc + 4].try_into().expect("4 bytes")),
            )
        })
        .collect())
}

/// The archive path of an effect variant import id.
pub fn variant_path(id: &str) -> String {
    let lower = id.to_ascii_lowercase();
    let (dir, file) = lower.rsplit_once('/').unwrap_or(("", &lower));
    format!("ffx_data/gamedata/{dir}/d3d11/{file}.phyre")
}

/// An effect variant's switches (name → value), render pass types and the
/// parameter layout of its own material.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Variant {
    pub switches: BTreeMap<String, String>,
    pub passes: Vec<String>,
    pub layout: Vec<(String, u8, u32)>,
}

impl Variant {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let edit = ClusterEdit::parse(bytes).context("effect variant")?;
        let mut v = Self::default();
        if edit.list_of("PMaterial").is_some_and(|l| edit.lists[l].count() > 0) {
            v.layout = buffer_layout(&edit, 0).unwrap_or_default();
        }
        if let Some(l) = edit.list_of("PMaterialSwitch") {
            let (n, val) = (offset_of(&edit, l, "m_name")?, offset_of(&edit, l, "m_value")?);
            for i in 0..edit.lists[l].count() {
                v.switches.insert(
                    array_string(&edit, l, i, n).unwrap_or_default(),
                    array_string(&edit, l, i, val).unwrap_or_default(),
                );
            }
        }
        // pass types are user fixups (enum names) of PSceneRenderPass.m_passType
        if let Some(l) = edit.list_of("PSceneRenderPass") {
            let field = offset_of(&edit, l, "m_passType")?;
            for i in 0..edit.lists[l].count() {
                let name = edit.lists[l]
                    .pointer_fixups
                    .iter()
                    .find(|f| f.object() == i && field_offset(&edit.cluster, f.member()) == Some(field))
                    .and_then(|f| f.user_fixup())
                    .and_then(|r| edit.cluster.user_fixups.get(r as usize))
                    .map(|u| u.text.clone())
                    .unwrap_or_default();
                v.passes.push(name);
            }
        }
        Ok(v)
    }

    pub fn on(&self, switch: &str) -> bool {
        self.switches.get(switch).is_some_and(|v| v == "1")
    }

    fn on_set(&self) -> BTreeSet<&str> {
        self.switches.iter().filter(|x| x.1 == "1").map(|x| x.0.as_str()).collect()
    }
}

/// The variants of the effects character materials use, by archive path.
pub struct Variants {
    by_path: BTreeMap<String, Variant>,
}

/// Effects whose variants are indexed (characters use these two).
const EFFECTS: [&str; 2] = ["phyrechrlitshader.fx#", "phyredefaultlitshader.fx#"];

static VARIANTS: OnceLock<Variants> = OnceLock::new();

impl Variants {
    /// Every variant of [`EFFECTS`] in the archive, read once per run.
    pub fn get(pc: &ffx_vbf::Archive) -> Result<&'static Variants> {
        if let Some(v) = VARIANTS.get() {
            return Ok(v);
        }
        use rayon::prelude::*;
        let paths: Vec<&str> = pc
            .entries()
            .iter()
            .map(|e| e.path.as_str())
            .filter(|p| {
                p.strip_prefix("ffx_data/gamedata/ps3data/shaders/d3d11/")
                    .is_some_and(|f| EFFECTS.iter().any(|e| f.starts_with(e)))
            })
            .collect();
        let parsed: Vec<Result<(String, Variant)>> = paths
            .par_iter()
            .map(|p| {
                let e = pc.find(p).context("variant")?;
                Ok(((*p).to_owned(), Variant::parse(&pc.read(e)?).with_context(|| (*p).to_owned())?))
            })
            .collect();
        let by_path = parsed.into_iter().collect::<Result<BTreeMap<_, _>>>()?;
        Ok(VARIANTS.get_or_init(|| Variants { by_path }))
    }

    pub fn of(&self, id: &str) -> Option<&Variant> {
        self.by_path.get(&variant_path(id))
    }

    /// The see-through twin of variant `id` (an import id): the same effect
    /// and switches with `UATFOPQ` on (alpha to coverage and the alpha test,
    /// drawn in the opaque pass). `id` itself when it has `UATFOPQ`, or
    /// `ALPHA_ENABLED` and `keep_blend`; else a blended variant becomes its
    /// `UATFOPQ` twin too (`ALPHA_ENABLED` off).
    pub fn blended(&self, id: &str, keep_blend: bool) -> Option<(String, &Variant)> {
        let v = self.of(id)?;
        if v.on("UATFOPQ") || (keep_blend && v.on("ALPHA_ENABLED")) {
            return Some((id.to_owned(), v));
        }
        let mut want = v.on_set();
        want.remove("ALPHA_ENABLED");
        want.insert("UATFOPQ");
        let (prefix, _) = id.rsplit_once('#')?;
        let dir_file = variant_path(prefix);
        let stem = dir_file.strip_suffix(".phyre")?;
        // Not every switch set has such a variant; the depth pre-pass
        // switches may go.
        const OPTIONAL: [&str; 3] = ["ADDITIONAL_ZPREPASS", "BUILT_IN_ZPREPASS", "ZPP_IGNORE_Z"];
        (0..1u32 << OPTIONAL.len()).find_map(|drop| {
            let mut want = want.clone();
            for (k, s) in OPTIONAL.iter().enumerate() {
                if drop & (1 << k) != 0 && !want.remove(s) {
                    return None;
                }
            }
            self.by_path
                .iter()
                .filter(|(p, _)| p.strip_prefix(stem).is_some_and(|r| r.starts_with('#')))
                .find(|(_, w)| w.on_set() == want && w.layout == v.layout)
                .map(|(p, w)| {
                    let hash = p.rsplit_once('#').map_or("", |x| x.1).trim_end_matches(".phyre");
                    (format!("{prefix}#{}", hash.to_ascii_uppercase()), w)
                })
        })
    }
}

/// `AlphaThreshold` of the see-through materials (at most; a lower one
/// stays): texels under it are dropped. For blended materials, lower lets
/// the faint fringe texels write depth over what the same draw puts behind
/// them later (pale outlines where a bird's wing feathers overlap); higher
/// cuts the fringe like the HD's 0.3-0.5.
pub const THRESHOLD: f32 = 0.1;

/// A texel counts as see-through below this alpha (PS2 alpha 0x78 of 0x80).
pub const OPAQUE: u8 = 240;

/// The share of see-through texels among those an HD material's triangles
/// sample above which a kept-mesh material is blended.
pub const MIN_SHARE: f32 = 0.01;

/// The texels a triangle samples and how many of them are see-through:
/// `uv` in texels (not wrapped), `alpha` the alpha of texel (x, y) (the
/// caller wraps or clamps). Texels whose centre lies inside the triangle
/// count; a triangle without one samples the texel under its centroid.
/// Its corners are left out: they sit on the edge of their island, where
/// the texels outside are usually alpha 0 but not drawn.
pub fn coverage(uv: [[f64; 2]; 3], alpha: impl Fn(i64, i64) -> u8) -> (usize, usize) {
    let see = |x: i64, y: i64| usize::from(alpha(x, y) < OPAQUE);
    let (x0, x1) = (uv.iter().map(|p| p[0]).fold(f64::MAX, f64::min), uv.iter().map(|p| p[0]).fold(f64::MIN, f64::max));
    let (y0, y1) = (uv.iter().map(|p| p[1]).fold(f64::MAX, f64::min), uv.iter().map(|p| p[1]).fold(f64::MIN, f64::max));
    let d = (uv[1][1] - uv[2][1]) * (uv[0][0] - uv[2][0]) + (uv[2][0] - uv[1][0]) * (uv[0][1] - uv[2][1]);
    let centroid = || {
        let c = [(uv[0][0] + uv[1][0] + uv[2][0]) / 3.0, (uv[0][1] + uv[1][1] + uv[2][1]) / 3.0];
        (see(c[0].floor() as i64, c[1].floor() as i64), 1)
    };
    if d.abs() < 1e-12 || (x1 - x0) * (y1 - y0) > 1.0e6 {
        return centroid();
    }
    let (mut n, mut total) = (0, 0);
    for y in (y0.floor() as i64)..=(y1.ceil() as i64) {
        for x in (x0.floor() as i64)..=(x1.ceil() as i64) {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let a = ((uv[1][1] - uv[2][1]) * (px - uv[2][0]) + (uv[2][0] - uv[1][0]) * (py - uv[2][1])) / d;
            let b = ((uv[2][1] - uv[0][1]) * (px - uv[2][0]) + (uv[0][0] - uv[2][0]) * (py - uv[2][1])) / d;
            if a >= 0.0 && b >= 0.0 && a + b <= 1.0 {
                total += 1;
                n += see(x, y);
            }
        }
    }
    if total == 0 { centroid() } else { (n, total) }
}

/// Where parameter `name` of material `m` lives: (buffer list, buffer
/// object, offset, size).
fn parameter_at(edit: &ClusterEdit, m: u32, name: &str) -> Result<(usize, u32, usize, usize)> {
    parameter_at_kind(edit, m, name, 0x40)
}

fn parameter_at_kind(edit: &ClusterEdit, m: u32, name: &str, kind: u8) -> Result<(usize, u32, usize, usize)> {
    let (_, _, loc) = buffer_layout(edit, m)?
        .into_iter()
        .find(|p| p.0 == name && p.1 == kind)
        .with_context(|| format!("material {m} has no {name}"))?;
    let materials = list(edit, "PMaterial")?;
    let field = offset_of(edit, materials, "m_parameterBuffer")?;
    let f = edit.lists[materials]
        .pointer_fixups
        .iter()
        .find(|f| f.object() == m && field_offset(&edit.cluster, f.member()) == Some(field))
        .context("material without a parameter buffer")?;
    Ok((f.target_list() as usize, f.target(), (loc & 0xffff) as usize, (loc >> 16) as usize))
}

/// The `f32` values of parameter `name` of material `m`.
pub fn parameter(edit: &ClusterEdit, m: u32, name: &str) -> Result<Vec<f32>> {
    let (l, o, offset, size) = parameter_at(edit, m, name)?;
    let obj = edit.lists[l].object(o);
    Ok((0..size / 4)
        .map(|k| f32::from_le_bytes(obj[offset + 4 * k..offset + 4 * k + 4].try_into().expect("4 bytes")))
        .collect())
}

/// Writes the `f32` values of parameter `name` of material `m`.
pub fn set_parameter(edit: &mut ClusterEdit, m: u32, name: &str, values: &[f32]) -> Result<()> {
    let (l, o, offset, size) = parameter_at(edit, m, name)?;
    ensure!(values.len() * 4 <= size, "{name} holds {size} bytes");
    let obj = edit.lists[l].object_mut(o);
    for (k, v) in values.iter().enumerate() {
        obj[offset + 4 * k..offset + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
    }
    Ok(())
}

/// What [`blend`] did to a material.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// Switched to the blended twin (the id).
    Switched(String),
    /// Blended already; its threshold lowered from the value.
    Lowered(f32),
    /// Blended already with a threshold at or under [`THRESHOLD`].
    None,
}

/// Makes material `m` blend by texture alpha like the PS2: its variant's
/// blended twin, `AlphaThreshold` at most [`THRESHOLD`]. Returns what
/// changed, or why it could not.
pub fn blend(
    edit: &mut ClusterEdit,
    m: u32,
    variants: &Variants,
    keep_blend: bool,
) -> Result<std::result::Result<Change, String>> {
    // for before/after comparisons in the game
    if std::env::var_os("DM_NO_BLEND").is_some() {
        return Ok(Err("DM_NO_BLEND set".into()));
    }
    let id = material_variants(edit)?.get(m as usize).cloned().context("material")?;
    let Some((twin, v)) = variants.blended(&id, keep_blend) else {
        return Ok(Err(format!("no blended twin of {id}")));
    };
    let current = parameter(edit, m, "AlphaThreshold")?.first().copied().unwrap_or(0.0);
    if twin == id {
        if current <= THRESHOLD {
            return Ok(Ok(Change::None));
        }
        set_parameter(edit, m, "AlphaThreshold", &[THRESHOLD])?;
        return Ok(Ok(Change::Lowered(current)));
    }
    let layout = buffer_layout(edit, m)?;
    if v.layout != layout {
        return Ok(Err(format!("{twin} has another parameter layout than {id}")));
    }
    set_material_variant(edit, m, &twin)?;
    set_parameter(edit, m, "AlphaThreshold", &[current.min(THRESHOLD)])?;
    Ok(Ok(Change::Switched(twin)))
}

/// Gives segments `segs` blended copies of their materials (one copy per
/// material) and returns a report line.
pub fn blend_segments(edit: &mut ClusterEdit, segs: &[u32], variants: &Variants) -> Result<String> {
    if segs.is_empty() {
        return Ok(String::new());
    }
    let mut by_material: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &s in segs {
        by_material.entry(edit.segment_material(s)?).or_default().push(s);
    }
    let mut out = String::new();
    for (m, ss) in by_material {
        // A copy, else (meshes without their own material set: the rigid
        // objects' meshes) the material itself when only these segments
        // use it.
        // (add_material has changed the cluster when it fails late)
        let backup = edit.clone();
        let copy = match edit.add_material(m, ss[0]) {
            Ok((copy, index)) => {
                for &s in &ss {
                    edit.set_segment_material(s, index)?;
                }
                copy
            }
            Err(e) => {
                *edit = backup;
                let n = edit.list_of("PMeshSegment").map_or(0, |l| edit.lists[l].count());
                let shared = (0..n).any(|s| !ss.contains(&s) && edit.segment_material(s).ok() == Some(m));
                if shared {
                    out += &format!("  see-through texels: segments {ss:?} left as they were: {e}\n");
                    continue;
                }
                m
            }
        };
        match blend(edit, copy, variants, false)? {
            Ok(c) => {
                let what = match c {
                    Change::Switched(id) => id.rsplit('#').next().unwrap_or(&id).to_owned(),
                    Change::Lowered(t) => format!("see-through already, threshold was {t}"),
                    Change::None => "see-through already".into(),
                };
                out += &format!("  see-through texels: segments {ss:?} blended (material {copy}, {what})\n");
            }
            Err(why) => out += &format!("  see-through texels: segments {ss:?} left as they were: {why}\n"),
        }
    }
    Ok(out)
}

/// The asset-list entries of every effect variant the cluster's materials
/// use. The character's `.ahwin32` loads the variants (`Ch_LoadHdRenderData`
/// reads its D3D11 file list), and a material whose variant was not loaded
/// is not drawn: a blended twin the model did not use before must be added
/// (`ffx_phyre::assets::add_files` skips the ones listed already).
pub fn list_entries(edit: &ClusterEdit) -> Result<Vec<String>> {
    let ids: BTreeSet<String> = material_variants(edit)?.into_iter().collect();
    Ok(ids.iter().map(|id| ffx_phyre::assets::texture_entry(id)).collect())
}

/// What [`blend_in_place`] did.
pub struct Blended {
    /// Materials switched to a blended variant.
    pub switched: usize,
    /// Materials that blended already, given the lower threshold.
    pub lowered: usize,
    pub text: String,
}

/// Blends the materials of segments `segs` in place (an HD model whose
/// pages were repainted: every segment of a material samples the same
/// page). Materials that blend already only get [`THRESHOLD`].
pub fn blend_in_place(edit: &mut ClusterEdit, segs: &[u32], variants: &Variants) -> Result<Blended> {
    let mut materials = BTreeSet::new();
    for &s in segs {
        materials.insert(edit.segment_material(s)?);
    }
    let mut out = Blended { switched: 0, lowered: 0, text: String::new() };
    for m in materials {
        match blend(edit, m, variants, true)? {
            Ok(Change::None) => {}
            Ok(Change::Lowered(t)) => {
                out.lowered += 1;
                out.text += &format!("material {m} see-through already, threshold {t} -> {THRESHOLD}; ");
            }
            Ok(Change::Switched(id)) => {
                out.switched += 1;
                out.text += &format!("material {m} see-through ({}); ", id.rsplit('#').next().unwrap_or(&id));
            }
            Err(why) => out.text += &format!("material {m} left: {why}; "),
        }
    }
    Ok(out)
}

/// Prints each segment's material, its variant's switches and its
/// `AlphaThreshold`.
pub fn debug(hd: &[u8], read: impl Fn(&str) -> Result<Option<Vec<u8>>>) -> Result<()> {
    let model = ffx_phyre::Model::parse(hd).context("HD model")?;
    let edit = ClusterEdit::parse(hd)?;
    let variants = material_variants(&edit)?;
    let mut cache: BTreeMap<String, Variant> = BTreeMap::new();
    for (s, seg) in model.segments.iter().enumerate() {
        let Some(m) = edit
            .segment_material(u32::try_from(s)?)
            .ok()
            .or_else(|| seg.material.and_then(|m| u32::try_from(m).ok()))
        else {
            println!("seg {s}: no material");
            continue;
        };
        let id = &variants[m as usize];
        if !cache.contains_key(id) {
            let v = read(&variant_path(id))?.map(|b| Variant::parse(&b)).transpose()?.unwrap_or_default();
            cache.insert(id.clone(), v);
        }
        let v = &cache[id];
        let sw: Vec<&str> = v.switches.iter().filter(|x| x.1 == "1").map(|x| x.0.as_str()).collect();
        let mat = seg.material.and_then(|i| model.materials.get(i));
        if std::env::var_os("LAYOUT").is_some() {
            let mine = buffer_layout(&edit, m)?;
            let theirs = read(&variant_path(id))?
                .map(|b| -> Result<_> { buffer_layout(&ClusterEdit::parse(&b)?, 0) })
                .transpose()?;
            println!("  layout {} defs, same as the variant's: {:?}", mine.len(), theirs.map(|t| t == mine));
        }
        println!(
            "seg {s}: {} tris, texture {}, material {m} {} alpha {:?} first pass {:?} switches {}",
            seg.indices.len() / 3,
            mat.and_then(|x| x.base_color()).map_or("-", |t| t.path.rsplit('/').next().unwrap_or("")),
            id.rsplit('#').next().unwrap_or(id),
            (mat.and_then(|x| x.parameter("AlphaThreshold")), mat.and_then(|x| x.parameter("clipPlane")), mat.and_then(|x| x.parameter("MaterialColour"))),
            v.passes.first(),
            sw.join(" ")
        );
    }
    Ok(())
}
