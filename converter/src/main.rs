mod bake;
mod chrjob;
mod disc;
mod glb;
mod models;
mod phyretex;
mod ps2chr;
mod ps2mesh;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "usage:
  demaster chr <iso> <FFX_Data.vbf> <out dir> all|<kind/name>...      bake character textures (PNG previews + .dds.phyre)
  demaster models <iso> <FFX_Data.vbf> <out dir> all|<kind/name>...   PS2 meshes into the remodelled characters (run after chr)
  demaster models-debug <chr file> <FFX_Data.vbf> <kind/name>          print a PS2 mesh against its HD model"
    );
    std::process::exit(2)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("no parent")?)?;
    std::fs::write(path, bytes).with_context(|| path.display().to_string())
}

/// Every character on the disc, as `kind/name`.
fn all_characters(d: &disc::Disc) -> Vec<String> {
    let mut specs: Vec<String> = d
        .names()
        .filter_map(|n| {
            let rest = n.strip_prefix("jppc/chr/")?;
            let mut p = rest.split('/');
            let (kind, name, mdl, file) = (p.next()?, p.next()?, p.next()?, p.next()?);
            (mdl == "mdl" && file == format!("{name}.chr")).then(|| format!("{kind}/{name}"))
        })
        .collect();
    specs.sort();
    specs
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
                specs = all_characters(&d);
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
            let pc = ffx_vbf::Archive::open(&a[3])?;
            let mut d = disc::Disc::open(Path::new(&a[2]), &pc)?;
            eprintln!("disc: region {}, {} files", d.region, d.file_count());
            let out = PathBuf::from(&a[4]);
            let mut specs: Vec<String> = a[5..].to_vec();
            if specs == ["all"] {
                // the characters with pages the texture bake skips
                specs.clear();
                for spec in all_characters(&d) {
                    let (kind, name) = spec.split_once('/').context("kind/name")?;
                    if let Ok(Some(r)) = chrjob::skipped(&mut d, &pc, kind, name)
                        && !r.skipped.is_empty()
                    {
                        specs.push(spec);
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
        _ => bail!("unknown command {cmd}"),
    }
    Ok(())
}
