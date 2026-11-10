//! The PS2 font (differences row 13) for English, French, Italian, German
//! and Spanish. Notes and evidence: `docs/mods/font.md`.
//!
//! Every European language of the PC draws its text (dialogue, menus,
//! battle text, help, damage numbers) with one font: the glyph pages
//! `menu_us/base_ftc/{font,shadow}_0_{0,1}` (texture id `0x3d40`) and the
//! width table of `uspc/menu/base.ftc` (dictionary type 4). The PC kept
//! the PS2 glyph grid in PS2 units: glyph `i` is the 14x18 cell at column
//! `(i % 18) / 2`, row `i / 18` of page `i & 1`, drawn `width / 4` texels
//! wide from the cell's left edge (`TOGetFontInfo`, `0x008b7240`), with
//! UVs over the page size of the FTC header (`TOGetImageWH(0x3d40)`). So
//! the PS2 font is a swap of the four pages (at PS2 size) and of the width
//! table; no code changes. The pages are written at 4x PS2 size (filtered
//! up within each glyph cell: [`page_image`]).
//!
//! The PS2's `<lang>pc/menu/base.ftc` holds the glyphs as one 4-bit page
//! with two 2-bit pages in it (bits 0-1 page 0, bits 2-3 page 1); a 2-bit
//! value is the glyph's coverage (0 none, 3 full). The PC's pages are white
//! with the coverage in alpha (`font`) and black with the same alpha
//! (`shadow`, drawn offset under the face and as the outline of the edge
//! fonts). The 30 glyphs the PC added (`★`, `■`, `Œ`, ...) have no PS2
//! cell: they are the HD glyphs scaled down to PS2 size, with their PC
//! widths.

use anyhow::{Context, Result, ensure};

use crate::disc::Disc;
use crate::formats::phyretex::{self, Rgba};
use crate::jobs::interface::{Done, load_hd, side_by_side};

const PS3: &str = "ffx_data/gamedata/ps3data";
/// The European width table (`TODVDFileLoad(4, 0)` in `TOInit`).
pub const PC_FTC: &str = "ffx_ps2/ffx/master/uspc/menu/base.ftc";
/// The European glyph pages (`TOGetTextureName(0x3d40)`: `0x00c595e8`,
/// `0x00c59628`). `menu_de`, `menu_fr`, ... `base_ftc` are not referenced
/// by the exe.
const PAGES: &str = "menu_us/base_ftc/d3d11";
/// The PC's charset of the European languages (index = code - 0x30), for
/// the preview.
const CHARSET: &str = "ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_us.bin";

const CELL_W: u32 = 14;
const CELL_H: u32 = 18;
/// Glyphs per row of the grid (both pages).
const ROW: usize = 18;

/// An `FTCX` font file (`TOFTCTransfer`, `0x008ac130`).
struct Ftc {
    /// Glyphs (`+4`).
    count: usize,
    /// Dictionary type (`+8`): 4 for the European base font.
    kind: u16,
    /// Page size in PS2 texels (`+0x28`, `+0x2a`).
    page: (u32, u32),
    /// Offset of the width table (`+0x30`), one signed byte per glyph.
    widths_at: usize,
    widths: Vec<i8>,
    /// The 4-bit page (`+0x20`, `+0x24`), empty on the PC.
    bitmap: Vec<u8>,
}

fn parse_ftc(b: &[u8]) -> Result<Ftc> {
    ensure!(b.len() >= 0x40 && &b[..4] == b"FTCX", "not an FTCX file");
    let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes")) as usize;
    let (cw, ch) = (u16_at(0x14), u16_at(0x16));
    ensure!(
        u32::from(cw) == CELL_W && u32::from(ch) == CELL_H,
        "cell {cw}x{ch}, not {CELL_W}x{CELL_H}"
    );
    let (bm_at, bm_len) = (u32_at(0x20), u32_at(0x24));
    let (wt_at, wt_len) = (u32_at(0x30), u32_at(0x10));
    let bitmap = b.get(bm_at..bm_at + bm_len).context("bitmap past the end")?.to_vec();
    let widths = b
        .get(wt_at..wt_at + wt_len)
        .context("width table past the end")?
        .iter()
        .map(|&w| w as i8)
        .collect();
    Ok(Ftc {
        count: usize::from(u16_at(4)),
        kind: u16_at(8),
        page: (u32::from(u16_at(0x28)), u32::from(u16_at(0x2a))),
        widths_at: wt_at,
        widths,
        bitmap,
    })
}

