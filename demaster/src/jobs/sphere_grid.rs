//! The sphere grid's textures and the world map picture (`ui.menus`, written
//! by `interface::convert`; notes and evidence: `docs/mods/interface.md`, "The sphere grid"
//! and "The world map").
//!
//! **Sphere grid.** `jppc/menu/abmap/dat00.dat` (standard grid) and
//! `dat12.dat` (expert grid) hold the grid's PS2 textures: TEX0 records of
//! 0x20 bytes from 0x60 (slot `k` at `0x60 + 0x20 k`: the TEX0, then u16
//! place (x / 16 in the low byte, y / 16 in the high byte), page size
//! (w / 16, h / 16), w, h, bytes), CLUT upload records of 0x10 from 0x580,
//! and at 0x810 a second header whose words 2 and 3 point (from 0x810) at
//! the image and CLUT offset tables (one u32 per slot, offsets from 0x810).
//! Images are linear PSMT8, CLUTs 256 PSMCT32 colours in CSM1 order. The HD
//! names `menu/abmap/datNN/tex/d3d11/<TBP>_19` are the PS2 pages (TBP, PSM 19
//! = PSMT8) redrawn at 2x in the same layout, except `00000_19` (one of the
//! two images uploaded at 13568), `12288_19` (an HD-only interface atlas:
//! the S.LV title plates and the localised titles), `lines` and the `aio`
//! atlases (the five node-icon pages in one sheet). Each HD texture gets the
//! PS2 page or image it correlates with best, at PS2 size.
//!
//! **World map.** `jppc/menu/worldmap.fmt` is the whole 512 x 416 screen
//! (PSMT8, loaded to TBP 0x3800 and drawn with CBP 0x2eac = `menu.clp`
//! CLUT 5, PS2 0x00200320). The PC draws `menu*/d3d11/worldmap` as one quad
//! over the same 512 x 416 PS2 screen with UVs 0..512 / 512, 0..416 / 416
//! (`TODrawUIPicture_008f2bf0`, `TOGetImageWH(0x2eac)` = 512 x 416), so the
//! PS2 picture fits as is; the place markers (`scene30` at the map
//! marker's x / 2, y / 2 - 16) are the same code on both.

use anyhow::Result;

use crate::disc::Disc;
use crate::formats::gsmem;
use crate::jobs::maps::{self, resize};
use crate::formats::phyretex::{self, Rgba};
use crate::jobs::interface::{load_hd, side_by_side, Done};

const PS3: &str = "ffx_data/gamedata/ps3data";

fn u16_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 2).map_or(0, |s| u32::from(u16::from_le_bytes([s[0], s[1]])))
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// One PS2 image of an abmap `datNN.dat`, coloured with its CLUT.
#[derive(Clone)]
pub struct Slot {
    pub slot: usize,
    pub tbp: u32,
    pub x: u32,
    pub y: u32,
    pub page_w: u32,
    pub page_h: u32,
    pub image: Rgba,
}

