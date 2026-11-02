mod bake;
mod chrjob;
mod disc;
mod fxjob;
mod phyretex;
mod ps2chr;
mod ps2fx;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "usage:\n  demaster chr <iso> <FFX_Data.vbf> <out dir> <kind/name>...   bake character textures (PNG previews + .dds.phyre)\n  demaster fx <iso> <FFX_Data.vbf> <out dir> all|<id>...        effect textures; id: 7, magic_0007, map/bika/bika00, btlmap/bika/bika00_a, chr/mon/m028"
    );
    std::process::exit(2)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("no parent")?)?;
    std::fs::write(path, bytes).with_context(|| path.display().to_string())
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let Some(cmd) = a.get(1) else { usage() };
    match cmd.as_str() {
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
                specs = d
                    .names()
                    .filter_map(|n| {
                        let rest = n.strip_prefix("jppc/chr/")?;
                        let mut p = rest.split('/');
                        let (kind, name, mdl, file) = (p.next()?, p.next()?, p.next()?, p.next()?);
                        (mdl == "mdl" && file == format!("{name}.chr"))
                            .then(|| format!("{kind}/{name}"))
                    })
                    .collect();
                specs.sort();
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
        "fx" => {
            if a.len() < 6 {
                usage();
            }
            fx(&a[2], &a[3], Path::new(&a[4]), &a[5..])?;
        }
        _ => bail!("unknown command {cmd}"),
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