/// Where glyph `i` is: page, and the cell's top left in texels.
fn cell(i: usize) -> (usize, u32, u32) {
    (i & 1, ((i % ROW) / 2) as u32 * CELL_W, (i / ROW) as u32 * CELL_H)
}

/// The PS2 font's coverage (0..3) per page, `page.0` x `page.1` each.
fn ps2_planes(f: &Ftc) -> Result<[Vec<u8>; 2]> {
    let (w, h) = f.page;
    let n = (w * h) as usize;
    ensure!(f.bitmap.len() * 2 >= n, "PS2 bitmap is {} bytes for {w}x{h}", f.bitmap.len());
    let mut planes = [vec![0u8; n], vec![0u8; n]];
    for (i, b) in f.bitmap.iter().enumerate() {
        for (k, t) in [b & 15, b >> 4].into_iter().enumerate() {
            let p = 2 * i + k;
            if p < n {
                planes[0][p] = t & 3;
                planes[1][p] = t >> 2;
            }
        }
    }
    Ok(planes)
}

/// The two European pages and their width table, PS2 glyphs where the PS2
/// font has them. Returns the alpha of each page (`pc.page` size), the new
/// widths and a note.
fn build(ps2: &Ftc, pc: &Ftc, hd: &[Rgba; 2]) -> Result<([Vec<u8>; 2], Vec<i8>, String)> {
    let planes = ps2_planes(ps2)?;
    let (pw, ph) = pc.page;
    let mut alpha = [vec![0u8; (pw * ph) as usize], vec![0u8; (pw * ph) as usize]];
    let mut widths = pc.widths.clone();
    let (mut from_ps2, mut from_hd) = (0, 0);
    for i in 0..pc.widths.len() {
        let (p, x0, y0) = cell(i);
        if x0 + CELL_W > pw || y0 + CELL_H > ph {
            continue;
        }
        let in_ps2 = i < ps2.count.min(ps2.widths.len()) && y0 + CELL_H <= ps2.page.1 && x0 + CELL_W <= ps2.page.0;
        let ps2_at = |x: u32, y: u32| planes[p][((y0 + y) * ps2.page.0 + x0 + x) as usize];
        let ink = in_ps2 && (0..CELL_H).any(|y| (0..CELL_W).any(|x| ps2_at(x, y) != 0));
        if in_ps2 && (ink || ps2.widths[i] > 0) {
            for y in 0..CELL_H {
                for x in 0..CELL_W {
                    alpha[p][((y0 + y) * pw + x0 + x) as usize] = ps2_at(x, y) * 85;
                }
            }
            // PS2 widths are texels, the PC's quarter texels
            widths[i] = ps2.widths[i].saturating_mul(4);
            from_ps2 += 1;
        } else {
            // a glyph the PC added: the HD cell's alpha, box-filtered down
            let img = &hd[p];
            let (sx, sy) = (img.width as f32 / pw as f32, img.height as f32 / ph as f32);
            let mut any = false;
            for y in 0..CELL_H {
                for x in 0..CELL_W {
                    let (fx0, fy0) = ((x0 + x) as f32 * sx, (y0 + y) as f32 * sy);
                    let (ax, ay) = (fx0 as u32, fy0 as u32);
                    let (bx, by) = (((fx0 + sx).ceil() as u32).min(img.width), ((fy0 + sy).ceil() as u32).min(img.height));
                    let (mut sum, mut n) = (0u32, 0u32);
                    for yy in ay..by {
                        for xx in ax..bx {
                            sum += u32::from(img.get(xx, yy)[3]);
                            n += 1;
                        }
                    }
                    let a = (sum / n.max(1)) as u8;
                    any |= a > 0;
                    alpha[p][((y0 + y) * pw + x0 + x) as usize] = a;
                }
            }
            from_hd += usize::from(any);
        }
    }
    let note = format!("{from_ps2} PS2 glyphs, {from_hd} HD glyphs the PS2 font lacks");
    Ok((alpha, widths, note))
}

/// How much larger than PS2 size the pages are written.
const SCALE: u32 = 4;

