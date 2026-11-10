//! Character materials and texture alpha (`docs/mods/characters.md`, "Alpha").
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
//! fade out; the alpha test (`TEST_2`, `NOTEQUAL 0`) drops only alpha 0.
//! The PC draws the same texels with the HD material's variant, which may
//! be opaque (alpha ignored: solid quads) or alpha tested at 0.5 (hard,
//! ragged cut-outs). Materials that sample such texels get a twin (the
//! variant with the same switches and one switch changed), as [`Draw`]
//! says: the remodelled models' PS2 meshes the blended (`ALPHA_ENABLED`)
//! twin with the PS2's alpha test, like the PS2; the kept HD meshes the
//! blended twin too, at [`THRESHOLD`]. (The `UATFOPQ` twin, alpha to
//! coverage, is soft only with MSAA; the `VQ_LOW` preset has none, and then
//! it is a plain alpha test.)

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

/// The import index of every `PMaterial`'s effect variant.
pub fn material_variant_indices(edit: &ClusterEdit) -> Result<Vec<u16>> {
    let l = list(edit, "PMaterial")?;
    (0..edit.lists[l].count())
        .map(|m| {
            let (l, k) = variant_fixup(edit, m)?;
            let r = edit.lists[l].pointer_fixups[k].user_fixup().context("effect variant is not a user fixup")?;
            let [_, len, at] = edit.user_records[r as usize];
            anyhow::ensure!(len == 2, "effect variant fixup of {len} bytes");
            Ok(u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]))
        })
        .collect()
}

/// The effect variant import id of every `PMaterial`.
pub fn material_variants(edit: &ClusterEdit) -> Result<Vec<String>> {
    let ids = imports(edit)?;
    material_variant_indices(edit)?
        .into_iter()
        .map(|i| ids.get(usize::from(i)).cloned().with_context(|| format!("import {i}")))
        .collect()
}

/// Gives import `i` the id `id` (as long as its old one) in place.
fn rename_import(edit: &mut ClusterEdit, i: u32, id: &str) -> Result<()> {
    let l = list(edit, "PAssetReferenceImport")?;
    let field = offset_of(edit, l, "m_id")?;
    let old = array_string(edit, l, i, field).context("import without an id")?;
    ensure!(old.len() == id.len(), "import id {old} and {id} differ in length");
    let f = edit.lists[l]
        .array_fixups
        .iter()
        .find(|f| f.object() == i && field_offset(&edit.cluster, f.member()) == Some(field))
        .context("import without an id")?;
    let at = f.0[3] as usize;
    edit.lists[l].arrays[at..at + id.len()].copy_from_slice(id.as_bytes());
    Ok(())
}

/// What a segment needs of its material's variant: the switches but the
/// alpha and sidedness ones (the vertex streams follow them: cloth,
/// skinning, colours, texture animation) and the parameter layout.
fn variant_key<'a>(variants: &'a Variants, id: &str) -> Option<(BTreeSet<&'a str>, &'a [(String, u8, u32)])> {
    let v = variants.of(id)?;
    let mut on = v.on_set();
    on.retain(|s| !["ALPHA_ENABLED", "UATFOPQ", "DOUBLE_SIDED"].contains(s));
    Some((on, &v.layout))
}

/// Frees materials for [`spare_imports`]: each segment not in `drawn` (the
/// hidden HD segments, still drawn as one degenerate triangle) takes the
/// material of a drawn segment whose variant has its switches (cloth on
/// cloth: a hidden cloth segment's import renamed to the non-cloth 3BF9, and
/// a non-cloth one's to the cloth 9EE8, ended the game in the intro), so its
/// own material may end up used by no segment. Returns the materials no
/// segment uses.
pub fn free_hidden_materials(edit: &mut ClusterEdit, drawn: &BTreeSet<u32>, variants: &Variants) -> Result<BTreeSet<u32>> {
    let segs = list(edit, "PMeshSegment")?;
    let n = edit.lists[segs].count();
    let index_at = offset_of(edit, segs, "m_materialIndex")? as usize;
    let ids = material_variants(edit)?;
    let slot = |edit: &ClusterEdit, s: u32| {
        u32::from_le_bytes(edit.lists[segs].object(s)[index_at..index_at + 4].try_into().expect("4 bytes"))
    };
    for s in (0..n).filter(|s| !drawn.contains(s)) {
        let mine = edit.segment_material(s)?;
        let key = variant_key(variants, &ids[mine as usize]);
        if key.is_none() {
            continue;
        }
        let old = slot(edit, s);
        for &d in drawn {
            let theirs = edit.segment_material(d)?;
            if theirs == mine || variant_key(variants, &ids[theirs as usize]) != key {
                continue;
            }
            edit.set_segment_material(s, slot(edit, d))?;
            if edit.segment_material(s).ok() == Some(theirs) {
                break;
            }
            // another mesh's material set
            edit.set_segment_material(s, old)?;
        }
    }
    let used: BTreeSet<u32> = (0..n).filter_map(|s| edit.segment_material(s).ok()).collect();
    let materials = list(edit, "PMaterial")?;
    Ok((0..edit.lists[materials].count()).filter(|m| !used.contains(m)).collect())
}

