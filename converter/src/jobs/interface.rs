//! The interface: PS2 menu atlases, portraits, the title screen, the loading
//! screen and the button glyphs (differences rows 12, 16, 22). Notes and
//! evidence: `docs/ui.md`.
//!
//! The PS2 menu textures are raw indexed images in `<lang>pc/menu/*.fmt`
//! with their palettes in `menu.clp` (a 64x64 PSMCT32 image: 16 CLUTs of
//! 256 colours, each a 16x16 CSM1 rectangle). The PC draws the HD
//! `menu*/d3d11/<name>.dds` with UVs divided by the PS2 size
//! (`TOGetImageWH`), so a PS2-layout image of any size fits where the HD
//! art kept the PS2 layout.

use anyhow::{Context, Result};

use crate::config::Config;
use crate::disc::Disc;
use crate::formats::gsmem;
use crate::jobs::{Ctx, Job, Output};
use crate::jobs::maps::{self, resize};
use crate::formats::phyretex::{self, Rgba};

const PS3: &str = "ffx_data/gamedata/ps3data";

/// One converted file.
pub struct Done {
    pub path: String,
    pub file: Vec<u8>,
    /// HD | result, for `demaster ui`'s previews.
    pub preview: Rgba,
    /// One line: what was done.
    pub note: String,
}

/// Which parts to convert (the settings of one job, or everything).
#[derive(Debug, Clone, Copy, Default)]
pub struct Parts {
    pub menus: bool,
    pub portraits: bool,
    pub title: bool,
    pub loading: bool,
    pub glyphs: bool,
    /// The PS2 damage digits in the battle atlas (`ui.font`, written with
    /// the atlases by the `ui.menus` job).
    pub font: bool,
    /// The PS2 font's pages and widths (`jobs/font.rs`).
    pub font_files: bool,
    /// The "SQUARESOFT presents" screen at boot (`ui.boot_logo`).
    pub boot_logo: bool,
}

impl Parts {
    pub fn all() -> Self {
        Parts {
            menus: true,
            portraits: true,
            title: true,
            loading: true,
            glyphs: true,
            font: true,
            font_files: true,
            boot_logo: true,
        }
    }
}

// ---------------------------------------------------------------- boot logo

/// `titl02` (map of the boot event `memochek`) layers that hold the
/// "SQUARESOFT® presents" screen on the PS2 (0 and 4 with ®, 7 without);
/// the HD put its "SQUARE ENIX" logo in the same layers. `memochek` shows
/// part 0 (Japanese/Asian language group) or 4 (the others).
const BOOT_LOGO_LAYERS: [usize; 3] = [0, 4, 7];

/// The PC's boot event, which shows the logo part but no longer waits on it.
const MEMOCHEK: &str = "ffx_ps2/ffx/master/jppc/event/obj/me/memochek/memochek.ebp";

/// The PS2's "SQUARESOFT® presents" screen at boot (`docs/ui.md`, "Boot
/// logo"): the three `titl02` layers from the disc, laid out like the title
/// (`title_placed`), and `memochek` with the request the PC removed.
fn boot_logo(disc: &mut Disc, pc: &ffx_vbf::Archive, done: &mut Vec<Done>, notes: &mut Vec<String>) -> Result<()> {
    let Some(vpa) = disc.read("jppc/map/titl/titl02/bin/mapout.vpa")? else {
        notes.push("boot logo: no titl02 on the disc".into());
        return Ok(());
    };
    let Some(e) = pc.find(MEMOCHEK) else {
        notes.push(format!("boot logo: no {MEMOCHEK}"));
        return Ok(());
    };
    let layers = bg_layers(&vpa);
    let mut preview = None;
    for k in BOOT_LOGO_LAYERS {
        let path = format!("{PS3}/map/titl/titl02/2d/tex/d3d11/0_{k}_512_448_0.dds.phyre");
        let (Some((template, hd)), Some(img)) = (load_hd(pc, &path)?, layers.get(k)) else {
            notes.push(format!("{path}: no HD layer or PS2 layer {k}"));
            continue;
        };
        // the PS2 screen is black on white: a white corner tells it from the
        // project logo and the other black screens of titl02
        if img.width != 512 || img.height != 448 || img.get(4, 4)[..3] != [255, 255, 255] {
            notes.push(format!("{path}: PS2 layer {k} is not the white logo screen, kept HD"));
            continue;
        }
        let img = title_placed(&maps::opaque_like(img.clone(), &hd));
        if k == 4 {
            preview = Some(img.clone());
        }
        done.push(Done {
            file: phyretex::argb8_one_level(&template, &img, None)?,
            preview: side_by_side(&hd, &img),
            note: format!("{path}: PS2 layer {k} (boot logo)"),
            path,
        });
    }
    let Some(preview) = preview else {
        notes.push("boot logo: no picture, memochek left as is".into());
        return Ok(());
    };
    done.push(Done {
        file: memochek_with_logo(pc.read(e)?)?,
        preview,
        note: format!("{MEMOCHEK}: waits on the logo part again"),
        path: MEMOCHEK.into(),
    });
    Ok(())
}

/// The PC's `memochek` with the PS2's wait on the logo: after it shows the
/// logo part (0 or 4) worker 0 requests worker 2's entry 3 and waits for its
/// end (`REQEW`): fade in 15 frames, hold 60 frames, up to 90 more (Start
/// ends them), fade out 15, 15 black; then the project logo. The PC's script
/// shows the part and hides it in the same frame (that request is the only
/// difference to the PS2 there; entry 3 is still in it, unused).
pub fn memochek_with_logo(ebp: Vec<u8>) -> Result<Vec<u8>> {
    let c = ffx_script::Container::parse(ebp).context("memochek")?;
    let text = ffx_script::disassemble(&c.script).context("memochek")?;
    let w0_end = text.find("\nworker 1 ").context("memochek: no worker 1")?;
    let w2 = text.find("\nworker 2 ").context("memochek: no worker 2")?;
    // worker 0 entry 1: after `MapPartShow(0)` / `MapPartShow(4)` both
    // branches meet at l3, where the PS2 sends the request
    let anchor = "l3:\n    pushv v1\n";
    let at = text[..w0_end].find(anchor).context("memochek: worker 0 has no l3 after the logo")?;
    anyhow::ensure!(
        text[..w0_end].matches(anchor).count() == 1 && text[at - 200.min(at)..at].contains("func_facc 0x800f"),
        "memochek: unexpected worker 0"
    );
    anyhow::ensure!(
        text[w2..].contains("e3:\n    pushimm 15\n    func_facc 0x4004"),
        "memochek: worker 2 entry 3 is not the logo's fade"
    );
    let insert = "    pushimm 1\n    pushimm 2\n    pushimm 3\n    reqe0\n    popfacc\n";
    let at = at + "l3:\n".len();
    let new = format!("{}{insert}{}", &text[..at], &text[at..]);
    let script = ffx_script::assemble(&c.script, &new).context("memochek: assembling")?;
    Ok(c.replace_script(&script).context("memochek")?.bytes().to_vec())
}

// ---------------------------------------------------------------- helpers

pub fn load_hd(pc: &ffx_vbf::Archive, path: &str) -> Result<Option<(Vec<u8>, Rgba)>> {
    let Some(e) = pc.find(path) else {
        return Ok(None);
    };
    let template = pc.read(e)?;
    let t = ffx_phyre::Texture::parse(template.clone()).with_context(|| path.to_string())?;
    let rgba = Rgba {
        width: t.width,
        height: t.height,
        pixels: t.to_rgba(),
    };
    Ok(Some((template, rgba)))
}

/// HD on the left, the result on the right, both at the larger size over grey.
pub fn side_by_side(hd: &Rgba, out: &Rgba) -> Rgba {
    let (w, h) = (hd.width.max(out.width), hd.height.max(out.height));
    let s = (1024 / w.max(1)).clamp(1, 4);
    let (w, h) = (w * s, h * s);
    let mut img = Rgba::new(2 * w + 4, h);
    for p in img.pixels.chunks_mut(4) {
        p.copy_from_slice(&[40, 40, 60, 255]);
    }
    let over = |c: [u8; 4]| {
        let a = u32::from(c[3]);
        let bg = [40u32, 40, 60];
        [0, 1, 2].map(|k| ((u32::from(c[k]) * a + bg[k] * (255 - a)) / 255) as u8)
    };
    for (k, src) in [hd, out].into_iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                let [r, g, b] = over(src.get(x * src.width / w, y * src.height / h));
                img.put(k as u32 * (w + 4) + x, y, [r, g, b, 255]);
            }
        }
    }
    img
}

