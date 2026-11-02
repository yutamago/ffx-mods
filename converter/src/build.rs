//! `demaster build`: runs every enabled job into the mod folder, removes what
//! an earlier build wrote and no longer belongs, and writes the build stamp
//! the C# mod checks at startup.
//!
//! Files in the mod folder:
//! - `efl/x/...`: the replacements (Fahrenheit's file loader).
//! - `build/files.json`: every file written, by job, with the job's key. A
//!   job whose key is unchanged and whose files are all there is skipped.
//! - `build/stamp.json`: the settings the files were built with. Deleted
//!   when a build starts, so a missing stamp means "not built" or "a build
//!   was interrupted".

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{AppliedBy, Config, DEFAULT_TOML, Loaded, SETTINGS};
use crate::jobs::{self, Ctx, Output};

/// The converter's version; release builds set `DEMASTER_VERSION` to the tag.
pub const VERSION: &str = match option_env!("DEMASTER_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// The Steam folder name of FINAL FANTASY X/X-2 HD Remaster (app 359870).
const STEAM_DIR: &str = "FINAL FANTASY FFX&FFX-2 HD Remaster";

pub const USAGE: &str = "  demaster build [--config <demaster.toml>] [--iso <disc.iso>] [--game <game folder>]
                 [--mod-dir <mod folder>] [--ffmpeg <ffmpeg program>]
                 [--only <job>,...] [--force] [--verbose]
      builds the PS2 files the settings in demaster.toml ask for
      (without arguments, demaster.exe does this and waits for Enter)
  demaster jobs
      lists the build jobs and their settings";

#[derive(Default)]
pub struct Args {
    pub config: Option<PathBuf>,
    pub iso: Option<PathBuf>,
    pub game: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub mod_dir: Option<PathBuf>,
    /// Run only these jobs (the others keep their files).
    pub only: Vec<String>,
    pub force: bool,
    pub verbose: bool,
}

impl Args {
    pub fn parse(a: &[String]) -> Result<Args> {
        let mut args = Args::default();
        let mut it = a.iter();
        while let Some(flag) = it.next() {
            let mut value = || -> Result<String> {
                Ok(it.next().with_context(|| format!("{flag} needs a value"))?.clone())
            };
            match flag.as_str() {
                "--config" | "-c" => args.config = Some(value()?.into()),
                "--iso" => args.iso = Some(value()?.into()),
                "--game" => args.game = Some(value()?.into()),
                "--ffmpeg" => args.ffmpeg = Some(value()?.into()),
                "--mod-dir" => args.mod_dir = Some(value()?.into()),
                "--only" => args.only.extend(value()?.split(',').map(|s| s.trim().to_string())),
                "--force" | "-f" => args.force = true,
                "--verbose" | "-v" => args.verbose = true,
                other => bail!("unknown option {other}"),
            }
        }
        Ok(args)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    converter_version: String,
    jobs: BTreeMap<String, JobFiles>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct JobFiles {
    /// Empty when the job failed or was interrupted: it reruns next time.
    key: String,
    /// Paths relative to the mod folder (`efl/x/...`) with their sizes.
    files: Vec<(String, u64)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Stamp {
    converter_version: String,
    /// SHA-256 of `key=value\n` for every converter setting, sorted by key.
    config_hash: String,
    /// Seconds since 1970.
    built_at: u64,
    config_path: String,
    /// The converter settings the files were built with (`models.monster` → `ps2`).
    settings: BTreeMap<String, String>,
    /// Their defaults, for keys missing from the file.
    defaults: BTreeMap<String, String>,
    jobs: Vec<StampJob>,
    files: usize,
    /// No job failed.
    complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct StampJob {
    name: String,
    settings: Vec<String>,
    /// `built`, `unchanged`, `off`, `failed`, `not run` (`--only`).
    status: String,
    files: usize,
    summary: String,
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The converter settings and their hash (see [`Stamp::config_hash`]).
fn converter_settings(config: &Config) -> (BTreeMap<String, String>, String) {
    let all = config.settings();
    let s: BTreeMap<String, String> = SETTINGS
        .iter()
        .filter(|(_, by)| *by == AppliedBy::Converter)
        .map(|(k, _)| (k.to_string(), all[*k].clone()))
        .collect();
    let text: String = s.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    (s, sha256_hex(&text))
}

/// Size and modification time: enough to notice another disc or a game update.
fn identity(path: &Path) -> String {
    let m = std::fs::metadata(path);
    let len = m.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = m
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{len}:{mtime}")
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `FFX_Data.vbf` under a game folder, its `data` folder, or the file itself.
fn vbf_in(p: &Path) -> Option<PathBuf> {
    [p.to_path_buf(), p.join("FFX_Data.vbf"), p.join("data").join("FFX_Data.vbf")]
        .into_iter()
        .find(|c| {
            c.is_file()
                && c.file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("FFX_Data.vbf"))
        })
}

/// Steam library folders: the usual Steam installs and the libraries their
/// `libraryfolders.vdf` lists.
fn steam_libraries() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(var) {
                roots.push(PathBuf::from(p).join("Steam"));
            }
        }
        for d in 'C'..='Z' {
            for sub in ["SteamLibrary", "Steam", r"Games\Steam", r"Games\SteamLibrary"] {
                roots.push(PathBuf::from(format!(r"{d}:\{sub}")));
            }
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    let mut libs = Vec::new();
    for root in roots {
        if let Ok(text) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
            for line in text.lines() {
                let mut parts = line.split('"').filter(|s| !s.trim().is_empty());
                if parts.next() == Some("path")
                    && let Some(p) = parts.next()
                {
                    libs.push(PathBuf::from(p.replace(r"\\", r"\")));
                }
            }
        }
        libs.push(root);
    }
    libs.dedup();
    libs
}

/// The game's `FFX_Data.vbf`: the argument, the config, a game folder above
/// the converter, then the Steam libraries.
fn find_vbf(arg: Option<&Path>, loaded: &Loaded) -> Result<PathBuf> {
    if let Some(p) = arg
        .map(Path::to_path_buf)
        .or_else(|| loaded.resolve(&loaded.config.source.game))
    {
        return vbf_in(&p)
            .with_context(|| format!("no data\\FFX_Data.vbf in the game folder {}", p.display()));
    }
    for dir in exe_dir().ancestors() {
        if let Some(v) = vbf_in(dir) {
            return Ok(v);
        }
    }
    for lib in steam_libraries() {
        if let Some(v) = vbf_in(&lib.join("steamapps").join("common").join(STEAM_DIR)) {
            return Ok(v);
        }
    }
    bail!(
        "could not find the game. Set `game` under [source] in {} to the folder with FFX.exe",
        loaded.path.display()
    )
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(path: &Path, v: &T) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("no parent")?)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(&tmp, path).with_context(|| path.display().to_string())
}

/// Deletes a file and the folders it leaves empty, up to `stop`.
fn remove_with_dirs(path: &Path, stop: &Path) {
    let _ = std::fs::remove_file(path);
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d == stop || !d.starts_with(stop) || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

/// Prints the job list (`demaster jobs`).
pub fn list_jobs() {
    for j in jobs::registry() {
        println!("{:<16} {}", j.name(), j.settings().join(", "));
    }
}

/// Runs `demaster build`; returns whether every job succeeded.
pub fn run(args: &Args) -> Result<bool> {
    let started = Instant::now();
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| exe_dir().join("demaster.toml"));
    if !config_path.exists() {
        if args.config.is_some() {
            bail!("{} does not exist", config_path.display());
        }
        std::fs::write(&config_path, DEFAULT_TOML)
            .with_context(|| format!("writing {}", config_path.display()))?;
        bail!(
            "wrote the default settings to {}. Set `iso` under [source] to your PS2 disc image and run again",
            config_path.display()
        );
    }
    let loaded = Config::load(&config_path)?;
    for k in &loaded.unknown {
        eprintln!("warning: {}: unknown setting `{k}`, ignored", config_path.display());
    }
    let config = &loaded.config;

    let iso = args
        .iso
        .clone()
        .or_else(|| loaded.resolve(&config.source.iso))
        .with_context(|| {
            format!(
                "no PS2 disc image: set `iso` under [source] in {}",
                config_path.display()
            )
        })?;
    if !iso.is_file() {
        bail!("the PS2 disc image {} does not exist", iso.display());
    }
    let vbf = find_vbf(args.game.as_deref(), &loaded)?;
    let mod_dir = args
        .mod_dir
        .clone()
        .or_else(|| loaded.resolve(&config.build.mod_dir))
        .unwrap_or_else(exe_dir);
    std::fs::create_dir_all(&mod_dir).with_context(|| mod_dir.display().to_string())?;
    println!("FFX Demaster converter {VERSION}");
    println!("settings: {}", config_path.display());
    println!("disc:     {}", iso.display());
    println!("game:     {}", vbf.display());
    println!("mod:      {}", mod_dir.display());
    if !mod_dir.join("yuta.demaster.manifest.json").exists() {
        println!("note: no yuta.demaster.manifest.json there; is it the mod folder?");
    }
    println!();

    let manifest_path = mod_dir.join("build").join("files.json");
    let stamp_path = mod_dir.join("build").join("stamp.json");
    let old: Manifest = read_json(&manifest_path);
    let _ = std::fs::remove_file(&stamp_path);

    let pc = ffx_vbf::Archive::open(&vbf).with_context(|| format!("opening {}", vbf.display()))?;
    let mut ctx = Ctx::new(config, &pc, &iso, args.verbose);
    ctx.ffmpeg = args.ffmpeg.clone().or_else(|| loaded.resolve(&config.source.ffmpeg));
    ctx.tools_dir = Some(mod_dir.join("tools"));
    let settings = config.settings();
    let inputs = format!("{}|{}", identity(&iso), identity(&vbf));

    let registry = jobs::registry();
    for name in &args.only {
        if !registry.iter().any(|j| j.name() == name) {
            bail!("no job {name} (`demaster jobs` lists them)");
        }
    }
    let mut new = Manifest {
        converter_version: VERSION.to_string(),
        jobs: BTreeMap::new(),
    };
    let mut report = Vec::new();
    let mut all_ok = true;
    for job in &registry {
        let name = job.name().to_string();
        let mut sj = StampJob {
            name: name.clone(),
            settings: job.settings().iter().map(|s| s.to_string()).collect(),
            status: String::new(),
            files: 0,
            summary: String::new(),
        };
        let previous = old.jobs.get(&name);
        if !args.only.is_empty() && !args.only.contains(&name) {
            sj.status = "not run".into();
            if let Some(p) = previous {
                sj.files = p.files.len();
                new.jobs.insert(name, p.clone());
            }
            report.push(sj);
            continue;
        }
        if !job.enabled(config) {
            println!("{name}: off (HD)");
            sj.status = "off".into();
            report.push(sj);
            continue;
        }
        let key = sha256_hex(&format!(
            "{VERSION}|{name}|{}|{}|{inputs}",
            job.version(),
            job.settings()
                .iter()
                .map(|k| format!("{k}={}", settings.get(*k).map_or("", String::as_str)))
                .collect::<Vec<_>>()
                .join(",")
        ));
        if !args.force
            && let Some(p) = previous
            && p.key == key
            && p.files
                .iter()
                .all(|(f, size)| std::fs::metadata(mod_dir.join(f)).is_ok_and(|m| m.len() == *size))
        {
            println!("{name}: unchanged, {} files", p.files.len());
            sj.status = "unchanged".into();
            sj.files = p.files.len();
            new.jobs.insert(name, p.clone());
            report.push(sj);
            continue;
        }

        // Until it finishes, the job's old and new files are on the books
        // without a key, so an interrupted build can clean up after it.
        let mut pending = Manifest {
            converter_version: VERSION.to_string(),
            jobs: old.jobs.clone(),
        };
        pending.jobs.extend(new.jobs.iter().map(|(k, v)| (k.clone(), v.clone())));
        if let Some(e) = pending.jobs.get_mut(&name) {
            e.key.clear();
        }
        write_json(&manifest_path, &pending)?;

        println!("{name}: building ({})", job.settings().join(", "));
        let t = Instant::now();
        let out = Output::new(&mod_dir);
        let result = job.run(&ctx, &out);
        let state = out.finish();
        sj.files = state.files.len();
        match result {
            Ok(summary) => {
                println!(
                    "{name}: {summary}; {} files written, {} unchanged ({:.1} s)",
                    state.written,
                    state.unchanged,
                    t.elapsed().as_secs_f32()
                );
                let taken: BTreeSet<&str> = new
                    .jobs
                    .values()
                    .flat_map(|j| j.files.iter().map(|(f, _)| f.as_str()))
                    .collect();
                let twice = state.files.iter().filter(|(f, _)| taken.contains(f.as_str())).count();
                if twice > 0 {
                    println!("{name}: warning: {twice} files were also written by an earlier job; this job's versions are used");
                }
                sj.status = "built".into();
                sj.summary = summary;
                new.jobs.insert(name, JobFiles { key, files: state.files });
            }
            Err(e) => {
                println!("{name}: FAILED: {e:#}");
                all_ok = false;
                sj.status = "failed".into();
                sj.summary = format!("{e:#}");
                new.jobs.insert(name, JobFiles { key: String::new(), files: state.files });
            }
        }
        report.push(sj);
    }

    // Files of the last build that this one didn't write.
    let keep: BTreeSet<&str> = new
        .jobs
        .values()
        .flat_map(|j| j.files.iter().map(|(f, _)| f.as_str()))
        .collect();
    let efl = mod_dir.join("efl").join("x");
    let mut removed = 0;
    for (f, _) in old.jobs.values().flat_map(|j| j.files.iter()) {
        if !keep.contains(f.as_str()) && f.starts_with("efl/x/") && !f.contains("..") {
            remove_with_dirs(&mod_dir.join(f), &efl);
            removed += 1;
        }
    }
    let files = keep.len();
    write_json(&manifest_path, &new)?;

    let (conv, hash) = converter_settings(config);
    let (defaults, _) = converter_settings(&Config::default());
    let covered: BTreeSet<&str> = registry
        .iter()
        .flat_map(|j| j.settings().iter().copied())
        .collect();
    let missing: Vec<String> = conv
        .iter()
        .filter(|(k, v)| v.as_str() != "hd" && !covered.contains(k.as_str()))
        .map(|(k, _)| k.clone())
        .collect();
    let stamp = Stamp {
        converter_version: VERSION.to_string(),
        config_hash: hash,
        built_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        config_path: config_path.display().to_string(),
        settings: conv,
        defaults,
        jobs: report,
        files,
        complete: all_ok,
    };
    write_json(&stamp_path, &stamp)?;

    println!();
    println!("{files} files in {}, {removed} old files removed", efl.display());
    if !missing.is_empty() {
        println!("not built yet (no converter job): {}", missing.join(", "));
    }
    if !all_ok {
        println!("some jobs failed; the game keeps the HD files for those");
    }
    println!(
        "done in {:.0} s. Restart the game to see the changes.",
        started.elapsed().as_secs_f32()
    );
    Ok(all_ok)
}
