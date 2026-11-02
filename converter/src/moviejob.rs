//! `movies`: the PS2 FMVs (`jppc/movie/adpcm/*.dat`, decoded by
//! [`crate::ps2movie`]) encoded as the WebM files the PC plays
//! (`docs/movies.md`).
//!
//! The PC player takes VP8 + Vorbis WebM, shows one frame per game update
//! (29.97 a second) and stretches the picture over the screen. So each PS2
//! movie (25 fps, 640 x 416 for a 4:3 screen) becomes 960 x 720 at
//! 30000/1001 fps when `render.aspect` is `"ps2"` (the game's picture is
//! 4:3 then, `docs/screen.md`), else 1280 x 720 with the 4:3 picture in the
//! middle and black bars; its studio-range YCbCr expanded to the full range
//! the PC's shader assumes. The sound is the PS2's, 48 kHz stereo.
//!
//! The encoding is done by an external ffmpeg (not shipped; [`find_ffmpeg`]:
//! `--ffmpeg`, `source.ffmpeg`, next to demaster.exe, the PATH, else on
//! Windows a pinned LGPL build downloaded into `<mod>/tools`).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::jobs::{Ctx, Job, Output};
use crate::ps2movie;

const VIDEO_DIR: &str = "ffx_data/gamedata/ps3data/video";
/// The list the game reads for the English/European versions (the JP one
/// differs only by naming the `_jp` cuts of movies that have both).
const VIDEO_LIST: &str = "ffx_data/gamedata/ps3data/video/us/ffx_videolist.txt";

/// Movies the PC plays with the 3D scene drawn over them (`movie_have_camera`:
/// ids 0x20, 0x2e, 0x46, 0x48). The PC re-rendered them for its own 16:9
/// cameras (`video/<id>.dat`); a 4:3 PS2 picture under a 16:9 scene would
/// not line up, so they stay HD. With `render.aspect = "ps2"` the scene shows
/// the middle 4:3 of the PC camera's view, so the HD movie is cut to its
/// middle 4:3 to match.
const WITH_3D_SCENE: [&str; 4] = ["mst", "far", "ggm", "sns"];

/// PS2 names the PC spells differently.
fn pc_name(ps2: &str) -> &str {
    match ps2 {
        "saw" => "swa",
        n => n,
    }
}

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![Box::new(MovieJob)]
}

struct MovieJob;

impl Job for MovieJob {
    fn name(&self) -> &'static str {
        "movies"
    }

    fn settings(&self) -> &'static [&'static str] {
        &["movies", "render.aspect"]
    }

    fn enabled(&self, config: &Config) -> bool {
        config.movies.ps2()
    }

    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        run(ctx, out, &[])
    }
}

/// Converts the movies (`only`: just these PS2 names; empty: all).
pub fn run(ctx: &Ctx, out: &Output, only: &[String]) -> Result<String> {
    {
        let ffmpeg = find_ffmpeg(ctx)?;
        let list = video_list(ctx.pc)?;
        let names: Vec<String> = ctx.disc(|d| {
            let mut v: Vec<String> = d
                .names()
                .filter_map(|n| n.strip_prefix("jppc/movie/adpcm/")?.strip_suffix(".sz").map(str::to_string))
                .collect();
            v.sort();
            Ok(v)
        })?;
        if names.is_empty() {
            bail!("no movies (jppc/movie/adpcm/*.sz) on the disc");
        }
        let four_three = ctx.config.render.aspect.ps2();
        let mut done = 0;
        let mut kept = Vec::new();
        for name in &names {
            if !only.is_empty() && !only.contains(name) {
                continue;
            }
            let Some(file) = list.iter().find(|f| f.split('_').next() == Some(pc_name(name))) else {
                kept.push(format!("{name} (not in the PC's video list)"));
                continue;
            };
            let archive = format!("{VIDEO_DIR}/{file}");
            let t = std::time::Instant::now();
            let info = if WITH_3D_SCENE.contains(&name.as_str()) {
                if !four_three {
                    kept.push(format!("{name} (3D scene on top)"));
                    continue;
                }
                kept.push(format!("{name} (3D scene on top: the HD movie's middle 4:3)"));
                crop_hd(ctx.pc, &ffmpeg, &archive, out).with_context(|| format!("movie {name} (HD, cut to 4:3)"))?
            } else {
                ctx.disc(|d| convert(d, &ffmpeg, name, &archive, out, four_three))
                    .with_context(|| format!("movie {name}"))?
            };
            println!("  {name} -> {file}: {info} ({:.0} s)", t.elapsed().as_secs_f32());
            done += 1;
        }
        let mut summary = format!("{done} movies");
        if !kept.is_empty() {
            summary += &format!("; left HD: {}", kept.join(", "));
        }
        Ok(summary)
    }
}