fn transparent_like(hd: &Rgba) -> Rgba {
    // 8x8 is enough: the texture is not drawn visibly at any size.
    let _ = hd;
    Rgba::new(8, 8)
}

/// Bilinear sample of `src` at texel coordinates (centres at +0.5).
fn bilinear(src: &Rgba, fx: f32, fy: f32) -> [u8; 4] {
    let x = (fx - 0.5).clamp(0.0, (src.width - 1) as f32);
    let y = (fy - 0.5).clamp(0.0, (src.height - 1) as f32);
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(src.width - 1), (y0 + 1).min(src.height - 1));
    let (tx, ty) = (x - x0 as f32, y - y0 as f32);
    let (a, b, c, d) = (src.get(x0, y0), src.get(x1, y0), src.get(x0, y1), src.get(x1, y1));
    let mut out = [0u8; 4];
    for k in 0..4 {
        let top = f32::from(a[k]) * (1.0 - tx) + f32::from(b[k]) * tx;
        let bot = f32::from(c[k]) * (1.0 - tx) + f32::from(d[k]) * tx;
        out[k] = (top * (1.0 - ty) + bot * ty).round() as u8;
    }
    out
}

/// `src`'s rectangle (x, y, w, h, texels, may be fractional) resampled into
/// `dst`'s rectangle.
fn blit(src: &Rgba, s: [f32; 4], dst: &mut Rgba, d: [u32; 4]) {
    for y in 0..d[3] {
        for x in 0..d[2] {
            let fx = s[0] + (x as f32 + 0.5) * s[2] / d[2] as f32;
            let fy = s[1] + (y as f32 + 0.5) * s[3] / d[3] as f32;
            let (ox, oy) = (d[0] + x, d[1] + y);
            if ox < dst.width && oy < dst.height {
                dst.put(ox, oy, bilinear(src, fx, fy));
            }
        }
    }
}

/// Palette `p` (0..15) of a `menu.clp`: RGBA with 8-bit alpha, by index.
fn clp_palette(clp: &[u8], p: usize) -> Vec<[u8; 4]> {
    let (bx, by) = ((p % 4) * 16, (p / 4) * 16);
    (0..256u32)
        .map(|i| {
            let j = gsmem::csm1(i) as usize;
            let (x, y) = (bx + j % 16, by + j / 16);
            let o = (y * 64 + x) * 4;
            let c = clp.get(o..o + 4).map_or([0; 4], |s| [s[0], s[1], s[2], s[3]]);
            [c[0], c[1], c[2], gsmem::alpha(c[3])]
        })
        .collect()
}

fn indexed(w: u32, h: u32, idx: &[u8], pal: &[[u8; 4]]) -> Rgba {
    let mut img = Rgba::new(w, h);
    for (i, &k) in idx.iter().take((w * h) as usize).enumerate() {
        img.pixels[4 * i..4 * i + 4].copy_from_slice(&pal[usize::from(k)]);
    }
    img
}

fn luma(c: [u8; 4]) -> f32 {
    (f32::from(c[0]) * 0.3 + f32::from(c[1]) * 0.59 + f32::from(c[2]) * 0.11) * f32::from(c[3])
        / 255.0
}

/// 8-connected regions of `mask` (w x h), each as its bounding box
/// `[x0, y0, x1, y1)` and its texels.
fn regions(mask: &[bool], w: u32, h: u32) -> Vec<([u32; 4], Vec<u32>)> {
    let mut seen = vec![false; mask.len()];
    let mut out = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start as u32];
        let mut texels = Vec::new();
        let mut b = [u32::MAX, u32::MAX, 0, 0];
        while let Some(i) = stack.pop() {
            texels.push(i);
            let (x, y) = (i % w, i / w);
            b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = (ny as u32 * w + nx as u32) as usize;
                    if mask[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j as u32);
                    }
                }
            }
        }
        out.push((b, texels));
    }
    out
}

fn dilate(mask: &[bool], w: u32, h: u32) -> Vec<bool> {
    let mut out = mask.to_vec();
    for y in 0..h {
        for x in 0..w {
            if !mask[(y * w + x) as usize] {
                continue;
            }
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32 {
                    out[(ny as u32 * w + nx as u32) as usize] = true;
                }
            }
        }
    }
    out
}

/// The menu folders the disc's menu files are in: `(disc dir, HD folder)`.
/// `jppc/menu` holds the disc's own language (German on the German disc),
/// `uspc/menu` English where the disc has it.
/// The menu sequence layout (`TOSeqInitParam`'s words, the scenes): the PC's copy it reads, and
/// the disc's (the PS2 reads its main folder's).
const MENU_SCRIPT_PC: &str = "ffx_ps2/ffx/master/jppc/menu/menu_script.bin";
const MENU_SCRIPT_PS2: &str = "jppc/menu/menu_script.bin";

fn menu_sources(region: &str) -> Vec<(&'static str, &'static str)> {
    let own = match region {
        "de" => "menu_de",
        "fr" => "menu_fr",
        "it" => "menu_it",
        "es" => "menu_es",
        "us" | "uk" => "menu_us",
        _ => "menu",
    };
    let mut v = vec![("jppc/menu", own)];
    if own != "menu_us" {
        v.push(("uspc/menu", "menu_us"));
    }
    v
}

// ----------------------------------------------------------------- atlases

/// A PS2 menu atlas: file, size, the `menu.clp` palette it is drawn with
/// (checked by eye against the HD art, `docs/ui.md`), and the region (PS2
/// texels) that holds portraits (`ui.portraits`).
struct Atlas {
    name: &'static str,
    w: u32,
    h: u32,
    palette: usize,
    portraits: Option<[u32; 4]>,
}

/// Overlap (IoU) of the HD and PS2 alpha masks for a PS2 element to
/// replace an HD one. Lower values let PS2 bars replace HD labels.
const MIN_IOU: f32 = 0.65;

const ATLASES: &[Atlas] = &[
    Atlas {
        name: "meswin",
        w: 256,
        h: 256,
        palette: 0,
        portraits: None,
    },
    Atlas {
        name: "battle",
        w: 256,
        h: 256,
        palette: 1,
        // the CTB list's faces and markers: the column right of x 190
        portraits: Some([190, 0, 256, 190]),
    },
];

