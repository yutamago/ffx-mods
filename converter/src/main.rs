mod bake;
mod build;
mod chrjob;
mod config;
mod disc;
mod jobs;
mod phyretex;
mod ps2chr;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn usage() -> ! {
    eprintln!(
        "FFX Demaster converter {}\nusage:\n{}\n  demaster chr <iso> <FFX_Data.vbf> <out dir> <kind/name>...\n      bake character textures (PNG previews + .dds.phyre)",
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
        _ => bail!("unknown command {cmd} (demaster help)"),
    }
    Ok(())
}
