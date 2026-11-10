//! Writes a `*.dds.phyre` texture with new pixels, size and format, keeping
//! everything else of the HD texture it replaces (its own path, the asset
//! reference). PS2 textures go in uncompressed (`ARGB8`) at their own size,
//! with a mip chain when the HD texture has one ([`mip_chain`]).

use anyhow::{Context, Result, bail, ensure};
use ffx_phyre::edit::ClusterEdit;

/// An image, rows top first, 4 bytes per pixel R, G, B, A (A 255 opaque).
#[derive(Debug, Clone)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[i..i + 4].try_into().expect("4 bytes")
    }
    pub fn put(&mut self, x: u32, y: u32, c: [u8; 4]) {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[i..i + 4].copy_from_slice(&c);
    }
}

fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// `PTextureCommonBase`'s members are at their namespace offsets in the
/// `PTexture2D` object. (ffx-phyre's "end of instance data" leaves out the
/// 0x24-byte list headers, which is why its distances look larger.)
const COMMON_BASE: usize = 0;
/// One less than the number of mip levels.
const M_MIPMAP_COUNT: usize = 0x0c;
const M_MAX_MIP_LEVEL: usize = 0x10;
const M_WIDTH: usize = 0x1c;
const M_HEIGHT: usize = 0x20;
/// Header word 20: the size of the top mip level.
const HEADER_TOP_LEVEL: usize = 4 * 20;

/// Replaces user fixup string `k` of `e` with `text` (NUL added).
fn set_user_string(e: &mut ClusterEdit, k: usize, text: &str) -> Result<()> {
    let [_, len, at] = e.user_records[k];
    let (len, at) = (len as usize, at as usize);
    let mut new = text.as_bytes().to_vec();
    new.push(0);
    let mut data = e.user_data[..at].to_vec();
    data.extend_from_slice(&new);
    data.extend_from_slice(&e.user_data[at + len..]);
    let grow = new.len() as i64 - len as i64;
    for r in &mut e.user_records {
        if r[2] as usize > at {
            r[2] = u32::try_from(i64::from(r[2]) + grow)?;
        }
    }
    e.user_records[k][1] = new.len() as u32;
    e.user_data = data;
    Ok(())
}

/// The next mip level: each texel the box average of its 2 x 2 (2 x 1, 1 x 2)
/// parents. Colour is averaged weighted by alpha, so transparent texels do
/// not bleed their colour in and colour x alpha (what a blend or an additive
/// effect draws, and the effects' brightness-in-alpha) is kept exactly.
fn half(image: &Rgba) -> Rgba {
    let (w, h) = ((image.width / 2).max(1), (image.height / 2).max(1));
    let mut out = Rgba::new(w, h);
    let (fx, fy) = (image.width / w, image.height / h);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for dy in 0..fy {
                for dx in 0..fx {
                    let p = image.get(x * fx + dx, y * fy + dy);
                    let a = u32::from(p[3]);
                    for c in 0..3 {
                        acc[c] += u32::from(p[c]) * a;
                    }
                    acc[3] += a;
                }
            }
            let n = fx * fy;
            let px = if acc[3] == 0 {
                // Fully transparent: the plain average.
                let mut s = [0u32; 3];
                for dy in 0..fy {
                    for dx in 0..fx {
                        let p = image.get(x * fx + dx, y * fy + dy);
                        for c in 0..3 {
                            s[c] += u32::from(p[c]);
                        }
                    }
                }
                [(s[0] + n / 2) / n, (s[1] + n / 2) / n, (s[2] + n / 2) / n, 0]
            } else {
                let a = acc[3];
                [(acc[0] + a / 2) / a, (acc[1] + a / 2) / a, (acc[2] + a / 2) / a, (a + n / 2) / n]
            };
            out.put(x, y, px.map(|v| v.min(255) as u8));
        }
    }
    out
}

