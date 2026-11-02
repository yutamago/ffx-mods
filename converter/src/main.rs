mod bake;
mod build;
mod chrjob;
mod config;
mod disc;
mod gsmem;
mod jobs;
mod mapjob;
mod phyretex;
mod ps2chr;
mod ps2map;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "FFX Demaster converter {}\nusage:\n{}\n  demaster chr <iso> <FFX_Data.vbf> <out dir> <kind/name>...\n      bake character textures (PNG previews + .dds.phyre)\n  demaster maps <iso> <FFX_Data.vbf> <out dir> <map|area|map|btlmap>...|all\n      field and battle map textures (PNG previews, maps.tsv + .dds.phyre)",
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
