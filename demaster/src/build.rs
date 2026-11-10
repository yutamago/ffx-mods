//! `demaster build`: runs every enabled job into the mod folder, removes what
//! an earlier build wrote and no longer belongs, and writes the build stamp
//! the C# mod checks at startup. For a mod of ffx-mods (`--mod`, or the
//! mod whose manifest is in the folder, `mods.rs`) only that mod's jobs and
//! settings, from its `settings.toml`; without one, every job, from
//! `demaster.toml` (development: everything in one folder). `--all`: every
//! installed mod, each as with `--mod`, sharing one opened disc.
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
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{Config, DEFAULT_TOML, Loaded};
use crate::jobs::{self, Ctx, Output};
use crate::mods::{self, DiscUse, Profile};

/// The converter's version; release builds set `DEMASTER_VERSION` to the tag.
pub const VERSION: &str = match option_env!("DEMASTER_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// The Steam folder name of FINAL FANTASY X/X-2 HD Remaster (app 359870).
const STEAM_DIR: &str = "FINAL FANTASY FFX&FFX-2 HD Remaster";

pub const USAGE: &str = "  demaster build --all [--mods-dir <Fahrenheit mods folder>] [--iso <disc.iso>|<drive>:]
                 [--no-disc] [--game <game folder>] [--ffmpeg <ffmpeg program>] [--force] [--verbose]
      builds every installed mod of ffx-mods that has PS2 files (listed in mods\\loadorder),
      each into its own folder with its own settings.toml, from one pass over the disc.
      The disc and game come from the arguments, else from the first mod's settings that
      has them, and are saved in every mod's settings. Without --mods-dir: the folder
      above demaster.exe's.
  demaster build [--mod <name>] [--config <settings.toml>] [--iso <disc.iso>|<drive>:]
                 [--no-disc] [--game <game folder>] [--save-source] [--mod-dir <mod folder>]
                 [--ffmpeg <ffmpeg program>] [--only <job>,...] [--force] [--verbose]
      builds the PS2 files the settings ask for: of one mod of ffx-mods (--mod, or the
      mod whose manifest is in the mod folder; settings.toml there), else of every job
      (demaster.toml). --iso takes a disc image or the drive with the PS2 disc (E:).
      --no-disc leaves out the jobs that need the disc. --save-source writes --iso and
      --game into the settings file. Without arguments, demaster.exe builds the mod
      folder it is in and waits for Enter.
  demaster jobs [--mod <name>]
      lists the build jobs and their settings
  demaster mods
      lists the mods of ffx-mods with their settings and jobs
  demaster find-game [--config <settings.toml>] [--game <game folder>]
      prints the game folder (the one with FFX.exe) the build would use
  demaster disc-check <disc.iso>|<drive>:
      checks that it is a Final Fantasy X PS2 disc";

#[derive(Default)]
pub struct Args {
    pub config: Option<PathBuf>,
    pub iso: Option<PathBuf>,
    pub game: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub mod_dir: Option<PathBuf>,
    /// The mod of ffx-mods to build (`mods.rs`).
    pub mod_name: Option<String>,
    /// Build every installed mod (`--all`), in `mods_dir`.
    pub all: bool,
    /// Fahrenheit's `mods` folder, for `--all`.
    pub mods_dir: Option<PathBuf>,
    /// Build without the PS2 disc: jobs that need it are left out.
    pub no_disc: bool,
    /// Write `--iso` and `--game` into the settings file's `[source]`.
    pub save_source: bool,
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
                "--mod" => args.mod_name = Some(value()?),
                "--all" => args.all = true,
                "--mods-dir" => args.mods_dir = Some(value()?.into()),
                "--no-disc" => args.no_disc = true,
                "--save-source" => args.save_source = true,
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
    /// Settings of other mods the jobs read, by key.
    #[serde(default)]
    foreign: BTreeMap<String, StampForeign>,
    jobs: Vec<StampJob>,
    files: usize,
    /// No job failed.
    complete: bool,
}

/// A setting of another mod the build read ([`mods::Foreign`]).
#[derive(Debug, Serialize, Deserialize)]
struct StampForeign {
    /// The mod it belongs to (`yuta.ps2-aspect-ratio`).
    #[serde(rename = "mod")]
    mod_id: String,
    /// The value the files were built with.
    value: String,
    /// The value when that mod is installed and its settings.toml leaves the key out.
    default: String,
    /// The value when that mod is not installed.
    absent: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StampJob {
    name: String,
    settings: Vec<String>,
    /// `built`, `unchanged`, `off`, `failed`, `not run` (`--only`), `no disc`
    /// (needs the disc, the build had none), `kept` (likewise, files of an
    /// earlier build kept).
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

/// The converter settings (of the mod, when there is one) and their hash
/// (see [`Stamp::config_hash`]).
fn converter_settings(config: &Config, profile: Option<&Profile>) -> (BTreeMap<String, String>, String) {
    let all = config.settings();
    let s: BTreeMap<String, String> = all
        .iter()
        .filter(|(k, _)| mods::is_build_setting(k) && profile.is_none_or(|p| p.settings().contains(&k.as_str())))
        .map(|(k, v)| (k.clone(), v.clone()))
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
/// the converter (`<game>/mods/<mod>/demaster.exe`), then the Steam libraries.
pub fn find_vbf(arg: Option<&Path>, loaded: &Loaded) -> Result<PathBuf> {
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

/// The mods Fahrenheit loads from `mods_dir`: the ids in its `loadorder`.
fn load_order(mods_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(mods_dir.join("loadorder"))
        .map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// The folder of the mod `p` in `mods_dir` when it is installed: its folder
/// with its manifest, listed in `mods\loadorder` (Fahrenheit loads only those).
pub fn installed(mods_dir: &Path, p: &Profile) -> Option<PathBuf> {
    let dir = mods_dir.join(p.id());
    (dir.join(format!("{}.manifest.json", p.id())).is_file() && load_order(mods_dir).contains(&p.id())).then_some(dir)
}

/// The value of another mod's setting as the game will have it: from that
/// mod's settings.toml (over its defaults) when it is installed next to
/// `mod_dir`, else `f.absent`. Returns the value and whether it is installed.
fn foreign_value(mod_dir: &Path, f: &mods::Foreign) -> Result<(String, bool)> {
    let owner = mods::find(f.from).with_context(|| format!("no mod {}", f.from))?;
    let Some(dir) = mod_dir.parent().and_then(|m| installed(m, owner)) else {
        return Ok((f.absent.to_string(), false));
    };
    let path = dir.join("settings.toml");
    let config = if path.is_file() {
        Config::load_with_defaults(&path, Some(owner.default_toml))?.config
    } else {
        Config::parse_with_defaults("", owner.default_toml)?.0
    };
    Ok((config.settings()[f.key].clone(), true))
}

/// Prints the job list (`demaster jobs [--mod <name>]`).
pub fn list_jobs(a: &[String]) -> Result<()> {
    let profile = match a {
        [] => None,
        [flag, name] if flag == "--mod" => Some(mods::find(name).with_context(|| format!("no mod {name} (`demaster mods` lists them)"))?),
        _ => bail!("usage: demaster jobs [--mod <name>]"),
    };
    for j in registry_for(profile) {
        println!("{:<24} {}{}", j.name(), j.settings().join(", "), if j.needs_disc() { "" } else { " (no disc needed)" });
    }
    Ok(())
}

/// Prints the mods (`demaster mods`).
pub fn list_mods() {
    for p in mods::PROFILES {
        let disc = match p.disc {
            DiscUse::Needed => "needs the PS2 disc",
            DiscUse::Optional => "PS2 disc optional",
            DiscUse::None => "code only",
        };
        println!("{} ({}, {disc})", p.id(), p.title);
        println!("    settings: {}", p.settings().join(", "));
        if !p.jobs.is_empty() {
            println!("    jobs:     {}", p.jobs.join(", "));
        }
    }
}

/// The jobs of a mod, in registry order; every job without one.
fn registry_for(profile: Option<&Profile>) -> Vec<Box<dyn jobs::Job>> {
    let mut all = jobs::registry();
    if let Some(p) = profile {
        all.retain(|j| p.jobs.contains(&j.name()));
    }
    all
}

/// `demaster find-game [--config <file>] [--game <folder>]`: prints the game
/// folder (the parent of `data\FFX_Data.vbf`), or fails.
pub fn find_game(a: &[String]) -> Result<()> {
    let args = Args::parse(a)?;
    let config_path = args.config.clone().unwrap_or_else(|| exe_dir().join("settings.toml"));
    let loaded = if config_path.is_file() {
        Config::load(&config_path)?
    } else {
        Loaded { config: Config::default(), path: config_path, unknown: Vec::new(), text: String::new() }
    };
    let vbf = find_vbf(args.game.as_deref(), &loaded)?;
    let game = vbf.parent().and_then(Path::parent).context("game folder")?;
    println!("{}", game.display());
    Ok(())
}

/// What `build --all` opens once for all its mods: the game's archive and the disc.
struct Shared {
    vbf: PathBuf,
    pc: ffx_vbf::Archive,
    iso: Option<PathBuf>,
    disc: Arc<Mutex<Option<crate::disc::Disc>>>,
    disc_id: Mutex<Option<String>>,
}

/// Runs `demaster build`; returns whether every job succeeded.
pub fn run(args: &Args) -> Result<bool> {
    if args.all {
        return run_all(args);
    }
    run_one(args, None)
}

/// One mod (or, without one, every job into one folder).
fn run_one(args: &Args, shared: Option<&Shared>) -> Result<bool> {
    let started = Instant::now();
    let base_dir = args.mod_dir.clone().unwrap_or_else(exe_dir);
    let profile = match &args.mod_name {
        Some(n) => Some(mods::find(n).with_context(|| format!("no mod {n} (`demaster mods` lists them)"))?),
        None => mods::in_dir(&base_dir),
    };
    let config_path = args.config.clone().unwrap_or_else(|| match profile {
        Some(_) => base_dir.join("settings.toml"),
        None => exe_dir().join("demaster.toml"),
    });
    if let Some(p) = profile
        && p.disc == DiscUse::None
    {
        println!("{} ({}) has no files to build: it is code only. Settings: {}", p.id(), p.title, config_path.display());
        return Ok(true);
    }
    if !config_path.exists() {
        if args.config.is_some() {
            bail!("{} does not exist", config_path.display());
        }
        std::fs::write(&config_path, profile.map_or(DEFAULT_TOML, |p| p.default_toml))
            .with_context(|| format!("writing {}", config_path.display()))?;
        if profile.is_none() {
            bail!(
                "wrote the default settings to {}. Set `iso` under [source] to your PS2 disc image and run again",
                config_path.display()
            );
        }
        println!("wrote the default settings to {}", config_path.display());
    }
    if args.save_source {
        let mut text = std::fs::read_to_string(&config_path)?;
        let old = text.clone();
        if let Some(iso) = &args.iso {
            text = crate::config::set_source(&text, "iso", &iso.to_string_lossy());
        }
        if let Some(game) = &args.game {
            text = crate::config::set_source(&text, "game", &game.to_string_lossy());
        }
        if text != old {
            std::fs::write(&config_path, &text).with_context(|| format!("writing {}", config_path.display()))?;
            println!("saved the disc and game folder in {}", config_path.display());
        }
    }
    let loaded = Config::load_with_defaults(&config_path, profile.map(|p| p.default_toml))?;
    let mut unknown = loaded.unknown.clone();
    if let Some(p) = profile {
        for k in Config::present_keys(&loaded.text)? {
            if !k.starts_with("source.") && !k.starts_with("build.") && !p.settings().contains(&k.as_str()) && !unknown.contains(&k) {
                unknown.push(k);
            }
        }
    }
    for k in &unknown {
        eprintln!("warning: {}: unknown setting `{k}`, ignored", config_path.display());
    }
    let config = &loaded.config;

    // The disc: an image or a drive (`E:`), from --iso or the settings.
    let iso_text: Option<String> = args
        .iso
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|| Some(config.source.iso.trim().to_string()).filter(|s| !s.is_empty()));
    let optional = profile.is_some_and(|p| p.disc == DiscUse::Optional);
    let no_disc = args.no_disc || (iso_text.is_none() && optional);
    let iso: Option<PathBuf> = if no_disc {
        None
    } else {
        let text = iso_text.with_context(|| {
            format!(
                "no PS2 disc: set `iso` under [source] in {} to your disc image or drive (run install.cmd)",
                config_path.display()
            )
        })?;
        let p = crate::disc::source_path(&text);
        let p = if crate::disc::is_device(&p) || args.iso.is_some() { p } else { loaded.resolve(&text).unwrap_or(p) };
        if !crate::disc::exists(&p) {
            if crate::disc::is_device(&p) {
                bail!("no disc in the drive {} (or it can't be read)", text);
            }
            bail!("the PS2 disc image {} does not exist", p.display());
        }
        Some(p)
    };
    let vbf = find_vbf(args.game.as_deref(), &loaded)?;
    let mod_dir = args
        .mod_dir
        .clone()
        .or_else(|| loaded.resolve(&config.build.mod_dir))
        .unwrap_or_else(exe_dir);
    std::fs::create_dir_all(&mod_dir).with_context(|| mod_dir.display().to_string())?;

    // Settings of other mods the jobs read, as the game will have them.
    let mut built_config = loaded.config.clone();
    let mut foreign = BTreeMap::new();
    let mut foreign_lines = Vec::new();
    for f in profile.map_or(&[][..], |p| p.reads) {
        let (value, there) = foreign_value(&mod_dir, f)?;
        built_config.set(f.key, &value)?;
        let owner = mods::find(f.from).context("owner")?;
        let default = owner.defaults()[f.key].clone();
        foreign_lines.push(if there {
            format!("{} = {value} (from {})", f.key, owner.id())
        } else {
            format!("{} = {value} ({} is not installed)", f.key, owner.id())
        });
        foreign.insert(
            f.key.to_string(),
            StampForeign { mod_id: owner.id(), value, default, absent: f.absent.to_string() },
        );
    }
    let config = &built_config;
    println!("demaster, the FFX PS2 mods converter, {VERSION}");
    if let Some(p) = profile {
        println!("mod:      {} ({})", p.id(), p.title);
    }
    println!("settings: {}", config_path.display());
    match &iso {
        Some(i) if crate::disc::is_device(i) => println!("disc:     {} (disc drive)", i.display()),
        Some(i) => println!("disc:     {}", i.display()),
        None => println!(
            "disc:     none{}",
            profile.map_or(String::new(), |p| format!(": {}", p.without_disc))
        ),
    }
    println!("game:     {}", vbf.display());
    println!("folder:   {}", mod_dir.display());
    for l in &foreign_lines {
        println!("reads:    {l}");
    }
    let manifest = format!("{}.manifest.json", profile.map_or("yuta.demaster".to_string(), |p| p.id()));
    if !mod_dir.join(&manifest).exists() {
        println!("note: no {manifest} there; is it the mod folder?");
    }
    println!();

    let manifest_path = mod_dir.join("build").join("files.json");
    let stamp_path = mod_dir.join("build").join("stamp.json");
    let old: Manifest = read_json(&manifest_path);
    let _ = std::fs::remove_file(&stamp_path);

    let own_pc;
    let pc = match shared {
        Some(sh) if sh.vbf == vbf => &sh.pc,
        _ => {
            own_pc = ffx_vbf::Archive::open(&vbf).with_context(|| format!("opening {}", vbf.display()))?;
            &own_pc
        }
    };
    let shared = shared.filter(|sh| sh.vbf == vbf && sh.iso == iso);
    let mut ctx = match &iso {
        Some(i) => Ctx::new(config, pc, i, args.verbose),
        None => Ctx::without_disc(config, pc, args.verbose),
    };
    if let Some(sh) = shared {
        ctx.share_disc(sh.disc.clone());
    }
    ctx.ffmpeg = args.ffmpeg.clone().or_else(|| loaded.resolve(&config.source.ffmpeg));
    ctx.tools_dir = Some(mod_dir.join("tools"));
    ctx.vbf = Some(vbf.clone());
    let settings = config.settings();
    let registry = registry_for(profile);
    // The inputs in a job's key: the game's archive, and the disc (by its
    // contents, so its image and the disc in a drive are the same) for jobs
    // that read it.
    let vbf_id = identity(&vbf);
    let needs_disc = registry.iter().any(|j| j.needs_disc() && j.enabled(config));
    let disc_id = match &iso {
        Some(i) if needs_disc => match shared {
            Some(sh) => {
                let mut id = sh.disc_id.lock().expect("disc id lock");
                if id.is_none() {
                    *id = Some(crate::disc::identity(i)?);
                }
                id.clone().unwrap_or_default()
            }
            None => crate::disc::identity(i)?,
        },
        _ => String::new(),
    };

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
        if iso.is_none() && job.needs_disc() {
            match previous {
                Some(p) if !p.files.is_empty() => {
                    println!("{name}: needs the PS2 disc; the files of the last build are kept");
                    sj.status = "kept".into();
                    sj.files = p.files.len();
                    new.jobs.insert(name, p.clone());
                }
                _ => {
                    println!(
                        "{name}: needs the PS2 disc, none given: not built{}",
                        profile.map_or(String::new(), |p| format!(" ({})", p.without_disc))
                    );
                    sj.status = "no disc".into();
                }
            }
            report.push(sj);
            continue;
        }
        let inputs = if job.needs_disc() { format!("{disc_id}|{vbf_id}") } else { vbf_id.clone() };
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
        if keep.contains(f.as_str()) || f.contains("..") {
            continue;
        }
        if f.starts_with("efl/x/") {
            remove_with_dirs(&mod_dir.join(f), &efl);
            removed += 1;
        } else if let Some(d) = crate::jobs::MOD_DIRS.iter().find(|d| f.starts_with(**d)) {
            remove_with_dirs(&mod_dir.join(f), &mod_dir.join(d.trim_end_matches('/')));
            removed += 1;
        }
    }
    let files = keep.len();
    write_json(&manifest_path, &new)?;

    let (conv, hash) = converter_settings(config, profile);
    let default_config = match profile {
        Some(p) => Config::parse_with_defaults("", p.default_toml)?.0,
        None => Config::default(),
    };
    let (defaults, _) = converter_settings(&default_config, profile);
    let covered: BTreeSet<&str> = registry
        .iter()
        .flat_map(|j| j.settings().iter().copied())
        .collect();
    let missing: Vec<String> = conv
        .iter()
        .filter(|(k, v)| v.as_str() != "hd" && !covered.contains(k.as_str()))
        .map(|(k, _)| k.clone())
        .collect();
    let left_out = report.iter().any(|j| j.status == "no disc");
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
        foreign,
        jobs: report,
        files,
        complete: all_ok,
    };
    write_json(&stamp_path, &stamp)?;

    println!();
    println!("{files} files in {}, {removed} old files removed", mod_dir.display());
    if !missing.is_empty() {
        println!("not built yet (no converter job): {}", missing.join(", "));
    }
    if !all_ok {
        println!("some jobs failed; the game keeps the HD files for those");
    }
    if left_out {
        println!(
            "built without the PS2 disc{}. Run again with the disc to build the rest.",
            profile.map_or(String::new(), |p| format!(": {}", p.without_disc))
        );
    }
    println!(
        "done in {:.0} s. Restart the game to see the changes.",
        started.elapsed().as_secs_f32()
    );
    Ok(all_ok)
}

/// `demaster build --all`: every installed mod with PS2 files, each into its
/// own folder with its own settings, sharing one opened game archive and one
/// opened disc. Returns whether every mod built.
fn run_all(args: &Args) -> Result<bool> {
    let started = Instant::now();
    if args.mod_name.is_some() || args.config.is_some() || args.mod_dir.is_some() || !args.only.is_empty() {
        bail!("--all builds every installed mod with its own settings: no --mod, --config, --mod-dir or --only with it");
    }
    let mods_dir = match &args.mods_dir {
        Some(d) => d.clone(),
        None => exe_dir().parent().map(Path::to_path_buf).context("the mods folder")?,
    };
    let targets: Vec<(&Profile, PathBuf)> = mods::PROFILES
        .iter()
        .filter(|p| p.disc != DiscUse::None)
        .filter_map(|p| installed(&mods_dir, p).map(|d| (p, d)))
        .collect();
    if targets.is_empty() {
        bail!(
            "no mod with PS2 files is installed in {} (a mod folder with its manifest, listed in loadorder)",
            mods_dir.display()
        );
    }
    println!("demaster, the FFX PS2 mods converter, {VERSION}");
    println!("mods:     {}", mods_dir.display());
    println!("building: {}", targets.iter().map(|(p, _)| p.id()).collect::<Vec<_>>().join(", "));

    // The disc and the game: the arguments, else the first mod's settings that name them.
    let mut iso: Option<PathBuf> = args.iso.clone();
    let mut game: Option<PathBuf> = args.game.clone();
    let mut first: Option<Loaded> = None;
    for (p, d) in &targets {
        let path = d.join("settings.toml");
        if !path.is_file() {
            continue;
        }
        let loaded = Config::load_with_defaults(&path, Some(p.default_toml))?;
        let text = loaded.config.source.iso.trim().to_string();
        if iso.is_none() && !args.no_disc && !text.is_empty() {
            let s = crate::disc::source_path(&text);
            iso = Some(if crate::disc::is_device(&s) { s } else { loaded.resolve(&text).unwrap_or(s) });
        }
        if game.is_none() {
            game = loaded.resolve(&loaded.config.source.game);
        }
        first.get_or_insert(loaded);
    }
    if args.no_disc {
        iso = None;
    }
    let needs: Vec<String> = targets.iter().filter(|(p, _)| p.disc == DiscUse::Needed).map(|(p, _)| p.id()).collect();
    if iso.is_none() && !args.no_disc && !needs.is_empty() {
        bail!(
            "no PS2 disc: pass --iso <disc image or drive> (or --no-disc to build only what needs none; {} need it)",
            needs.join(", ")
        );
    }
    if let Some(i) = &iso
        && !crate::disc::exists(i)
    {
        bail!("the PS2 disc {} can't be read (no such image, or no disc in the drive)", i.display());
    }
    let first = first.unwrap_or_else(|| Loaded {
        config: Config::default(),
        path: mods_dir.join("settings.toml"),
        unknown: Vec::new(),
        text: String::new(),
    });
    let vbf = find_vbf(game.as_deref(), &first)?;
    let pc = ffx_vbf::Archive::open(&vbf).with_context(|| format!("opening {}", vbf.display()))?;
    let shared = Shared {
        vbf: vbf.clone(),
        pc,
        iso: iso.clone(),
        disc: Arc::new(Mutex::new(None)),
        disc_id: Mutex::new(None),
    };

    let mut results = Vec::new();
    let mut all_ok = true;
    for (p, dir) in &targets {
        println!();
        println!("==== {} ({})", p.id(), p.title);
        if iso.is_none() && p.disc == DiscUse::Needed {
            println!("{}: needs the PS2 disc: not built", p.id());
            results.push((p.id(), "not built (needs the PS2 disc)".to_string()));
            continue;
        }
        let a = Args {
            config: Some(dir.join("settings.toml")),
            iso: iso.clone(),
            game: game.clone(),
            ffmpeg: args.ffmpeg.clone(),
            mod_dir: Some(dir.clone()),
            mod_name: Some(p.name.to_string()),
            all: false,
            mods_dir: None,
            no_disc: iso.is_none(),
            save_source: true,
            only: Vec::new(),
            force: args.force,
            verbose: args.verbose,
        };
        if !a.config.as_ref().is_some_and(|c| c.is_file()) {
            std::fs::write(dir.join("settings.toml"), p.default_toml)?;
        }
        match run_one(&a, Some(&shared)) {
            Ok(true) => results.push((p.id(), "ok".to_string())),
            Ok(false) => {
                all_ok = false;
                results.push((p.id(), "some jobs FAILED".to_string()));
            }
            Err(e) => {
                all_ok = false;
                println!("{}: FAILED: {e:#}", p.id());
                results.push((p.id(), format!("FAILED: {e:#}")));
            }
        }
    }
    println!();
    println!("==== all mods");
    for (id, r) in &results {
        println!("{id:<26} {r}");
    }
    println!("done in {:.0} s. Restart the game to see the changes.", started.elapsed().as_secs_f32());
    Ok(all_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// render.aspect for PS2 movies: PS2 aspect ratio's value when it is
    /// installed (folder, manifest, loadorder), else "hd".
    #[test]
    fn foreign_setting_follows_the_installed_mod() {
        let mods_dir = std::env::temp_dir().join(format!("demaster-foreign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mods_dir);
        let movies = mods_dir.join("yuta.ps2-movies");
        let aspect = mods_dir.join("yuta.ps2-aspect-ratio");
        std::fs::create_dir_all(&movies).unwrap();
        let read = mods::find("ps2-movies").unwrap().reads;
        let value = || foreign_value(&movies, &read[0]).unwrap();
        assert_eq!(value(), ("hd".to_string(), false));
        std::fs::create_dir_all(&aspect).unwrap();
        std::fs::write(aspect.join("yuta.ps2-aspect-ratio.manifest.json"), "{}").unwrap();
        // not in loadorder: Fahrenheit doesn't load it
        assert_eq!(value(), ("hd".to_string(), false));
        std::fs::write(mods_dir.join("loadorder"), "yuta.ffx-mods\r\nyuta.ps2-aspect-ratio\r\n").unwrap();
        // no settings.toml: its default
        assert_eq!(value(), ("ps2".to_string(), true));
        std::fs::write(aspect.join("settings.toml"), "[render]\naspect = \"hd\"\n").unwrap();
        assert_eq!(value(), ("hd".to_string(), true));
        let _ = std::fs::remove_dir_all(&mods_dir);
    }
}
