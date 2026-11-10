//! `render.lighting = "ps2"`, the shader half (the mod sets the light
//! levels; `docs/render.md`): the character pixel shaders light with the
//! PS2's falloff.
//!
//! The PS2 lit characters per vertex on VU1 (`SLPM_675.13` `.vutext`,
//! program 1 at VU address `0xcd`..`0xf4`): `max(0, L x N)` for the three
//! directional lights, times the light colour matrix, plus the ambient
//! colour. The PC's character pixel shaders (`phyrechrlitshader.fx#*`, the
//! variants that read `Light0`..`Light2`) add each light as
//! `m_colorIntensity x (dot(m_direction, N) x 0.5 + 0.5)^2`, a wrapped
//! falloff that still lights the sides and back the PS2 left at the
//! ambient. The compiler emits that per light as
//!
//! ```text
//! dp3 rA.c, cb0[Light_i.m_direction].xyz, rN.xyz
//! mad rA.c, rA.c, l(0.5), l(0.5)
//! mul rA.c, rA.c, rA.c
//! ```
//!
//! which this job rewrites in place to
//!
//! ```text
//! mad_sat rA.c, rA.c, l(1.0), l(0.0)   (saturate(N.L) = max(0, N.L), N and L unit vectors)
//! max     rA.c, rA.c, rA.c             (a copy)
//! ```
//!
//! Same instruction lengths, so only the container checksum changes. A
//! shader whose pattern count differs from its used lights is left as it
//! is (and counted in the summary).

use anyhow::Result;
use rayon::prelude::*;

use crate::config::Config;
use crate::formats::dxbc;
use crate::jobs::{Ctx, Job, Output};

pub const SHADERS: &str = "ffx_data/gamedata/ps3data/shaders/d3d11/";
/// The character shader family (220 files of the 793).
const CHR_LIT: &str = "phyrechrlitshader.fx";

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(LightingJob)]
}

struct LightingJob;