/// The effect variant imports of `edit` that only the materials `spare`
/// (and their parameter buffers) refer to: imports whose id can change.
pub fn spare_imports(edit: &ClusterEdit, spare: &BTreeSet<u32>) -> Result<Vec<u32>> {
    let ids = imports(edit)?;
    let mut users: BTreeMap<u16, BTreeSet<u32>> = BTreeMap::new();
    let materials = list(edit, "PMaterial")?;
    for (li, l) in edit.lists.iter().enumerate() {
        let name = &edit.cluster.classes[l.class].name;
        for f in &l.pointer_fixups {
            let Some(r) = f.user_fixup() else { continue };
            let [_, len, at] = edit.user_records[r as usize];
            if len != 2 {
                continue;
            }
            let i = u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]);
            // a parameter buffer counts as its material's (one buffer per material)
            let who = if li == materials {
                f.object()
            } else if name == "PParameterBuffer" {
                continue;
            } else {
                u32::MAX
            };
            users.entry(i).or_default().insert(who);
        }
    }
    Ok((0..ids.len())
        .filter(|&i| ids[i].contains(".fx#"))
        .filter(|&i| users.get(&(i as u16)).is_some_and(|u| u.iter().all(|m| spare.contains(m))))
        .map(|i| i as u32)
        .collect())
}