/// Alpha-test threshold of the character and map materials (`AlphaThreshold`
/// 0.5 on Tidus, ffx-rust `docs/formats/phyre.md`).
const CUTOUT: u8 = 128;

/// Whether `image` is a cut-out (alpha almost only 0 or 255, some of each):
/// its smaller levels keep the share of texels that pass the alpha test,
/// else hair and leaves thin out and vanish with distance.
fn cutout(image: &Rgba) -> bool {
    let (mut clear, mut solid, mut between) = (0usize, 0usize, 0usize);
    for p in image.pixels.chunks_exact(4) {
        match p[3] {
            0..=15 => clear += 1,
            240..=255 => solid += 1,
            _ => between += 1,
        }
    }
    clear > 0 && solid > 0 && between * 10 < clear + solid + between
}

fn coverage(image: &Rgba, scale: f32) -> f32 {
    let n = image.pixels.len() / 4;
    let pass = image
        .pixels
        .chunks_exact(4)
        .filter(|p| (f32::from(p[3]) * scale).min(255.0) >= f32::from(CUTOUT))
        .count();
    pass as f32 / n as f32
}

/// Scales `level`'s alpha so that as many texels pass the alpha test as in
/// the top level (`target`).
fn keep_coverage(level: &mut Rgba, target: f32) {
    let (mut lo, mut hi) = (0.0f32, 8.0f32);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if coverage(level, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    for p in level.pixels.chunks_exact_mut(4) {
        p[3] = (f32::from(p[3]) * hi).round().min(255.0) as u8;
    }
}

/// The mip chain of `image` down to 1 x 1 (top level first).
pub fn mip_chain(image: &Rgba) -> Vec<Rgba> {
    let cut = cutout(image).then(|| coverage(image, 1.0));
    let mut chain = vec![image.clone()];
    // Each level is made from the previous one before its coverage fix.
    let mut plain = image.clone();
    while plain.width > 1 || plain.height > 1 {
        plain = half(&plain);
        let mut level = plain.clone();
        if let Some(target) = cut {
            keep_coverage(&mut level, target);
        }
        chain.push(level);
    }
    chain
}

/// Replaces `template` (an HD `*.dds.phyre`) with `image` as an `ARGB8`
/// texture: with the full mip chain when the HD texture has mip levels (the
/// game samples them with the material's trilinear or anisotropic filter;
/// one level alone shimmers and looks pixelated when minified), else one
/// level, as the HD texture.
pub fn argb8_from(template: &[u8], image: &Rgba) -> Result<Vec<u8>> {
    argb8_as(template, image, None)
}

/// [`argb8_from`], and with `id = (old, new)` the texture's asset id
/// (`m_id`, its import id such as `PS3Data/chr/pc/c001/tex/c001.dds`)
/// `old` becomes `new`, of the same length: a new texture made from an HD
/// one.
pub fn argb8_as(template: &[u8], image: &Rgba, id: Option<(&str, &str)>) -> Result<Vec<u8>> {
    let hd_mips = ffx_phyre::Texture::parse(template.to_vec()).is_ok_and(|t| t.mip_levels > 1);
    argb8_levels(template, image, id, hd_mips)
}

/// [`argb8_as`] with one level whatever the HD texture has. The game's
/// texture quality drops the top levels of a texture with a chain (2 of
/// them at `TQ_LOW`, the `VQ_LOW` preset; ffx-rust `docs/formats/phyre.md`,
/// "How the game loads a texture"), never of a one-level texture: a PS2
/// texture that must keep its texels at every setting (the characters') is
/// written this way.
pub fn argb8_one_level(template: &[u8], image: &Rgba, id: Option<(&str, &str)>) -> Result<Vec<u8>> {
    argb8_levels(template, image, id, false)
}

fn argb8_levels(template: &[u8], image: &Rgba, id: Option<(&str, &str)>, mips: bool) -> Result<Vec<u8>> {
    let mut e = ClusterEdit::parse(template).context("parsing the HD texture")?;
    let list = e
        .list_of("PTexture2D")
        .context("not a PTexture2D cluster")?;
    let obj = e.lists[list].object_mut(0);
    ensure!(
        obj.len() == 0x74,
        "unexpected PTexture2D size {:#x}",
        obj.len()
    );
    let (w, h) = (image.width, image.height);
    ensure!(
        w > 0 && h > 0 && image.pixels.len() == (w * h * 4) as usize,
        "bad image"
    );
    let chain = if mips { mip_chain(image) } else { vec![image.clone()] };
    let levels = chain.len() as u32;
    put_u32(obj, COMMON_BASE + M_MIPMAP_COUNT, levels - 1);
    put_u32(obj, COMMON_BASE + M_MAX_MIP_LEVEL, w.max(h).ilog2());
    put_u32(obj, COMMON_BASE + M_WIDTH, w);
    put_u32(obj, COMMON_BASE + M_HEIGHT, h);

    // The format is the second user fixup string ("DXT1\0" -> "ARGB8\0").
    let k = e
        .cluster
        .user_fixups
        .iter()
        .position(|u| matches!(u.text.as_str(), "DXT1" | "DXT3" | "DXT5" | "ARGB8" | "L8"))
        .context("no format string")?;
    set_user_string(&mut e, k, "ARGB8")?;

    // Pixels: B, G, R, A, rows bottom first, each level after the other.
    let mut px = Vec::with_capacity(image.pixels.len() * 4 / 3 + 4);
    for level in &chain {
        for y in (0..level.height).rev() {
            for x in 0..level.width {
                let [r, g, b, a] = level.get(x, y);
                px.extend_from_slice(&[b, g, r, a]);
            }
        }
    }
    e.buffers = px;
    let mut out = e.write()?;
    if let Some((old, new)) = id {
        ensure!(old.len() == new.len(), "texture ids of different length");
        let (old, new) = (old.as_bytes(), new.as_bytes());
        let mut found = 0;
        let mut i = 0;
        while i + old.len() <= out.len() {
            if &out[i..i + old.len()] == old {
                out[i..i + old.len()].copy_from_slice(new);
                found += 1;
                i += old.len();
            } else {
                i += 1;
            }
        }
        ensure!(found > 0, "texture id {} not found", String::from_utf8_lossy(old));
    }
    put_u32(&mut out, HEADER_TOP_LEVEL, w * h * 4);

    // The result must read back as what we meant.
    let t = ffx_phyre::Texture::parse(out.clone()).context("re-reading the written texture")?;
    if (t.width, t.height, t.mip_levels, t.format) != (w, h, levels, ffx_phyre::Format::Argb8) {
        bail!(
            "written texture reads back as {}x{} {:?}",
            t.width,
            t.height,
            t.format
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_sizes_and_weighting() {
        let mut img = Rgba::new(8, 4);
        // Left half opaque red, right half transparent green: no green bleeds in.
        for y in 0..4 {
            for x in 0..8 {
                img.put(x, y, if x < 4 { [255, 0, 0, 255] } else { [0, 255, 0, 0] });
            }
        }
        let chain = mip_chain(&img);
        let sizes: Vec<_> = chain.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(sizes, [(8, 4), (4, 2), (2, 1), (1, 1)]);
        assert_eq!(&chain[3].get(0, 0)[..3], &[255, 0, 0]);
    }

    #[test]
    fn cutouts_keep_coverage() {
        // A checker of opaque and clear texels: half pass the alpha test.
        let mut img = Rgba::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                img.put(x, y, [200, 200, 200, if (x + y) % 2 == 0 { 255 } else { 0 }]);
            }
        }
        for level in &mip_chain(&img)[1..4] {
            let c = coverage(level, 1.0);
            assert!(c >= 0.45, "{}x{}: {c}", level.width, level.height);
        }
    }
}