/// The names (lower case) the US video list plays: `opn_us.webm`, ...
fn video_list(pc: &ffx_vbf::Archive) -> Result<Vec<String>> {
    let e = pc.find(VIDEO_LIST).with_context(|| format!("{VIDEO_LIST} is missing"))?;
    let text = String::from_utf8_lossy(&pc.read(e)?).to_string();
    // Lines `id:,FILE.webm,FILE.webm`.
    Ok(text
        .lines()
        .filter_map(|l| l.split(',').nth(1))
        .map(|f| f.trim().to_ascii_lowercase())
        .filter(|f| f.ends_with(".webm"))
        .collect())
}

/// The ffmpeg build downloaded on Windows when none is found: BtbN's LGPL
/// (v3) build of FFmpeg 8.1.3 (it has libvpx and libvorbis). Pinned by URL
/// and SHA-256; only `ffmpeg.exe` and the licence are kept.
const FFMPEG_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-11-01-12-49/ffmpeg-n8.1.3-29-gad3b795b72-win64-lgpl-8.1.zip";
const FFMPEG_SHA256: &str = "a152a35aacfe7972dce74f1e1d2a1cfaabc8daa7bf8632e9473f70ad18c6c5d4";
const FFMPEG_DIR: &str = "ffmpeg-n8.1.3-29-gad3b795b72-win64-lgpl-8.1";

