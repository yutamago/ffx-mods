//! PS2 character archives (`jppc/chr/<kind>/<name>/mdl/<name>.chr`):
//! textures and the triangles with their UVs. Layout:
//! `docs/research/ps2-textures.md` ("`.chr`"), ffx-rust `docs/formats/chara.md`.

use anyhow::{Context, Result, ensure};

fn u8_(b: &[u8], o: usize) -> Result<u8> {
    b.get(o).copied().context("chr: read past the end")
}
fn u16_(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(o..o + 2)
            .context("chr: read past the end")?
            .try_into()?,
    ))
}
fn i16_(b: &[u8], o: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(
        b.get(o..o + 2)
            .context("chr: read past the end")?
            .try_into()?,
    ))
}
fn u32_(b: &[u8], o: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(
        b.get(o..o + 4)
            .context("chr: read past the end")?
            .try_into()?,
    ) as usize)
}

/// An 8-bit image.
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub texels: Vec<u8>,
}

/// One texture-list entry ("material"): an image read through a palette.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub image: usize,
    pub clut: usize,
}

/// One triangle corner.
#[derive(Debug, Clone, Copy)]
pub struct Corner {
    /// Position in the model's frame (the PS2's `i16` units).
    pub pos: [i16; 3],
    /// UV in 1/16 texel (a 256-texel image spans 0..4096).
    pub uv: [u16; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub material: usize,
    pub corners: [Corner; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Chr {
    pub images: Vec<Image>,
    /// 256 RGBA entries each, already in index order (CSM1 undone), alpha
    /// scaled from the GS's 0x80 to 255.
    pub cluts: Vec<[[u8; 4]; 256]>,
    pub materials: Vec<Material>,
    pub triangles: Vec<Triangle>,
    /// Every model's vertex array, as stored.
    pub vertices: Vec<Vec<[i16; 3]>>,
}

/// CSM1 stores a 256-entry palette with index bits 3 and 4 swapped.
fn csm1(i: usize) -> usize {
    (i & 0xe7) | (i & 8) << 1 | (i & 0x10) >> 1
}

fn gs_alpha(a: u8) -> u8 {
    if a >= 0x80 {
        255
    } else {
        ((u32::from(a) * 255 + 64) / 128) as u8
    }
}

impl Chr {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let n = u32_(b, 4)?;
        ensure!((1..64).contains(&n), "chr: {n} entries");
        let entry = |i: usize| u32_(b, 0x10 + 8 * i);
        let mut chr = Self::default();

        let t = if n > 1 { entry(1)? } else { 0 };
        if t != 0 {
            let lists = usize::from(u16_(b, t + 4)?);
            let cluts = usize::from(u16_(b, t + 8)?);
            let images = usize::from(u16_(b, t + 0xc)?);
            let (lp, cp, ip) = (
                t + u32_(b, t + 0x14)?,
                t + u32_(b, t + 0x18)?,
                t + u32_(b, t + 0x1c)?,
            );
            for k in 0..images {
                let e = ip + 16 * k;
                ensure!(u16_(b, e)? == 0x13, "chr: image {k} is not 8-bit");
                let (w, h) = (u32::from(u16_(b, e + 8)?), u32::from(u16_(b, e + 0xa)?));
                let at = t + u32_(b, e + 0xc)?;
                let len = (w * h) as usize;
                let texels = b
                    .get(at..at + len)
                    .context("chr: image data past the end")?
                    .to_vec();
                chr.images.push(Image {
                    width: w,
                    height: h,
                    texels,
                });
            }
            for k in 0..cluts {
                let at = t + u32_(b, cp + 8 * k + 4)?;
                let raw = b.get(at..at + 1024).context("chr: palette past the end")?;
                let mut pal = [[0u8; 4]; 256];
                for (i, c) in pal.iter_mut().enumerate() {
                    let s = &raw[4 * csm1(i)..4 * csm1(i) + 4];
                    *c = [s[0], s[1], s[2], gs_alpha(s[3])];
                }
                chr.cluts.push(pal);
            }
            for k in 0..lists {
                let e = lp + 16 * k;
                chr.materials.push(Material {
                    image: usize::from(u8_(b, e)?),
                    clut: usize::from(u8_(b, e + 1)?),
                });
            }
        }

        let h = entry(0)?;
        if h == 0 {
            return Ok(chr);
        }
        let version = u16_(b, h + 4)?;
        let models = usize::from(u16_(b, h + 6)?);
        let esize = if (0x831..0x1314).contains(&version) {
            24
        } else {
            40
        };
        let mt = h + u32_(b, h + 0x10)?;
        for m in 0..models {
            let e = mt + esize * m;
            let nv = usize::from(u16_(b, e + 4)?);
            let ng = usize::from(u16_(b, e + 6)?);
            let vp = h + u32_(b, e + 8)?;
            let gp = h + u32_(b, e + 0x10)?;
            chr.vertices.push(
                (0..nv)
                    .map(|v| {
                        Ok([
                            i16_(b, vp + 6 * v)?,
                            i16_(b, vp + 6 * v + 2)?,
                            i16_(b, vp + 6 * v + 4)?,
                        ])
                    })
                    .collect::<Result<_>>()?,
            );
            // `None` for an index past the model's vertices (seen in a few
            // files; those triangles are left out).
            let pos = |v: usize| -> Result<Option<[i16; 3]>> {
                if v >= nv {
                    return Ok(None);
                }
                Ok(Some([
                    i16_(b, vp + 6 * v)?,
                    i16_(b, vp + 6 * v + 2)?,
                    i16_(b, vp + 6 * v + 4)?,
                ]))
            };
            for g in 0..ng {
                let ge = gp + 12 * g;
                let (word, material) = (u8_(b, ge)?, usize::from(u8_(b, ge + 1)?));
                let count = usize::from(u16_(b, ge + 4)?);
                let mut q = h + u32_(b, ge + 8)?;
                if word & 1 != 0 {
                    // Packets of strips: (vertex, normal, u, v) per point.
                    for _ in 0..count {
                        let size = usize::from(u8_(b, q)?) << 4;
                        let mut point = q + usize::from(u8_(b, q + 1)?);
                        let strips = usize::from(u8_(b, q + 2)?);
                        for s in 0..strips {
                            let len = usize::from(u8_(b, q + 4 + s)?);
                            let mut c: Vec<Option<Corner>> = Vec::with_capacity(len);
                            for _ in 0..len {
                                let vi = usize::from(u16_(b, point)? & 0x7fff);
                                let uv = [u16_(b, point + 4)?, u16_(b, point + 6)?];
                                c.push(pos(vi)?.map(|pos| Corner { pos, uv }));
                                point += 8;
                            }
                            for k in 2..c.len() {
                                if let (Some(a), Some(b_), Some(c_)) = (c[k - 2], c[k - 1], c[k]) {
                                    chr.triangles.push(Triangle {
                                        material,
                                        corners: [a, b_, c_],
                                    });
                                }
                            }
                        }
                        if size == 0 {
                            break;
                        }
                        q += size;
                    }
                } else {
                    // Plain triangles, 0x18 bytes: (vertex, ?, u, v) per corner.
                    for _ in 0..count / 3 {
                        let mut c = [Corner {
                            pos: [0; 3],
                            uv: [0; 2],
                        }; 3];
                        let mut ok = true;
                        for (k, corner) in c.iter_mut().enumerate() {
                            let w0 = u16_(b, q + 8 * k)?;
                            if w0 & 0x8000 != 0 || usize::from(w0) >= nv {
                                ok = false;
                                continue;
                            }
                            match pos(usize::from(w0))? {
                                Some(pos) => {
                                    *corner = Corner {
                                        pos,
                                        uv: [u16_(b, q + 8 * k + 4)?, u16_(b, q + 8 * k + 6)?],
                                    }
                                }
                                None => ok = false,
                            }
                        }
                        if ok {
                            chr.triangles.push(Triangle {
                                material,
                                corners: c,
                            });
                        }
                        q += 0x18;
                    }
                }
            }
        }
        Ok(chr)
    }

    /// The colour of material `m` at UV (in 1/16 texel, wrapping).
    pub fn sample(&self, m: usize, u: f32, v: f32) -> Option<[u8; 4]> {
        let mat = self.materials.get(m)?;
        let img = self.images.get(mat.image)?;
        let pal = self.cluts.get(mat.clut)?;
        let x = ((u / 16.0).floor() as i64).rem_euclid(i64::from(img.width));
        let y = ((v / 16.0).floor() as i64).rem_euclid(i64::from(img.height));
        Some(pal[usize::from(img.texels[(y * i64::from(img.width) + x) as usize])])
    }
}