impl Job for LightingJob {
    fn name(&self) -> &'static str {
        "render.lighting"
    }
    fn settings(&self) -> &'static [&'static str] {
        &["render.lighting"]
    }
    /// 2: N.L with the interpolated normal, not renormalised (Gouraud).
    fn version(&self) -> u32 {
        2
    }
    fn enabled(&self, config: &Config) -> bool {
        config.render.lighting.ps2()
    }
    /// Made from the PC's own shaders.
    fn needs_disc(&self) -> bool {
        false
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        let files: Vec<_> = ctx
            .pc
            .entries()
            .iter()
            .filter(|e| e.path.strip_prefix(SHADERS).is_some_and(|f| f.starts_with(CHR_LIT)))
            .collect();
        let results: Vec<_> = files
            .par_iter()
            .map(|e| -> Result<_> {
                let mut bytes = ctx.pc.read(e)?;
                let s = patch_file(&mut bytes)?;
                Ok((e.path.clone(), bytes, s))
            })
            .collect::<Result<_>>()?;
        let mut total = Stats::default();
        let mut n = 0;
        for (path, bytes, s) in results {
            if s.patched > 0 && out.put(&path, &bytes)? {
                n += 1;
            }
            ctx.note(format!("{path}: {s:?}"));
            total.add(&s);
        }
        Ok(format!(
            "{n} shader files: {} lit pixel shaders, {} lights changed to the PS2 falloff, {} normals left unnormalised, {} shaders left as they were",
            total.patched, total.lights, total.unnormalised, total.skipped
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Stats {
    /// Containers changed.
    pub patched: usize,
    /// Lights changed (one per pattern).
    pub lights: usize,
    /// Lit pixel shaders whose code did not have one pattern per light.
    pub skipped: usize,
    /// Normal renormalisations removed.
    pub unnormalised: usize,
}

impl Stats {
    fn add(&mut self, o: &Stats) {
        self.patched += o.patched;
        self.lights += o.lights;
        self.skipped += o.skipped;
        self.unnormalised += o.unnormalised;
    }
}

/// Patches every lit character pixel shader in a shader `.phyre`.
pub fn patch_file(bytes: &mut [u8]) -> Result<Stats> {
    let mut s = Stats::default();
    for (at, size) in dxbc::containers(bytes) {
        match patch_container(&mut bytes[at..at + size])? {
            None => {}
            // A lit shader the pattern did not fit.
            Some(Lit { lights: 0, .. }) => s.skipped += 1,
            Some(lit) => {
                dxbc::reseal(bytes, at, size);
                s.patched += 1;
                s.lights += lit.lights;
                s.unnormalised += lit.unnormalised;
            }
        }
    }
    Ok(s)
}

/// The `cb0` registers of `Light0..2.m_direction` a pixel shader reads.
fn lit_lights(c: &[u8]) -> Option<Vec<u32>> {
    let parsed = ffx_dxbc::Container::parse(c).ok()?;
    if parsed.program()?.0 != "ps" {
        return None;
    }
    let refl = parsed.reflection().ok()??;
    let cb = refl.buffer_at(0)?;
    let mut regs = Vec::new();
    for name in ["Light0", "Light1", "Light2"] {
        if let Some(v) = cb.variable(name).filter(|v| v.used) {
            let dir = v.members.iter().find(|m| m.name == "m_direction")?;
            regs.push((v.offset + dir.offset) / 16);
        }
    }
    Some(regs)
}

fn imm_all(t: &[u32], o: &dxbc::Operand, v: f32) -> bool {
    o.reg == dxbc::REG_IMMEDIATE32 && !o.values.is_empty() && o.values.clone().all(|p| f32::from_bits(t[p]) == v)
}

/// What [`patch_container`] changed in a lit shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lit {
    /// Lights given the PS2 falloff (0 when the code did not match, then
    /// nothing is written).
    pub lights: usize,
    /// Normal renormalisations removed (the `rsq`s the lights' normals came through).
    pub unnormalised: usize,
}

/// Patches one container in place (checksum not updated). `None`: not a lit
/// character pixel shader.
pub fn patch_container(c: &mut [u8]) -> Result<Option<Lit>> {
    let Some(lights) = lit_lights(c) else { return Ok(None) };
    if lights.is_empty() {
        return Ok(None);
    }
    let Some(&(_, at, size)) = dxbc::chunks(c)?.iter().find(|(t, ..)| t == b"SHDR" || t == b"SHEX") else {
        return Ok(Some(Lit { lights: 0, unnormalised: 0 }));
    };
    let mut t = dxbc::tokens(&c[at..at + size]);
    let code = dxbc::instructions(&t, &dxbc::ALU)?;

    // Per component of a temp register: (temp, component).
    type Slot = (u32, u32);
    // Components holding N.L of a light, from their dp3 on.
    let mut ndotl: Vec<Slot> = Vec::new();
    // Components holding N.L x 0.5 + 0.5, with the mad (index into `mads`) that wrote them.
    let mut half: Vec<(Slot, usize)> = Vec::new();
    // The matching mads: (instruction index, components written, components squared since).
    let mut mads: Vec<(usize, u32, u32)> = Vec::new();
    let mut muls: Vec<usize> = Vec::new();
    // The dp3s of the lights.
    let mut light_dp3: Vec<usize> = Vec::new();
    // The component a source operand gives to destination component `c`.
    let read = |o: &dxbc::Operand, c: u32| -> Option<Slot> {
        if o.neg || o.abs {
            return None;
        }
        let r = o.temp()?;
        match (o.comps, o.mode) {
            (2, 1) => Some((r, (o.sel >> (2 * c)) & 3)),
            (2, 2) => Some((r, o.sel & 3)),
            _ => None,
        }
    };
    let comps = |mask: u32| (0..4).filter(move |c| mask & (1 << c) != 0);
    for (k, i) in code.iter().enumerate() {
        let Some(dst) = i.operands.first() else {
            continue;
        };
        let Some(r) = dst.temp() else {
            continue;
        };
        let mask = dst.dst_mask();
        // mad d, n, l(0.5), l(0.5) with every n an N.L.
        let is_half = i.opcode == dxbc::OP_MAD
            && i.operands.len() == 4
            && !i.saturate
            && dst.comps == 2
            && dst.mode == 0
            && imm_all(&t, &i.operands[2], 0.5)
            && imm_all(&t, &i.operands[3], 0.5)
            && comps(mask).all(|c| read(&i.operands[1], c).is_some_and(|s| ndotl.contains(&s)));
        // mul d, d, d with every d a half.
        let is_square = i.opcode == dxbc::OP_MUL
            && i.operands.len() == 3
            && !i.saturate
            && dst.comps == 2
            && dst.mode == 0
            && comps(mask).all(|c| {
                half.iter().any(|&(s, _)| s == (r, c))
                    && read(&i.operands[1], c) == Some((r, c))
                    && read(&i.operands[2], c) == Some((r, c))
            });
        if is_square {
            for c in comps(mask) {
                let p = half.iter().position(|&(s, _)| s == (r, c)).expect("checked");
                mads[half[p].1].2 |= 1 << c;
                half.remove(p);
            }
            muls.push(k);
        }
        // The write ends what the components held before.
        ndotl.retain(|&(tr, tc)| tr != r || mask & (1 << tc) == 0);
        half.retain(|&((tr, tc), _)| tr != r || mask & (1 << tc) == 0);
        if is_half {
            mads.push((k, mask, 0));
            half.extend(comps(mask).map(|c| ((r, c), mads.len() - 1)));
        }
        if i.opcode == dxbc::OP_DP3
            && i.operands.len() == 3
            && let Some(c) = dst.dst_component()
            && i.operands[1..].iter().any(|o| o.cbuffer().is_some_and(|(slot, reg)| slot == 0 && lights.contains(&reg)))
        {
            ndotl.push((r, c));
            light_dp3.push(k);
        }
    }
    // One light per component of a matched mad. Most variants square it (half-Lambert squared);
    // a few use the half-Lambert as it is, and get the mad alone.
    let found: usize = mads.iter().map(|&(_, m, _)| m.count_ones() as usize).sum();
    if found != lights.len() || mads.iter().any(|&(_, m, sq)| sq != 0 && m != sq) {
        if std::env::var_os("DEMASTER_SHADER_DEBUG").is_some() {
            eprintln!("lights {lights:?} mads {mads:?} muls {muls:?} half {half:?} ndotl {ndotl:?}");
        }
        return Ok(Some(Lit { lights: 0, unnormalised: 0 }));
    }
    for &(mad, ..) in &mads {
        let m = &code[mad];
        t[m.at] |= dxbc::SATURATE;
        for p in m.operands[2].values.clone() {
            t[p] = 1.0f32.to_bits();
        }
        for p in m.operands[3].values.clone() {
            t[p] = 0.0f32.to_bits();
        }
    }
    for &mul in &muls {
        let q = code[mul].at;
        t[q] = (t[q] & !0x7ff) | dxbc::OP_MAX;
    }
    // Gouraud: N.L with the interpolated normal as it is, not renormalised.
    let mut rsqs: Vec<usize> = light_dp3.iter().filter_map(|&k| normalisation(&code, k)).collect();
    rsqs.sort_unstable();
    rsqs.dedup();
    if rsqs.is_empty() && std::env::var_os("DEMASTER_SHADER_DEBUG").is_some() {
        eprintln!("lights {lights:?}: no normalisation found");
    }
    for &k in &rsqs {
        let src = code[k].operands[1].at;
        t[src] = dxbc::IMMEDIATE32_SCALAR;
        t[src + 1] = 1.0f32.to_bits();
    }
    for (k, w) in t.iter().enumerate() {
        c[at + 4 * k..at + 4 * k + 4].copy_from_slice(&w.to_le_bytes());
    }
    Ok(Some(Lit { lights: found, unnormalised: rsqs.len() }))
}

/// The last instruction before `k` that writes any of `comps` of temp `r`;
/// `None` across flow control (the writer would depend on the path).
fn last_writer(code: &[dxbc::Instruction], k: usize, r: u32, comps: u32) -> Option<usize> {
    for j in (0..k).rev() {
        let i = &code[j];
        if dxbc::FLOW.contains(&i.opcode) {
            return None;
        }
        if i.operands.first().is_some_and(|d| d.temp() == Some(r) && d.dst_mask() & comps != 0) {
            return Some(j);
        }
    }
    None
}

/// The `rsq` that renormalises the interpolated normal a light's `dp3`
/// (instruction `k`) reads:
///
/// ```text
/// dp3 rK.c, vM.xyz, vM.xyz
/// rsq rK.c, rK.c
/// mul rN.xyz, rK.cccc, vM.xyz      (either order; or rN.yzw)
/// dp3 rA.c, cb0[Light_i.m_direction].xyz, rN.xyz
/// ```
///
fn normalisation(code: &[dxbc::Instruction], k: usize) -> Option<usize> {
    let normal = code[k].operands[1..].iter().find(|o| o.temp().is_some())?;
    let n = normal.temp()?;
    // The components the dp3 reads (its swizzle's first three).
    let read = match (normal.comps, normal.mode) {
        (2, 1) => (0..3).fold(0, |m, c| m | 1 << ((normal.sel >> (2 * c)) & 3)),
        _ => return None,
    };
    let mul = last_writer(code, k, n, read)?;
    let m = &code[mul];
    if m.opcode != dxbc::OP_MUL || m.saturate || m.operands.len() != 3 || m.operands[0].dst_mask() & read != read {
        return None;
    }
    let (scale, vin) = match (&m.operands[1], &m.operands[2]) {
        (a, b) if a.input().is_some() => (b, a.input()?),
        (a, b) => (a, b.input()?),
    };
    let rk = scale.temp()?;
    let c = scale.src_component()?;
    if scale.neg || scale.abs {
        return None;
    }
    let rsq = last_writer(code, mul, rk, 1 << c)?;
    let q = &code[rsq];
    if q.opcode != dxbc::OP_RSQ || q.saturate || q.operands.len() != 2 || q.operands[0].dst_component() != Some(c) {
        return None;
    }
    let src = &q.operands[1];
    if src.temp() != Some(rk) || src.src_component() != Some(c) || src.neg || src.abs || src.len != 2 {
        return None;
    }
    let dp = last_writer(code, rsq, rk, 1 << c)?;
    let d = &code[dp];
    (d.opcode == dxbc::OP_DP3
        && d.operands.len() == 3
        && d.operands[0].dst_component() == Some(c)
        && d.operands[1..].iter().all(|o| o.input() == Some(vin) && !o.neg && !o.abs))
        .then_some(rsq)
}


#[cfg(test)]
mod tests {
    use super::*;

    /// With the game installed: every container's stored checksum is what
    /// [`dxbc::checksum`] computes, and every lit character pixel shader
    /// takes the patch (one per light).
    #[test]
    fn game_shaders() {
        let vbf = std::env::var("FFX_VBF").map_or_else(
            |_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("steam-games/FFX-X2-HD/data/FFX_Data.vbf"),
            Into::into,
        );
        if !vbf.exists() {
            eprintln!("skipped: no {}", vbf.display());
            return;
        }
        let pc = ffx_vbf::Archive::open(&vbf).unwrap();
        let mut total = Stats::default();
        let mut checked = 0;
        for e in pc.entries().iter().filter(|e| e.path.starts_with(SHADERS)) {
            let bytes = pc.read(e).unwrap();
            for (at, size) in dxbc::containers(&bytes) {
                assert_eq!(dxbc::checksum(&bytes[at..at + size]), bytes[at + 4..at + 20], "{} {at:#x}", e.path);
                checked += 1;
            }
            if e.path.contains(CHR_LIT) {
                let mut b = bytes.clone();
                let s = patch_file(&mut b).unwrap();
                if s.skipped > 0 {
                    eprintln!("{}: {s:?}", e.path);
                }
                total.add(&s);
                for (at, size) in dxbc::containers(&b) {
                    assert!(ffx_dxbc::Container::parse(&b[at..at + size]).unwrap().program_code().is_ok());
                }
            }
        }
        assert!(checked > 10_000, "{checked}");
        assert_eq!(total.skipped, 0, "{total:?}");
        assert_eq!(total.patched, 1854, "{total:?}");
        assert_eq!(total.lights, 1854 + 1236 + 618, "{total:?}");
        eprintln!("{total:?}");
        assert_eq!(total.unnormalised, 1854, "{total:?}");
    }
}
