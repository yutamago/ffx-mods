mod bake;
mod bgjob;
mod build;
mod chrjob;
mod config;
mod disc;
mod dxbc;
mod fontjob;
mod fsb5;
mod fxjob;
mod glb;
mod gsmem;
mod jobs;
mod mapjob;
mod models;
mod moviejob;
mod musicjob;
mod phyretex;
mod ps2chr;
mod ps2fx;
mod ps2map;
mod ps2mesh;
mod ps2movie;
mod shaderjob;
mod spu2;
mod sqseq;
mod texanim;
mod uijob;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "FFX Demaster converter {}\nusage:\n{}\n  demaster chr <iso> <FFX_Data.vbf> <out dir> <kind/name>...\n      bake character textures (PNG previews + .dds.phyre)\n  demaster models <iso> <FFX_Data.vbf> <out dir> all|<kind/name>...\n      PS2 meshes into the remodelled characters (run after chr; docs/models.md)\n  demaster models-debug <chr file> <FFX_Data.vbf> <kind/name>\n      print a PS2 mesh against its HD model\n  demaster fx <iso> <FFX_Data.vbf> <out dir> all|<id>...\n      effect textures; id: 7, magic_0007, map/bika/bika00, btlmap/bika/bika00_a, chr/mon/m028\n  demaster maps <iso> <FFX_Data.vbf> <out dir> <map|area|kind>...|all\n      field and battle map textures (PNG previews, maps.tsv + .dds.phyre)\n  demaster ui <iso> <FFX_Data.vbf> <out dir>\n      menus, portraits, title, loading screen, button glyphs, font (PNG previews + .dds.phyre, base.ftc)\n  demaster movies <iso> <FFX_Data.vbf> <mod dir> all|<name>...\n      PS2 movies as WebM (name: opn, sak, ...; ffmpeg: $FFMPEG, else found or downloaded as by build; docs/movies.md)\n  demaster movie-debug <iso> <FFX_Data.vbf> <out dir> <name> [frame]...\n      decode one PS2 movie: statistics, PNGs of the frames, the sound as WAV\n  demaster music <iso> <FFX_Data.vbf> <out dir> all|<id|name>...\n      render PS2 music tracks (musicNNN.bgm) as 48 kHz WAVs (docs/music.md)\n  demaster music-bank <bank.fsb> <out dir> [name...]\n      decode a music bank (FSB5, IMA ADPCM) to WAVs\n  demaster texanim-debug <chr file> <FFX_Data.vbf> <kind/name>\n      print a PS2 texture animation against the HD .ags",
        build::VERSION,
        build::USAGE
    );
    std::process::exit(2)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("no parent")?)?;
    std::fs::write(path, bytes).with_context(|| path.display().to_string())
}