/// The images of an abmap `datNN.dat` (the format in the module notes).
pub fn slots(b: &[u8]) -> Vec<Slot> {
    const TEX0: usize = 0x60;
    const CLUTS: usize = 0x580;
    const SUB: usize = 0x810;
    let images = SUB + u32_at(b, SUB + 8) as usize;
    let cluts = SUB + u32_at(b, SUB + 12) as usize;
    let mut out = Vec::new();
    for slot in 0..(CLUTS - TEX0) / 0x20 {
        let o = TEX0 + 0x20 * slot;
        let lo = u32_at(b, o);
        let hi = u32_at(b, o + 4);
        if lo == 0 && hi == 0 {
            continue;
        }
        let tex0 = u64::from(lo) | u64::from(hi) << 32;
        let tbp = (tex0 & 0x3fff) as u32;
        let psm = ((tex0 >> 20) & 0x3f) as u32;
        let (place, page, w, h) = (u16_at(b, o + 8), u16_at(b, o + 10), u16_at(b, o + 12), u16_at(b, o + 14));
        if psm != 19 || w == 0 || h == 0 {
            continue;
        }
        let io = SUB + u32_at(b, images + 4 * slot) as usize;
        let co = SUB + u32_at(b, cluts + 4 * slot) as usize;
        let (Some(pix), Some(clut)) = (b.get(io..io + (w * h) as usize), b.get(co..co + 1024)) else {
            continue;
        };
        let pal: Vec<[u8; 4]> = (0..256u32)
            .map(|i| {
                let j = 4 * gsmem::csm1(i) as usize;
                [clut[j], clut[j + 1], clut[j + 2], gsmem::alpha(clut[j + 3])]
            })
            .collect();
        let mut image = Rgba::new(w, h);
        for (i, &k) in pix.iter().enumerate() {
            image.pixels[4 * i..4 * i + 4].copy_from_slice(&pal[usize::from(k)]);
        }
        out.push(Slot {
            slot,
            tbp,
            x: (place & 0xff) * 16,
            y: (place >> 8) * 16,
            page_w: (page & 0xff) * 16,
            page_h: (page >> 8) * 16,
            image,
        });
    }
    out
}

fn paste(dst: &mut Rgba, src: &Rgba, x: u32, y: u32) {
    for sy in 0..src.height {
        for sx in 0..src.width {
            if x + sx < dst.width && y + sy < dst.height {
                let o = 4 * ((y + sy) * dst.width + x + sx) as usize;
                dst.pixels[o..o + 4].copy_from_slice(&src.get(sx, sy));
            }
        }
    }
}

/// The candidates an HD texture can be: each TBP's page with every image
/// uploaded there (in file order), and each image alone on its page.
fn candidates(slots: &[Slot]) -> Vec<(String, Rgba)> {
    let mut out: Vec<(String, Rgba)> = Vec::new();
    let mut tbps: Vec<u32> = slots.iter().map(|s| s.tbp).collect();
    tbps.dedup();
    tbps.sort_unstable();
    tbps.dedup();
    for tbp in tbps {
        let on: Vec<&Slot> = slots.iter().filter(|s| s.tbp == tbp).collect();
        let (w, h) = (on[0].page_w, on[0].page_h);
        let mut page = Rgba::new(w, h);
        for s in &on {
            paste(&mut page, &s.image, s.x, s.y);
        }
        let ids: Vec<String> = on.iter().map(|s| s.slot.to_string()).collect();
        out.push((format!("page {tbp} (images {})", ids.join(", ")), page));
        if on.len() > 1 {
            for s in on {
                let mut page = Rgba::new(w, h);
                paste(&mut page, &s.image, s.x, s.y);
                out.push((format!("image {} (at {tbp})", s.slot), page));
            }
        }
    }
    out
}

/// Correlation an HD texture needs with a PS2 page to take it (the HD pages
/// redrawn from the PS2's correlate 0.8 and more; the HD-only interface
/// atlas `12288_19` below 0.3).
const MIN_CORRELATION: f32 = 0.6;

/// The node-icon pages in an `aio` atlas: (TBP, x, y) in HD texels of the
/// 512 x 512 sheet, each page at its own HD size (2x the PS2's), found by
/// matching the HD pages in it.
const AIO: [(u32, u32, u32); 5] = [(14176, 0, 0), (14240, 256, 0), (14112, 0, 256), (14080, 0, 384), (14144, 256, 384)];