fn runs(exe: &Path) -> bool {
    Command::new(exe)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The ffmpeg program to run: the configured one (`--ffmpeg`,
/// `source.ffmpeg`), else `ffmpeg.exe` next to demaster.exe or in the mod's
/// `tools` folder, else `ffmpeg` on the PATH, else (Windows) the pinned
/// build, downloaded into `tools`.
pub fn find_ffmpeg(ctx: &Ctx) -> Result<PathBuf> {
    if let Some(exe) = &ctx.ffmpeg {
        if runs(exe) {
            return Ok(exe.clone());
        }
        bail!("the ffmpeg set in demaster.toml ({}) does not run", exe.display());
    }
    let local = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let mut candidates = vec![crate::build::exe_dir().join(local)];
    if let Some(t) = &ctx.tools_dir {
        candidates.push(t.join(local));
    }
    candidates.push(PathBuf::from("ffmpeg"));
    if let Some(exe) = candidates.into_iter().find(|c| runs(c)) {
        return Ok(exe);
    }
    match (&ctx.tools_dir, cfg!(windows)) {
        (Some(tools), true) => download_ffmpeg(tools).map_err(|e| {
            e.context(
                "no ffmpeg for the movies, and downloading one failed (offline?). The movies stay HD; \
                 the next build tries again. Or install ffmpeg yourself and set `ffmpeg` under [source]",
            )
        }),
        _ => bail!(
            "ffmpeg not found: the movies need it. Install it (https://ffmpeg.org/download.html) and put it \
             on the PATH or set `ffmpeg` under [source] in demaster.toml. The movies stay HD"
        ),
    }
}

/// Downloads the pinned ffmpeg build with Windows' own `curl.exe`, checks
/// its SHA-256 and unpacks `ffmpeg.exe` and its licence with `tar.exe`.
fn download_ffmpeg(tools: &Path) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    std::fs::create_dir_all(tools)?;
    let zip = tools.join("ffmpeg-download.zip");
    println!("  no ffmpeg found: downloading the LGPL build of FFmpeg 8.1.3 (168 MB) from");
    println!("  {FFMPEG_URL}");
    let ok = Command::new("curl.exe")
        .args(["-fL", "-#", "--retry", "2", "-o"])
        .arg(&zip)
        .arg(FFMPEG_URL)
        .status()
        .with_context(|| "running curl.exe")?
        .success();
    if !ok {
        let _ = std::fs::remove_file(&zip);
        bail!("the download failed");
    }
    let mut hasher = Sha256::new();
    std::io::copy(&mut std::fs::File::open(&zip)?, &mut hasher)?;
    let hash: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if hash != FFMPEG_SHA256 {
        let _ = std::fs::remove_file(&zip);
        bail!("the downloaded file is not the expected one (SHA-256 {hash})");
    }
    let unpack = tools.join("ffmpeg-unpack");
    std::fs::create_dir_all(&unpack)?;
    let ok = Command::new("tar.exe")
        .arg("-xf")
        .arg(&zip)
        .arg("-C")
        .arg(&unpack)
        .arg(format!("{FFMPEG_DIR}/bin/ffmpeg.exe"))
        .arg(format!("{FFMPEG_DIR}/LICENSE.txt"))
        .status()
        .with_context(|| "running tar.exe")?
        .success();
    let exe = tools.join("ffmpeg.exe");
    let moved = ok
        && std::fs::rename(unpack.join(FFMPEG_DIR).join("bin/ffmpeg.exe"), &exe).is_ok()
        && std::fs::rename(unpack.join(FFMPEG_DIR).join("LICENSE.txt"), tools.join("ffmpeg-LICENSE.txt")).is_ok();
    let _ = std::fs::remove_dir_all(&unpack);
    let _ = std::fs::remove_file(&zip);
    if !moved || !runs(&exe) {
        bail!("unpacking ffmpeg.exe from the download failed");
    }
    println!(
        "  downloaded {} (FFmpeg 8.1.3, LGPL v3; licence in ffmpeg-LICENSE.txt, source at https://ffmpeg.org)",
        exe.display()
    );
    Ok(exe)
}

/// The output picture: the PS2's (640 or 512) x 416, all frames scaled to
/// this width before they go to ffmpeg.
const WIDTH: usize = 640;

fn chunks(
    d: &mut crate::disc::Disc,
    name: &str,
    sizes: &[u32],
    mut f: impl FnMut(usize, &[u8]) -> Result<()>,
) -> Result<()> {
    let dat = format!("jppc/movie/adpcm/{name}.dat");
    let (mut r, size) = d.open_stored(&dat)?.with_context(|| format!("{dat} is missing"))?;
    let total: u64 = sizes.iter().map(|&s| u64::from(s)).sum();
    if total > size {
        bail!("{dat}: the chunks add up to {total} bytes, the file has {size}");
    }
    let mut buf = Vec::new();
    for (i, &s) in sizes.iter().enumerate() {
        buf.resize(s as usize, 0);
        r.read_exact(&mut buf)?;
        f(i, &buf).with_context(|| format!("frame {i}"))?;
    }
    Ok(())
}

/// The VP8 settings of every movie the job writes.
const VP8: [&str; 16] = [
    "-c:v", "libvpx", "-deadline", "good", "-cpu-used", "4", "-b:v", "4M", "-crf", "8", "-qmin", "0", "-qmax", "40",
    "-auto-alt-ref", "0",
];

/// The 4:3 picture over the whole (4:3) screen with `render.aspect = "ps2"`,
/// else in the middle of a 16:9 one with black bars.
const SCALE_43: &str = "scale=960:720:flags=lanczos:in_range=tv:out_range=pc,fps=30000/1001,format=yuv420p";
const SCALE_169: &str =
    "scale=960:720:flags=lanczos:in_range=tv:out_range=pc,pad=1280:720:160:0:black,fps=30000/1001,format=yuv420p";

/// A movie with the 3D scene on top, for `render.aspect = "ps2"`: the HD
/// file's middle 4:3 (960 x 720 of its 1280 x 720), the sound as it is.
fn crop_hd(pc: &ffx_vbf::Archive, ffmpeg: &Path, archive: &str, out: &Output) -> Result<String> {
    let e = pc.find(archive).with_context(|| format!("{archive} is missing"))?;
    let hd = out.temp_path(&format!("{archive}.hd"))?;
    std::fs::write(&hd, pc.read(e)?)?;
    let webm = out.temp_path(archive)?;
    let result = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&hd)
        .args(["-map", "0:v", "-map", "0:a?", "-vf", "crop=iw*3/4:ih,format=yuv420p"])
        .args(VP8)
        .args(["-threads", "4", "-c:a", "copy"])
        .args(["-map_metadata", "-1", "-fflags", "+bitexact", "-flags:v", "+bitexact", "-f", "webm"])
        .arg(&webm)
        .output()
        .with_context(|| format!("starting {}", ffmpeg.display()));
    let _ = std::fs::remove_file(&hd);
    let result = result?;
    if !result.status.success() {
        let _ = std::fs::remove_file(&webm);
        bail!("ffmpeg failed ({}): {}", result.status, String::from_utf8_lossy(&result.stderr).trim());
    }
    out.put_file(archive, &webm)?;
    Ok("HD, middle 4:3".into())
}

