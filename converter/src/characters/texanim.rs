//! Character texture animation (eyes, mouths): the PS2 `.chr` patches and
//! animation entries, and what the PC needs to show them over our PS2
//! textures. Notes: `docs/models.md`, "Texture animation".
//!
//! The PC still runs the PS2 texanim logic (`FUN_0077f360`) and calls
//! `TextureAnimationManager_setAnimatedMaterialsChar` with group `G<entry>`,
//! animation `Animation<state>` and the frame index. That looks up the
//! record `(Animation<s>, G<e>)` of the character's
//! `textureanimationchar.ags.phyre`: its `m_subTextureIDs[frame]` picks a
//! texture (the n-th cluster of the asset list, which lists the animation
//! textures first) and its four floats a rectangle. The animated material
//! (named `Mat_CTA_G0G1…`) stacks the groups in layers 1-3 (1-6 with
//! `CHR_TEXTURE_ANIMATION_UV`), each drawn by the shader over a rectangle
//! of the base texture's UVs, replacing the colour inside it:
//! `t = (uv - T) / S`, drawn where `0.001 <= t <= 0.999`. Shader variants:
//! - `_GAME`: `T`, `S` per layer from the record (`AnimTranslate_GAME<n>`
//!   = floats 2, 3; `AnimScale_GAME<n>` = floats 0, 1; the files store the
//!   PS2 rectangle as `w/W, h/H, x/W, y/H`);
//! - `_MAYA`: `T`, `S` fixed in the material (`AnimTranslate_MAYA<n>`,
//!   `AnimScale_MAYA<n>`), the record's floats unused.
//!
//! UVs as the shader sees them have v up from the image's bottom row (the
//! textures are stored bottom row first).

use anyhow::{Context, Result, bail, ensure};
use ffx_phyre::edit::ClusterEdit;

use crate::formats::phyretex::Rgba;

fn u16_(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).context("texanim: past the end")?.try_into()?))
}
fn i16_(b: &[u8], o: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(b.get(o..o + 2).context("texanim: past the end")?.try_into()?))
}
fn u32_(b: &[u8], o: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("texanim: past the end")?.try_into()?) as usize)
}

/// An 8-bit sub-image the PS2 blits into the character's image.
#[derive(Debug, Clone)]
pub struct Patch {
    pub width: u32,
    pub height: u32,
    pub texels: Vec<u8>,
}

/// One step of a frame list: how long, which patch.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub duration: i16,
    pub patch: usize,
}

/// One animation entry (group `G<index>`): where its patches go and, per
/// state (`Animation<s>`), the frames.
#[derive(Debug, Clone)]
pub struct Entry {
    pub group: usize,
    /// Per-entry type byte (0 blinks at random, 4 plays the list, ...).
    pub mode: u8,
    pub x: u32,
    pub y: u32,
    /// The image the patches go into (entry `+0xc`).
    pub image: usize,
    pub states: Vec<(usize, Vec<Frame>)>,
}

impl Entry {
    /// The rectangle every frame of `state` covers: the entry's corner and
    /// the largest patch (frames of one list may differ in size).
    pub fn rect(&self, patches: &[Patch], state: usize) -> (u32, u32, u32, u32) {
        let frames = self.states.iter().find(|s| s.0 == state).map_or(&[][..], |s| &s.1[..]);
        let w = frames.iter().filter_map(|f| patches.get(f.patch)).map(|p| p.width).max().unwrap_or(0);
        let h = frames.iter().filter_map(|f| patches.get(f.patch)).map(|p| p.height).max().unwrap_or(0);
        (self.x, self.y, w, h)
    }
}

#[derive(Debug, Clone)]
pub struct TexAnim {
    pub patches: Vec<Patch>,
    pub entries: Vec<Entry>,
}