/// Where the HD element of an atlas has a PS2 element at the same place,
/// the PS2 texels replace it (scaled up to the HD size, bilinear, as the
/// game would filter the PS2-size image); elsewhere the HD art stays (the
/// PC moved or redrew it). Elements: connected regions of the HD alpha,
/// grown by the PS2 regions they touch; a match is an overlap (IoU) of
/// the two alpha masks of at least 0.5 in that box.
fn convert_atlas(
    a: &Atlas,
    idx: &[u8],
    clp: &[u8],
    hd: &Rgba,
    menus: bool,
    portraits: bool,
) -> Option<(Rgba, usize, usize)> {
    let (w, h) = (a.w, a.h);
    if hd.width % w != 0 || hd.height % h != 0 || hd.width / w != hd.height / h {
        return None;
    }
    let s = hd.width / w;
    let ps2 = indexed(w, h, idx, &clp_palette(clp, a.palette));
    let small = resize(hd, w, h);
    let hmask: Vec<bool> = small.pixels.chunks_exact(4).map(|p| p[3] > 24).collect();
    let pmask: Vec<bool> = ps2.pixels.chunks_exact(4).map(|p| p[3] > 24).collect();
    let (hd_d, ps2_d) = (dilate(&hmask, w, h), dilate(&pmask, w, h));
    // PS2 regions, and which region each texel is in
    let pregs = regions(&pmask, w, h);
    let mut owner = vec![usize::MAX; pmask.len()];
    for (k, (_, t)) in pregs.iter().enumerate() {
        for &i in t {
            owner[i as usize] = k;
        }
    }
    let mut out = hd.clone();
    let (mut used, mut kept) = (0, 0);
    for (b0, texels) in regions(&hd_d, w, h) {
        // Grow the box by the PS2 regions the element touches that lie
        // mostly inside it (a longer word, a slightly larger icon). A big
        // PS2 region that reaches far out of it (a bar where the HD has a
        // label) means the layouts differ here: keep the HD element.
        let (bw, bh) = (b0[2] - b0[0], b0[3] - b0[1]);
        let near = [
            b0[0].saturating_sub(bw / 4 + 2),
            b0[1].saturating_sub(bh / 4 + 2),
            (b0[2] + bw / 4 + 2).min(w),
            (b0[3] + bh / 4 + 2).min(h),
        ];
        let mut b = b0;
        let mut touched: Vec<usize> = texels
            .iter()
            .filter_map(|&i| owner.get(i as usize).copied().filter(|&k| k != usize::MAX))
            .collect();
        touched.sort_unstable();
        touched.dedup();
        let mut foreign = false;
        for k in touched {
            let (p, t) = &pregs[k];
            let inside = t
                .iter()
                .filter(|&&i| {
                    let (x, y) = (i % w, i / w);
                    x >= near[0] && x < near[2] && y >= near[1] && y < near[3]
                })
                .count();
            if inside * 10 >= t.len() * 6 {
                b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[2]), b[3].max(p[3])];
            } else if t.len() > 20 {
                foreign = true;
            }
        }
        let (mut inter, mut union) = (0u32, 0u32);
        let (mut lh, mut lp, mut n) = (0f32, 0f32, 0f32);
        for y in b[1]..b[3] {
            for x in b[0]..b[2] {
                let i = (y * w + x) as usize;
                inter += u32::from(hd_d[i] && ps2_d[i]);
                union += u32::from(hd_d[i] || ps2_d[i]);
                if hmask[i] || pmask[i] {
                    lh += luma(small.get(x, y));
                    lp += luma(ps2.get(x, y));
                    n += 1.0;
                }
            }
        }
        let iou = inter as f32 / union.max(1) as f32;
        // PS2 texels right beside the box (up to 3 texels left or right):
        // the PS2 element is wider (a longer word), and cutting it would
        // draw part of it. Portrait cells touch each other and skip this.
        let mut cut = 0;
        for y in b[1]..b[3] {
            for x in b[0].saturating_sub(3)..(b[2] + 3).min(w) {
                if x < b[0] || x >= b[2] {
                    cut += usize::from(pmask[(y * w + x) as usize]);
                }
            }
        }
        // A dark HD element where the PS2 one is bright is a shadow the PC
        // draws from its own copy of the art (meswin's feather): keep it.
        let shadow = n > 0.0 && lh / n < 30.0 && lp / n > 90.0;
        let is_portrait = a.portraits.is_some_and(|r| {
            let (cx, cy) = ((b[0] + b[2]) / 2, (b[1] + b[3]) / 2);
            cx >= r[0] && cx < r[2] && cy >= r[1] && cy < r[3]
        });
        let wanted = if is_portrait { portraits } else { menus };
        let foreign = foreign || (!is_portrait && cut > 3);
        if iou >= MIN_IOU && !shadow && !foreign && wanted {
            let r = [b[0] as f32, b[1] as f32, (b[2] - b[0]) as f32, (b[3] - b[1]) as f32];
            blit(&ps2, r, &mut out, [b[0] * s, b[1] * s, (b[2] - b[0]) * s, (b[3] - b[1]) * s]);
            used += 1;
        } else {
            kept += 1;
        }
    }
    Some((out, used, kept))
}

/// The menu folders of the European languages (`ui.font`).
const EUROPEAN: [&str; 5] = ["menu_us", "menu_de", "menu_fr", "menu_it", "menu_es"];

/// Every menu folder of the PC (each has its own `meswin`).
const ALL_MENU_FOLDERS: [&str; 8] =
    ["menu", "menu_us", "menu_de", "menu_fr", "menu_it", "menu_es", "menu_kr", "menu_ch"];

/// The PS2 message window's pieces in `meswin`, PS2 texels (x, y, w, h):
/// FFX.exe's meswin layout table (`0x00c5b460`) entries 0-3, shapes
/// 200-203 (`TOMkpWindowPlain`, `0x008f9a40`; the PS2's `0x001fa550`):
/// the top-left corner with the brushed left edge, the top edge, the left
/// edge and the body, white, tinted at run time by `col_mssgwind0/1`. The
/// HD atlas is empty there (the remaster draws its own stone plate), so
/// the PS2 pieces go in for the PS2 window draw the mod restores
/// (`mod/MenuPatches.cs`, docs/ui.md). No text: every folder gets them.
const WINDOW_PIECES: [u32; 4] = [160, 56, 24, 32];

/// The whole PS2 `meswin` as a texture of its own, `menu*/d3d11/ps2win`
/// (PS2 size, 256x256): the HD atlas reuses the cells of the PS2 menus'
/// brush plates, cursor and labels for other art, so the restored PS2
/// menu draws (`mod/MainMenuPatches.cs`) read their meswin shapes from
/// this texture instead (the mod answers `TOGetShapTextureName(0x3e00)`
/// with this name while they draw; FFX.exe's meswin layout table equals
/// the PS2's, so the shapes' UVs fit). One per menu folder, the text cells
/// in that folder's language where the disc has it. The name and its
/// texture id have the length of `meswin` so the mod can derive the name.
pub const PS2_MESWIN: &str = "ps2win";
const PS2_MESWIN_ID: &str = "ps2win.dds";
/// The whole PS2 `battle` atlas (256x256, CLUT 1) the same way,
/// `menu*/d3d11/ps2btl`: the battle sequence's Overdrive tab (battle
/// layout 13, the "EKSTASE!!" brush plate) has no HD counterpart in the HD
/// atlas's cell; the mod answers `TOGetShapTextureName(0x3f00)` with it while
/// it draws that tab (`BattlePatches`, docs/ui.md "The Overdrive tab").
pub const PS2_BATTLE: &str = "ps2btl";
const PS2_BATTLE_ID: &str = "ps2btl.dds";

/// The PS2's small menu fonts M and S (`xfont1208.fmt` = `subfont.fmt`,
/// sheet `0x3d80`: 128 x 256, 4 bit, two 2-bit pages in bits 0-1 and 2-3),
/// rendered in their CLUT colours as `menu*/d3d11/ps2fnt` (1024 x 1024).
/// The PS2 draws each glyph twice (SLPM_675.13 `0x001f6110` font M,
/// `0x001f66f0` font S): the shadow with `menu.clp` CLUT 7 (block `0x2ebc`)
/// row `2 (colour >> 4) + page`, then the face with CLUT 6 (`0x2eb8`) row
/// `2 (colour & 15) + page`. The PC's copy of that sheet names a file the
/// archive lacks, so its value draws show nothing; the mod draws the glyphs
/// as meswin shapes with explicit UVs from this texture instead
/// (`mod/Ps2Menu.cs`). Tile `k` (256 x 256 at `(k % 4, k / 4)`): faces
/// 0..7, then shadows 0..7. In a tile glyph `i` (`c - 0x20`, 0..95) of
/// font M sits in the 16 x 16 cell `(i % 16, i / 16)`, of font S in
/// `(i % 16, 6 + i / 16)`, at (1, 1) in the cell: the empty border keeps
/// the filter from pulling in the neighbouring glyphs.
pub const PS2_FONT: &str = "ps2fnt";
const PS2_FONT_ID: &str = "ps2fnt.dds";

/// The [`PS2_FONT`] texture from `xfont1208.fmt` and `menu.clp`.
fn ps2_font(raw: &[u8], clp: &[u8]) -> Rgba {
    let idx = nibbles(raw);
    let mut img = Rgba::new(1024, 1024);
    for k in 0..16u32 {
        let (clut, colour) = if k < 8 { (6, k) } else { (7, k - 8) };
        let (tx, ty) = (256 * (k % 4), 256 * (k / 4));
        let rows = [0, 1].map(|page| clut_row(clp, clut, (2 * colour + page) as usize));
        for (font, (w, h, per_row, v0)) in [(10u32, 12u32, 24u32, 144u32), (8, 10, 32, 192)].into_iter().enumerate() {
            for i in 0..96u32 {
                let page = i & 1;
                let (sx, sy) = ((i % per_row) / 2 * w, v0 + i / per_row * h);
                let (cx, cy) = (tx + 16 * (i % 16) + 1, ty + 16 * (6 * font as u32 + i / 16) + 1);
                for y in 0..h {
                    for x in 0..w {
                        let n = idx.get(((sy + y) * 128 + sx + x) as usize).copied().unwrap_or(0);
                        // the page's 2-bit coverage is what the CLUT row maps
                        img.put(cx + x, cy + y, rows[page as usize][usize::from(n)]);
                    }
                }
            }
        }
    }
    img
}