/// Points material `m` at effect variant `id` (an import id). `Ok(Err(why))`
/// when the cluster has no import it can give the variant.
///
/// The game resolves a model's imports by id (`PAssetReferenceImport::
/// resolve` `0x0045e280`), and an import the converter appends to the
/// cluster is not found, though the variant is in the character's asset
/// list: "Failed to find asset %s of type PEffectVariant, using fallback"
/// (a hook on `Phyre::PhyrePrintf` in the boot intro, five times: the
/// blended twins new to Lulu, Tidus, Kimahri and Yuna). The fallback draws
/// nothing, so every segment of such a material was missing (Lulu's fur
/// collar and necklace, Tidus's pant lattice, Kimahri's belt trims). An
/// import the model has, renamed in place (all variant ids have one
/// length), is found. So a variant the model does not import takes over an
/// import: one of `spare` (only materials no segment uses refer to them,
/// [`spare_imports`]), else the material's own when no other material uses
/// it (the kept meshes, whose materials change in place).
pub fn set_material_variant(
    edit: &mut ClusterEdit,
    m: u32,
    id: &str,
    spare: &mut Vec<u32>,
) -> Result<std::result::Result<(), String>> {
    let (l, k) = variant_fixup(edit, m)?;
    let old = edit.lists[l].pointer_fixups[k].user_fixup().context("effect variant is not a user fixup")?;
    let index = match edit.import_index(id)? {
        Some(i) => i,
        None => {
            let [_, _, at] = edit.user_records[old as usize];
            let current = u32::from(u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]));
            let own = spare_imports(edit, &BTreeSet::from([m]))?.contains(&current).then_some(current);
            let pick = (!spare.is_empty()).then(|| spare.remove(0));
            let Some(i) = pick.or(own) else {
                return Ok(Err(format!("the model imports no {} and has no import to spare", id.rsplit('#').next().unwrap_or(id))));
            };
            if std::env::var_os("DM_DEBUG").is_some() {
                eprintln!("  material {m}: import {i} {} renamed to {id}", imports(edit)?[i as usize]);
            }
            rename_import(edit, i, id)?;
            i
        }
    };
    let index = u16::try_from(index).context("imports")?;
    let at = u32::try_from(edit.user_data.len())?;
    edit.user_data.extend_from_slice(&index.to_be_bytes());
    let kind = edit.user_records[old as usize][0];
    edit.user_records.push([kind, 2, at]);
    let record = u32::try_from(edit.user_records.len() - 1)?;
    edit.lists[l].pointer_fixups[k].0[6] = record;
    Ok(Ok(()))
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

    /// Every indexed variant by archive path (`materials-debug --variants`).
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Variant)> {
        self.by_path.iter()
    }

    /// The see-through twin of variant `id` (an import id): the same effect
    /// and switches with the switch of `draw` on and the other off. `id`
    /// itself when it has that switch, and for [`Draw::Coverage`] when it
    /// blends already.
    pub fn twin(&self, id: &str, draw: Draw) -> Option<(String, &Variant)> {
        let v = self.of(id)?;
        let (on, off) = match draw {
            Draw::Blend | Draw::BlendKept => ("ALPHA_ENABLED", "UATFOPQ"),
            Draw::Coverage => ("UATFOPQ", "ALPHA_ENABLED"),
        };
        if v.on(on) || (draw == Draw::Coverage && v.on(off)) {
            return Some((id.to_owned(), v));
        }
        let mut want = v.on_set();
        want.remove(off);
        want.insert(on);
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

/// How see-through texels are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    /// Blended by texture alpha like the PS2: the `ALPHA_ENABLED` twin
    /// (`Transparent` pass), `AlphaThreshold` [`PS2_ALPHA_TEST`]. The PS2
    /// meshes of the remodelled characters.
    Blend,
    /// The blended twin as [`Draw::Blend`], `AlphaThreshold` [`THRESHOLD`]:
    /// the kept HD meshes with PS2 pages (`chr.*`, job version 3). A whole
    /// HD material is switched there, feathers and fur included, and under
    /// 0.1 their faint fringe texels write depth over the feathers drawn
    /// after them (pale outlines on the Condor's wings).
    BlendKept,
    /// Alpha to coverage: the `UATFOPQ` twin in the opaque pass,
    /// `AlphaThreshold` at most [`THRESHOLD`]; HD materials that blend
    /// already keep blending. Soft only with MSAA: without it, a plain alpha
    /// test at the threshold. The kept HD meshes with PS2 pages until `chr.*`
    /// job version 3 (now [`Draw::BlendKept`]).
    Coverage,
}

/// `AlphaThreshold` of [`Draw::Blend`]: the PS2's alpha test for characters
/// (`TEST_2`: ATE on, ATST `NOTEQUAL`, AREF 0, from a GS register dump of
/// the intro), which drops only texels of alpha 0, here the GS's alpha step
/// (1 of 0x80).
pub const PS2_ALPHA_TEST: f32 = 1.0 / 128.0;

/// `AlphaThreshold` of the [`Draw::BlendKept`] materials (and at most of the
/// [`Draw::Coverage`] ones): texels under it are dropped. For blended materials, lower lets
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
    /// See-through already with the threshold wanted.
    None,
}

/// Makes material `m` draw texture alpha the way `draw` says: its variant's
/// twin and the threshold. Returns what changed, or why it could not.
pub fn blend(
    edit: &mut ClusterEdit,
    m: u32,
    variants: &Variants,
    draw: Draw,
    spare: &mut Vec<u32>,
) -> Result<std::result::Result<Change, String>> {
    // for before/after comparisons in the game
    if std::env::var_os("DM_NO_BLEND").is_some() {
        return Ok(Err("DM_NO_BLEND set".into()));
    }
    let id = material_variants(edit)?.get(m as usize).cloned().context("material")?;
    let Some((twin, v)) = variants.twin(&id, draw) else {
        return Ok(Err(format!("no see-through twin of {id}")));
    };
    let current = parameter(edit, m, "AlphaThreshold")?.first().copied().unwrap_or(0.0);
    let threshold = match draw {
        Draw::Blend => PS2_ALPHA_TEST,
        Draw::BlendKept => THRESHOLD,
        Draw::Coverage => current.min(THRESHOLD),
    };
    if twin == id {
        if current == threshold {
            return Ok(Ok(Change::None));
        }
        set_parameter(edit, m, "AlphaThreshold", &[threshold])?;
        return Ok(Ok(Change::Lowered(current)));
    }
    let layout = buffer_layout(edit, m)?;
    if v.layout != layout {
        return Ok(Err(format!("{twin} has another parameter layout than {id}")));
    }
    if let Err(why) = set_material_variant(edit, m, &twin, spare)? {
        return Ok(Err(why));
    }
    set_parameter(edit, m, "AlphaThreshold", &[threshold])?;
    Ok(Ok(Change::Switched(twin)))
}

