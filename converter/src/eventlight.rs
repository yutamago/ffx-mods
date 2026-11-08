//! `render.lighting = "ps2"`, the event half: the PS2's light colours and
//! directions in the PC's event scripts.
//!
//! Event scripts set the character lights with `Light_Set` (native `0x4013`,
//! FFX.exe `0x00836520`: light 0-3, r, g, b as bytes, two angles in degrees),
//! almost always from immediates: six `pushimm` before `func_facc 0x4013`.
//! The HD Remaster retuned some of them (the boot intro `znkd0600`: Tidus's
//! close-up light 0 (255, 225, 147) and ambient (16, 18, 39) where the PS2
//! disc has (225, 199, 118) and (20, 22, 49)), which made its lit areas 10%
//! brighter than the PS2's (`docs/render.md`). This job reads each event's
//! `.ebp` from the PS2 disc and from `FFX_Data.vbf`, pairs the calls worker
//! by worker in code order, and writes the PS2's six immediates into the PC
//! file in place (same instruction lengths; nothing else changes). A file
//! whose calls do not pair up (different counts or light numbers per
//! worker) is left as it is and counted.

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::config::Config;
use crate::jobs::{Ctx, Job, Output};

/// `Light_Set`'s native number.
pub const LIGHT_SET: u16 = 0x4013;
/// `func_facc` (call a native, its result to the accumulator).
const FUNC_FACC: u8 = 0x58;
/// `pushimm`: a 16-bit immediate.
const PUSHIMM: u8 = 0x2e;
const EVENTS: &str = "ffx_ps2/ffx/master/";
/// Written by the `ui.title` job (`uijob::memochek_with_logo`); it sets no
/// light.
const SKIP: &[&str] = &["ffx_ps2/ffx/master/jppc/event/obj/me/memochek/memochek.ebp"];

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(EventLightJob)]
}

struct EventLightJob;