/// Pastes [`WINDOW_PIECES`] into a `meswin` (HD or converted) where the
/// HD has nothing. False if the sizes do not fit or the HD drew there.
fn paste_window_pieces(a: &Atlas, idx: &[u8], clp: &[u8], hd: &Rgba, img: &mut Rgba) -> bool {
    if img.width % a.w != 0 || img.height % a.h != 0 || img.width / a.w != img.height / a.h {
        return false;
    }
    let s = img.width / a.w;
    let [x, y, w, h] = WINDOW_PIECES;
    for yy in y * s..(y + h) * s {
        for xx in x * s..(x + w) * s {
            if hd.get(xx, yy)[3] > 8 {
                return false;
            }
        }
    }
    let ps2 = indexed(a.w, a.h, idx, &clp_palette(clp, a.palette));
    // texel for texel: the pieces' edges are cut to their layout boxes,
    // so a bilinear blend with the empty texels around would thin them
    for yy in 0..h * s {
        for xx in 0..w * s {
            img.put(x * s + xx, y * s + yy, ps2.get(x + xx / s, y + yy / s));
        }
    }
    true
}

/// The damage digits 0-9 of the battle atlas, in PS2 texels: 5 x 2 cells
/// of 12.25 x 16.25 from (111, 63.5) (`TOMakePktBtlDamageNumRGBA`,
/// `0x008fb840`: HD pixels `444 + 49 col`, `254 + 65 row`, 49 x 65 each).
/// The PS2 atlas has its serif digits at the same place; the element
/// matching of [`convert_atlas`] keeps the HD ones (the shapes differ).
/// Half a texel short of the cells at the bottom: the filter would pull in
/// the dark edge of the bar below (row 96).
const DIGITS: [f32; 4] = [111.0, 63.5, 61.25, 32.0];

/// Pastes the PS2 damage digits into `img` (the HD battle atlas or its
/// conversion), scaled up bilinear. False if the sizes do not fit.
fn paste_digits(a: &Atlas, idx: &[u8], clp: &[u8], img: &mut Rgba) -> bool {
    if img.width % a.w != 0 || img.height % a.h != 0 || img.width / a.w != img.height / a.h {
        return false;
    }
    let s = (img.width / a.w) as f32;
    let ps2 = indexed(a.w, a.h, idx, &clp_palette(clp, a.palette));
    let d = DIGITS.map(|v| (v * s).round() as u32);
    blit(&ps2, DIGITS, img, d);
    true
}

// ------------------------------------------------------------- icon, strtex

/// `icon.fmt`: 256x128 texels, 4 bit; item icon `id` is the 15 x 18 cell at
/// `((id % 17) 15, (id / 17) 18)`. The PS2's `TOAddPktMenuIconRGBA`
/// (SLPM_675.13 `0x001f9798`) loads the 256-colour CLUT at block
/// `0x2ec0 + 4 (id >> 4)` and draws with `CSA = id & 15`: `menu.clp` CLUT
/// `ICON_CLUTS[id >> 4]`, entries `16 (id & 15)` .. `+ 15` (8-bit index
/// order). Every row's entries 2..15 are opaque (0x80), so each icon sits on
/// an opaque slanted tile (index 0, alpha 0, only cuts its corners): the PS2
/// look. The PC's `DrawCrossMenuIconXYWHRGBA` (`0x008e6af0`) samples the same
/// cell UVs over `TOGetImageWH(0x3dc0)` = 256 x 128. docs/ui.md ("Item icons").
const ICON_W: u32 = 256;
const ICON_H: u32 = 128;
const ICON_CLUTS: [usize; 8] = [8, 9, 12, 13, 10, 11, 14, 15];
/// Ids 0..=44 are the item icons; 45..=50 hold the "NEW" label (text: the
/// HD has it per language), the pink marker and the "B" box, and below them
/// (from PS2 v 54) the button art: shapes 600+ drawn by other code with
/// other CLUTs and HD-pixel UVs. Those stay HD.
const ICON_ITEMS: u32 = 45;

/// The battle words of `icon` (PS2 `0x001f9b50`, rectangles from its table
/// `0x00570000`, PS2 texels x0, y0, x1, y1): kind 0 OVERKILL, 1 IMMUNE,
/// 2 MISS, 3 MP; each in `menu.clp` CLUT 13 row `13 - kind` (CBP `0x2ecc`,
/// `CSA = 13 - kind`). The PC's `TOMakePktBtlIconA` (`0x008fbcc0`) draws them
/// from its own HD rectangles (HD pixels, the second box) at about half the
/// PS2 width; the mod (ui.menus) draws them from the PS2 rectangles at the
/// PS2 size (`BattlePatches.battle_word`), so the converter puts every
/// folder's words at the PS2 places: the disc's own words where the folder
/// has a disc source, else the HD words resampled there.
pub const BATTLE_WORDS: [([u32; 4], [u32; 4]); 4] = [
    ([0, 110, 164, 127], [108, 452, 550, 506]),
    ([164, 96, 231, 118], [632, 382, 952, 474]),
    ([55, 96, 152, 110], [304, 392, 464, 432]),
    ([0, 96, 45, 106], [0, 382, 73, 420]),
];

/// 4-bit texels, low nibble first.
fn nibbles(raw: &[u8]) -> Vec<u8> {
    raw.iter().flat_map(|b| [b & 15, b >> 4]).collect()
}

/// Row `row` (16 colours) of `menu.clp` CLUT `clut`.
fn clut_row(clp: &[u8], clut: usize, row: usize) -> Vec<[u8; 4]> {
    clp_palette(clp, clut)[16 * row..16 * row + 16].to_vec()
}

/// The rectangle `r` (x, y, w, h) of a `w`-wide indexed image in `pal`.
fn crop_indexed(idx: &[u8], w: u32, r: [u32; 4], pal: &[[u8; 4]]) -> Rgba {
    let mut img = Rgba::new(r[2], r[3]);
    for y in 0..r[3] {
        for x in 0..r[2] {
            let k = idx.get(((r[1] + y) * w + r[0] + x) as usize).copied().unwrap_or(0);
            img.put(x, y, pal.get(usize::from(k)).copied().unwrap_or([0; 4]));
        }
    }
    img
}

fn crop(src: &Rgba, r: [u32; 4]) -> Rgba {
    let mut img = Rgba::new(r[2], r[3]);
    for y in 0..r[3] {
        for x in 0..r[2] {
            img.put(x, y, src.get((r[0] + x).min(src.width - 1), (r[1] + y).min(src.height - 1)));
        }
    }
    img
}

fn clear(img: &mut Rgba, r: [u32; 4]) {
    for y in r[1]..(r[1] + r[3]).min(img.height) {
        for x in r[0]..(r[0] + r[2]).min(img.width) {
            img.put(x, y, [0; 4]);
        }
    }
}

/// `src` scaled bilinear into `dst`'s rectangle, edges clamped to `src`.
fn paste_scaled(src: &Rgba, dst: &mut Rgba, d: [u32; 4]) {
    blit(src, [0.0, 0.0, src.width as f32, src.height as f32], dst, d);
}

/// The HD `icon` (1024 x 512) with the PS2 item icons (ids below
/// [`ICON_ITEMS`], every folder: no text) and the battle words at their PS2
/// places ([`BATTLE_WORDS`]; `words` = the disc's for this folder, else the
/// HD's moved there). None if the HD atlas is not the PS2 layout at 4x.
fn convert_icon(idx: &[u8], clp: &[u8], hd: &Rgba, words: bool) -> Option<Rgba> {
    if hd.width != ICON_W * 4 || hd.height != ICON_H * 4 || idx.len() < (ICON_W * ICON_H) as usize {
        return None;
    }
    let s = 4;
    let mut img = hd.clone();
    for id in 0..ICON_ITEMS {
        let r = [(id % 17) * 15, (id / 17) * 18, 15, 18];
        let pal = clut_row(clp, ICON_CLUTS[(id >> 4) as usize], (id & 15) as usize);
        let cell = crop_indexed(idx, ICON_W, r, &pal);
        // the cell alone: its neighbours use other palettes
        paste_scaled(&cell, &mut img, r.map(|v| v * s));
    }
    let pieces: Vec<(Rgba, [u32; 4])> = BATTLE_WORDS
        .iter()
        .enumerate()
        .map(|(kind, &(p, h))| {
            let ps2 = [p[0], p[1], p[2] - p[0], p[3] - p[1]];
            let piece = if words {
                crop_indexed(idx, ICON_W, ps2, &clut_row(clp, 13, 13 - kind))
            } else {
                crop(hd, [h[0], h[1], h[2] - h[0], h[3] - h[1]])
            };
            (piece, ps2.map(|v| v * s))
        })
        .collect();
    for &(p, h) in &BATTLE_WORDS {
        clear(&mut img, [h[0], h[1], h[2] - h[0], h[3] - h[1]]);
        clear(&mut img, [p[0] * s, p[1] * s, (p[2] - p[0]) * s, (p[3] - p[1]) * s]);
    }
    for (piece, d) in &pieces {
        paste_scaled(piece, &mut img, *d);
    }
    Some(img)
}