impl TexAnim {
    /// The texture-animation block of a `.chr` (texture block `+0x20`), if
    /// it has one in the new format (kind 2, version 1; the only kind the
    /// survey of the converted models found).
    pub fn parse(b: &[u8]) -> Result<Option<Self>> {
        let n = u32_(b, 4)?;
        if n < 2 {
            return Ok(None);
        }
        let t = u32_(b, 0x18)?;
        if t == 0 || u16_(b, t + 0x10)? < 0x1126 || u32_(b, t + 0x20)? == 0 {
            return Ok(None);
        }
        let a = t + u32_(b, t + 0x20)?;
        let (kind, version) = (b[a], *b.get(a + 2).context("texanim header")?);
        if kind != 2 || version != 1 {
            bail!("texanim kind {kind} version {version} not supported");
        }
        let (pt, et) = (a + u32_(b, a + 4)?, a + u32_(b, a + 8)?);
        ensure!(et >= pt && (et - pt) % 12 == 0, "texanim: patch table");
        let mut patches = Vec::new();
        for k in 0..(et - pt) / 12 {
            let o = pt + 12 * k;
            let (w, h) = (u32::from(u16_(b, o + 4)?), u32::from(u16_(b, o + 6)?));
            // a 32-byte upload header, then the texels
            let at = a + u32_(b, o + 8)? + 0x20;
            let texels = b.get(at..at + (w * h) as usize).context("texanim: patch past the end")?.to_vec();
            patches.push(Patch { width: w, height: h, texels });
        }
        let types = b.get(a + 0xc..a + 0x2c).context("texanim types")?;
        let mut entries = Vec::new();
        for (e, &mode) in types.iter().enumerate() {
            if mode == 0xff {
                continue;
            }
            let o = et + 0x110 * e;
            let mut states = Vec::new();
            for s in 0..32 {
                let list = a + u32_(b, o + 0x10 + 8 * s)?;
                let count = i16_(b, o + 0x14 + 8 * s)?;
                if count <= 0 {
                    continue;
                }
                let frames = (0..count as usize)
                    .map(|f| {
                        Ok(Frame {
                            duration: i16_(b, list + 4 * f)?,
                            patch: usize::from(*b.get(list + 4 * f + 2).context("texanim frame")?),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                ensure!(frames.iter().all(|f| f.patch < patches.len()), "texanim: frame names a missing patch");
                states.push((s, frames));
            }
            entries.push(Entry {
                group: e,
                mode,
                x: u32::from(u16_(b, o + 8)?),
                y: u32::from(u16_(b, o + 10)?),
                image: usize::from(u16_(b, o + 12)?),
                states,
            });
        }
        Ok(Some(Self { patches, entries }))
    }
}

/// An area of an image, in texels from the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn union(self, o: Rect) -> Rect {
        let (x0, y0) = (self.x.min(o.x), self.y.min(o.y));
        let (x1, y1) = ((self.x + self.w).max(o.x + o.w), (self.y + self.h).max(o.y + o.h));
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }

    /// `T`, `S` of the shader for this rectangle of a page placed at
    /// (`px`, `py`) in an atlas `aw` x `ah`: v up from the bottom row.
    pub fn shader(self, px: u32, py: u32, aw: u32, ah: u32) -> ([f32; 2], [f32; 2]) {
        let (aw, ah) = (f64::from(aw), f64::from(ah));
        let t = [f64::from(px + self.x) / aw, 1.0 - f64::from(py + self.y + self.h) / ah];
        let s = [f64::from(self.w) / aw, f64::from(self.h) / ah];
        (t.map(|v| v as f32), s.map(|v| v as f32))
    }
}

/// `rect` of `base` with `patch` (through `clut`) put at (`dx`, `dy`) in it.
pub fn compose(base: &Rgba, rect: Rect, patch: &Patch, clut: &[[u8; 4]; 256], dx: u32, dy: u32) -> Rgba {
    let mut out = Rgba::new(rect.w, rect.h);
    for y in 0..rect.h {
        for x in 0..rect.w {
            let (bx, by) = ((rect.x + x).min(base.width - 1), (rect.y + y).min(base.height - 1));
            let mut c = base.get(bx, by);
            if (dx..dx + patch.width).contains(&x) && (dy..dy + patch.height).contains(&y) {
                c = clut[usize::from(patch.texels[((y - dy) * patch.width + (x - dx)) as usize])];
            }
            out.put(x, y, c);
        }
    }
    out
}

/// One record of a `textureanimationchar.ags.phyre`.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// `m_name`: `Animation<state>`.
    pub animation: String,
    /// `m_groupName`: `G<entry>`.
    pub group: String,
    pub interval: f32,
    pub ids: Vec<u32>,
    /// `mUStart, mVStart, mUEnd, mVEnd`: `S.x, S.y, T.x, T.y`.
    pub rect: [f32; 4],
}

const AGS_CLASS: &str = "PSpriteAnimationInfoChar";

fn field(edit: &ClusterEdit, word: u32) -> Option<u32> {
    if word & 0x8000_0000 != 0 {
        Some(word & 0x7fff_ffff)
    } else {
        edit.cluster.member_by_index(word).map(|(_, m)| m.offset)
    }
}

fn string_at(arrays: &[u8], at: usize) -> String {
    let tail = arrays.get(at..).unwrap_or_default();
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    String::from_utf8_lossy(&tail[..end]).into_owned()
}

/// Offsets of the record's members.
struct AgsLayout {
    list: usize,
    name: u32,
    ids: u32,
    group: u32,
    interval: usize,
    floats: usize,
}

fn ags_layout(edit: &ClusterEdit) -> Result<AgsLayout> {
    let list = edit.list_of(AGS_CLASS).context("no PSpriteAnimationInfoChar list")?;
    let off = |m: &str| edit.cluster.member_offset(AGS_CLASS, m).with_context(|| format!("no {m}"));
    Ok(AgsLayout {
        list,
        name: off("m_name")?,
        ids: off("m_subTextureIDs")?,
        group: off("m_groupName")?,
        interval: off("m_timeInterval")? as usize,
        floats: off("mUStart")? as usize,
    })
}

/// The records of an `.ags`.
pub fn read_ags(bytes: &[u8]) -> Result<Vec<Record>> {
    let edit = ClusterEdit::parse(bytes).context("ags")?;
    let lay = ags_layout(&edit)?;
    let l = &edit.lists[lay.list];
    let mut out = Vec::new();
    for i in 0..l.count() {
        let obj = l.object(i);
        let f32_at = |o: usize| f32::from_le_bytes(obj[o..o + 4].try_into().expect("4 bytes"));
        let mut rec = Record {
            animation: String::new(),
            group: String::new(),
            interval: f32_at(lay.interval),
            ids: Vec::new(),
            rect: [0, 1, 2, 3].map(|k| f32_at(lay.floats + 4 * k)),
        };
        let count = u32::from_le_bytes(obj[lay.ids as usize..lay.ids as usize + 4].try_into()?) as usize;
        for f in l.array_fixups.iter().filter(|f| f.object() == i) {
            let at = f.0[3] as usize;
            match field(&edit, f.member()) {
                Some(o) if o == lay.name => rec.animation = string_at(&l.arrays, at),
                Some(o) if o == lay.group => rec.group = string_at(&l.arrays, at),
                Some(o) if o == lay.ids || o == lay.ids + 4 => {
                    rec.ids = (0..count)
                        .map(|k| {
                            l.arrays
                                .get(at + 4 * k..at + 4 * k + 4)
                                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
                                .context("ids past the end")
                        })
                        .collect::<Result<_>>()?;
                }
                _ => {}
            }
        }
        out.push(rec);
    }
    Ok(out)
}

/// An `.ags` with `records`, made from `template` (an HD one): the record
/// list is rebuilt from its first object and that object's array fixups.
pub fn write_ags(template: &[u8], records: &[Record]) -> Result<Vec<u8>> {
    let mut edit = ClusterEdit::parse(template).context("ags template")?;
    let lay = ags_layout(&edit)?;
    ensure!(edit.lists.len() == 1, "ags with {} lists", edit.lists.len());
    let l = &edit.lists[lay.list];
    ensure!(l.count() > 0 && l.pointer_fixups.is_empty() && l.pointer_array_fixups.is_empty(), "unexpected ags");
    let proto = l.object(0).to_vec();
    let fixups: Vec<_> = l.array_fixups.iter().filter(|f| f.object() == 0).copied().collect();
    let kind = |f: &ffx_phyre::fixup::ArrayFixup| field(&edit, f.member());
    let (mut objects, mut arrays, mut out_fixups) = (Vec::new(), Vec::new(), Vec::new());
    let push = |arrays: &mut Vec<u8>, data: &[u8]| {
        arrays.resize(arrays.len().next_multiple_of(4), 0);
        let at = arrays.len();
        arrays.extend_from_slice(data);
        at as u32
    };
    for (i, r) in records.iter().enumerate() {
        let mut obj = proto.clone();
        obj[lay.interval..lay.interval + 4].copy_from_slice(&r.interval.to_le_bytes());
        for (k, v) in r.rect.iter().enumerate() {
            obj[lay.floats + 4 * k..lay.floats + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
        }
        let ids_at = lay.ids as usize;
        obj[ids_at..ids_at + 4].copy_from_slice(&(r.ids.len() as u32).to_le_bytes());
        objects.extend_from_slice(&obj);
        for f in &fixups {
            let mut g = *f;
            g.0[1] = i as u32;
            match kind(f) {
                Some(o) if o == lay.name => {
                    g.0[3] = push(&mut arrays, format!("{}\0", r.animation).as_bytes());
                }
                Some(o) if o == lay.group => {
                    g.0[3] = push(&mut arrays, format!("{}\0", r.group).as_bytes());
                }
                Some(o) if o == lay.ids || o == lay.ids + 4 => {
                    let data: Vec<u8> = r.ids.iter().flat_map(|v| v.to_le_bytes()).collect();
                    g.0[3] = push(&mut arrays, &data);
                    if g.0[2] != 0 {
                        g.0[2] = r.ids.len() as u32;
                    }
                }
                _ => bail!("ags record has an array field this writer does not know"),
            }
            out_fixups.push(g);
        }
    }
    let l = &mut edit.lists[lay.list];
    l.objects = objects;
    l.arrays = arrays;
    l.array_fixups = out_fixups;
    let bytes = edit.write().context("writing the ags")?;
    let back = read_ags(&bytes).context("re-reading the ags")?;
    ensure!(back == records, "the written ags reads back differently");
    Ok(bytes)
}

/// The `PMaterial` names of a model cluster, by `PMaterial` index.
pub fn material_names(edit: &ClusterEdit) -> Result<Vec<String>> {
    let list = edit.list_of("PMaterial").context("no PMaterial list")?;
    let name = edit.cluster.member_offset("PMaterial", "m_name");
    let l = &edit.lists[list];
    Ok((0..l.count())
        .map(|i| {
            l.array_fixups
                .iter()
                .find(|f| f.object() == i && (name.is_none() || field(edit, f.member()) == name))
                .map(|f| string_at(&l.arrays, f.0[3] as usize))
                .unwrap_or_default()
        })
        .collect())
}

/// Sets the value parameter `parameter` of `PMaterial` `material` (in its
/// parameter buffer). Returns false when the material has no such value.
pub fn set_value(edit: &mut ClusterEdit, material: u32, parameter: &str, values: &[f32]) -> Result<bool> {
    let materials = edit.list_of("PMaterial").context("no PMaterial list")?;
    let pb = edit.cluster.member_offset("PMaterial", "m_parameterBuffer").context("no m_parameterBuffer")?;
    let Some(buf) = edit.lists[materials]
        .pointer_fixups
        .iter()
        .find(|f| f.object() == material && field(edit, f.member()) == Some(pb))
        .copied()
    else {
        return Ok(false);
    };
    let (bl, bo) = (buf.target_list() as usize, buf.target());
    let defs_field = edit
        .cluster
        .member_offset("PParameterBuffer", "m_tweakableShaderParameterDefinitions")
        .context("no definitions field")?;
    let Some(defs) = edit.lists[bl]
        .pointer_fixups
        .iter()
        .find(|f| f.object() == bo && field(edit, f.member()) == Some(defs_field + 4))
        .copied()
    else {
        return Ok(false);
    };
    let dl = defs.target_list() as usize;
    let c = "PShaderParameterDefinition";
    let o_name = edit.cluster.member_offset(c, "m_name").context("no m_name")?;
    let o_type = edit.cluster.member_offset(c, "m_parameterType").context("no m_parameterType")? as usize;
    let o_loc = edit.cluster.member_offset(c, "m_bufferLoc").context("no m_bufferLoc")? as usize;
    for d in defs.target()..defs.target() + defs.count() {
        let def = edit.lists[dl].object(d);
        if def[o_type] != 0x40 {
            continue;
        }
        let loc = u32::from_le_bytes(def[o_loc..o_loc + 4].try_into()?) as usize;
        let named = edit.lists[dl]
            .array_fixups
            .iter()
            .any(|f| f.object() == d && field(edit, f.member()) == Some(o_name) && string_at(&edit.lists[dl].arrays, f.0[3] as usize) == parameter);
        if !named {
            continue;
        }
        let (offset, size) = (loc & 0xffff, loc >> 16);
        ensure!(size == 4 * values.len(), "{parameter} holds {size} bytes, not {}", 4 * values.len());
        let obj = edit.lists[bl].object_mut(bo);
        for (k, v) in values.iter().enumerate() {
            obj[offset + 4 * k..offset + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
        }
        return Ok(true);
    }
    Ok(false)
}

/// The D3D11 section of an asset list (`.ahwin32`) with `files` put right
/// before the model (`.dae.phyre`), the ones listed already left out; the
/// count and the named indices of the files after them move up.
///
/// The game takes a character's animation textures by position, the n-th
/// file of the list for subtexture n (`PhyreScene_doOnCharacterAllAssetsLoaded`),
/// so nothing may go before `textureAnimationChar`:
/// `ffx_phyre::assets::add_files` puts its files first, and the two shader
/// variants it put before Lulu's list made them her animation textures 0
/// and 1. (The game loads the whole list before it resolves the model's
/// imports, `PApplication_FixupClusters`, so the place after
/// `textureAnimationChar` does not matter otherwise; `DM_LIST_AT=end`
/// appends instead.)
pub fn insert_files(list: &[u8], files: &[String]) -> Result<Vec<u8>> {
    const SECTION: &[u8] = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11";
    let start = list.windows(SECTION.len()).position(|w| w == SECTION).context("no D3D11 section")?;
    let end = list[start..].windows(6).position(|w| w == b"#endif").map_or(list.len(), |e| start + e);
    let section = String::from_utf8_lossy(&list[start..end]).into_owned();
    let new: Vec<String> = files.iter().filter(|f| !section.contains(&format!("\"{f}\""))).cloned().collect();
    if new.is_empty() {
        return Ok(list.to_vec());
    }
    let eol = if section.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = section.split(eol).collect();
    let open = lines.iter().position(|l| l.trim_end().ends_with("g_fileNames[] = {")).context("no file list")?;
    let close = open + lines[open..].iter().position(|l| l.trim() == "};").context("no file list end")?;
    let old: Vec<String> = lines[open + 1..close]
        .iter()
        .map(|l| l.trim().trim_start_matches(',').trim_matches('"').to_owned())
        .collect();
    let lower: Vec<String> = old.iter().map(|f| f.to_ascii_lowercase()).collect();
    let after_anim = lower.iter().position(|f| f.ends_with("/textureanimationchar.ags.phyre")).map_or(0, |k| k + 1);
    let at = match std::env::var("DM_LIST_AT").as_deref() {
        Ok("end") => old.len(),
        _ => lower.iter().position(|f| f.ends_with(".dae.phyre")).map_or(old.len(), |k| k.max(after_anim)),
    };
    let mut names = old.clone();
    names.splice(at..at, new.iter().cloned());
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + new.len());
    for (i, line) in lines.iter().enumerate() {
        if i > open && i < close {
            continue;
        }
        if i == open {
            out.push((*line).to_owned());
            for (k, f) in names.iter().enumerate() {
                out.push(format!("{}\"{f}\"", if k == 0 { "" } else { "," }));
            }
            continue;
        }
        let text = line.trim_end();
        if let Some(rest) = text.strip_prefix("const Phyre::PUInt32 ")
            && let Some((name, value)) = rest.split_once(" = ")
            && let Ok(v) = value.trim_end_matches(';').parse::<usize>()
        {
            let v = if name.trim() == "g_fileNamesCount" || v >= at { v + new.len() } else { v };
            out.push(format!("const Phyre::PUInt32 {name} = {v};"));
            continue;
        }
        out.push((*line).to_owned());
    }
    let mut bytes = list[..start].to_vec();
    bytes.extend(out.join(eol).into_bytes());
    bytes.extend_from_slice(&list[end..]);
    Ok(bytes)
}

/// The D3D11 section of an asset list (`.ahwin32`) with the files before
/// `textureAnimationChar` (the animation textures) replaced by `anim`, and
/// `extra` put right after `textureAnimationChar` (outside the range the
/// game reads animation textures from). Named indices follow their files;
/// one unnamed index per animation texture, as the HD lists have.
pub fn rewrite_list(list: &[u8], anim: &[String], extra: &[String]) -> Result<Vec<u8>> {
    const SECTION: &[u8] = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11";
    let start = list.windows(SECTION.len()).position(|w| w == SECTION).context("no D3D11 section")?;
    let end = list[start..].windows(6).position(|w| w == b"#endif").map_or(list.len(), |e| start + e);
    let section = String::from_utf8_lossy(&list[start..end]).into_owned();
    let eol = if section.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = section.split(eol).collect();
    // the file list
    let open = lines.iter().position(|l| l.trim_end().ends_with("g_fileNames[] = {")).context("no file list")?;
    let close = open + lines[open..].iter().position(|l| l.trim() == "};").context("no file list end")?;
    let files: Vec<String> = lines[open + 1..close]
        .iter()
        .map(|l| l.trim().trim_start_matches(',').trim_matches('"').to_owned())
        .collect();
    let ags = files
        .iter()
        .position(|f| f.to_ascii_lowercase().ends_with("/textureanimationchar.ags.phyre"))
        .context("no textureAnimationChar in the list")?;
    let mut new_files: Vec<String> = anim.to_vec();
    new_files.push(files[ags].clone());
    new_files.extend(extra.iter().filter(|e| !files.contains(e)).cloned());
    new_files.extend(files[ags + 1..].iter().cloned());
    let index_of = |old: usize| -> Option<usize> { new_files.iter().position(|f| *f == files[old]) };
    let mut out: Vec<String> = Vec::new();
    let mut unnamed_done = false;
    for (i, line) in lines.iter().enumerate() {
        if i > open && i < close {
            continue;
        }
        if i == open {
            out.push((*line).to_owned());
            for (k, f) in new_files.iter().enumerate() {
                out.push(format!("{}\"{f}\"", if k == 0 { "" } else { "," }));
            }
            continue;
        }
        let text = line.trim_end();
        if let Some(rest) = text.strip_prefix("const Phyre::PUInt32 ")
            && let Some((name, value)) = rest.split_once(" = ")
            && let Ok(v) = value.trim_end_matches(';').parse::<usize>()
        {
            let name = name.trim();
            if name == "g_fileNamesCount" {
                out.push(format!("const Phyre::PUInt32 g_fileNamesCount = {};", new_files.len()));
            } else if name.is_empty() {
                // the animation textures' unnamed indices: written once, anew
                if !unnamed_done {
                    for k in 0..anim.len() {
                        out.push(format!("const Phyre::PUInt32  = {k};"));
                    }
                    unnamed_done = true;
                }
            } else {
                let n = index_of(v).with_context(|| format!("{name} names a file that is gone"))?;
                out.push(format!("const Phyre::PUInt32 {name} = {n};"));
            }
            continue;
        }
        out.push((*line).to_owned());
    }
    let mut bytes = list[..start].to_vec();
    bytes.extend(out.join(eol).into_bytes());
    bytes.extend_from_slice(&list[end..]);
    Ok(bytes)
}

/// The archive path of a texture import id
/// (`PS3Data/chr/pc/c001/tex/c001_anim_n0_p0.dds` ->
/// `ffx_data/gamedata/ps3data/chr/pc/c001/tex/d3d11/c001_anim_n0_p0.dds.phyre`).
pub fn archive_path(id: &str) -> String {
    let lower = id.to_ascii_lowercase();
    let (dir, file) = lower.rsplit_once('/').unwrap_or(("", &lower));
    format!("ffx_data/gamedata/{dir}/d3d11/{file}.phyre")
}

/// The archive paths of the animation textures an asset list names: its
/// D3D11 files before `textureAnimationChar`, in order (texture id = place).
pub fn anim_files(list: &[u8]) -> Result<Vec<String>> {
    let text = String::from_utf8_lossy(list);
    let start = text.find("#ifdef PHYRE_RENDERING_PLATFORM_D3D11").context("no D3D11 section")?;
    let section = &text[start..];
    let open = section.find("g_fileNames[] = {").context("no file list")?;
    let mut out = Vec::new();
    for line in section[open..].lines().skip(1) {
        let line = line.trim();
        if line.starts_with("};") {
            break;
        }
        let f = line.trim_start_matches(',').trim_matches('"').to_ascii_lowercase();
        if f.ends_with("/textureanimationchar.ags.phyre") {
            return Ok(out);
        }
        out.push(f.trim_start_matches('/').to_owned());
    }
    Ok(Vec::new())
}

/// The name tag of animation texture `k`: `n<k % 10>_p<k / 10>`, as long as
/// the HD's `n0_p0` for every `k` below 100 (texture ids are patched in
/// place, so they keep their length).
pub fn anim_tag(k: usize) -> String {
    format!("n{}_p{}", k % 10, k / 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_rewrite_moves_named_indices() {
        let list = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11 // 32 Bit\r\n#ifndef PHYRE_64BIT_PTR\r\n\
const Phyre::PUInt32  = 0;\r\nconst Phyre::PUInt32  = 1;\r\n\
const Phyre::PUInt32 textureAnimationChar = 2;\r\nconst Phyre::PUInt32 c001 = 3;\r\n\
const Phyre::PUInt32 g_fileNamesCount = 5;\r\nconst Phyre::PChar *g_fileNamesPrefix = \"/FFX_Data/\";\r\n\
const Phyre::PChar *g_fileNames[] = {\r\n\"/a_n0.dds.phyre\"\r\n,\"/a_n1.dds.phyre\"\r\n\
,\"/x/textureAnimationChar.ags.phyre\"\r\n,\"/c001.dae.phyre\"\r\n,\"/c001.dds.phyre\"\r\n};\r\n\
#endif //! PHYRE_64BIT_PTR\r\n#endif\r\n";
        let anim = ["/b_0.dds.phyre".to_owned(), "/b_1.dds.phyre".to_owned(), "/b_2.dds.phyre".to_owned()];
        let out = String::from_utf8(rewrite_list(list, &anim, &["/p001.dds.phyre".to_owned()]).unwrap()).unwrap();
        assert!(out.contains("const Phyre::PUInt32  = 2;\r\nconst Phyre::PUInt32 textureAnimationChar = 3;"), "{out}");
        assert!(out.contains("c001 = 5;") && out.contains("g_fileNamesCount = 7;"), "{out}");
        assert!(
            out.contains("{\r\n\"/b_0.dds.phyre\"\r\n,\"/b_1.dds.phyre\"\r\n,\"/b_2.dds.phyre\"\r\n,\"/x/textureAnimationChar.ags.phyre\"\r\n,\"/p001.dds.phyre\"\r\n,\"/c001.dae.phyre\""),
            "{out}"
        );
        assert!(out.ends_with("#endif //! PHYRE_64BIT_PTR\r\n#endif\r\n"), "{out}");
    }

    #[test]
    fn inserted_files_go_before_the_model() {
        let list = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11 // 32 Bit\r\n#ifndef PHYRE_64BIT_PTR\r\n\
const Phyre::PUInt32  = 0;\r\nconst Phyre::PUInt32 textureAnimationChar = 1;\r\nconst Phyre::PUInt32 c006 = 2;\r\n\
const Phyre::PUInt32 g_fileNamesCount = 4;\r\nconst Phyre::PChar *g_fileNamesPrefix = \"/FFX_Data/\";\r\n\
const Phyre::PChar *g_fileNames[] = {\r\n\"/a_n0.dds.phyre\"\r\n\
,\"/x/textureAnimationChar.ags.phyre\"\r\n,\"/c006.dae.phyre\"\r\n,\"/s#F2E8.phyre\"\r\n};\r\n\
#endif //! PHYRE_64BIT_PTR\r\n#endif\r\n";
        let add = ["/s#3BF9.phyre".to_owned(), "/s#F2E8.phyre".to_owned()];
        let out = String::from_utf8(insert_files(list, &add).unwrap()).unwrap();
        assert!(out.contains("const Phyre::PUInt32  = 0;\r\nconst Phyre::PUInt32 textureAnimationChar = 1;\r\nconst Phyre::PUInt32 c006 = 3;\r\nconst Phyre::PUInt32 g_fileNamesCount = 5;"), "{out}");
        assert!(
            out.contains("{\r\n\"/a_n0.dds.phyre\"\r\n,\"/x/textureAnimationChar.ags.phyre\"\r\n,\"/s#3BF9.phyre\"\r\n,\"/c006.dae.phyre\"\r\n,\"/s#F2E8.phyre\"\r\n};\r\n#endif //! PHYRE_64BIT_PTR\r\n#endif\r\n"),
            "{out}"
        );
        // listed already: unchanged
        assert_eq!(insert_files(out.as_bytes(), &add).unwrap(), out.as_bytes());
    }

    #[test]
    fn shader_rect_counts_v_from_the_bottom() {
        let r = Rect { x: 86, y: 90, w: 24, h: 16 };
        let (t, s) = r.shader(2, 2, 516, 260);
        assert!((t[0] - 88.0 / 516.0).abs() < 1e-6);
        assert!((t[1] - (1.0 - 108.0 / 260.0)).abs() < 1e-6);
        assert!((s[0] - 24.0 / 516.0).abs() < 1e-6 && (s[1] - 16.0 / 260.0).abs() < 1e-6);
    }

    #[test]
    fn tags_keep_their_length() {
        assert_eq!(anim_tag(0), "n0_p0");
        assert_eq!(anim_tag(13), "n3_p1");
        assert!((0..100).all(|k| anim_tag(k).len() == 5));
    }
}