impl Job for EventLightJob {
    fn name(&self) -> &'static str {
        "render.lighting.events"
    }
    fn settings(&self) -> &'static [&'static str] {
        &["render.lighting"]
    }
    fn enabled(&self, config: &Config) -> bool {
        config.render.lighting.ps2()
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        let files: Vec<_> = ctx
            .pc
            .entries()
            .iter()
            .filter(|e| {
                e.path.strip_prefix(EVENTS).is_some_and(|p| p.contains("/event/") && p.ends_with(".ebp"))
                    && !SKIP.contains(&e.path.as_str())
            })
            .collect();
        let ps2: Vec<Option<Vec<u8>>> = ctx.disc(|d| {
            files.iter().map(|e| d.read(e.path.strip_prefix(EVENTS).expect("filtered"))).collect::<Result<_>>()
        })?;
        let results: Vec<_> = files
            .par_iter()
            .zip(ps2)
            .map(|(e, ps2)| -> Result<_> {
                let Some(ps2) = ps2 else { return Ok((e.path.clone(), None, Outcome::NoPs2)) };
                let mut pc = ctx.pc.read(e)?;
                let o = patch(&mut pc, &ps2).with_context(|| e.path.clone())?;
                Ok((e.path.clone(), Some(pc), o))
            })
            .collect::<Result<_>>()?;
        let (mut written, mut calls, mut changed, mut same, mut unpaired, mut no_ps2) = (0, 0, 0, 0, 0, 0);
        for (path, bytes, o) in results {
            match o {
                Outcome::NoPs2 => no_ps2 += 1,
                Outcome::Unpaired(why) => {
                    unpaired += 1;
                    ctx.note(format!("{path}: left as it is ({why})"));
                }
                Outcome::Paired { calls: n, changed: c } => {
                    calls += n;
                    changed += c;
                    if c == 0 {
                        same += 1;
                    } else if out.put(&path, &bytes.expect("paired"))? {
                        written += 1;
                        ctx.note(format!("{path}: {c} of {n} Light_Set calls set to the PS2's"));
                    }
                }
            }
        }
        Ok(format!(
            "{written} event scripts with the PS2's lights ({changed} of {calls} Light_Set calls changed); {same} the same, {unpaired} not paired, {no_ps2} not on the disc"
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    NoPs2,
    Unpaired(String),
    Paired { calls: usize, changed: usize },
}

/// One `Light_Set` call: its worker, the six arguments and the file offsets
/// of their operands (`None` where an argument is not a `pushimm`).
#[derive(Debug, Clone)]
pub struct Call {
    pub worker: usize,
    pub args: [Option<i16>; 6],
    pub at: [Option<usize>; 6],
}

/// The `Light_Set` calls of an event file, in worker and code order.
pub fn calls(file: &[u8]) -> Result<Vec<Call>> {
    let c = ffx_script::Container::parse(file.to_vec()).context("event script")?;
    // the script section: the first non-zero offset word (`Container::parse`)
    let start = (1..16)
        .map(|i| i * 4)
        .map_while(|o| file.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes"))))
        .take_while(|&v| v != u32::MAX)
        .find(|&v| v != 0)
        .context("no script section")? as usize;
    let s = &c.script;
    anyhow::ensure!(file.get(start..start + s.bytes().len()) == Some(s.bytes()), "script section not found");
    let code = start + u32::from_le_bytes(s.bytes()[0x30..0x34].try_into()?) as usize;
    let mut out = Vec::new();
    for (wi, w) in s.workers().iter().enumerate() {
        let code_of = s.decode(w.range.clone())?;
        for (k, i) in code_of.iter().enumerate() {
            if i.opcode != FUNC_FACC || i.operand != Some(LIGHT_SET) {
                continue;
            }
            let mut call = Call { worker: wi, args: [None; 6], at: [None; 6] };
            if k >= 6 {
                for (j, p) in code_of[k - 6..k].iter().enumerate() {
                    if p.opcode == PUSHIMM {
                        call.args[j] = p.operand.map(|v| v as i16);
                        call.at[j] = Some(code + p.offset as usize + 1);
                    }
                }
            }
            out.push(call);
        }
    }
    Ok(out)
}

/// Writes the PS2 file's `Light_Set` immediates into the PC file.
pub fn patch(pc: &mut [u8], ps2: &[u8]) -> Result<Outcome> {
    let a = calls(ps2).context("PS2")?;
    let b = calls(pc).context("PC")?;
    if a.len() != b.len() {
        return Ok(Outcome::Unpaired(format!("{} Light_Set calls on the PS2, {} on the PC", a.len(), b.len())));
    }
    for (x, y) in a.iter().zip(&b) {
        if x.worker != y.worker || x.args[0] != y.args[0] {
            return Ok(Outcome::Unpaired(format!(
                "worker {} light {:?} on the PS2, worker {} light {:?} on the PC",
                x.worker, x.args[0], y.worker, y.args[0]
            )));
        }
    }
    let mut changed = 0;
    for (x, y) in a.iter().zip(&b) {
        let mut any = false;
        for j in 1..6 {
            if let (Some(v), Some(at)) = (x.args[j], y.at[j])
                && y.args[j] != Some(v)
            {
                pc[at..at + 2].copy_from_slice(&v.to_le_bytes());
                any = true;
            }
        }
        changed += usize::from(any);
    }
    Ok(Outcome::Paired { calls: a.len(), changed })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the game and the PS2 disc (`FFX_VBF`, `FFX_ISO`; defaults
    /// `~/steam-games/FFX-X2-HD/data/FFX_Data.vbf`, `/tmp/dm-ffx.iso`): the
    /// boot intro gets the PS2's lights, Tidus's close-up included, and
    /// only `pushimm` operands change.
    #[test]
    fn intro() {
        let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
        let vbf = std::env::var("FFX_VBF").map_or_else(|_| home.join("steam-games/FFX-X2-HD/data/FFX_Data.vbf"), Into::into);
        let iso = std::env::var("FFX_ISO").map_or_else(|_| std::path::PathBuf::from("/tmp/dm-ffx.iso"), Into::into);
        if !vbf.exists() || !iso.exists() {
            eprintln!("skipped: no {} or {}", vbf.display(), iso.display());
            return;
        }
        let pc = ffx_vbf::Archive::open(&vbf).unwrap();
        let path = "ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0600/znkd0600.ebp";
        let original = pc.read(pc.find(path).unwrap()).unwrap();
        let ps2 = crate::disc::Disc::open(&iso, &pc).unwrap().read(path.strip_prefix(EVENTS).unwrap()).unwrap().unwrap();
        let mut b = original.clone();
        assert_eq!(patch(&mut b, &ps2).unwrap(), Outcome::Paired { calls: 36, changed: 11 });
        let args = |f: &[u8]| calls(f).unwrap().iter().map(|c| c.args.map(Option::unwrap)).collect::<Vec<_>>();
        let (got, want) = (args(&b), args(&ps2));
        assert_eq!(got, want);
        assert!(args(&original).contains(&[0, 255, 225, 147, 26, 15]));
        assert!(got.contains(&[0, 225, 199, 118, 26, 15]) && got.contains(&[3, 20, 22, 49, 0, 0]));
        // only operands of the calls' pushimm moved
        let operands: std::collections::BTreeSet<usize> =
            calls(&b).unwrap().iter().flat_map(|c| c.at.into_iter().flatten().flat_map(|a| [a, a + 1])).collect();
        assert_eq!(b.len(), original.len());
        assert!(b.iter().zip(&original).enumerate().all(|(i, (x, y))| x == y || operands.contains(&i)));
    }
}