fn convert(
    d: &mut crate::disc::Disc,
    ffmpeg: &Path,
    name: &str,
    archive: &str,
    out: &Output,
    four_three: bool,
) -> Result<String> {
    let sz_name = format!("jppc/movie/adpcm/{name}.sz");
    let sz = d.read(&sz_name)?.with_context(|| format!("{sz_name} is missing"))?;
    let index = ps2movie::Index::parse(&sz)?;
    let frames = index.sizes.len();

    // Pass 1: the sound, into a WAV next to the output.
    let mut pcm = Vec::new();
    chunks(d, name, &index.sizes, |_, c| {
        for s in ps2movie::chunk_sound(c)? {
            pcm.extend(s.to_le_bytes());
        }
        Ok(())
    })?;
    let wav = out.temp_path(&format!("{archive}.wav"))?;
    std::fs::write(&wav, ps2movie::wav(&pcm))?;
    let webm = out.temp_path(archive)?;

    // Pass 2: the pictures, piped to ffmpeg.
    let seconds = frames as f64 / 25.0;
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "rawvideo", "-pix_fmt", "yuv420p", "-video_size", &format!("{WIDTH}x416")])
        .args(["-framerate", "25", "-i", "pipe:0", "-i"])
        .arg(&wav)
        .args(["-map", "0:v", "-map", "1:a", "-t", &format!("{seconds:.3}")])
        .args(["-vf", if four_three { SCALE_43 } else { SCALE_169 }])
        .args(VP8)
        .args(["-threads", "4"])
        .args(["-c:a", "libvorbis", "-q:a", "5", "-ar", "48000", "-ac", "2"])
        .args(["-map_metadata", "-1", "-fflags", "+bitexact", "-flags:v", "+bitexact"])
        .args(["-flags:a", "+bitexact", "-f", "webm"])
        .arg(&webm)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("starting {}", ffmpeg.display()))?;
    let mut stderr = child.stderr.take().context("ffmpeg stderr")?;
    let errors = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut stdin = child.stdin.take().context("ffmpeg stdin")?;
    let mut dec = ps2movie::Decoder::new(&index);
    let fed = chunks(d, name, &index.sizes, |_, c| {
        let p = dec.chunk(c)?.picture.widen(WIDTH);
        stdin.write_all(&p.y)?;
        stdin.write_all(&p.cb)?;
        stdin.write_all(&p.cr)?;
        Ok(())
    });
    drop(stdin);
    let status = child.wait()?;
    let errors = errors.join().unwrap_or_default();
    let _ = std::fs::remove_file(&wav);
    if let Err(e) = fed {
        let _ = std::fs::remove_file(&webm);
        return Err(e.context(errors));
    }
    if !status.success() {
        let _ = std::fs::remove_file(&webm);
        bail!("ffmpeg failed ({status}): {}", errors.trim());
    }
    out.put_file(archive, &webm)?;
    let s = &dec.stats;
    let mut info = format!(
        "{frames} frames ({:.2} s), {} intra, sound {:.2} s",
        seconds,
        s.intra,
        pcm.len() as f64 / 4.0 / 48000.0
    );
    if s.misaligned > 0 {
        info += &format!(", {} pictures off their record size", s.misaligned);
    }
    Ok(info)
}