/// `strtex.fmt`: 512 x 128, 4 bit, 27 menu labels in 3 columns x 9 rows of
/// 170 x 14 texels (cell n at ((n / 9) 170, (n % 9) 14)). The PS2 draws them
/// (`0x00201768`, `0x002018e4`) with `menu.clp` CLUT 13 row 7 (a grey ramp
/// with an alpha ramp); the PC's `TOMkpShape2ECCCell_008f8d50` takes the
/// same cells over `TOGetImageWH(0x2ecc)` = 512 x 128. Text: only into the
/// folder of the disc source's language.
fn convert_strtex(idx: &[u8], clp: &[u8], hd: &Rgba) -> Option<Rgba> {
    let (w, h) = (512u32, 128u32);
    if hd.width % w != 0 || hd.height % h != 0 || hd.width / w != hd.height / h || idx.len() < (w * h) as usize {
        return None;
    }
    let s = hd.width / w;
    let pal = clut_row(clp, 13, 7);
    let mut img = Rgba::new(hd.width, hd.height);
    for n in 0..27 {
        let r = [(n / 9) * 170, (n % 9) * 14, 170, 14];
        let cell = crop_indexed(idx, w, r, &pal);
        paste_scaled(&cell, &mut img, r.map(|v| v * s));
    }
    Some(img)
}

// --------------------------------------------------------------- portraits

/// A portrait sheet: a grid of faces. The PS2 and HD grids have the same
/// number of cells but not the same cell size (PS2 face_ply 91x98 texels,
/// HD 200x200 pixels) or order (face_smn's Magus Sisters), so every HD
/// cell gets the PS2 cell that looks most like it, resampled into the HD
/// cell's place at PS2 resolution.
struct Sheet {
    name: &'static str,
    w: u32,
    h: u32,
    palette: usize,
    cols: u32,
    rows: u32,
}

const SHEETS: &[Sheet] = &[
    Sheet {
        name: "face_ply",
        w: 512,
        h: 256,
        palette: 10,
        cols: 4,
        rows: 2,
    },
    Sheet {
        name: "face_smn",
        w: 512,
        h: 256,
        palette: 11,
        cols: 5,
        rows: 2,
    },
    Sheet {
        name: "sface_ply",
        w: 256,
        h: 128,
        palette: 10,
        cols: 4,
        rows: 4,
    },
];

/// The box of the texels that differ from the corner texel (the sheet's
/// unused background).
fn content_box(img: &Rgba, by_alpha: bool) -> [u32; 4] {
    let bg = img.get(img.width - 1, img.height - 1);
    let mut b = [u32::MAX, u32::MAX, 0, 0];
    for y in 0..img.height {
        for x in 0..img.width {
            let c = img.get(x, y);
            let used = if by_alpha { c[3] > 16 } else { c != bg };
            if used {
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            }
        }
    }
    if b[0] == u32::MAX { [0, 0, img.width, img.height] } else { b }
}

/// Luminance of `img`'s rectangle at 24x24, normalised (for correlation).
fn signature(img: &Rgba, r: [f32; 4]) -> Vec<f32> {
    let n = 24u32;
    let mut v: Vec<f32> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            let c = bilinear(img, r[0] + (x as f32 + 0.5) * r[2] / n as f32, r[1] + (y as f32 + 0.5) * r[3] / n as f32);
            luma([c[0], c[1], c[2], 255])
        })
        .collect();
    let m = v.iter().sum::<f32>() / v.len() as f32;
    v.iter_mut().for_each(|x| *x -= m);
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    v.iter_mut().for_each(|x| *x /= norm);
    v
}

fn convert_sheet(sh: &Sheet, idx: &[u8], clp: &[u8], hd: &Rgba) -> Option<(Rgba, String)> {
    let ps2 = indexed(sh.w, sh.h, idx, &clp_palette(clp, sh.palette));
    let pb = content_box(&ps2, false);
    let hb = content_box(hd, true);
    let cell = |b: [u32; 4], i: u32| -> [f32; 4] {
        let (cw, ch) = ((b[2] - b[0]) as f32 / sh.cols as f32, (b[3] - b[1]) as f32 / sh.rows as f32);
        [b[0] as f32 + (i % sh.cols) as f32 * cw, b[1] as f32 + (i / sh.cols) as f32 * ch, cw, ch]
    };
    let n = sh.cols * sh.rows;
    let psig: Vec<Vec<f32>> = (0..n).map(|i| signature(&ps2, cell(pb, i))).collect();
    let hsig: Vec<Vec<f32>> = (0..n).map(|i| signature(hd, cell(hb, i))).collect();
    // Same order as the PS2 sheet, except where swapping two cells is
    // clearly better (face_smn's Sandy and Cindy). The HD faces are new
    // crops of the same art, so the correlations are low in general.
    let corr = |i: u32, j: u32| -> f32 {
        hsig[i as usize].iter().zip(&psig[j as usize]).map(|(a, b)| a * b).sum()
    };
    let mut assign: Vec<u32> = (0..n).collect();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (assign[i as usize], assign[j as usize]);
            if corr(i, b) + corr(j, a) > corr(i, a) + corr(j, b) + 0.4 {
                assign.swap(i as usize, j as usize);
            }
        }
    }
    // the result at the PS2 sheet's size, in the HD layout
    let (ow, oh) = (sh.w, sh.h);
    let mut out = Rgba::new(ow, oh);
    let (sx, sy) = (ow as f32 / hd.width as f32, oh as f32 / hd.height as f32);
    let mut notes = Vec::new();
    for i in 0..n {
        let j = assign[i as usize];
        let c = corr(i, j);
        let hc = cell(hb, i);
        let d = [
            (hc[0] * sx).round() as u32,
            (hc[1] * sy).round() as u32,
            ((hc[0] + hc[2]) * sx).round() as u32,
            ((hc[1] + hc[3]) * sy).round() as u32,
        ];
        blit(&ps2, cell(pb, j), &mut out, [d[0], d[1], d[2] - d[0], d[3] - d[1]]);
        notes.push(format!("{i}<-{j} ({c:.2})"));
    }
    if std::env::var_os("DM_DEBUG").is_some() {
        eprintln!("{}: PS2 box {pb:?}, HD box {hb:?}: {}", sh.name, notes.join(" "));
    }

    // opaque faces: the HD material may ignore alpha, the PS2 alpha is 0x80
    for p in out.pixels.chunks_exact_mut(4) {
        if p[3] > 0 {
            p[3] = 255;
        }
    }
    Some((out, notes.join(" ")))
}

// ------------------------------------------------------------------- title