/// The sphere grid textures for one grid (`dat00` or `dat12`).
pub fn sphere_grid(disc: &mut Disc, pc: &ffx_vbf::Archive, region: &str, done: &mut Vec<Done>, notes: &mut Vec<String>) -> Result<()> {
    for dat in ["dat00", "dat12"] {
        let Some(raw) = disc.read(&format!("jppc/menu/abmap/{dat}.dat"))? else { continue };
        let slots = slots(&raw);
        if slots.is_empty() {
            notes.push(format!("jppc/menu/abmap/{dat}.dat: no images"));
            continue;
        }
        let cands = candidates(&slots);
        let prefix = format!("{PS3}/menu/abmap/{dat}/tex/d3d11/");
        let names: Vec<String> = pc
            .entries()
            .into_iter()
            .filter(|e| e.path.starts_with(&prefix))
            .map(|e| e.path.clone())
            .collect();
        let mut pages_by_tbp: Vec<(u32, Rgba)> = Vec::new();
        for path in &names {
            let file = &path[prefix.len()..];
            if file.starts_with("aio") || file.starts_with("lines") {
                continue;
            }
            let Some((template, hd)) = load_hd(pc, path)? else { continue };
            // the page of the name's own TBP when it passes, else any (00000_19 is an image
            // uploaded at 13568; similar glows sit at several TBPs)
            let own = file.split('_').next().map_or(String::new(), |t| format!("page {t} "));
            // (the same aspect only: a 128 x 64 page is not a 128 x 128 one)
            let scored: Vec<(f32, &(String, Rgba))> = cands
                .iter()
                .filter(|c| u64::from(c.1.width) * u64::from(hd.height) == u64::from(c.1.height) * u64::from(hd.width))
                .map(|c| (maps::correlation(&resize(&hd, c.1.width, c.1.height), &c.1), c))
                .collect();
            let pick = |own_only: bool| {
                scored
                    .iter()
                    .filter(|(r, c)| !own_only || (c.0.starts_with(&own) && *r >= MIN_CORRELATION))
                    .fold(None::<(f32, &(String, Rgba))>, |b, &(r, c)| if b.is_none_or(|(br, _)| r > br) { Some((r, c)) } else { b })
            };
            let best = pick(true).or_else(|| pick(false));
            let Some((r, (what, img))) = best.filter(|(r, _)| *r >= MIN_CORRELATION) else {
                notes.push(format!(
                    "{path}: no PS2 page like it (best {:.2}), kept HD",
                    best.map_or(0.0, |(r, _)| r)
                ));
                continue;
            };
            if let Some(tbp) = file.split('_').next().and_then(|t| t.parse::<u32>().ok()) {
                pages_by_tbp.push((tbp, img.clone()));
            }
            let glow = additive(&hd, img);
            let img = if glow { crate::jobs::effects::brightness_to_alpha(img) } else { img.clone() };
            done.push(Done {
                file: phyretex::argb8_from(&template, &img)?,
                preview: side_by_side(&hd, &resize(&img, hd.width, hd.height)),
                note: format!(
                    "{path}: PS2 {what} ({dat}, correlation {r:.2}{})",
                    if glow { ", brightness as alpha" } else { "" }
                ),
                path: path.clone(),
            });
        }
        // the node icons in one sheet: the base sheet and the disc's language's
        let lang = match region {
            "es" => "aio_es",
            "fr" => "aio_fr",
            "it" => "aio_it",
            _ => "aio",
        };
        let path = format!("{prefix}{lang}.dds.phyre");
        if let Some((template, hd)) = load_hd(pc, &path)? {
            let mut img = hd.clone();
            let mut n = 0;
            for (tbp, x, y) in AIO {
                let Some((_, page)) = pages_by_tbp.iter().find(|(t, _)| *t == tbp) else { continue };
                let up = resize_bilinear(page, page.width * 2, page.height * 2);
                clear(&mut img, x, y, up.width, up.height);
                paste(&mut img, &up, x, y);
                n += 1;
            }
            if n > 0 {
                done.push(Done {
                    file: phyretex::argb8_from(&template, &img)?,
                    preview: side_by_side(&hd, &img),
                    note: format!("{path}: {n} PS2 node-icon pages at 2x ({dat})"),
                    path,
                });
            }
        }
    }
    Ok(())
}