/// Gives segments `segs` blended copies of their materials (one copy per
/// material) and returns a report line.
pub fn blend_segments(
    edit: &mut ClusterEdit,
    segs: &[u32],
    variants: &Variants,
    draw: Draw,
    spare: &mut Vec<u32>,
) -> Result<String> {
    if segs.is_empty() {
        return Ok(String::new());
    }
    let mut by_material: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &s in segs {
        by_material.entry(edit.segment_material(s)?).or_default().push(s);
    }
    // per material, the segments of one mesh at a time: a material copy
    // joins the material set of one mesh (models with several skinned
    // instances share materials across meshes)
    let mut groups: Vec<(u32, Vec<u32>)> = Vec::new();
    for (m, ss) in by_material {
        let mut rest = ss;
        while !rest.is_empty() {
            // on a scratch copy: which of them see the new material at its index
            let mut trial = edit.clone();
            let mine = match trial.add_material(m, rest[0]) {
                Ok((copy, index)) => {
                    let mut mine = vec![rest[0]];
                    for &s in &rest[1..] {
                        trial.set_segment_material(s, index)?;
                        if trial.segment_material(s).ok() == Some(copy) {
                            mine.push(s);
                        }
                    }
                    mine
                }
                Err(_) => rest.clone(),
            };
            rest.retain(|s| !mine.contains(s));
            groups.push((m, mine));
        }
    }
    let mut out = String::new();
    for (m, ss) in groups {
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
        match blend(edit, copy, variants, draw, spare)? {
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
/// use, for the character's `.ahwin32` (`Ch_LoadHdRenderData` reads its
/// D3D11 file list; the HD lists name the variants their model imports).
/// `texanim::insert_files` puts the ones not listed yet before the model.
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
        match blend(edit, m, variants, Draw::BlendKept, &mut Vec::new())? {
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
    if std::env::var_os("IMPORTS").is_some() {
        // who points at each import (user fixups of 2 bytes holding its index)
        let ids = imports(&edit)?;
        let mut users: BTreeMap<u16, Vec<String>> = BTreeMap::new();
        for l in &edit.lists {
            let name = &edit.cluster.classes[l.class].name;
            for f in &l.pointer_fixups {
                let Some(r) = f.user_fixup() else { continue };
                let [kind, len, at] = edit.user_records[r as usize];
                if len != 2 {
                    continue;
                }
                let i = u16::from_be_bytes([edit.user_data[at as usize], edit.user_data[at as usize + 1]]);
                users.entry(i).or_default().push(format!("{name}#{}(k{kind})", f.object()));
            }
        }
        for (i, id) in ids.iter().enumerate() {
            println!("import {i} {id}: {:?}", users.get(&(i as u16)));
        }
    }
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
        if std::env::var_os("PARAMS").is_some() {
            for (name, _, _) in buffer_layout(&edit, m)? {
                println!("  m{m} {name} = {:?}", parameter(&edit, m, &name).ok());
            }
        }
        println!(
            "seg {s}: {} tris, texture {}, material {m} {} alpha {:?} passes {:?} switches {}",
            seg.indices.len() / 3,
            mat.and_then(|x| x.base_color()).map_or("-", |t| t.path.rsplit('/').next().unwrap_or("")),
            id.rsplit('#').next().unwrap_or(id),
            (mat.and_then(|x| x.parameter("AlphaThreshold")), mat.and_then(|x| x.parameter("clipPlane")), mat.and_then(|x| x.parameter("MaterialColour"))),
            v.passes,
            sw.join(" ")
        );
    }
    Ok(())
}