/// A page `SCALE` times the PS2 size, `rgb` with `alpha` (`w` x `h`)
/// filtered up bilinear inside each glyph cell. The game's bilinear filter
/// on the PS2-size page would blend in the edge texels of the cell above
/// (a `g`'s descender over a `y`); here the filter reaches at most 1/8 of a
/// PS2 texel past a cell, and the glyphs look as soft as at PS2 size.
fn page_image(alpha: &[u8], w: u32, h: u32, rgb: u8) -> Rgba {
    let mut img = Rgba::new(w * SCALE, h * SCALE);
    let at = |x: u32, y: u32| f32::from(alpha[(y * w + x) as usize]);
    for y in 0..h * SCALE {
        let cy = (y / SCALE / CELL_H * CELL_H) as f32;
        let fy = ((y as f32 + 0.5) / SCALE as f32 - 0.5).clamp(cy, (cy + CELL_H as f32 - 1.0).min(h as f32 - 1.0));
        let (y0, ty) = (fy.floor() as u32, fy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..w * SCALE {
            let cx = (x / SCALE / CELL_W * CELL_W) as f32;
            let fx = ((x as f32 + 0.5) / SCALE as f32 - 0.5).clamp(cx, (cx + CELL_W as f32 - 1.0).min(w as f32 - 1.0));
            let (x0, tx) = (fx.floor() as u32, fx.fract());
            let x1 = (x0 + 1).min(w - 1);
            let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
            let bot = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
            let a = (top * (1.0 - ty) + bot * ty).round() as u8;
            img.put(x, y, [rgb, rgb, rgb, a]);
        }
    }
    img
}

/// Draws `text` as the game lays it out (`TOGetFontInfo`,
/// `TOAddFontWidthSpace`: advance = width + 0.8 PS2 px at 1080p), `scale`
/// screen pixels per PS2 texel, white glyphs over a dark blue window with
/// the shadow 1 px down right.
fn render_text(pages: &[Rgba; 2], page: (u32, u32), widths: &[i8], charset: &[char], lines: &[&str], scale: u32) -> Rgba {
    let index = |c: char| charset.iter().position(|&k| k == c);
    let line_h = 20 * scale;
    let width = 120 * scale * 4;
    let mut img = Rgba::new(width, line_h * lines.len() as u32 + 4 * scale);
    for p in img.pixels.chunks_exact_mut(4) {
        p.copy_from_slice(&[24, 32, 88, 255]);
    }
    let blend = |img: &mut Rgba, x: u32, y: u32, c: u8, a: f32| {
        if x < img.width && y < img.height {
            let o = img.get(x, y);
            let m = |k: usize| (f32::from(o[k]) * (1.0 - a) + f32::from(c) * a).round() as u8;
            img.put(x, y, [m(0), m(1), m(2), 255]);
        }
    };
    for (l, text) in lines.iter().enumerate() {
        let mut pen = 4.0f32;
        let top = (l as u32 * line_h + 2 * scale) as f32;
        for c in text.chars() {
            let Some(i) = index(c) else { continue };
            let Some(&w) = widths.get(i) else { continue };
            let w = f32::from(w) / 4.0;
            let (p, x0, y0) = cell(i);
            let src = &pages[p];
            let (sx, sy) = (src.width as f32 / page.0 as f32, src.height as f32 / page.1 as f32);
            let (qw, qh) = ((w * scale as f32) as u32, CELL_H * scale);
            for (dx, dy, rgb) in [(1u32, 1u32, 0u8), (0, 0, 255)] {
                for y in 0..qh {
                    for x in 0..qw {
                        // nearest texel of the glyph's UV rectangle
                        let u = x0 as f32 + (x as f32 + 0.5) / scale as f32;
                        let v = y0 as f32 + (y as f32 + 0.5) / scale as f32;
                        let a = src.get(((u * sx) as u32).min(src.width - 1), ((v * sy) as u32).min(src.height - 1))[3];
                        if a > 0 {
                            let (px, py) = ((pen * scale as f32) as u32 + x + dx * scale / 2, top as u32 + y + dy * scale / 2);
                            blend(&mut img, px, py, rgb, f32::from(a) / 255.0 * if rgb == 0 { 0.7 } else { 1.0 });
                        }
                    }
                }
            }
            pen += w + 0.8;
        }
    }
    img
}

/// The sample lines of the preview: every accent the European scripts use.
const SAMPLE: &[&str] = &[
    "Listen to my story... This may be our last chance.",
    "Gr\u{fc}\u{df}e aus Luca! \u{c4}rger, \u{d6}l, \u{dc}bel: \u{e4}\u{f6}\u{fc} \u{c4}\u{d6}\u{dc} \u{df}",
    "\u{c0} bient\u{f4}t, \u{e9}t\u{e9}, gar\u{e7}on, No\u{eb}l, \u{e8}\u{ea}\u{e0}\u{e2}\u{ee}\u{ef}\u{f9}\u{fb}",
    "\u{bf}Qu\u{e9} pasa? \u{a1}Se\u{f1}or! \u{e1}\u{ed}\u{f3}\u{fa} \u{d1}  Perch\u{e9}? Citt\u{e0}, pi\u{f9}",
    "HP 9999/9999  MP 999  1234567890  -  +  %  ( ) [ ] : ;",
];

/// The font files: the four pages, the width table, previews.
pub fn convert(disc: &mut Disc, pc: &ffx_vbf::Archive) -> Result<(Vec<Done>, Vec<String>)> {
    let mut done = Vec::new();
    let mut notes = Vec::new();
    // `jppc/menu` is the disc's own language; a Japanese disc has a type-0
    // font there and the International one in `uspc/menu`
    let mut ps2 = None;
    for dir in ["jppc/menu", "uspc/menu"] {
        if let Some(b) = disc.read(&format!("{dir}/base.ftc"))? {
            let f = parse_ftc(&b).with_context(|| format!("{dir}/base.ftc"))?;
            if f.kind == 4 && !f.bitmap.is_empty() {
                ps2 = Some(f);
                break;
            }
        }
    }
    let Some(ps2) = ps2 else {
        notes.push("no European base.ftc with a glyph page on the disc: font kept HD".into());
        return Ok((done, notes));
    };
    let pc_bytes = pc.read(pc.find(PC_FTC).context(PC_FTC)?)?;
    let pc_ftc = parse_ftc(&pc_bytes).context(PC_FTC)?;
    ensure!(pc_ftc.kind == 4, "{PC_FTC}: dictionary type {}, not 4", pc_ftc.kind);
    let (pw, ph) = pc_ftc.page;

    let mut hd = Vec::new();
    for p in 0..2 {
        let path = format!("{PS3}/{PAGES}/font_0_{p}.dds.phyre");
        hd.push(load_hd(pc, &path)?.with_context(|| path.clone())?);
    }
    let hd_pages = [hd[0].1.clone(), hd[1].1.clone()];
    let (alpha, widths, note) = build(&ps2, &pc_ftc, &hd_pages)?;

    let mut ours = Vec::new();
    for p in 0..2 {
        let face = page_image(&alpha[p], pw, ph, 255);
        for (kind, rgb) in [("font", 255u8), ("shadow", 0u8)] {
            let path = format!("{PS3}/{PAGES}/{kind}_0_{p}.dds.phyre");
            let Some((template, hd_img)) = load_hd(pc, &path)? else {
                notes.push(format!("{path}: not in the archive"));
                continue;
            };
            let img = page_image(&alpha[p], pw, ph, rgb);
            done.push(Done {
                file: phyretex::argb8_one_level(&template, &img, None)?,
                preview: side_by_side(&hd_img, &img),
                note: format!("{path}: {}x{}, {note}", img.width, img.height),
                path,
            });
        }
        ours.push(face);
    }

    let mut ftc = pc_bytes.clone();
    for (k, &w) in widths.iter().enumerate() {
        ftc[pc_ftc.widths_at + k] = w as u8;
    }
    let changed = widths.iter().zip(&pc_ftc.widths).filter(|(a, b)| a != b).count();
    let charset: Vec<char> = pc
        .find(CHARSET)
        .map(|e| pc.read(e))
        .transpose()?
        .map(|b| String::from_utf8_lossy(&b).chars().collect())
        .unwrap_or_default();
    let ours = [ours[0].clone(), ours[1].clone()];
    let before = render_text(&hd_pages, (pw, ph), &pc_ftc.widths, &charset, SAMPLE, 3);
    let after = render_text(&ours, (pw, ph), &widths, &charset, SAMPLE, 3);
    let mut preview = Rgba::new(before.width, before.height * 2 + 6);
    for y in 0..before.height {
        for x in 0..before.width {
            preview.put(x, y, before.get(x, y));
            preview.put(x, y + before.height + 6, after.get(x, y));
        }
    }
    done.push(Done {
        path: PC_FTC.to_string(),
        file: ftc,
        preview,
        note: format!("{PC_FTC}: {changed} of {} widths from the PS2 font", widths.len()),
    });
    Ok((done, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid() {
        // '0' page 0 at the origin, '1' page 1 at the origin, 'A' (32)
        // page 0 row 1 column 7
        assert_eq!(cell(0), (0, 0, 0));
        assert_eq!(cell(1), (1, 0, 0));
        assert_eq!(cell(32), (0, 7 * 14, 18));
        assert_eq!(cell(229), (1, 6 * 14, 12 * 18));
    }

    #[test]
    fn planes_split() {
        let f = Ftc {
            count: 200,
            kind: 4,
            page: (2, 1),
            widths_at: 0,
            widths: vec![],
            bitmap: vec![0b1110_0111],
        };
        let p = ps2_planes(&f).unwrap();
        assert_eq!(p[0], vec![3, 2]);
        assert_eq!(p[1], vec![1, 3]);
    }
}
