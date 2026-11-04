//! `audio.music_engine = "live"`: the PS2 files the live music engine
//! (`live/`, `demaster_live.dll`) plays in the game, copied from the disc
//! into `<mod>/music/`: the sound driver and `libsd` (for their tables:
//! pan law, random bytes, reverb presets), every `musicNNN.bgm` and the
//! instrument bank (`waveNNNN.wd`) each one names. `docs/music.md`,
//! "Phase 2".

use anyhow::Result;

use crate::config::{Config, MusicEngine};
use crate::jobs::{Ctx, Job, Output};
use crate::musicjob::Sources;
use crate::sqseq::Bgm;

struct LiveJob;

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(LiveJob)]
}

impl Job for LiveJob {
    fn name(&self) -> &'static str {
        "audio.live"
    }
    fn settings(&self) -> &'static [&'static str] {
        &["audio.music_engine"]
    }
    fn enabled(&self, config: &Config) -> bool {
        config.audio.music_engine == MusicEngine::Live
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        let (n, banks, missing) = ctx.disc(|d| {
            let src = Sources::load(d)?;
            for m in ["iopsound.irx", "libsd.irx"] {
                let b = d.read(&format!("jppc/irx/{m}"))?.unwrap_or_default();
                out.put_mod(&format!("music/{m}"), &b)?;
            }
            let mut banks = std::collections::BTreeSet::new();
            let mut missing = Vec::new();
            let ids = src.music_ids(d);
            for &id in &ids {
                let b = src.sequence(d, id)?;
                let wd = Bgm::parse(&b)?.wd;
                out.put_mod(&format!("music/music{id:03}.bgm"), &b)?;
                if banks.contains(&wd) {
                    continue;
                }
                match src.bank_bytes(d, wd) {
                    Ok(w) => {
                        out.put_mod(&format!("music/wave{wd:04}.wd"), &w)?;
                        banks.insert(wd);
                    }
                    // the ten sequences whose banks are not on the disc
                    // (docs/music.md, "Track notes", 2): the engine leaves
                    // them to the bank (the game never plays them)
                    Err(_) => missing.push(id),
                }
            }
            Ok((ids.len(), banks.len(), missing))
        })?;
        Ok(format!("{n} sequences, {banks} instrument banks; no bank on the disc for {missing:?}"))
    }
}