/// The 2D layers of a `MAP1` `.vpa`'s section 0 (`YNSC`): every PSMT8 image
/// transfer (BITBLTBUF, TRXPOS, TRXREG, GIF IMAGE) with the 16x16 CT32 CLUT
/// that follows it (`docs/research/ps2-textures.md`).
pub fn bg_layers(vpa: &[u8]) -> Vec<Rgba> {
    let u32_at = |o: usize| vpa.get(o..o + 4).map_or(0, |s| u32::from_le_bytes(s.try_into().unwrap()));
    let u64_at = |o: usize| vpa.get(o..o + 8).map_or(0, |s| u64::from_le_bytes(s.try_into().unwrap()));
    if vpa.len() < 0x80 || &vpa[..4] != b"MAP1" {
        return Vec::new();
    }
    let s0 = u32_at(0x10) as usize;
    let end = (0x14..0x80)
        .step_by(4)
        .map(|o| u32_at(o) as usize)
        .filter(|&s| s > s0 && s < vpa.len())
        .min()
        .unwrap_or(vpa.len());
    // (psm, w, h, data offset)
    let mut xfers: Vec<(u32, u32, u32, usize)> = Vec::new();
    let mut o = s0.max(0x20);
    while o + 0x30 < end {
        let lo = u64_at(o);
        let (nloop, flg) = ((lo & 0x7fff) as usize, (lo >> 58) & 3);
        if flg == 2 && nloop > 0 && (lo >> 15) & 1 == 1 && u64_at(o + 8) == 0 {
            let bb = u64_at(o - 0x20);
            let (w, h) = (u32_at(o - 0x10), u32_at(o - 0xc));
            let psm = ((bb >> 56) & 63) as u32;
            if (1..=1024).contains(&w) && (1..=1024).contains(&h) && (psm == 0x13 || psm == 0) {
                xfers.push((psm, w, h, o + 16));
                o += 16 + nloop * 16;
                continue;
            }
        }
        o += 16;
    }
    let mut out = Vec::new();
    for (k, &(psm, w, h, data)) in xfers.iter().enumerate() {
        if psm != 0x13 {
            continue;
        }
        let Some(&(_, _, _, cl)) = xfers[k + 1..].iter().find(|x| x.0 == 0 && x.1 == 16 && x.2 == 16) else {
            continue;
        };
        let Some(raw) = vpa.get(cl..cl + 0x400) else { continue };
        let pal: Vec<[u8; 4]> = (0..256u32)
            .map(|i| {
                let j = gsmem::csm1(i) as usize * 4;
                [raw[j], raw[j + 1], raw[j + 2], gsmem::alpha(raw[j + 3])]
            })
            .collect();
        let Some(idx) = vpa.get(data..data + (w * h) as usize) else { continue };
        out.push(indexed(w, h, idx, &pal));
    }
    out
}

/// HD 2D-layer names: `0_<layer>_<w>_<h>_0.dds.phyre`.
fn layer_name(file: &str) -> Option<(usize, u32, u32)> {
    let s = file.strip_suffix(".dds.phyre")?;
    let p: Vec<&str> = s.split('_').collect();
    match p.as_slice() {
        ["0", k, w, h, "0"] => Some((k.parse().ok()?, w.parse().ok()?, h.parse().ok()?)),
        _ => None,
    }
}

/// The title screen's layers by language: HD layer -> PS2 layer. Both
/// have 0 (International-style English, upper case), 1 (black), 5 Italian,
/// 7 French, 9 German, 11 Spanish and the "Install" labels 2, 4, 6, 8, 10,
/// 12. The HD put Chinese at 3, where the PS2 has the PAL English title; the
/// HD English is 13. 14 and 15 (Japanese/Chinese, Korean) have no PS2 layer.
fn title_layer(hd: usize) -> Option<usize> {
    match hd {
        3 | 14 | 15 => None,
        13 => Some(3),
        k => Some(k),
    }
}