/// Keeps the console open when demaster.exe was started by double-click.
fn wait_for_enter() {
    if cfg!(windows) {
        println!("\nPress Enter to close.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let Some(cmd) = a.get(1) else {
        // No arguments: build with the demaster.toml next to the exe.
        let ok = build::run(&build::Args::default()).unwrap_or_else(|e| {
            eprintln!("error: {e:#}");
            false
        });
        wait_for_enter();
        std::process::exit(if ok { 0 } else { 1 });
    };
    match cmd.as_str() {
        "build" => {
            let args = build::Args::parse(&a[2..])?;
            if !build::run(&args)? {
                std::process::exit(1);
            }
        }
        "jobs" => build::list_jobs(),
        "--version" | "-V" => println!("demaster {}", build::VERSION),
        "help" | "--help" | "-h" => usage(),
        "chr" => {
            if a.len() < 6 {
                usage();
            }
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let mut d = disc::Disc::open(Path::new(&a[2]), &pc)?;
            eprintln!("disc: region {}, {} files", d.region, d.file_count());
            let out = PathBuf::from(&a[4]);
            let mut specs: Vec<String> = a[5..].to_vec();
            if specs == ["all"] {
                specs = chrjob::models(&d)
                    .into_iter()
                    .map(|(kind, name)| format!("{kind}/{name}"))
                    .collect();
            }
            for spec in &specs {
                let (kind, name) = spec.split_once('/').context("kind/name")?;
                let r = match chrjob::bake(&mut d, &pc, kind, name) {
                    Ok(r) => r,
                    Err(e) => {
                        println!("{spec}: error: {e:#}");
                        continue;
                    }
                };
                let Some(r) = r else {
                    println!("{spec}: no PS2 or HD model");
                    continue;
                };
                println!(
                    "{spec}: frame matches {:.0}% of HD vertices",
                    r.frame_share * 100.0
                );
                for p in &r.pages {
                    println!(
                        "  {} -> {}x{} (1/{}), {:.0}% triangles matched, {:.0}% painted",
                        p.path,
                        p.image.width,
                        p.image.height,
                        p.scale,
                        p.matched * 100.0,
                        p.covered * 100.0
                    );
                    write(&out.join("efl/x").join(&p.path), &p.file)?;
                    let png = ffx_phyre::png(p.image.width, p.image.height, &p.image.pixels);
                    let file = p
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or("x")
                        .replace(".dds.phyre", ".png");
                    write(&out.join("preview").join(format!("{name}-{file}")), &png)?;
                }
                for (p, m) in &r.skipped {
                    println!("  {p}: skipped, {:.0}% triangles matched", m * 100.0);
                }
            }
        }
        "maps" => {
            if a.len() < 6 {
                usage();
            }
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let mut d = disc::Disc::open(Path::new(&a[2]), &pc)?;
            eprintln!("disc: region {}, {} files", d.region, d.file_count());
            let out = PathBuf::from(&a[4]);
            maps(&mut d, &pc, &out, &a[5..])?;
        }
        "models" => {
            if a.len() < 6 {
                usage();
            }
            models_cmd(&a[2], &a[3], Path::new(&a[4]), &a[5..])?;
        }
        "models-debug" => {
            if a.len() < 5 {
                usage();
            }
            let chr = std::fs::read(&a[2])?;
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let (kind, name) = a[4].split_once('/').context("kind/name")?;
            let p = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/{name}.dae.phyre");
            let hd = pc.read(pc.find(&p).context("no HD model")?)?;
            models::debug(&chr, &hd)?;
        }
        "texanim-debug" => {
            if a.len() < 5 {
                usage();
            }
            texanim_debug(&a[2], &a[3], &a[4])?;
        }
        "fx" => {
            if a.len() < 6 {
                usage();
            }
            fx(&a[2], &a[3], Path::new(&a[4]), &a[5..])?;
        }
        "ui" => {
            if a.len() < 5 {
                usage();
            }
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let mut d = disc::Disc::open(Path::new(&a[2]), &pc)?;
            eprintln!("disc: region {}, {} files", d.region, d.file_count());
            let out = PathBuf::from(&a[4]);
            let (done, notes) = uijob::convert(&mut d, &pc, uijob::Parts::all())?;
            for t in &done {
                println!("{}", t.note);
                write(&out.join("efl/x").join(&t.path), &t.file)?;
                let name = t
                    .path
                    .trim_start_matches("ffx_data/gamedata/ps3data/")
                    .replace('/', "_")
                    .replace("_d3d11_", "_")
                    .replace(".dds.phyre", ".png");
                // other files (the font's base.ftc) get a picture too
                let name = if name.ends_with(".png") { name } else { format!("{name}.png") };
                let p = &t.preview;
                write(&out.join("preview").join(name), &ffx_phyre::png(p.width, p.height, &p.pixels))?;
            }
            for n in &notes {
                println!("{n}");
            }
            println!("{} files, {} left HD", done.len(), notes.len());
        }
        "backgrounds" => {
            if a.len() < 6 {
                usage();
            }
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let mut d = disc::Disc::open(Path::new(&a[2]), &pc)?;
            let out = PathBuf::from(&a[4]);
            let only: Vec<String> = a[5..].iter().filter(|s| *s != "all").cloned().collect();
            // DEMASTER_ASPECT=hd: as with render.aspect = "hd" (re-framed rooms stay HD)
            let four_three = std::env::var("DEMASTER_ASPECT").map_or(true, |v| v != "hd");
            let (done, notes) = bgjob::convert(&mut d, &pc, &only, four_three)?;
            for t in &done {
                println!("{}", t.note);
                write(&out.join("efl/x").join(&t.path), &t.file)?;
                let name = t.path.trim_start_matches("ffx_data/gamedata/ps3data/map/").replace('/', "_");
                let p = &t.preview;
                write(
                    &out.join("preview").join(name.replace(".dds.phyre", ".png")),
                    &ffx_phyre::png(p.width, p.height, &p.pixels),
                )?;
            }
            for n in &notes {
                println!("{n}");
            }
            println!("{} layers, {} notes", done.len(), notes.len());
        }
        "movies" => {
            if a.len() < 6 {
                usage();
            }
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let config = config::Config::default();
            let mut ctx = jobs::Ctx::new(&config, &pc, Path::new(&a[2]), true);
            ctx.ffmpeg = std::env::var_os("FFMPEG").map(PathBuf::from);
            ctx.tools_dir = Some(Path::new(&a[4]).join("tools"));
            let out = jobs::Output::new(Path::new(&a[4]));
            let only: Vec<String> = a[5..].iter().filter(|s| *s != "all").cloned().collect();
            println!("{}", moviejob::run(&ctx, &out, &only)?);
        }
        "music" => {
            if a.len() < 6 {
                usage();
            }
            musicjob::debug(Path::new(&a[2]), &a[3], Path::new(&a[4]), &a[5..])?;
        }
        "music-bank" => {
            if a.len() < 4 {
                usage();
            }
            musicjob::bank_debug(Path::new(&a[2]), Path::new(&a[3]), &a[4..])?;
        }
        "movie-debug" => {
            if a.len() < 6 {
                usage();
            }
            movie_debug(&a[2], &a[3], Path::new(&a[4]), &a[5], &a[6..])?;
        }
        _ => bail!("unknown command {cmd} (demaster help)"),
    }
    Ok(())
}

fn maps(d: &mut disc::Disc, pc: &ffx_vbf::Archive, out: &Path, specs: &[String]) -> Result<()> {
    let all = mapjob::hd_maps(pc);
    let chosen: Vec<_> = all
        .iter()
        .filter(|(id, _)| {
            specs
                .iter()
                .any(|s| s == "all" || *s == id.name || *s == id.area || *s == id.kind)
        })
        .collect();
    if chosen.is_empty() {
        bail!("no HD map matches {specs:?}");
    }
    let mut log = String::from("map\ttexture\tresult\terror\tsecond\thow\n");
    let (mut conv, mut hd_only, mut failed, mut guessed) = (0, 0, 0, 0);
    for (id, paths) in chosen {
        let r = match mapjob::convert(d, pc, id, paths, true) {
            Ok(r) => r,
            Err(e) => {
                println!("{}/{}: error: {e:#}", id.kind, id.name);
                failed += paths.len();
                continue;
            }
        };
        let g = r
            .converted
            .iter()
            .filter(|c| c.how.ends_with("[unbound]"))
            .count();
        println!(
            "{}/{}: {} converted ({} unbound), {} HD-only, {} failed",
            id.kind,
            id.name,
            r.converted.len(),
            g,
            r.hd_only.len(),
            r.failed.len()
        );
        conv += r.converted.len();
        guessed += g;
        hd_only += r.hd_only.len();
        failed += r.failed.len();
        for c in &r.converted {
            write(&out.join("efl/x").join(&c.path), &c.file)?;
            let file = c
                .path
                .rsplit('/')
                .next()
                .unwrap_or("x")
                .replace(".dds.phyre", ".png");
            let sub = if c.region.frame.is_some() {
                "texswap/"
            } else {
                ""
            };
            if let Some(p) = &c.preview {
                let png = ffx_phyre::png(p.width, p.height, &p.pixels);
                write(
                    &out.join("preview")
                        .join(&id.name)
                        .join(format!("{sub}{file}")),
                    &png,
                )?;
            }
            log += &format!(
                "{}\t{}\tok\t{:.1}\t{}\t{}\n",
                id.name,
                c.path,
                c.error,
                c.second.map_or(String::new(), |s| format!("{s:.1}")),
                c.how
            );
        }
        for p in &r.hd_only {
            log += &format!("{}\t{p}\thd-only\t\t\t\n", id.name);
        }
        for (p, why) in &r.failed {
            println!("  {p}: {why}");
            log += &format!("{}\t{p}\tfailed\t\t\t{why}\n", id.name);
        }
    }
    write(&out.join("maps.tsv"), log.as_bytes())?;
    println!("total: {conv} converted ({guessed} unbound), {hd_only} HD-only, {failed} failed");
    Ok(())
}

fn fx(iso: &str, vbf: &str, out: &Path, specs: &[String]) -> Result<()> {
    use std::fmt::Write as _;
    let pc = ffx_vbf::Archive::open(vbf)?;
    let mut d = disc::Disc::open(Path::new(iso), &pc)?;
    eprintln!("disc: region {}, {} files", d.region, d.file_count());
    let hd = fxjob::HdIndex::new(&pc);
    let sources = if specs == ["all"] {
        let s = hd.sources(&d);
        let missing = hd.without_ps2(&d);
        if !missing.is_empty() {
            println!("{} HD effect folders without a PS2 file:", missing.len());
            for m in &missing {
                println!("  {m}");
            }
        }
        s
    } else {
        specs
            .iter()
            .map(|s| fxjob::Source::parse(s).with_context(|| format!("bad id {s}")))
            .collect::<Result<_>>()?
    };
    let mut gs = ps2fx::Gs::default();
    let mut tsv = String::from("path\thd\tps2\tcorrelation\talpha_diff\torientation\tconvention\n");
    let (mut done, mut hd_only, mut other, mut unused, mut failed, mut nopal) = (0, 0, 0, 0, 0, 0);
    let mut runtime = 0;
    for src in &sources {
        let r = match fxjob::convert(&mut d, &pc, &hd, src, &mut gs) {
            Ok(r) => r,
            Err(e) => {
                println!("{}: error: {e:#}", src.id);
                failed += 1;
                continue;
            }
        };
        println!(
            "{}: {} converted, {} HD-only, {} runtime, {} other language, {} PS2 unused, {} failed",
            src.id,
            r.done.len(),
            r.hd_only.len(),
            r.runtime.len(),
            r.other_language.len(),
            r.ps2_unused.len(),
            r.failed.len()
        );
        for p in &r.hd_only {
            println!("  HD-only {p}");
        }
        for (p, e) in &r.failed {
            println!("  failed {p}: {e}");
        }
        if std::env::var_os("DM_DEBUG").is_some() {
            for p in &r.ps2_unused {
                println!("  PS2 unused {p}");
            }
        }
        for p in &r.missing_clut {
            println!("  no palette {p}");
        }
        let preview_dir = out.join("preview").join(src.id.replace('/', "_"));
        for t in &r.done {
            write(&out.join("efl/x").join(&t.path), &t.file)?;
            let rel = &t.path[src.hd_dir.len() + 1..];
            let file = rel.replace("/d3d11/", "_").replace(".dds.phyre", ".png");
            let p = &t.preview;
            if std::env::var_os("DM_DEBUG").is_some() {
                let i = &t.image;
                write(
                    &preview_dir.join(file.replace(".png", ".ps2.png")),
                    &ffx_phyre::png(i.width, i.height, &i.pixels),
                )?;
            }
            write(
                &preview_dir.join(file),
                &ffx_phyre::png(p.width, p.height, &p.pixels),
            )?;
            let diff = t
                .diff
                .map_or("-\t-".into(), |(c, a)| format!("{c:.3}\t{a:.1}"));
            let flipped = t.orientation;
            let white = if t.white { "white" } else { "" };
            writeln!(
                tsv,
                "{}\t{}x{}\t{}x{}\t{diff}\t{flipped}\t{white}",
                t.path, t.hd.0, t.hd.1, t.image.width, t.image.height
            )?;
        }
        done += r.done.len();
        hd_only += r.hd_only.len();
        runtime += r.runtime.len();
        other += r.other_language.len();
        unused += r.ps2_unused.len();
        failed += r.failed.len();
        nopal += r.missing_clut.len();
    }
    write(&out.join("fx-report.tsv"), tsv.as_bytes())?;
    println!(
        "total: {} sources, {done} converted, {hd_only} HD-only, {runtime} runtime (trxtex), {other} other language (kept HD), {unused} PS2 images unused, {nopal} without palette, {failed} failed",
        sources.len()
    );
    Ok(())
}

fn models_cmd(iso: &str, vbf: &str, out: &Path, specs: &[String]) -> Result<()> {
    let pc = ffx_vbf::Archive::open(vbf)?;
    let mut d = disc::Disc::open(Path::new(iso), &pc)?;
    eprintln!("disc: region {}, {} files", d.region, d.file_count());
    let mut specs: Vec<String> = specs.to_vec();
    if specs == ["all"] {
        specs.clear();
        for (kind, name) in chrjob::models(&d) {
            let Some(chr) = d.read(&format!("jppc/chr/{kind}/{name}/mdl/{name}.chr"))? else {
                continue;
            };
            if models::remodelled(&chr, &pc, &kind, &name).unwrap_or(false) {
                specs.push(format!("{kind}/{name}"));
            }
        }
        eprintln!("{} remodelled characters", specs.len());
    }
    let (mut ok, mut failed) = (0, 0);
    for spec in &specs {
        let (kind, name) = spec.split_once('/').context("kind/name")?;
        let c = match models::convert(&mut d, &pc, kind, name) {
            Ok(Some(c)) => c,
            Ok(None) => {
                println!("{spec}: no PS2 or HD model");
                continue;
            }
            Err(e) => {
                println!("{spec}: error: {e:#}");
                failed += 1;
                continue;
            }
        };
        println!("{spec}:\n{}", c.report.trim_end());
        for (path, bytes) in &c.files {
            write(&out.join("efl/x").join(path), bytes)?;
        }
        let png = ffx_phyre::png(c.atlas.width, c.atlas.height, &c.atlas.pixels);
        write(&out.join("preview").join(format!("{name}-atlas.png")), &png)?;
        write(&out.join("preview").join(format!("{name}.glb")), &c.glb)?;
        for (tag, im) in &c.anim {
            write(&out.join("preview").join(format!("{name}-anim-{tag}.png")), &ffx_phyre::png(im.width, im.height, &im.pixels))?;
        }
        ok += 1;
    }
    eprintln!("{ok} converted, {failed} failed");
    Ok(())
}

/// Decodes one PS2 movie: statistics, PNGs of the given frames, and
/// `<name>.wav`.
fn movie_debug(iso: &str, vbf: &str, out: &Path, name: &str, frames: &[String]) -> Result<()> {
    let pc = ffx_vbf::Archive::open(vbf)?;
    let mut d = disc::Disc::open(Path::new(iso), &pc)?;
    let sz = d.read(&format!("jppc/movie/adpcm/{name}.sz"))?.context("no .sz")?;
    let dat = d.read(&format!("jppc/movie/adpcm/{name}.dat"))?.context("no .dat")?;
    let index = ps2movie::Index::parse(&sz)?;
    let want: Vec<usize> = frames.iter().filter_map(|f| f.parse().ok()).collect();
    let mut dec = ps2movie::Decoder::new(&index);
    let mut off = 0usize;
    let mut pcm: Vec<u8> = Vec::new();
    for (i, &s) in index.sizes.iter().enumerate() {
        let chunk = dat.get(off..off + s as usize).context("chunk past the file")?;
        off += s as usize;
        let u = dec.chunk(chunk).with_context(|| format!("frame {i}"))?;
        for v in &u.sound {
            pcm.extend(v.to_le_bytes());
        }
        if want.contains(&i) {
            let p = &u.picture;
            write(
                &out.join(format!("{name}-{i:05}.png")),
                &ffx_phyre::png(p.width as u32, p.height as u32, &p.rgba()),
            )?;
        }
    }
    write(&out.join(format!("{name}.wav")), &ps2movie::wav(&pcm))?;
    println!("{name}: {:?}, sound {:.2} s", dec.stats, pcm.len() as f64 / 4.0 / 48000.0);
    Ok(())
}

fn texanim_debug(chr: &str, vbf: &str, spec: &str) -> Result<()> {
    let chr = std::fs::read(chr)?;
    let pc = ffx_vbf::Archive::open(vbf)?;
    let (kind, name) = spec.split_once('/').context("kind/name")?;
    let Some(t) = texanim::TexAnim::parse(&chr)? else {
        println!("no texture animation");
        return Ok(());
    };
    for (k, p) in t.patches.iter().enumerate() {
        println!("patch {k}: {}x{}", p.width, p.height);
    }
    for e in &t.entries {
        println!("G{} mode {} at ({}, {}) image {}", e.group, e.mode, e.x, e.y, e.image);
        for (s, frames) in &e.states {
            let f: Vec<String> = frames.iter().map(|f| format!("{}:p{}", f.duration, f.patch)).collect();
            println!("  Animation{s}: {}", f.join(" "));
        }
    }
    let p = format!("ffx_data/gamedata/ps3data/chr/{kind}/{name}/mdl/d3d11/textureanimationchar.ags.phyre");
    let Some(e) = pc.find(&p) else {
        println!("no HD ags");
        return Ok(());
    };
    let bytes = pc.read(e)?;
    let recs = texanim::read_ags(&bytes)?;
    for r in &recs {
        println!("HD {} {}: ids {:?} rect {:?} interval {}", r.group, r.animation, r.ids, r.rect, r.interval);
    }
    let again = texanim::write_ags(&bytes, &recs)?;
    println!("ags round trip: {} -> {} bytes, records equal", bytes.len(), again.len());
    Ok(())
}
