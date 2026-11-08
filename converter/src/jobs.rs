//! The jobs `demaster build` runs. A job turns part of the PS2 disc into
//! files for the mod folder and is switched by one or more settings.
//!
//! Adding a job: implement [`Job`] in its module and add the module's jobs to
//! [`registry`]. `build.rs` does the rest (skipping unchanged jobs, the file
//! manifest, removing files of jobs that are off, the build stamp).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::disc::Disc;

/// Every job, in the order they run.
pub fn registry() -> Vec<Box<dyn Job>> {
    let mut jobs: Vec<Box<dyn Job>> = Vec::new();
    jobs.extend(crate::chrjob::jobs());
    jobs.extend(crate::mapjob::jobs());
    jobs.extend(crate::bgjob::jobs());
    jobs.extend(crate::fxjob::jobs());
    jobs.extend(crate::uijob::jobs());
    jobs.extend(crate::models::jobs());
    jobs.extend(crate::moviejob::jobs());
    jobs.extend(crate::musicjob::jobs());
    jobs.extend(crate::livejob::jobs());
    jobs.extend(crate::shaderjob::jobs());
    jobs.extend(crate::eventlight::jobs());
    jobs
}

/// Folders of the mod (besides `efl/x`) that jobs write: `music/` holds the
/// live music engine's PS2 files (`livejob.rs`).
pub const MOD_DIRS: &[&str] = &["music/"];

pub trait Job: Sync {
    /// Unique and stable (`chr.monster`); names the job in the manifest.
    fn name(&self) -> &'static str;
    /// The settings it reads (`models.monster`). Its output may depend only
    /// on these, the converter version, [`Job::version`] and the disc and
    /// game files; a change in any of them reruns the job.
    fn settings(&self) -> &'static [&'static str];
    /// Whether it runs under this config. A job that is off writes nothing,
    /// and the files it wrote before are removed.
    fn enabled(&self, config: &Config) -> bool;
    /// Bump when the job's output changes without a converter version bump.
    fn version(&self) -> u32 {
        1
    }
    /// Whether it reads the PS2 disc. A job that doesn't (it builds from the
    /// PC game alone) also runs in a build without the disc, and its key
    /// leaves the disc out.
    fn needs_disc(&self) -> bool {
        true
    }
    /// Writes the job's files through `out`. Returns a one-line summary.
    /// Per-item problems belong in the summary or in `ctx.note`, not in an
    /// error; an error marks the whole job failed.
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String>;
}

/// What a job reads from.
pub struct Ctx<'a> {
    /// For jobs whose output depends on a setting's value beyond on/off.
    #[allow(dead_code)]
    pub config: &'a Config,
    /// `FFX_Data.vbf`.
    pub pc: &'a ffx_vbf::Archive,
    /// The disc image or drive volume (`disc::source_path`); `None` in a build without the disc.
    iso: Option<PathBuf>,
    disc: Mutex<Option<Disc>>,
    /// Print per-item details.
    pub verbose: bool,
    /// ffmpeg, from `--ffmpeg` or `source.ffmpeg`; `None`: look for it
    /// (`moviejob::find_ffmpeg`).
    pub ffmpeg: Option<PathBuf>,
    /// Where a downloaded ffmpeg goes (`<mod>/tools`); `None`: no download.
    pub tools_dir: Option<PathBuf>,
    /// The path of `FFX_Data.vbf`, for jobs that read part of a big file.
    pub vbf: Option<PathBuf>,
}

impl<'a> Ctx<'a> {
    pub fn new(config: &'a Config, pc: &'a ffx_vbf::Archive, iso: &Path, verbose: bool) -> Self {
        Ctx {
            config,
            pc,
            iso: Some(iso.to_path_buf()),
            disc: Mutex::new(None),
            verbose,
            ffmpeg: None,
            tools_dir: None,
            vbf: None,
        }
    }

    /// For a build without the PS2 disc: [`Ctx::disc`] fails.
    pub fn without_disc(config: &'a Config, pc: &'a ffx_vbf::Archive, verbose: bool) -> Self {
        let mut ctx = Ctx::new(config, pc, Path::new(""), verbose);
        ctx.iso = None;
        ctx
    }

    /// The PS2 disc, opened on first use (a build where every job is
    /// unchanged never opens it).
    pub fn disc<T>(&self, f: impl FnOnce(&mut Disc) -> Result<T>) -> Result<T> {
        let mut d = self.disc.lock().expect("disc lock");
        if d.is_none() {
            let iso = self.iso.as_deref().context("this build has no PS2 disc")?;
            let disc = Disc::open(iso, self.pc)?;
            eprintln!("disc: {} (region {}, {} files)", iso.display(), disc.region, disc.file_count());
            *d = Some(disc);
        }
        f(d.as_mut().expect("opened"))
    }