/// Whether a PS2 page was drawn additively: most of its opaque dark texels
/// (the black around glows and rings, which an additive draw leaves out) are
/// transparent in the HD redraw. Such pages get their brightness as alpha
/// (`effects::brightness_to_alpha`), so the PC's alpha blend shows what the
/// PS2's additive draw did instead of black boxes.
fn additive(hd: &Rgba, ps2: &Rgba) -> bool {
    let h = resize(hd, ps2.width, ps2.height);
    let (mut dark, mut clear) = (0u32, 0u32);
    for (p, q) in ps2.pixels.chunks_exact(4).zip(h.pixels.chunks_exact(4)) {
        if p[3] > 200 && p[0].max(p[1]).max(p[2]) < 24 {
            dark += 1;
            if q[3] < 32 {
                clear += 1;
            }
        }
    }
    dark > 16 && clear * 2 > dark
}

fn clear(img: &mut Rgba, x: u32, y: u32, w: u32, h: u32) {
    for yy in y..(y + h).min(img.height) {
        for xx in x..(x + w).min(img.width) {
            let o = 4 * (yy * img.width + xx) as usize;
            img.pixels[o..o + 4].copy_from_slice(&[0; 4]);
        }
    }
}

/// Bilinear upscale (texel centres), the filter the game's sampler would
/// apply to the PS2-size page.
fn resize_bilinear(src: &Rgba, w: u32, h: u32) -> Rgba {
    let mut out = Rgba::new(w, h);
    let (sx, sy) = (src.width as f32 / w as f32, src.height as f32 / h as f32);
    for y in 0..h {
        for x in 0..w {
            let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (src.width - 1) as f32);
            let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (src.height - 1) as f32);
            let (x0, y0) = (fx as u32, fy as u32);
            let (x1, y1) = ((x0 + 1).min(src.width - 1), (y0 + 1).min(src.height - 1));
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let (a, b, c, d) = (src.get(x0, y0), src.get(x1, y0), src.get(x0, y1), src.get(x1, y1));
            let mut p = [0u8; 4];
            for k in 0..4 {
                let top = f32::from(a[k]) * (1.0 - tx) + f32::from(b[k]) * tx;
                let bot = f32::from(c[k]) * (1.0 - tx) + f32::from(d[k]) * tx;
                p[k] = (top * (1.0 - ty) + bot * ty).round() as u8;
            }
            let o = 4 * (y * w + x) as usize;
            out.pixels[o..o + 4].copy_from_slice(&p);
        }
    }
    out
}

/// The world map's CLUT: CBP 0x2eac (PS2 0x00200320), `menu.clp` at
/// block 0x2ea0, so CLUT 5 (block 0xc of the first page: column 1, row 1).
pub const WORLDMAP_PALETTE: usize = 5;

/// The PS2 world map picture for every menu folder that has the HD one
/// (no text on it: the disc's own picture everywhere).
pub fn world_map(
    disc: &mut Disc,
    pc: &ffx_vbf::Archive,
    palette: impl Fn(&[u8], usize) -> Vec<[u8; 4]>,
    folders: &[&str],
    done: &mut Vec<Done>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let (Some(idx), Some(clp)) = (disc.read("jppc/menu/worldmap.fmt")?, disc.read("jppc/menu/menu.clp")?) else {
        return Ok(());
    };
    if idx.len() < 512 * 416 {
        notes.push("jppc/menu/worldmap.fmt: not 512 x 416, kept HD".to_string());
        return Ok(());
    }
    let pal = palette(&clp, WORLDMAP_PALETTE);
    let mut img = Rgba::new(512, 416);
    for (i, &k) in idx.iter().take(512 * 416).enumerate() {
        img.pixels[4 * i..4 * i + 4].copy_from_slice(&pal[usize::from(k)]);
    }
    for folder in folders {
        let path = format!("{PS3}/{folder}/d3d11/worldmap.dds.phyre");
        let Some((template, hd)) = load_hd(pc, &path)? else { continue };
        let img = maps::opaque_like(img.clone(), &hd);
        done.push(Done {
            file: phyretex::argb8_one_level(&template, &img, None)?,
            preview: side_by_side(&hd, &resize(&img, hd.width, hd.height)),
            note: format!("{path}: the PS2 world map, 512 x 416, CLUT {WORLDMAP_PALETTE} (jppc/menu/worldmap.fmt)"),
            path,
        });
    }
    Ok(())
}
