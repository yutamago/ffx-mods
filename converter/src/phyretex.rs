//! Writes a `*.dds.phyre` texture with new pixels, size and format, keeping
//! everything else of the HD texture it replaces (its own path, the asset
//! reference). PS2 textures go in uncompressed (`ARGB8`) at their own size.

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

/// Replaces `template` (an HD `*.dds.phyre`) with `image` as a one-level
/// `ARGB8` texture.
pub fn argb8_from(template: &[u8], image: &Rgba) -> Result<Vec<u8>> {
    argb8_as(template, image, None)
}

/// [`argb8_from`], and with `id = (old, new)` the texture's asset id
/// (`m_id`, its import id such as `PS3Data/chr/pc/c001/tex/c001.dds`)
/// `old` becomes `new`, of the same length: a new texture made from an HD
/// one.
pub fn argb8_as(template: &[u8], image: &Rgba, id: Option<(&str, &str)>) -> Result<Vec<u8>> {
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
    put_u32(obj, COMMON_BASE + M_MIPMAP_COUNT, 0);
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

    // Pixels: B, G, R, A, rows bottom first.
    let mut px = Vec::with_capacity(image.pixels.len());
    for y in (0..h).rev() {
        for x in 0..w {
            let [r, g, b, a] = image.get(x, y);
            px.extend_from_slice(&[b, g, r, a]);
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
    if (t.width, t.height, t.mip_levels, t.format) != (w, h, 1, ffx_phyre::Format::Argb8) {
        bail!(
            "written texture reads back as {}x{} {:?}",
            t.width,
            t.height,
            t.format
        );
    }
    Ok(out)
}