    /// A detail line, shown with `--verbose`.
    pub fn note(&self, line: impl AsRef<str>) {
        if self.verbose {
            println!("    {}", line.as_ref());
        }
    }
}

/// Where a job's files go: `<mod>/efl/x/<archive path>`. Safe to use from
/// several threads.
pub struct Output {
    efl: PathBuf,
    state: Mutex<OutState>,
}

#[derive(Default)]
pub struct OutState {
    /// Paths relative to the mod folder (`efl/x/...`) with their sizes.
    pub files: Vec<(String, u64)>,
    pub written: usize,
    pub unchanged: usize,
    taken: std::collections::HashSet<String>,
}

impl Output {
    pub fn new(mod_dir: &Path) -> Self {
        Output {
            efl: mod_dir.join("efl/x"),
            state: Mutex::new(OutState::default()),
        }
    }

    /// Puts a replacement for the game file at `archive_path`
    /// (`ffx_data/gamedata/...`, as stored in `FFX_Data.vbf`). The first
    /// put of a path wins; returns false for later ones, which are ignored.
    /// A file already on disk with the same bytes is left alone.
    pub fn put(&self, archive_path: &str, bytes: &[u8]) -> Result<bool> {
        let rel = archive_path.trim_start_matches('/').to_ascii_lowercase();
        anyhow::ensure!(
            !rel.split('/').any(|p| p == ".." || p.is_empty()),
            "bad archive path {archive_path}"
        );
        let key = format!("efl/x/{rel}");
        if !self.state.lock().expect("output lock").taken.insert(key.clone()) {
            return Ok(false);
        }
        let path = self.efl.join(&rel);
        let same = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64)
            && std::fs::read(&path).is_ok_and(|old| old == bytes);
        if !same {
            std::fs::create_dir_all(path.parent().context("no parent")?)?;
            std::fs::write(&path, bytes).with_context(|| path.display().to_string())?;
        }
        let mut s = self.state.lock().expect("output lock");
        if same {
            s.unchanged += 1;
        } else {
            s.written += 1;
        }
        s.files.push((key, bytes.len() as u64));
        Ok(true)
    }

    /// Where [`Output::put_file`] wants a file for `archive_path` written
    /// first (next to its final place, so the move is a rename).
    pub fn temp_path(&self, archive_path: &str) -> Result<PathBuf> {
        let rel = archive_path.trim_start_matches('/').to_ascii_lowercase();
        let path = self.efl.join(format!("{rel}.part"));
        std::fs::create_dir_all(path.parent().context("no parent")?)?;
        Ok(path)
    }

    /// Like [`Output::put`] for a large file already written at `temp`
    /// (from [`Output::temp_path`]): moved into place.
    pub fn put_file(&self, archive_path: &str, temp: &Path) -> Result<bool> {
        let rel = archive_path.trim_start_matches('/').to_ascii_lowercase();
        anyhow::ensure!(
            !rel.split('/').any(|p| p == ".." || p.is_empty()),
            "bad archive path {archive_path}"
        );
        let key = format!("efl/x/{rel}");
        if !self.state.lock().expect("output lock").taken.insert(key.clone()) {
            let _ = std::fs::remove_file(temp);
            return Ok(false);
        }
        let path = self.efl.join(&rel);
        let size = std::fs::metadata(temp).with_context(|| temp.display().to_string())?.len();
        std::fs::rename(temp, &path).with_context(|| path.display().to_string())?;
        let mut s = self.state.lock().expect("output lock");
        s.written += 1;
        s.files.push((key, size));
        Ok(true)
    }

    /// Puts a file of the mod's own (not a game file) at `<mod>/<rel>`;
    /// `rel` must be under one of [`MOD_DIRS`]. Same rules as [`Output::put`].
    pub fn put_mod(&self, rel: &str, bytes: &[u8]) -> Result<bool> {
        let rel = rel.trim_start_matches('/').to_ascii_lowercase();
        anyhow::ensure!(
            MOD_DIRS.iter().any(|d| rel.starts_with(d)) && !rel.split('/').any(|p| p == ".." || p.is_empty()),
            "bad mod path {rel}"
        );
        if !self.state.lock().expect("output lock").taken.insert(rel.clone()) {
            return Ok(false);
        }
        let mod_dir = self.efl.parent().and_then(Path::parent).context("mod folder")?;
        let path = mod_dir.join(&rel);
        let same = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64)
            && std::fs::read(&path).is_ok_and(|old| old == bytes);
        if !same {
            std::fs::create_dir_all(path.parent().context("no parent")?)?;
            std::fs::write(&path, bytes).with_context(|| path.display().to_string())?;
        }
        let mut s = self.state.lock().expect("output lock");
        if same {
            s.unchanged += 1;
        } else {
            s.written += 1;
        }
        s.files.push((rel, bytes.len() as u64));
        Ok(true)
    }

    pub fn finish(self) -> OutState {
        self.state.into_inner().expect("output lock")
    }
}