/// A full-screen `titl00` layer (512 x 448) laid out so the PC's quad shows
/// it where the PS2 did (`docs/ui.md`, "Title screen and logos").
///
/// The PS2 shows rows 16..432 of the image 1:1 on its 512 x 416 screen. The
/// PC draws the layer as a 4:3 quad as wide as the screen, bottom-aligned
/// (measured, 16:9 and 4:3 alike: texture row v at screen line
/// `v * 960 / 448 - 240` of 720), so it showed only rows 112..448, 1.24
/// times as big and 60 lines higher at the logo. The texture here holds, at
/// row v, the PS2 row that the PS2 showed at that screen line:
/// `16 + (v * 960 / 448 - 240) * 416 / 720`; columns stay. Written at twice
/// the PS2 size (1024 x 896) so the 0.8 scale loses no PS2 detail. The
/// PAL web-site banner and the still water strip under it are kept, as the
/// PS2 showed them: the mod turns the PC's animated water band
/// (`menu/titl/water_NN`) off (`mod/TitlePatches.cs`).
pub fn title_placed(img: &Rgba) -> Rgba {
    let (w, h) = (img.width * 2, img.height * 2);
    let mut out = Rgba::new(w, h);
    let sample = |x: f32, y: f32| -> [u8; 4] {
        let x = (x - 0.5).clamp(0.0, (img.width - 1) as f32);
        let y = (y - 0.5).clamp(0.0, (img.height - 1) as f32);
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(img.width - 1), (y0 + 1).min(img.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let (a, b, c, d) = (img.get(x0, y0), img.get(x1, y0), img.get(x0, y1), img.get(x1, y1));
        std::array::from_fn(|k| {
            let top = a[k] as f32 * (1.0 - fx) + b[k] as f32 * fx;
            let bot = c[k] as f32 * (1.0 - fx) + d[k] as f32 * fx;
            (top * (1.0 - fy) + bot * fy).round() as u8
        })
    };
    let hs = img.height as f32;
    for y in 0..h {
        let v = (y as f32 + 0.5) / 2.0;
        let line = v * 960.0 / hs - 240.0;
        let r = 16.0 + line * 416.0 / 720.0;
        for x in 0..w {
            out.put(x, y, sample((x as f32 + 0.5) / 2.0, r));
        }
    }
    out
}

fn lower_half(img: &Rgba) -> Rgba {
    let h = img.height / 2;
    let row = img.width as usize * 4;
    Rgba {
        width: img.width,
        height: img.height - h,
        pixels: img.pixels[h as usize * row..].to_vec(),
    }
}

// ----------------------------------------------------------------- glyphs

/// `pad_icon`: a 12x8 grid of cells (`RemapButtonIcon_getJoypadUV`, u =
/// column / 12, v = row / 8). Row 0: X A B Y LB RB LT RT Back Start LS RS.
/// The PlayStation glyphs come from the PC's own `icon` atlas (HD pixels,
/// the PS2 layout x4: `docs/ui.md`), SELECT and START from the PS2
/// `icon.fmt` (a grey ramp: the PS2 colours these at run time).
const PAD_GLYPHS: &[(u32, [u32; 4])] = &[
    (0, [290, 225, 42, 42]), // X -> square
    (1, [194, 225, 42, 42]), // A -> cross
    (2, [97, 225, 42, 42]),  // B -> circle
    (3, [1, 225, 42, 42]),   // Y -> triangle
    (4, [512, 224, 47, 31]), // LB -> L1
    (5, [705, 224, 47, 31]), // RB -> R1
    (6, [609, 224, 47, 31]), // LT -> L2
    (7, [801, 224, 47, 31]), // RT -> R2
];

/// SELECT and START in the PS2 `icon.fmt` (256x128, 4 bits): cells 8 and 9.
const PAD_PS2_TEXT: &[(u32, [u32; 4])] = &[(8, [1, 80, 41, 14]), (9, [51, 80, 38, 14])];

fn fit_into(dst: &mut Rgba, cell: u32, src: &Rgba, r: [u32; 4]) {
    let cw = dst.width / 12;
    let ch = dst.height / 8;
    let (cx, cy) = ((cell % 12) * cw, (cell / 12) * ch);
    for y in 0..ch {
        for x in 0..cw {
            dst.put(cx + x, cy + y, [0; 4]);
        }
    }
    let inner = cw.saturating_sub(4) as f32;
    let k = (inner / r[2] as f32).min(inner / r[3] as f32);
    let (w, h) = ((r[2] as f32 * k).round() as u32, (r[3] as f32 * k).round() as u32);
    let (ox, oy) = (cx + (cw - w) / 2, cy + (ch - h) / 2);
    blit(src, [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32], dst, [ox, oy, w, h]);
}

// ----------------------------------------------------------------- driver

/// Every UI file of `parts`, from the disc and the game's archive.
pub fn convert(disc: &mut Disc, pc: &ffx_vbf::Archive, parts: Parts) -> Result<(Vec<Done>, Vec<String>)> {
    let mut done = Vec::new();
    let mut notes = Vec::new();
    let region = disc.region.clone();

    if parts.menus || parts.portraits || parts.font {
        // the battle atlas's damage digits (ui.font) go to every European
        // folder, from the disc's own atlas where a folder has no source
        let sources = menu_sources(&region);
        let mut todo = sources.clone();
        if parts.font {
            for folder in EUROPEAN {
                if !todo.iter().any(|(_, f)| *f == folder) {
                    todo.push((sources[0].0, folder));
                }
            }
        }
        if parts.menus {
            for folder in ALL_MENU_FOLDERS {
                if !todo.iter().any(|(_, f)| *f == folder) {
                    todo.push((sources[0].0, folder));
                }
            }
        }
        for (k, (dir, folder)) in todo.into_iter().enumerate() {
            // a folder without a disc source of its own: digits and the
            // message window's pieces only
            let own = k < sources.len();
            let Some(clp) = disc.read(&format!("{dir}/menu.clp"))? else {
                continue;
            };
            for a in ATLASES {
                let digits = parts.font && a.name == "battle" && EUROPEAN.contains(&folder);
                let pieces = parts.menus && a.name == "meswin";
                if !own && !digits && !pieces {
                    continue;
                }
                let path = format!("{PS3}/{folder}/d3d11/{}.dds.phyre", a.name);
                let Some(idx) = disc.read(&format!("{dir}/{}.fmt", a.name))? else { continue };
                let Some((template, hd)) = load_hd(pc, &path)? else { continue };
                let (mut img, used, kept) = if own && (parts.menus || parts.portraits) {
                    convert_atlas(a, &idx, &clp, &hd, parts.menus, parts.portraits).unwrap_or((hd.clone(), 0, 0))
                } else {
                    (hd.clone(), 0, 0)
                };
                let mut what = vec![];
                if used > 0 {
                    what.push(format!("{used} PS2 elements, {kept} kept HD"));
                }
                if digits && paste_digits(a, &idx, &clp, &mut img) {
                    what.push("PS2 damage digits".to_string());
                }
                if pieces && paste_window_pieces(a, &idx, &clp, &hd, &mut img) {
                    what.push("PS2 message window pieces".to_string());
                }
                if what.is_empty() {
                    notes.push(format!("{path}: no PS2 element matches"));
                    continue;
                }
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, None)?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{path}: {} ({dir})", what.join(", ")),
                    path,
                });
            }
        }
    }

    if parts.menus {
        // the menu layout words (TOSeqInitParam's, TkMn2SeqGet*): the PC reads the Japanese
        // International's `jppc/menu/menu_script.bin` for every language; the PAL game reads
        // its own disc's, whose words differ (Config: plates from y 65, 47 apart, not 55 / 45)
        if let (Some(raw), Some(e)) = (disc.read(MENU_SCRIPT_PS2)?, pc.find(MENU_SCRIPT_PC)) {
            if pc.read(e)? != raw {
                done.push(Done {
                    path: MENU_SCRIPT_PC.to_string(),
                    file: raw,
                    preview: Rgba::new(1, 1),
                    note: format!("{MENU_SCRIPT_PC}: the disc's menu layout ({MENU_SCRIPT_PS2})"),
                });
            }
        }
        let sources = menu_sources(&region);
        for folder in ALL_MENU_FOLDERS {
            // the English game reads some sheets (`icon`) from `menu`, not `menu_us`
            // (seen in the harness: the words drawn were `menu`'s), so `menu` gets the
            // English source too
            let lang = if folder == "menu" { "menu_us" } else { folder };
            let own = sources.iter().any(|(_, f)| *f == lang);
            let dir = sources.iter().find(|(_, f)| *f == lang).map_or(sources[0].0, |(d, _)| *d);
            let Some(clp) = disc.read(&format!("{dir}/menu.clp"))? else { continue };
            for (a, (name, id)) in ATLASES.iter().zip([(PS2_MESWIN, PS2_MESWIN_ID), (PS2_BATTLE, PS2_BATTLE_ID)]) {
                let Some(idx) = disc.read(&format!("{dir}/{}.fmt", a.name))? else { continue };
                let hd_path = format!("{PS3}/{folder}/d3d11/{}.dds.phyre", a.name);
                let Some((template, hd)) = load_hd(pc, &hd_path)? else { continue };
                let img = indexed(a.w, a.h, &idx, &clp_palette(&clp, a.palette));
                let path = format!("{PS3}/{folder}/d3d11/{name}.dds.phyre");
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, Some((&format!("{}.dds", a.name), id)))?,
                    preview: side_by_side(&hd, &resize(&img, hd.width, hd.height)),
                    note: format!("{path}: the whole PS2 {}, PS2 size ({dir})", a.name),
                    path,
                });
            }
            // the PS2 fonts M and S (no text of their own: every folder)
            let font_hd = format!("{PS3}/{folder}/d3d11/meswin.dds.phyre");
            // (`uspc/menu` has no `xfont1208.fmt`: the first source's, with its `menu.clp`)
            let fdir = sources[0].0;
            let font = match disc.read(&format!("{fdir}/xfont1208.fmt"))? {
                Some(raw) => disc.read(&format!("{fdir}/menu.clp"))?.map(|c| (raw, c)),
                None => None,
            };
            if let (Some((raw, fclp)), Some((template, hd))) = (font, load_hd(pc, &font_hd)?) {
                let img = ps2_font(&raw, &fclp);
                let path = format!("{PS3}/{folder}/d3d11/{PS2_FONT}.dds.phyre");
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, Some(("meswin.dds", PS2_FONT_ID)))?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{path}: the PS2 fonts M and S in their colours ({fdir})"),
                    path,
                });
            }
            // the item icons and battle words
            let path = format!("{PS3}/{folder}/d3d11/icon.dds.phyre");
            if let (Some(raw), Some((template, hd))) = (disc.read(&format!("{dir}/icon.fmt"))?, load_hd(pc, &path)?) {
                match convert_icon(&nibbles(&raw), &clp, &hd, own) {
                    Some(img) => done.push(Done {
                        file: phyretex::argb8_one_level(&template, &img, None)?,
                        preview: side_by_side(&hd, &img),
                        note: format!(
                            "{path}: {ICON_ITEMS} PS2 item icons, battle words at the PS2 places ({})",
                            if own { dir } else { "HD words" }
                        ),
                        path,
                    }),
                    None => notes.push(format!("{path}: not the PS2 layout at 4x, kept HD")),
                }
            }
            // the battle and menu labels (text: the disc's languages only)
            let path = format!("{PS3}/{folder}/d3d11/strtex.dds.phyre");
            if own {
                if let (Some(raw), Some((template, hd))) = (disc.read(&format!("{dir}/strtex.fmt"))?, load_hd(pc, &path)?) {
                    match convert_strtex(&nibbles(&raw), &clp, &hd) {
                        Some(img) => done.push(Done {
                            file: phyretex::argb8_one_level(&template, &img, None)?,
                            preview: side_by_side(&hd, &img),
                            note: format!("{path}: the PS2 labels ({dir})"),
                            path,
                        }),
                        None => notes.push(format!("{path}: not the PS2 layout, kept HD")),
                    }
                }
            }
        }
        // the world map picture and the sphere grid's pages (jobs/sphere_grid.rs)
        crate::jobs::sphere_grid::world_map(disc, pc, clp_palette, &ALL_MENU_FOLDERS, &mut done, &mut notes)?;
        crate::jobs::sphere_grid::sphere_grid(disc, pc, &region, &mut done, &mut notes)?;
    }

    if parts.portraits {
        // Faces are not text: the disc's own sheets go to every folder.
        let (dir, _) = menu_sources(&region)[0];
        if let Some(clp) = disc.read(&format!("{dir}/menu.clp"))? {
            for sh in SHEETS {
                let Some(idx) = disc.read(&format!("{dir}/{}.fmt", sh.name))? else { continue };
                for folder in ["menu", "menu_us", "menu_de", "menu_fr", "menu_it", "menu_es"] {
                    let path = format!("{PS3}/{folder}/d3d11/{}.dds.phyre", sh.name);
                    let Some((template, hd)) = load_hd(pc, &path)? else { continue };
                    match convert_sheet(sh, &idx, &clp, &hd) {
                        Some((img, how)) => done.push(Done {
                            file: phyretex::argb8_one_level(&template, &img, None)?,
                            preview: side_by_side(&hd, &img),
                            note: format!("{path}: cells {how}"),
                            path,
                        }),
                        None => notes.push(format!("{path}: a face matches no PS2 face, kept HD")),
                    }
                }
            }
        }
    }

    if parts.title {
        for map in ["titl00", "titl01", "titl02"] {
            let Some(vpa) = disc.read(&format!("jppc/map/titl/{map}/bin/mapout.vpa"))? else { continue };
            let layers = bg_layers(&vpa);
            let prefix = format!("{PS3}/map/titl/{map}/2d/tex/d3d11/");
            let names: Vec<String> = pc
                .entries()
                .into_iter()
                .filter(|e| e.path.starts_with(&prefix))
                .map(|e| e.path.clone())
                .collect();
            for path in names {
                let file = &path[prefix.len()..];
                let Some((hk, w, h)) = layer_name(file) else { continue };
                let k = if map == "titl00" { title_layer(hk) } else { Some(hk) };
                let Some((k, img)) = k.and_then(|k| layers.get(k).map(|i| (k, i))) else {
                    notes.push(format!("{path}: no PS2 layer for it (an HD-only language)"));
                    continue;
                };
                if w.abs_diff(img.width) > 8 || h.abs_diff(img.height) > 8 {
                    notes.push(format!("{path}: PS2 layer {k} is {}x{}", img.width, img.height));
                    continue;
                }
                let Some((template, hd)) = load_hd(pc, &path)? else { continue };
                // the same picture? (a reordered language would correlate badly)
                // (lower half only: above it the HD layers are black where
                // the PS2 ones hold the water strip and the web site banner)
                let small = resize(&hd, img.width, img.height);
                let c = maps::correlation(&lower_half(&small), &lower_half(img));
                // titl00's layers are paired by language (`title_layer`);
                // titl02's boot logo (1, 5: the HD redrew the emblem and the
                // text, 0.54) by place; titl01 ("Fine") only where the HD
                // kept the PS2 picture: their layouts differ. (titl02 0, 4,
                // 7: Squaresoft against the HD's Square Enix, 0.03.)
                let min = match map {
                    "titl00" => 0.3,
                    "titl02" => 0.5,
                    _ => 0.9,
                };
                if c < min {
                    notes.push(format!("{path}: PS2 layer {k} looks different (correlation {c:.2}), kept HD"));
                    continue;
                }
                let mut img = maps::opaque_like(img.clone(), &hd);
                if map != "titl01" && img.height == 448 {
                    img = title_placed(&img);
                }
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, None)?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{path}: PS2 layer {k} (correlation {c:.2})"),
                    path,
                });
            }
        }
        // "HD Remaster" under the logo of the opening (an effect texture)
        let path = format!("{PS3}/magic/magic_0662/tex/d3d11/logo_hdremaster.dds.phyre");
        if let Some((template, hd)) = load_hd(pc, &path)? {
            let img = transparent_like(&hd);
            done.push(Done {
                file: phyretex::argb8_one_level(&template, &img, None)?,
                preview: side_by_side(&hd, &img),
                note: format!("{path}: transparent"),
                path,
            });
        }
    }

    if parts.boot_logo {
        boot_logo(disc, pc, &mut done, &mut notes)?;
    }

    if parts.loading {
        let mut paths: Vec<String> = (0..25).map(|i| format!("{PS3}/menu/loading/d3d11/ld{i:02}.dds.phyre")).collect();
        paths.push(format!("{PS3}/menu/loading/d3d11/loadingbg.dds.phyre"));
        for path in paths {
            if let Some((template, hd)) = load_hd(pc, &path)? {
                let img = transparent_like(&hd);
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, None)?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{path}: transparent"),
                    path,
                });
            }
        }
    }

    if parts.glyphs {
        let pad = format!("{PS3}/menu/d3d11/pad_icon.dds.phyre");
        let icon = format!("{PS3}/menu/d3d11/icon.dds.phyre");
        let ps2_icon = disc.read("jppc/menu/icon.fmt")?;
        if let (Some((template, hd)), Some((_, icons))) = (load_hd(pc, &pad)?, load_hd(pc, &icon)?) {
            let mut img = hd.clone();
            if icons.width == 1024 && icons.height == 512 && hd.width % 12 == 0 && hd.height % 8 == 0 {
                for &(cell, r) in PAD_GLYPHS {
                    fit_into(&mut img, cell, &icons, r);
                }
                if let Some(raw) = ps2_icon.filter(|r| r.len() >= 256 * 128 / 2) {
                    let idx: Vec<u8> = raw.iter().flat_map(|b| [b & 15, b >> 4]).collect();
                    let ramp: Vec<[u8; 4]> = (0..16u8)
                        .map(|i| [i * 17, i * 17, i * 17, if i == 0 { 0 } else { 255 }])
                        .collect();
                    let text = indexed(256, 128, &idx, &ramp);
                    for &(cell, r) in PAD_PS2_TEXT {
                        fit_into(&mut img, cell, &text, r);
                    }
                }
                done.push(Done {
                    file: phyretex::argb8_one_level(&template, &img, None)?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{pad}: PlayStation glyphs"),
                    path: pad,
                });
            } else {
                notes.push(format!("{pad}: unexpected sizes, kept HD"));
            }
        }
    }
    if parts.font_files {
        let (d, n) = crate::jobs::font::convert(disc, pc)?;
        done.extend(d);
        notes.extend(n);
    }
    Ok((done, notes))
}

