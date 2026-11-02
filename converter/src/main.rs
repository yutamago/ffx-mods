mod bake;
mod build;
mod chrjob;
mod config;
mod disc;
mod fxjob;
mod glb;
mod jobs;
mod models;
mod phyretex;
mod ps2chr;
mod ps2fx;
mod ps2mesh;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "FFX Demaster converter {}\nusage:\n{}\n  demaster chr <iso> <FFX_Data.vbf> <out dir> <kind/name>...\n      bake character textures (PNG previews + .dds.phyre)\n  demaster models <iso> <FFX_Data.vbf> <out dir> all|<kind/name>...\n      PS2 meshes into the remodelled characters (run after chr; docs/models.md)\n  demaster models-debug <chr file> <FFX_Data.vbf> <kind/name>\n      print a PS2 mesh against its HD model\n  demaster fx <iso> <FFX_Data.vbf> <out dir> all|<id>...\n      effect textures; id: 7, magic_0007, map/bika/bika00, btlmap/bika/bika00_a, chr/mon/m028",
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
        "fx" => {
            if a.len() < 6 {
                usage();
            }
            fx(&a[2], &a[3], Path::new(&a[4]), &a[5..])?;
        }
        _ => bail!("unknown command {cmd} (demaster help)"),
    }
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
    let mut tsv = String::from("path\thd\tps2\tcorrelation\talpha_diff\torientation\n");
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
            writeln!(
                tsv,
                "{}\t{}x{}\t{}x{}\t{diff}\t{flipped}",
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
        ok += 1;
    }
    eprintln!("{ok} converted, {failed} failed");
    Ok(())
}