// ------------------------------------------------------------------- jobs

pub fn jobs() -> Vec<Box<dyn Job>> {
    vec![
        Box::new(UiJob {
            name: "ui.menus",
            settings: &["ui.menus", "ui.portraits", "ui.font"],
        }),
        Box::new(UiJob {
            name: "ui.title",
            settings: &["ui.title"],
        }),
        Box::new(UiJob {
            name: "ui.loading",
            settings: &["ui.loading"],
        }),
        Box::new(UiJob {
            name: "ui.glyphs",
            settings: &["ui.button_glyphs"],
        }),
        Box::new(UiJob {
            name: "ui.font",
            settings: &["ui.font"],
        }),
        Box::new(UiJob {
            name: "ui.boot_logo",
            settings: &["ui.boot_logo"],
        }),
    ]
}

struct UiJob {
    name: &'static str,
    settings: &'static [&'static str],
}

impl UiJob {
    fn parts(&self, c: &Config) -> Parts {
        let ui = &c.ui;
        match self.name {
            // ui.font: the damage digits of the battle atlas this job writes
            "ui.menus" => Parts {
                menus: ui.menus.ps2(),
                portraits: ui.portraits.ps2(),
                font: ui.font.ps2(),
                ..Parts::default()
            },
            "ui.title" => Parts {
                title: ui.title.ps2(),
                ..Parts::default()
            },
            "ui.loading" => Parts {
                loading: ui.loading.ps2(),
                ..Parts::default()
            },
            "ui.font" => Parts {
                font_files: ui.font.ps2(),
                ..Parts::default()
            },
            "ui.boot_logo" => Parts {
                boot_logo: ui.boot_logo.ps2(),
                ..Parts::default()
            },
            _ => Parts {
                glyphs: ui.button_glyphs.ps2(),
                ..Parts::default()
            },
        }
    }
}

impl Job for UiJob {
    /// 2: textures with mip chains (phyretex::mip_chain). 3: the PS2 meswin
    /// as its own texture (`ps2win`, ui.menus). 4: `ps2btl`, the PS2 item
    /// icons and battle words (`icon`), `strtex`. 5: `ps2fnt`, the PS2 fonts
    /// M and S. 6: the disc's `menu_script.bin` (the PAL menu layout).
    /// 7: the title layers at the PS2's place (`title_placed`). 8: the PS2
    /// world map picture and the sphere grid's pages (`jobs/sphere_grid.rs`, ui.menus).
    /// 9: one mip level (`phyretex::argb8_one_level`) for everything drawn
    /// on the screen (not the sphere grid's pages): the game's texture
    /// quality shrinks no one-level texture (`docs/render.md`). 10: the
    /// title keeps the PS2 banner and water strip (the mod drops the PC's
    /// water band); the PS2 boot logo (`titl02` 1 and 5), and `titl02` at
    /// the PS2's place (`title_placed`).
    fn version(&self) -> u32 {
        10
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn settings(&self) -> &'static [&'static str] {
        self.settings
    }
    fn enabled(&self, config: &Config) -> bool {
        let p = self.parts(config);
        p.menus || p.portraits || p.title || p.loading || p.glyphs || p.font || p.font_files || p.boot_logo
    }
    fn run(&self, ctx: &Ctx, out: &Output) -> Result<String> {
        let parts = self.parts(ctx.config);
        let (done, notes) = ctx.disc(|d| convert(d, ctx.pc, parts))?;
        let mut n = 0;
        for d in &done {
            if out.put(&d.path, &d.file)? {
                n += 1;
            }
            ctx.note(&d.note);
        }
        for line in &notes {
            ctx.note(line);
        }
        Ok(format!("{n} files ({} left HD, see --verbose)", notes.len()))
    }
}
